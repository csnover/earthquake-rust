use std::io::{Read, Seek};

pub(crate) mod compression;
pub mod detect;
pub(crate) mod resources;
pub(crate) mod string;

pub trait Reader: Read + Seek + std::fmt::Debug {}
impl<T: Read + Seek + ?Sized + std::fmt::Debug> Reader for T {}

#[derive(Debug, Clone, Copy, Eq, PartialEq)]
pub enum Endianness {
    Little,
    Big,
    Unknown,
}

#[derive(Clone, Copy, Eq, Hash, PartialEq)]
pub struct OSType([u8; 4]);

// TODO: Find a better way to do this. User-defined literals would be nice.
#[macro_export]
macro_rules! os {
    ($os_type:literal) => (OSType(*$os_type));
}

impl OSType {
    #[inline]
    fn fmt_write(self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        // TODO: Find a less incredibly stupid way to do this
        use std::char;
        for b in self.0.iter() {
            write!(f, "{}", char::from_u32(u32::from(*b)).unwrap_or(char::REPLACEMENT_CHARACTER))?;
        }
        Ok(())
    }

    pub fn as_bytes(&self) -> &[u8; 4] {
        &self.0
    }
}

impl From<u32> for OSType {
    fn from(number: u32) -> Self {
        OSType(number.to_be_bytes())
    }
}

impl std::fmt::Display for OSType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        self.fmt_write(f)
    }
}

impl std::fmt::Debug for OSType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "OSType(")?;
        self.fmt_write(f)?;
        write!(f, ")")?;
        Ok(())
    }
}

pub trait OSTypeReadExt: Read {
    #[inline]
    fn read_os_type(&mut self) -> std::io::Result<OSType> {
        let mut buf = [0; 4];
        self.read_exact(&mut buf)?;
        Ok(OSType(buf))
    }

    #[inline]
    fn read_le_os_type(&mut self) -> std::io::Result<OSType> {
        let mut buf = [0; 4];
        self.read_exact(&mut buf)?;
        buf.reverse();
        Ok(OSType(buf))
    }
}

impl<T: Read + ?Sized> OSTypeReadExt for T {}

#[cfg(test)]
mod tests {
    use std::io::Cursor;
    use super::*;

    #[test]
    fn os_type_macro() {
        let os_type = os!(b"HeLO");
        assert_eq!(os_type, OSType(b"HeLO"));
    }

    #[test]
    fn os_type_primitive() {
        let os_type = OSType(b"HeLO");
        assert_eq!(format!("{}", os_type), "HeLO");
        assert_eq!(format!("{:?}", os_type), "OSType(HeLO)");
    }

    #[test]
    fn os_type_from_u32() {
        let os_type = OSType::from(0x54647461);
        assert_eq!(os_type, OSType(b"Tdta"));
    }

    #[test]
    fn os_type_read() {
        let c = Cursor::new(b"HeLO");
        assert_eq!(c.read_os_type(), OSType(b"HeLO"));
    }
}
