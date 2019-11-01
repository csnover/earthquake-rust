// TODO: Director did not support Unicode, so relying on utf8 validity here will
// explode eventually. Use encoding_rs.

use byteorder::{BigEndian, ByteOrder, LittleEndian};
use std::{io, io::Read, str::{from_utf8, Utf8Error}, string::FromUtf8Error};

type ReadExtResult = io::Result<String>;

fn read_pascal_int<T: Read + ?Sized>(reader: &mut T, size: usize) -> ReadExtResult {
    let mut result = String::with_capacity(size);
    match reader.take(size as u64).read_to_string(&mut result) {
        Ok(_) => Ok(result),
        Err(e) => Err(io::Error::new(io::ErrorKind::InvalidData, e))
    }
}

pub(crate) trait StringReadExt: Read {
    #[inline]
    fn read_c_str(&mut self) -> ReadExtResult {
        let mut result = Vec::new();
        for value in self.bytes() {
            match value {
                Ok(0) => break,
                Ok(byte) => result.push(byte),
                Err(e) => return Err(e),
            }
        }

        String::from_utf8(result).or_else(|e| Err(io::Error::new(io::ErrorKind::InvalidData, e)))
    }

    #[inline]
    fn read_pascal_str(&mut self) -> ReadExtResult {
        let mut buf = [0; 1];
        self.read_exact(&mut buf)?;
        read_pascal_int(self, buf[0].into())
    }

    #[inline]
    fn read_pascal_16_str<T: ByteOrder>(&mut self) -> ReadExtResult {
        let mut buf = [0; 2];
        self.read_exact(&mut buf)?;
        let size = T::read_u16(&buf);
        read_pascal_int(self, size.into())
    }
}

impl<T: Read + ?Sized> StringReadExt for T {}

type StringResult = Result<String, FromUtf8Error>;

pub(crate) trait PascalString {
    fn from_pascal(v: Vec<u8>) -> StringResult {
        let size = v[0] as usize;
        String::from_utf8(v[1..1 + size].to_vec())
    }
}
impl PascalString for String {}

pub(crate) trait PascalStr {
    fn from_pascal(v: &[u8]) -> Result<&str, Utf8Error> {
        let size = v[0] as usize;
        from_utf8(&v[1..1 + size])
    }
}
impl PascalStr for str {}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn str_pascal() {
        let data = b"\x02okx";
        assert_eq!(str::from_pascal(data).unwrap(), "ok");
    }

    #[test]
    fn string_pascal() {
        let data = b"\x02okx".to_vec();
        assert_eq!(String::from_pascal(data).unwrap(), "ok");
    }

    #[test]
    fn read_c_str() {
        let data = b"ok\0x";
        let mut reader = io::Cursor::new(data);
        assert_eq!(reader.read_c_str().unwrap(), "ok");
    }

    #[test]
    fn read_pascal_str() {
        let data = b"\x02okx";
        let mut reader = io::Cursor::new(data);
        assert_eq!(reader.read_pascal_str().unwrap(), "ok");
    }

    #[test]
    fn read_pascal_16_le_str() {
        let data = b"\x02\x00okx";
        let mut reader = io::Cursor::new(data);
        assert_eq!(reader.read_pascal_16_str::<LittleEndian>().unwrap(), "ok");
    }

    #[test]
    fn read_pascal_16_be_str() {
        let data = b"\x00\x02okx";
        let mut reader = io::Cursor::new(data);
        assert_eq!(reader.read_pascal_16_str::<BigEndian>().unwrap(), "ok");
    }
}
