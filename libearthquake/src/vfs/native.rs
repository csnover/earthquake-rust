use anyhow::{anyhow, bail, Result as AResult};
use libcommon::{flatten_errors, SharedStream, vfs::{VirtualFile, VirtualFileSystem}};
use libmactoolbox::{AppleDouble, MacBinary};
use std::{ffi::OsString, fs::{File, metadata}, io, path::{Path, PathBuf}};

pub struct Native;

impl Native {
    pub fn new() -> Self {
        Self
    }
}

impl VirtualFileSystem<File> for Native {
    fn open(&self, path: impl AsRef<Path>) -> AResult<Box<dyn VirtualFile<File>>> {
        if !metadata(&path)?.is_file() {
            bail!("{} is not a file", path.as_ref().display());
        }

        let inner = AppleDouble::open(&path)
            .map(NativeFileInner::AppleDouble)
            .map_err(|e| anyhow!("Not an AppleSingle/AppleDouble file: {}", e))
            .or_else(|e| {
                let r = MacBinary::open(&path)
                    .map(NativeFileInner::MacBinary)
                    .map_err(|e| anyhow!("Not a MacBinary file: {}", e));
                flatten_errors(r, &e)
            })
            .or_else(|e| {
                let resource_fork = open_resource_fork(&path).map(SharedStream::from).ok();
                let data_fork = File::open(&path).map(SharedStream::from).ok();

                if resource_fork.is_some() || data_fork.is_some() {
                    Ok(NativeFileInner::Native { resource_fork, data_fork })
                } else {
                    Err(e)
                }
            })?;

        Ok(Box::new(NativeFile {
            path: path.as_ref().to_path_buf(),
            inner,
        }))
    }
}

#[derive(Debug)]
struct NativeFile {
    path: PathBuf,
    inner: NativeFileInner,
}

impl VirtualFile<File> for NativeFile {
    fn data_fork(&self) -> Option<&SharedStream<File>> {
        match &self.inner {
            NativeFileInner::MacBinary(bin) => bin.data_fork(),
            NativeFileInner::AppleDouble(bin) => bin.data_fork(),
            NativeFileInner::Native { data_fork, .. } => data_fork.as_ref(),
        }
    }

    fn name(&self) -> Option<PathBuf> {
        match &self.inner {
            NativeFileInner::MacBinary(bin) => Some(bin.name().into()),
            NativeFileInner::AppleDouble(bin) => bin.name().map(PathBuf::from),
            NativeFileInner::Native { .. } => self.path.file_name().map(PathBuf::from),
        }
    }

    fn path(&self) -> &Path {
        &self.path
    }

    fn resource_fork(&self) -> Option<&SharedStream<File>> {
        match &self.inner {
            NativeFileInner::MacBinary(bin) => bin.resource_fork(),
            NativeFileInner::AppleDouble(bin) => bin.resource_fork(),
            NativeFileInner::Native { resource_fork, .. } => resource_fork.as_ref(),
        }
    }
}

#[derive(Debug)]
enum NativeFileInner {
    MacBinary(MacBinary<File>),
    AppleDouble(AppleDouble<File>),
    Native {
        resource_fork: Option<SharedStream<File>>,
        data_fork: Option<SharedStream<File>>,
    },
}

fn open_named_fork<T: AsRef<Path>>(path: T) -> io::Result<File> {
    let mut path = path.as_ref().to_path_buf();
    path.push("..namedfork/rsrc");
    let metadata = metadata(&path)?;
    if metadata.len() > 0 {
        File::open(&path)
    } else {
        Err(io::Error::from(io::ErrorKind::NotFound))
    }
}

fn open_resource_fork<T: AsRef<Path>>(filename: T) -> io::Result<File> {
    open_named_fork(filename.as_ref())
        .or_else(|_| File::open({
            let mut path = filename.as_ref().to_path_buf();
            path.set_extension({
                path.extension().map_or_else(|| OsString::from("rsrc"), |ext| {
                    let mut ext = ext.to_os_string();
                    ext.push(".rsrc");
                    ext
                })
            });
            path
        }))
}
