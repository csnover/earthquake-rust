use byteorder::{ByteOrder, BigEndian};
use std::io::{Error, ErrorKind, Result as IoResult};

#[derive(Debug)]
enum Op {
    /* 0 */ SingleShared { offset: u16 },
    /* 01 */ DecompRel { offset: u16, count: u16 },
    /* 011 */ SharedAbs { offset: u16, add_local: bool },
    /* 0111 */ DecompAbs { offset: u16, count: u16, add_local: bool },
    /* 1111 */ Local { count: u16 },
}

fn consume_op(code_stream: &mut &[u8]) -> Op {
    let mut code = consume_u8(code_stream);

    if consume_bit(&mut code) == 0 {
        Op::SingleShared { offset: code * 2 }
    } else if consume_bit(&mut code) == 0 {
        let count = (code & 7) + 1;
        let offset = consume_u8(code_stream) << 3;
        let offset = offset | (code >> 3);
        let offset = offset + 1;
        let offset = offset * 2;
        Op::DecompRel { offset, count }
    } else if consume_bit(&mut code) == 0 {
        let offset = consume_u8(code_stream) << 5;
        let offset = offset | code;
        let offset = offset + 0x80;
        let offset = offset.wrapping_mul(2);
        let add_local = offset & 0x2000 == 0x2000;
        let offset = offset & !0x2000;
        Op::SharedAbs { offset, add_local }
    } else if consume_bit(&mut code) == 0 {
        let count = code + 1;
        let offset = consume_u8(code_stream) << 8;
        let offset = offset | consume_u8(code_stream);
        let add_local = offset & 0x8000 == 0x8000;
        let offset = offset.wrapping_shl(1);
        Op::DecompAbs { offset, count, add_local }
    } else {
        Op::Local { count: code }
    }
}

struct ApplicationVise {
    shared_data: Vec<u8>,
}

fn copy_u16(from: &[u8], to: &mut Vec<u8>) {
    to.push(from[0]);
    to.push(from[1]);
}

fn copy_consume_u16(from: &mut &[u8], to: &mut Vec<u8>) {
    copy_u16(from, to);
    *from = &from[2..];
}

fn consume_bit(data: &mut u16) -> u16 {
    let flag = *data & 1;
    *data >>= 1;
    flag as u16
}

fn consume_u8(data: &mut &[u8]) -> u16 {
    let value = data[0];
    *data = &data[1..];
    value as u16
}

fn consume_u32(data: &mut &[u8]) -> u32 {
    let value = BigEndian::read_u32(&data);
    *data = &data[4..];
    value
}

impl ApplicationVise {
    fn new(shared_data: Vec<u8>) -> ApplicationVise {
        ApplicationVise { shared_data }
    }

    fn is_compressed(data: &[u8]) -> bool {
        &data[0..4] == b"\xa8\x9f\x00\x0c"
    }

    fn validate(data: &[u8]) -> IoResult<()> {
        let expected = BigEndian::read_u32(&data[4..]);

        let mut actual = 0xAAAA_AAAAu32;
        let mut index = 8;
        let size = data.len() - index;
        for _ in 0..size / 4 {
            actual ^= BigEndian::read_u32(&data[index..]);
            index += 4;
        }
        for _ in 0..size & 3 {
            actual ^= data[index] as u32;
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

    fn decompress(&self, data: &[u8]) -> IoResult<Vec<u8>> {
        const USE_SHARED_DICT: u32 = 0x8000_0000;

        ApplicationVise::validate(data)?;

        let decompressed_size = BigEndian::read_u32(&data[8..]) as usize;
        let odd_sized_output = decompressed_size & 1 == 1;
        let mut local_data = &data[16..];

        let (mut code_stream, mut code_count) = {
            let local_data_size = consume_u32(&mut local_data) as usize;
            (
                &data[local_data_size..],
                data.len() - local_data_size - if odd_sized_output { 2 } else { 1 }
            )
        };

        let shared_data = {
            let config = consume_u32(&mut local_data);
            if config & USE_SHARED_DICT == USE_SHARED_DICT {
                let offset = BigEndian::read_u16(&self.shared_data[(1 << (config & 3)) + 6..]);
                &self.shared_data[offset as usize..]
            } else {
                // TODO: This branch is untested. Find a sample!
                &data[config as usize..]
            }
        };

        let mut output = Vec::with_capacity(decompressed_size);

        loop {
            match consume_op(&mut code_stream) {
                Op::SingleShared { offset } => {
                    copy_u16(&shared_data[offset as usize..], &mut output);
                },

                Op::DecompRel { offset, count } => {
                    for _ in 0..(count + 1) * 2 {
                        output.push(output[output.len() - offset as usize]);
                    }
                    code_count = code_count - 1;
                },

                Op::SharedAbs { offset, add_local } => {
                    if add_local {
                        copy_consume_u16(&mut local_data, &mut output);
                    }
                    copy_u16(&shared_data[offset as usize..], &mut output);
                    code_count = code_count - 1;
                },

                Op::DecompAbs { offset, count, add_local } => {
                    if add_local {
                        copy_consume_u16(&mut local_data, &mut output);
                    }
                    for i in 0..(count + 1) * 2 {
                        output.push(output[offset as usize + i as usize]);
                    }
                    code_count = code_count - 2;
                },

                Op::Local { count } => {
                    for _ in 0..=count {
                        copy_consume_u16(&mut local_data, &mut output);
                    }
                },
            }

            if code_count == 0 {
                break;
            }

            code_count = code_count - 1;
        }

        if odd_sized_output {
            output.push(code_stream[0]);
        }

        if output.len() == decompressed_size {
            Ok(output)
        } else {
            Err(Error::new(ErrorKind::UnexpectedEof, format!("Incomplete data (expected {}, got {})", output.len(), decompressed_size)))
        }
    }
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
}
