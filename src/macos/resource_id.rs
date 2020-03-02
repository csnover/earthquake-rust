use crate::OSType;
use derive_more::Display;
use std::fmt;

#[derive(Copy, Clone, Display, Hash, PartialEq, Eq)]
#[display(fmt = "{}({})", _0, _1)]
pub struct ResourceId(pub OSType, pub i16);

impl fmt::Debug for ResourceId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "ResourceId({}({}))", self.0, self.1)
    }
}

#[macro_export]
macro_rules! rsid {
    ($os_type:expr, $id:expr) => ($crate::ResourceId($crate::OSType::new(*$os_type), $id));
}

#[cfg(test)]
mod tests {
    #![allow(clippy::wildcard_imports)]
    use crate::os;
    use super::*;

    #[test]
    fn basic() {
        println!("{:?}", ResourceId(os!(b"HELO"), 123));
        let id = ResourceId(os!(b"HELO"), 123);
        assert_eq!(id.0, OSType::new(*b"HELO"));
        assert_eq!(id.1, 123);
    }

    #[test]
    fn rsid_macro() {
        assert_eq!(ResourceId(os!(b"HELO"), 123), rsid!(b"HELO", 123));
    }
}
