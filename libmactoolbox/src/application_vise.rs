use byteorder::{ByteOrder, BigEndian};
use std::{convert::{TryFrom, TryInto}, io::{Error, ErrorKind, Result as IoResult}};

/// A decompressor for the Application VISE runtime executable compression
/// format.
#[derive(Debug)]
pub struct ApplicationVise {
    shared_data: Vec<u8>,
}

impl ApplicationVise {
    /// Creates a new Application VISE decompressor using the given shared data
    /// dictionary.
    #[must_use]
    pub fn new(shared_data: Vec<u8>) -> Self {
        Self { shared_data }
    }

    /// Decompresses the given data.
    pub fn decompress(&self, data: &[u8]) -> IoResult<Vec<u8>> {
        const USE_SHARED_DICT: u32 = 0x8000_0000;

        Self::validate(data)?;

        let decompressed_size = usize::try_from(BigEndian::read_u32(&data[8..])).unwrap();
        let odd_sized_output = decompressed_size & 1 == 1;
        let mut local_data = &data[16..];

        let (mut op_stream, mut op_count) = {
            let local_data_size = usize::try_from(consume_u32(&mut local_data)).unwrap();
            (
                &data[local_data_size..],
                data.len() - local_data_size - if odd_sized_output { 2 } else { 1 }
            )
        };

        let shared_data = {
            let config = consume_u32(&mut local_data);
            if config & USE_SHARED_DICT == 0 {
                // TODO: This branch is untested. Find a sample!
                &data[config.try_into().unwrap()..]
            } else {
                let offset = usize::from(BigEndian::read_u16(&self.shared_data[(1 << (config & 3)) + 6..]));
                &self.shared_data[offset..]
            }
        };

        let mut output = Vec::with_capacity(decompressed_size);

        loop {
            match consume_op(&mut op_stream) {
                Op::Shared { offset } => {
                    copy_u16(&shared_data[offset.into()..], &mut output);
                },

                Op::DecompressedEnd { offset, count } => {
                    for _ in 0..(count + 1) * 2 {
                        output.push(output[output.len() - usize::from(offset)]);
                    }
                    op_count -= 1;
                },

                Op::SharedAndLocal { offset, add_local } => {
                    if add_local {
                        copy_consume_u16(&mut local_data, &mut output);
                    }
                    copy_u16(&shared_data[offset.into()..], &mut output);
                    op_count -= 1;
                },

                Op::DecompressedStart { offset, count, add_local } => {
                    if add_local {
                        copy_consume_u16(&mut local_data, &mut output);
                    }
                    for i in 0..(count + 1) * 2 {
                        output.push(output[usize::from(offset) + usize::from(i)]);
                    }
                    op_count -= 2;
                },

                Op::Local { count } => {
                    for _ in 0..=count {
                        copy_consume_u16(&mut local_data, &mut output);
                    }
                },
            }

            if op_count == 0 {
                break;
            }

            op_count -= 1;
        }

        if odd_sized_output {
            output.push(op_stream[0]);
        }

        if output.len() == decompressed_size {
            Ok(output)
        } else {
            Err(Error::new(ErrorKind::UnexpectedEof, format!("Incomplete data (expected {}, got {})", decompressed_size, output.len())))
        }
    }

    /// Finds the shared data dictionary in `data`. `data` should contain the
    /// CODE resource of the VISE decompressor within a compressed executable.
    #[must_use]
    pub fn find_shared_data(data: &[u8]) -> Option<&[u8]> {
        if data.get(18..22)? != b"VISE" || data.get(60..62)? != b"\x47\xfa" {
            None
        } else {
            let offset = usize::from(BigEndian::read_u16(&data.get(62..)?));
            data.get(62 + offset..)
        }
    }

    /// Determines whether the given data is compressed by Application VISE.
    #[must_use]
    pub fn is_compressed(data: &[u8]) -> bool {
        data.len() >= 4 && &data[0..4] == b"\xa8\x9f\x00\x0c"
    }

    /// Checks whether the given data is valid according to the embedded
    /// checksum.
    pub fn validate(data: &[u8]) -> IoResult<()> {
        let expected = BigEndian::read_u32(data.get(4..).ok_or(ErrorKind::UnexpectedEof)?);

        let mut actual = 0xAAAA_AAAA;
        let mut index = 8;
        let size = data.len() - index;
        for _ in 0..size / 4 {
            actual ^= BigEndian::read_u32(data.get(index..).ok_or(ErrorKind::UnexpectedEof)?);
            index += 4;
        }
        for _ in 0..size & 3 {
            actual ^= u32::from(*data.get(index).ok_or(ErrorKind::UnexpectedEof)?);
            index += 1;
        }

        if expected == actual {
            Ok(())
        } else {
            Err(Error::new(
                ErrorKind::InvalidData,
                format!("Checksum mismatch: 0x{:08x} != 0x{:08x}", actual, expected)
            ))
        }
    }
}

