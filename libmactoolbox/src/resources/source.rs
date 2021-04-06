//! Traits for implementing objects that load resources.

use binrw::BinRead;
use std::rc::Rc;
use super::{Error, OsType, ResNum, ResourceId, Result};

/// A trait for objects which can specify their own `OsType`.
pub trait TypedResource {
    const OS_TYPES: &'static [OsType];
}

// TODO: Interpolate the OsType list into documentation once
// https://github.com/rust-lang/rust/issues/78835 is stable
#[macro_export]
macro_rules! typed_resource {
    ($ident:ident => $($os_type:literal)+) => {
        impl $crate::resources::TypedResource for $ident {
            const OS_TYPES: &'static [$crate::resources::OsType] = &[
                $($crate::resources::OsType::from_raw(*$os_type)),+
            ];
        }
    };
}

/// A trait for objects which can be used to load resources.
pub trait Source {
    /// Returns whether or not the source contains a given resource.
    fn contains(&self, id: impl Into<ResourceId>) -> bool;

    /// Loads a resource with the given ID.
    fn load<ResType>(&self, id: ResourceId) -> Result<Rc<ResType>>
    where
        ResType: BinRead + 'static,
        ResType::Args: Default + Sized
    {
        self.load_args(id, ResType::Args::default())
    }

    /// Loads a resource with the given ID and arguments.
    ///
    /// If the resource has already been loaded into memory, the arguments will
    /// be ignored and an [`Rc`](std::rc::Rc) to the cached copy of the resource
    /// will be returned.
    fn load_args<ResType: BinRead + 'static>(&self, id: ResourceId, args: ResType::Args) -> Result<Rc<ResType>>;

    /// Loads a resource with the given resource number using the default
    /// `OsType` of the resource.
    fn load_num<ResType>(&self, num: ResNum) -> Result<Rc<ResType>>
    where
        ResType: TypedResource + BinRead + 'static,
        ResType::Args: Default + Sized
    {
        self.load_num_args(num, ResType::Args::default())
    }

    /// Loads a resource with the given resource number and arguments using the
    /// default `OsType` of the resource.
    fn load_num_args<ResType>(&self, num: ResNum, args: ResType::Args) -> Result<Rc<ResType>>
    where
        ResType: TypedResource + BinRead + 'static,
    {
        for &os_type in ResType::OS_TYPES {
            let id = ResourceId::new(os_type, num);
            if self.contains(id) {
                return self.load_args(id, args);
            }
        }

        Err(Error::NotFoundNum(ResType::OS_TYPES, num))
    }
}
