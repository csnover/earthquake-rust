use byteorder::{ByteOrder};
use encoding::types::{DecoderTrap, Encoding};
use std::{io::{Error, ErrorKind, Read, Result as IoResult}};

fn read_pascal_int<T: Read + ?Sized>(decoder: &dyn Encoding, reader: &mut T, size: usize) -> IoResult<String> {
    let mut result = Vec::with_capacity(size);
    match reader.take(size as u64).read_to_end(&mut result) {
        Ok(_) => decoder.decode(&result, DecoderTrap::Replace).or_else(|e| Err(Error::new(ErrorKind::InvalidData, e))),
        Err(e) => Err(Error::new(ErrorKind::InvalidData, e))
    }
}

pub(crate) trait StringReadExt: Read {
    #[inline]
    fn read_c_str<D: Encoding>(&mut self, decoder: &D) -> IoResult<String> {
        let mut result = Vec::with_capacity(16);

        for value in self.bytes() {
            match value {
                Ok(0) => return decoder.decode(&result, DecoderTrap::Replace).or_else(|e| Err(Error::new(ErrorKind::InvalidData, e))),
                Ok(byte) => result.push(byte),
                Err(e) => return Err(e),
            }
        }

        Err(Error::from(ErrorKind::UnexpectedEof))
    }

    #[inline]
    fn read_pascal_str<D: Encoding>(&mut self, decoder: &D) -> IoResult<String> {
        let mut buf = [0; 1];
        self.read_exact(&mut buf)?;
        read_pascal_int(decoder, self, buf[0].into())
    }

    #[inline]
    fn read_pascal_16_str<T: ByteOrder>(&mut self, decoder: &dyn Encoding) -> IoResult<String> {
        let mut buf = [0; 2];
        self.read_exact(&mut buf)?;
        let size = T::read_u16(&buf);
        read_pascal_int(decoder, self, size.into())
    }
}

impl<T: Read + ?Sized> StringReadExt for T {}

#[cfg(test)]
mod tests {
    use byteorder::{BigEndian, LittleEndian};
    use encoding::all::ASCII;
    use std::io::Cursor;
    use super::*;

    #[test]
    fn read_c_str() {
        let data = b"ok\0x";
        let mut reader = Cursor::new(data);
        assert_eq!(reader.read_c_str(ASCII).unwrap(), "ok");
    }

    #[test]
    fn read_pascal_str() {
        let data = b"\x02okx";
        let mut reader = Cursor::new(data);
        assert_eq!(reader.read_pascal_str(ASCII).unwrap(), "ok");
    }

    #[test]
    fn read_pascal_16_le_str() {
        let data = b"\x02\x00okx";
        let mut reader = Cursor::new(data);
        assert_eq!(reader.read_pascal_16_str::<LittleEndian>(ASCII).unwrap(), "ok");
    }

    #[test]
    fn read_pascal_16_be_str() {
        let data = b"\x00\x02okx";
        let mut reader = Cursor::new(data);
        assert_eq!(reader.read_pascal_16_str::<BigEndian>(ASCII).unwrap(), "ok");
    }
}
