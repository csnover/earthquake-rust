use anyhow::Result as AResult;
use crate::{Reader, SharedStream};
use std::{fs::File, path::{Path, PathBuf}};

pub trait VirtualFileSystem<T: Reader> {
    fn open(&self, path: impl AsRef<Path>) -> AResult<Box<dyn VirtualFile<T>>>;
}

pub trait VirtualFile<T: Reader> {
    fn data_fork(&self) -> Option<&SharedStream<T>>;
    fn name(&self) -> Option<PathBuf>;
    fn path(&self) -> &Path;
    fn resource_fork(&self) -> Option<&SharedStream<T>>;
}
