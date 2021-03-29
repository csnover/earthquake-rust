//! Traits for implementing objects that load resources.

use binrw::BinRead;
use std::rc::Rc;
use super::{ResourceId, Result};

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
}