#[derive(Debug)]
enum Op {
    /// Copy one block from the shared dictionary at the given offset.
    Shared { offset: u16 },

    /// Copy `count` blocks from the decompressed data at `offset` bytes back
    /// from the decompression cursor.
    DecompressedEnd { offset: u16, count: u16 },

    /// Consume one block from the local dictionary if `add_local` is set, then
    /// one block from the shared dictionary at the given offset.
    SharedAndLocal { offset: u16, add_local: bool },

    /// Consume one block from the local dictionary if `add_local` is set, then
    /// `count` blocks from the decompressed data at `offset` bytes from the
    /// start of the data.
    DecompressedStart { offset: u16, count: u16, add_local: bool },

    /// Consume `count` blocks from the local dictionary.
    Local { count: u16 },
}

#[inline]
fn consume_bit(data: &mut u16) -> u16 {
    let flag = *data & 1;
    *data >>= 1;
    flag
}

#[inline]
#[allow(clippy::comparison_chain, clippy::same_functions_in_if_condition)] // https://github.com/rust-lang/rust-clippy/issues/5212
fn consume_op(op_stream: &mut &[u8]) -> Op {
    let mut code = consume_u8(op_stream);

    if consume_bit(&mut code) == 0 { // 0
        Op::Shared { offset: code * 2 }
    } else if consume_bit(&mut code) == 0 { // 01
        let count = (code & 7) + 1;
        let offset = consume_u8(op_stream) << 3;
        let offset = offset | (code >> 3);
        let offset = offset + 1;
        let offset = offset * 2;
        Op::DecompressedEnd { offset, count }
    } else if consume_bit(&mut code) == 0 { // 011
        const LOCAL_FLAG: u16 = 0x2000;
        let offset = consume_u8(op_stream) << 5;
        let offset = offset | code;
        let offset = offset + 0x80;
        let offset = offset.wrapping_mul(2);
        let add_local = offset & LOCAL_FLAG != 0;
        let offset = offset & !LOCAL_FLAG;
        Op::SharedAndLocal { offset, add_local }
    } else if consume_bit(&mut code) == 0 { // 0111
        let count = code + 1;
        let offset = consume_u8(op_stream) << 8;
        let offset = offset | consume_u8(op_stream);
        let add_local = offset & 0x8000 != 0;
        let offset = offset.wrapping_shl(1);
        Op::DecompressedStart { offset, count, add_local }
    } else { // 1111
        Op::Local { count: code }
    }
}

#[inline]
fn consume_u8(data: &mut &[u8]) -> u16 {
    let value = data[0];
    *data = &data[1..];
    value.into()
}

#[inline]
fn consume_u32(data: &mut &[u8]) -> u32 {
    let value = BigEndian::read_u32(&data);
    *data = &data[4..];
    value
}

#[inline]
fn copy_consume_u16(from: &mut &[u8], to: &mut Vec<u8>) {
    copy_u16(from, to);
    *from = &from[2..];
}

#[inline]
fn copy_u16(from: &[u8], to: &mut Vec<u8>) {
    to.push(from[0]);
    to.push(from[1]);
}

#[cfg(test)]
mod tests {
    #![allow(clippy::wildcard_imports)]
    use super::*;

    const SHARED: &'_ [u8] = include_bytes!("./tests/data/application_vise/shared.bin");

    #[test]
    fn decompress_data1() {
        const DATA: &'_ [u8] = include_bytes!("./tests/data/application_vise/data1.bin");
        const EXPECTED: &'_ [u8] = include_bytes!("./tests/data/application_vise/data1.expected.bin");
        let vise = ApplicationVise::new(SHARED.to_vec());
        assert_eq!(vise.decompress(&DATA).unwrap(), EXPECTED);
    }

    #[test]
    fn decompress_data2() {
        const DATA: &'_ [u8] = include_bytes!("./tests/data/application_vise/data2.bin");
        const EXPECTED: &'_ [u8] = include_bytes!("./tests/data/application_vise/data2.expected.bin");
        let vise = ApplicationVise::new(SHARED.to_vec());
        assert_eq!(vise.decompress(&DATA).unwrap(), EXPECTED);
    }

    #[test]
    fn decompress_empty() {
        const DATA: &'_ [u8] = include_bytes!("./tests/data/application_vise/data0.bin");
        let expected = [0_u8; 68].to_vec();
        let vise = ApplicationVise::new(SHARED.to_vec());
        assert_eq!(vise.decompress(&DATA).unwrap(), expected);
    }

    #[test]
    fn find_shared_data() {
        const DATA: &'_ [u8] = include_bytes!("./tests/data/application_vise/code.bin");
        assert_eq!(ApplicationVise::find_shared_data(&DATA).unwrap(), &DATA[62..]);
    }

    #[test]
    fn validate() {
        const DATA: &'_ [u8] = include_bytes!("./tests/data/application_vise/data0.bin");
        ApplicationVise::validate(&DATA).unwrap();
    }
}
