use byteorder::ByteOrder;
use std::{char, fmt, io};

// TODO: Find a better way to do this. User-defined literals would be nice.
#[macro_export]
macro_rules! os {
    ($os_type:literal) => ($crate::OSType::new(*$os_type));
}

#[derive(Clone, Copy, Eq, Hash, PartialEq)]
pub struct OSType([u8; 4]);

impl OSType {
    #[must_use]
    pub fn new(os_type: [u8; 4]) -> Self {
        Self(os_type)
    }

    #[inline]
    fn fmt_write(self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        // TODO: Find a less incredibly stupid way to do this
        for b in &self.0 {
            write!(f, "{}", char::from_u32(u32::from(*b)).unwrap_or(char::REPLACEMENT_CHARACTER))?;
        }
        Ok(())
    }

    #[must_use]
    pub fn as_bytes(&self) -> &[u8; 4] {
        &self.0
    }
}

impl From<u32> for OSType {
    fn from(value: u32) -> Self {
        Self(value.to_be_bytes())
    }
}

impl Default for OSType {
    fn default() -> Self {
        Self::new([0; 4])
    }
}

impl fmt::Display for OSType {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.fmt_write(f)
    }
}

impl fmt::Debug for OSType {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "OSType(")?;
        self.fmt_write(f)?;
        write!(f, ")")?;
        Ok(())
    }
}

pub trait OSTypeReadExt: io::Read {
    #[inline]
    fn read_os_type<T: ByteOrder>(&mut self) -> io::Result<OSType> {
        let mut buf = [ 0; 4 ];
        self.read_exact(&mut buf)?;
        Ok(OSType::from(T::read_u32(&buf)))
    }
}

impl<T: io::Read + ?Sized> OSTypeReadExt for T {}

#[cfg(test)]
mod tests {
    #![allow(clippy::wildcard_imports)]
    use byteorder::{BigEndian, LittleEndian};
    use std::io::Cursor;
	use super::*;

    #[test]
    fn os_type_macro() {
        let os_type = os!(b"HeLO");
        assert_eq!(os_type, OSType(*b"HeLO"));
    }

    #[test]
    fn os_type_primitive() {
        let os_type = OSType(*b"HeLO");
        assert_eq!(format!("{}", os_type), "HeLO");
        assert_eq!(format!("{:?}", os_type), "OSType(HeLO)");
    }

    #[test]
    fn os_type_read() {
        let mut c = Cursor::new(b"HeLOOLeH");
        assert_eq!(c.read_os_type::<BigEndian>().unwrap(), OSType(*b"HeLO"));
        assert_eq!(c.read_os_type::<LittleEndian>().unwrap(), OSType(*b"HeLO"));
    }

    #[test]
    fn os_type_from_u32() {
        let os_type = 0x48_65_4c_4f;
        assert_eq!(OSType::from(os_type), OSType(*b"HeLO"));
    }
}
