use byteorder::{ByteOrder, BigEndian};
use std::io::{Error, ErrorKind, Result as IoResult};

/// A decompressor for the Application VISE runtime executable compression
/// format.
struct ApplicationVise {
    shared_data: Vec<u8>,
}

impl ApplicationVise {
    /// Creates a new Application VISE decompressor using the given shared data
    /// dictionary.
    fn new(shared_data: Vec<u8>) -> Self {
        Self { shared_data }
    }

    /// Finds the shared data dictionary in `data`. `data` should contain the
    /// CODE resource of the VISE decompressor within a compressed executable.
    fn find_shared_data(data: &[u8]) -> Option<&[u8]> {
        if data.get(18..22)? != b"VISE" {
            None
        } else if data.get(60..62)? != b"\x47\xfa" {
            None
        } else {
            let offset = BigEndian::read_u16(&data.get(62..)?);
            data.get(62 + offset as usize..)
        }
    }

    /// Determines whether the given data is compressed by Application VISE.
    fn is_compressed(data: &[u8]) -> bool {
        data.len() > 4 && &data[0..4] == b"\xa8\x9f\x00\x0c"
    }

    /// Checks whether the given data is valid according to the embedded
    /// checksum.
    fn validate(data: &[u8]) -> IoResult<()> {
        let expected = BigEndian::read_u32(data.get(4..).ok_or(ErrorKind::UnexpectedEof)?);

        let mut actual = 0xAAAA_AAAAu32;
        let mut index = 8;
        let size = data.len() - index;
        for _ in 0..size / 4 {
            actual ^= BigEndian::read_u32(data.get(index..).ok_or(ErrorKind::UnexpectedEof)?);
            index += 4;
        }
        for _ in 0..size & 3 {
            actual ^= *data.get(index).ok_or(ErrorKind::UnexpectedEof)? as u32;
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

    /// Decompresses the given data.
    fn decompress(&self, data: &[u8]) -> IoResult<Vec<u8>> {
        const USE_SHARED_DICT: u32 = 0x8000_0000;

        Self::validate(data)?;

        let decompressed_size = BigEndian::read_u32(&data[8..]) as usize;
        let odd_sized_output = decompressed_size & 1 == 1;
        let mut local_data = &data[16..];

        let (mut op_stream, mut op_count) = {
            let local_data_size = consume_u32(&mut local_data) as usize;
            (
                &data[local_data_size..],
                data.len() - local_data_size - if odd_sized_output { 2 } else { 1 }
            )
        };

        let shared_data = {
            let config = consume_u32(&mut local_data);
            if config & USE_SHARED_DICT != 0 {
                let offset = BigEndian::read_u16(&self.shared_data[(1 << (config & 3)) + 6..]);
                &self.shared_data[offset as usize..]
            } else {
                // TODO: This branch is untested. Find a sample!
                &data[config as usize..]
            }
        };

        let mut output = Vec::with_capacity(decompressed_size);

        loop {
            match consume_op(&mut op_stream) {
                Op::Shared { offset } => {
                    copy_u16(&shared_data[offset as usize..], &mut output);
                },

                Op::DecompressedEnd { offset, count } => {
                    for _ in 0..(count + 1) * 2 {
                        output.push(output[output.len() - offset as usize]);
                    }
                    op_count = op_count - 1;
                },

                Op::SharedAndLocal { offset, add_local } => {
                    if add_local {
                        copy_consume_u16(&mut local_data, &mut output);
                    }
                    copy_u16(&shared_data[offset as usize..], &mut output);
                    op_count = op_count - 1;
                },

                Op::DecompressedStart { offset, count, add_local } => {
                    if add_local {
                        copy_consume_u16(&mut local_data, &mut output);
                    }
                    for i in 0..(count + 1) * 2 {
                        output.push(output[offset as usize + i as usize]);
                    }
                    op_count = op_count - 2;
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

            op_count = op_count - 1;
        }

        if odd_sized_output {
            output.push(op_stream[0]);
        }

        if output.len() == decompressed_size {
            Ok(output)
        } else {
            Err(Error::new(ErrorKind::UnexpectedEof, format!("Incomplete data (expected {}, got {})", output.len(), decompressed_size)))
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
    flag as u16
}

#[inline]
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
    value as u16
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
    use super::*;

    const SHARED: &'static [u8] = include!("tests/data/compression/shared.in");

    #[test]
    fn validate() {
        const DATA: &'static [u8] = include!("tests/data/compression/data0.in");
        ApplicationVise::validate(&DATA).unwrap();
    }

    #[test]
    fn decompress_empty() {
        const DATA: &'static [u8] = include!("tests/data/compression/data0.in");
        let expected = [0u8; 68].to_vec();
        let vise = ApplicationVise::new(SHARED.to_vec());
        assert_eq!(vise.decompress(&DATA).unwrap(), expected);
    }

    #[test]
    fn decompress_data1() {
        const DATA: &'static [u8] = include!("tests/data/compression/data1.in");
        const EXPECTED: &'static [u8] = include!("tests/data/compression/data1.expected.in");
        let vise = ApplicationVise::new(SHARED.to_vec());
        assert_eq!(vise.decompress(&DATA).unwrap(), EXPECTED);
    }

    #[test]
    fn decompress_data2() {
        const DATA: &'static [u8] = include!("tests/data/compression/data2.in");
        const EXPECTED: &'static [u8] = include!("tests/data/compression/data2.expected.in");
        let vise = ApplicationVise::new(SHARED.to_vec());
        assert_eq!(vise.decompress(&DATA).unwrap(), EXPECTED);
    }

    #[test]
    fn find_shared_data() {
        const DATA: &'static [u8] = include!("tests/data/compression/code.in");
        assert_eq!(ApplicationVise::find_shared_data(&DATA).unwrap(), &DATA[62..]);
    }
}
