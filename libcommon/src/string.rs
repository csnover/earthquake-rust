use byteorder::ByteOrder;
use crate::encodings::Decoder;
use std::{convert::TryInto, io::{Error, ErrorKind, Read, Result as IoResult}};

fn read_pascal_int<T: Read + ?Sized>(decoder: &dyn Decoder, reader: &mut T, size: usize) -> IoResult<String> {
    let mut result = Vec::with_capacity(size);
    match reader.take(size.try_into().unwrap()).read_to_end(&mut result) {
        Ok(_) => Ok(decoder.decode(&result)),
        Err(e) => Err(Error::new(ErrorKind::InvalidData, e))
    }
}

pub trait ReadExt: Read {
    #[inline]
    fn read_c_str(&mut self, decoder: &dyn Decoder) -> IoResult<String> {
        let mut result = Vec::with_capacity(16);

        for value in self.bytes() {
            match value {
                Ok(0) => return Ok(decoder.decode(&result)),
                Ok(byte) => result.push(byte),
                Err(e) => return Err(e),
            }
        }

        Err(ErrorKind::UnexpectedEof.into())
    }

    #[inline]
    fn read_utf16_c_str<T: ByteOrder>(&mut self) -> IoResult<String> {
        use byteorder::ReadBytesExt;
        let mut result = Vec::with_capacity(16);
        loop {
            let value = self.read_u16::<T>()?;
            if value == 0 {
                break String::from_utf16(&result).map_err(|e| Error::new(ErrorKind::InvalidData, e));
            } else {
                result.push(value);
            }
        }
    }

    #[inline]
    fn read_pascal_str(&mut self, decoder: &dyn Decoder) -> IoResult<String> {
        let mut buf = [ 0; 1 ];
        self.read_exact(&mut buf)?;
        read_pascal_int(decoder, self, buf[0].into())
    }

    #[inline]
    fn read_pascal_16_str<T: ByteOrder, D: Decoder>(&mut self, decoder: &D) -> IoResult<String> {
        let mut buf = [ 0; 2 ];
        self.read_exact(&mut buf)?;
        let size = T::read_u16(&buf);
        read_pascal_int(decoder, self, size.into())
    }
}

impl<T: Read + ?Sized> ReadExt for T {}

#[cfg(test)]
mod test {
    #![allow(clippy::wildcard_imports)]
    use byteorder::{BigEndian, LittleEndian};
    use crate::encodings::WIN_ROMAN;
    use std::io::Cursor;
    use super::*;

    #[test]
    fn read_c_str() {
        let data = b"ok\0x";
        let mut reader = Cursor::new(data);
        assert_eq!(reader.read_c_str(WIN_ROMAN).unwrap(), "ok");
    }

    #[test]
    fn read_pascal_str() {
        let data = b"\x02okx";
        let mut reader = Cursor::new(data);
        assert_eq!(reader.read_pascal_str(WIN_ROMAN).unwrap(), "ok");
    }

    #[test]
    fn read_pascal_16_le_str() {
        let data = b"\x02\x00okx";
        let mut reader = Cursor::new(data);
        assert_eq!(reader.read_pascal_16_str::<LittleEndian, _>(WIN_ROMAN).unwrap(), "ok");
    }

    #[test]
    fn read_pascal_16_be_str() {
        let data = b"\x00\x02okx";
        let mut reader = Cursor::new(data);
        assert_eq!(reader.read_pascal_16_str::<BigEndian, _>(WIN_ROMAN).unwrap(), "ok");
    }
}
