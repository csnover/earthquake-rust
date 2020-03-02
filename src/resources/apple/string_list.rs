use anyhow::Result as AResult;
use byteorder::BigEndian;
use byteordered::{ByteOrdered, StaticEndianness};
use crate::{encodings::DecoderRef, Reader, string::StringReadExt};

pub struct Resource(pub Vec<String>);
impl Resource {
    pub fn parse<T: Reader>(input: &mut ByteOrdered<T, StaticEndianness<BigEndian>>, str_encoding: DecoderRef) -> AResult<Vec<String>> {
        let count = input.read_u16()?;
        let mut strings = Vec::with_capacity(count as usize);
        for _ in 0..count {
            strings.push(input.read_pascal_str(str_encoding)?);
        }
        Ok(strings)
    }
}
