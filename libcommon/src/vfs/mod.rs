use crate::Reader;
use std::path::Path;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ForkKind {
    Data,
    Resource,
}

#[derive(Debug, thiserror::Error)]
#[non_exhaustive]
pub enum Error {
    #[error(transparent)]
    Io(#[from] binrw::io::Error),
    #[error("{0}")]
    Chained(Box<Self>, #[source] Box<Self>),
}

pub trait ResultExt<T> {
    fn chain(self, source: impl Into<Error>) -> Result<T> where Self: Sized;
}

impl <T, E> ResultExt<T> for core::result::Result<T, E> where E: Into<Error> {
    fn chain(self, source: impl Into<Error>) -> Result<T> {
        self.map_err(|e| Error::Chained(Box::new(e.into()), Box::new(source.into())))
    }
}

pub type Result<T> = core::result::Result<T, Error>;

pub trait VirtualFileSystem {
    fn open<'a>(&'a self, path: &dyn AsRef<Path>) -> Result<Box<dyn VirtualFile + 'a>>;
    fn open_resource_fork<'a>(&'a self, path: &dyn AsRef<Path>) -> Result<Box<dyn VirtualFile + 'a>>;
}

pub trait VirtualFile : Reader {
    /// The original name of the file, which may come from internal file
    /// metadata.
    fn name(&self) -> Option<&Path> {
        Some(self.path())
    }

    /// The path of the file on disk.
    fn path(&self) -> &Path;
}
