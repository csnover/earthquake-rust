use anyhow::{anyhow, bail, Result as AResult};
use crate::{AppleDouble, MacBinary};
use libcommon::{flatten_errors, SharedStream, vfs::{ForkKind, VirtualFile, VirtualFileSystem}};
use std::{ffi::OsString, fs::{File, metadata}, io::{prelude::*, self}, path::{Path, PathBuf}};

#[derive(Default)]
pub struct HostFileSystem;

impl HostFileSystem {
    #[must_use]
    pub fn new() -> Self {
        Self
    }
}

impl HostFileSystem {
    fn open_impl(path: impl AsRef<Path>, kind: ForkKind) -> AResult<Box<dyn VirtualFile>> {
        let (name, inner) = Self::try_apple_double(&path, kind)
            .or_else(|e| flatten_errors(Self::try_mac_binary(&path, kind), &e))
            .or_else(|e| flatten_errors(Self::try_raw_files(&path, kind), &e))?;

        match inner {
            Some(inner) => Ok(Box::new(HostFile {
                name,
                path: path.as_ref().to_path_buf(),
                inner,
            })),
            None => bail!("No {} fork", match kind {
                ForkKind::Data => "data",
                ForkKind::Resource => "resource"
            })
        }
    }

    fn try_apple_double(path: impl AsRef<Path>, kind: ForkKind) -> AResult<(Option<PathBuf>, Option<SharedStream<File>>)> {
        AppleDouble::new(File::open(&path)?, open_apple_double(&path).ok())
            .map(|f| (
                f.name().map(PathBuf::from),
                match kind {
                    ForkKind::Data => f.data_fork(),
                    ForkKind::Resource => f.resource_fork(),
                }.cloned()
            ))
            .map_err(|e| anyhow!("Not an AppleSingle/AppleDouble file: {}", e))
    }

    fn try_mac_binary(path: impl AsRef<Path>, kind: ForkKind) -> AResult<(Option<PathBuf>, Option<SharedStream<File>>)> {
        let file = File::open(&path)
            .or_else(|_| open_file_with_ext(&path, "bin"))?;

        MacBinary::new(file)
            .map(|f| (
                Some(f.name().into()),
                match kind {
                    ForkKind::Data => f.data_fork(),
                    ForkKind::Resource => f.resource_fork(),
                }.cloned()
            ))
            .map_err(|e| anyhow!("Not a MacBinary file: {}", e))
    }

    fn try_raw_files(path: impl AsRef<Path>, kind: ForkKind) -> AResult<(Option<PathBuf>, Option<SharedStream<File>>)> {
        Ok((
            None::<PathBuf>,
            Some(SharedStream::from(match kind {
                ForkKind::Data => File::open(&path)?,
                ForkKind::Resource => open_resource_fork(&path)?,
            }))
        ))
    }
}

impl VirtualFileSystem for HostFileSystem {
    fn open<'a>(&'a self, path: &dyn AsRef<Path>) -> AResult<Box<dyn VirtualFile + 'a>> {
        Self::open_impl(&path, ForkKind::Data)
    }

    fn open_resource_fork<'a>(&'a self, path: &dyn AsRef<Path>) -> AResult<Box<dyn VirtualFile + 'a>> {
        Self::open_impl(&path, ForkKind::Resource)
    }
}

#[derive(Debug)]
struct HostFile {
    name: Option<PathBuf>,
    path: PathBuf,
    inner: SharedStream<File>,
}

impl VirtualFile for HostFile {
    fn name(&self) -> Option<&Path> {
        self.name.as_deref()
    }

    fn path(&self) -> &Path {
        &self.path
    }
}

impl Read for HostFile {
    fn read(&mut self, buf: &mut [u8]) -> io::Result<usize> {
        self.inner.read(buf)
    }
}

impl Seek for HostFile {
    fn seek(&mut self, pos: io::SeekFrom) -> io::Result<u64> {
        self.inner.seek(pos)
    }
}

fn open_apple_double(path: impl AsRef<Path>) -> io::Result<File> {
    let mut path = PathBuf::from(path.as_ref());
    path.set_file_name({
        let mut file_name = OsString::from("._");
        file_name.push(path.file_name().unwrap());
        file_name
    });
    File::open(path)
}

fn open_file_with_ext(path: impl AsRef<Path>, new_ext: impl AsRef<Path>) -> io::Result<File> {
    let mut path = path.as_ref().to_path_buf();
    path.set_extension({
        path.extension().map_or_else(|| OsString::from(new_ext.as_ref()), |ext| {
            let mut ext = ext.to_os_string();
            ext.push(".");
            ext.push(new_ext.as_ref());
            ext
        })
    });
    File::open(path)
}

fn open_named_fork<T: AsRef<Path>>(path: T) -> io::Result<File> {
    let mut path = path.as_ref().to_path_buf();
    path.push("..namedfork/rsrc");
    let metadata = metadata(&path)?;
    if metadata.len() > 0 {
        File::open(&path)
    } else {
        Err(io::ErrorKind::NotFound.into())
    }
}

fn open_resource_fork(path: impl AsRef<Path>) -> io::Result<File> {
    open_named_fork(path.as_ref())
        .or_else(|_| open_file_with_ext(path, "rsrc"))
}

mod tests {
    // TODO
}
