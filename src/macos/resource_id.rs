use crate::OSType;
use std::fmt;

#[derive(Copy, Clone, Hash, PartialEq, Eq)]
pub struct ResourceId(pub OSType, pub i16);

impl fmt::Display for ResourceId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}({})", self.0, self.1)
    }
}

impl fmt::Debug for ResourceId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "ResourceId({}({}))", self.0, self.1)
    }
}

#[macro_export]
macro_rules! rsid {
    ($os_type:expr, $id:expr) => (ResourceId(OSType::new(*$os_type), $id));
}

#[cfg(test)]
mod tests {
    use crate::os;
    use super::*;

    #[test]
    fn basic() {
        let id = ResourceId(os!(b"HELO"), 123);
        assert_eq!(id.0, OSType::new(*b"HELO"));
        assert_eq!(id.1, 123);
    }

    #[test]
    fn rsid_macro() {
        assert_eq!(ResourceId(os!(b"HELO"), 123), rsid!(b"HELO", 123));
    }
}
