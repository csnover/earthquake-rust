use crate::OSType;
use derive_more::Display;
use std::fmt;

#[derive(Copy, Clone, Display, Hash, PartialEq, Eq)]
#[display(fmt = "{}({})", _0, _1)]
pub struct ResourceId(OSType, i16);

impl ResourceId {
    pub fn new(os_type: impl Into<OSType>, id: i16) -> Self {
        Self(os_type.into(), id)
    }

    #[must_use]
    pub fn id(self) -> i16 {
        self.1
    }

    #[must_use]
    pub fn os_type(self) -> OSType {
        self.0
    }
}

impl From<(OSType, i16)> for ResourceId {
    fn from(value: (OSType, i16)) -> Self {
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

#[cfg(test)]
mod tests {
    #![allow(clippy::wildcard_imports)]
    use super::*;

    #[test]
    fn basic() {
        println!("{:?}", ResourceId(b"HELO".into(), 123));
        let id = ResourceId(b"HELO".into(), 123);
        assert_eq!(id.0, OSType::new(*b"HELO"));
        assert_eq!(id.1, 123);
    }
}
