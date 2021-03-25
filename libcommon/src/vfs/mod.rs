use anyhow::Result as AResult;
use std::path::Path;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ForkKind {
    Data,
    Resource,
}

pub trait VirtualFileSystem {
    fn open<'a>(&'a self, path: &dyn AsRef<Path>) -> AResult<Box<dyn VirtualFile + 'a>>;
    fn open_resource_fork<'a>(&'a self, path: &dyn AsRef<Path>) -> AResult<Box<dyn VirtualFile + 'a>>;
}

pub trait VirtualFile : binrw::io::Read + binrw::io::Seek + core::fmt::Debug {
    /// The original name of the file, which may come from internal file
    /// metadata.
    fn name(&self) -> Option<&Path> {
        Some(self.path())
    }

    /// The path of the file on disk.
    fn path(&self) -> &Path;
}
