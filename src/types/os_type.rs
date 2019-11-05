use std::{char, fmt, io};

// TODO: Find a better way to do this. User-defined literals would be nice.
#[macro_export]
macro_rules! os {
    ($os_type:literal) => (OSType::new(*$os_type));
}

#[derive(Clone, Copy, Eq, Hash, PartialEq)]
pub struct OSType([u8; 4]);

impl OSType {
    pub fn new(os_type: [u8; 4]) -> OSType {
        OSType(os_type)
    }

    #[inline]
    fn fmt_write(self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        // TODO: Find a less incredibly stupid way to do this
        for b in self.0.iter() {
            write!(f, "{}", char::from_u32(u32::from(*b)).unwrap_or(char::REPLACEMENT_CHARACTER))?;
        }
        Ok(())
    }

    pub fn as_bytes(&self) -> &[u8; 4] {
        &self.0
    }
}

impl Default for OSType {
    fn default() -> Self {
        OSType::new([0; 4])
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
    fn read_os_type(&mut self) -> io::Result<OSType> {
        let mut buf = [0; 4];
        self.read_exact(&mut buf)?;
        Ok(OSType(buf))
    }

    #[inline]
    fn read_le_os_type(&mut self) -> io::Result<OSType> {
        let mut buf = [0; 4];
        self.read_exact(&mut buf)?;
        buf.reverse();
        Ok(OSType(buf))
    }
}

impl<T: io::Read + ?Sized> OSTypeReadExt for T {}

#[cfg(test)]
mod tests {
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
        let mut c = Cursor::new(b"HeLO");
        assert_eq!(c.read_os_type().unwrap(), OSType(*b"HeLO"));
    }
}
