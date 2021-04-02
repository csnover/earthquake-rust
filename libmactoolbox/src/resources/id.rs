//! Type definitions for identifying resources.

use binrw::io;
use byteorder::ByteOrder;
use core::{char, fmt};
use derive_more::Display;
use super::Error;

/// A data format identifier.
#[derive(Clone, Copy, Eq, Hash, PartialEq)]
pub struct OsType([u8; 4]);

impl OsType {
    /// Makes a new `OSType`.
    #[must_use]
    pub fn new(os_type: impl Into<[u8; 4]>) -> Self {
        Self(os_type.into())
    }

    #[inline]
    fn fmt_write(self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        // TODO: Find a less incredibly stupid way to do this
        for b in &self.0 {
            write!(f, "{}", char::from_u32((*b).into()).unwrap_or(char::REPLACEMENT_CHARACTER))?;
        }
        Ok(())
    }

    /// Gets the underlying byte view of the `OSType`.
    #[must_use]
    pub fn as_bytes(&self) -> &[u8; 4] {
        &self.0
    }
}

impl core::str::FromStr for OsType {
    type Err = Error;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        if s.len() == 4 {
            let mut value = [ 0; 4 ];
            value.copy_from_slice(s.as_bytes());
            Ok(Self(value))
        } else {
            Err(Error::BadOsTypeSize)
        }
    }
}

impl From<&[u8; 4]> for OsType {
    fn from(value: &[u8; 4]) -> Self {
        Self(*value)
    }
}

impl From<u32> for OsType {
    fn from(value: u32) -> Self {
        Self(value.to_be_bytes())
    }
}

impl Default for OsType {
    fn default() -> Self {
        Self::new([0; 4])
    }
}

impl fmt::Display for OsType {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.fmt_write(f)
    }
}

impl fmt::Debug for OsType {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "OSType(")?;
        self.fmt_write(f)?;
        write!(f, ")")
    }
}

pub trait OsTypeReadExt: io::Read {
    #[inline]
    fn read_os_type<T: ByteOrder>(&mut self) -> io::Result<OsType> {
        let mut buf = [ 0; 4 ];
        self.read_exact(&mut buf)?;
        Ok(T::read_u32(&buf).into())
    }
}

impl<T: io::Read + ?Sized> OsTypeReadExt for T {}

/// A resource identifier.
#[derive(Copy, Clone, Display, Hash, PartialEq, Eq)]
#[display(fmt = "{}({})", _0, _1)]
pub struct ResourceId(OsType, i16);

impl ResourceId {
    /// Makes a new resource identifier for the given data format and number.
    pub fn new(os_type: impl Into<OsType>, id: i16) -> Self {
        Self(os_type.into(), id)
    }

    /// Gets the resource number.
    #[must_use]
    pub fn id(self) -> i16 {
        self.1
    }

    /// Gets the data format identifier.
    #[must_use]
    pub fn os_type(self) -> OsType {
        self.0
    }
}

impl From<(OsType, i16)> for ResourceId {
    fn from(value: (OsType, i16)) -> Self {
        Self(value.0, value.1)
    }
}

impl From<(&[u8; 4], i16)> for ResourceId {
    fn from(value: (&[u8; 4], i16)) -> Self {
        Self(value.0.into(), value.1)
    }
}

impl fmt::Debug for ResourceId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "ResourceId({}({}))", self.0, self.1)
    }
}