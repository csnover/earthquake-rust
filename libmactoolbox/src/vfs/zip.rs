use anyhow::{anyhow, bail, Result as AResult};
use libcommon::{Reader, SharedStream, vfs::{VirtualFile, VirtualFileSystem}};
use rc_zip::{Archive, EntryReader, ReadZip, EntryContents, StoredEntry};
use std::{convert::TryFrom, fmt, fs::File, io::{Read, Seek, SeekFrom, self}, path::{Path, PathBuf}};
use tempfile::SpooledTempFile;

#[derive(Debug)]
pub struct Zip {
    path: PathBuf,
    archive: Archive,
    stream: SharedStream<File>,
}

impl Zip {
    pub fn new(path: impl AsRef<Path>) -> AResult<Self> {
        let file = File::open(&path)?;
        let archive = file.read_zip()?;
        let stream = SharedStream::new(file);

        Ok(Self {
            path: path.as_ref().to_path_buf(),
            archive,
            stream,
        })
    }

    fn make_file<'a>(&'a self, path: impl AsRef<Path>, entry: &'a StoredEntry) -> AResult<Box<dyn VirtualFile + 'a>> {
        match entry.contents() {
            EntryContents::Directory(_) | EntryContents::Symlink(_) => bail!("Not a file"),
            EntryContents::File(_) => Ok({
                let reader = entry.reader(|offset| self.stream.substream(offset, offset + entry.compressed_size));
                let size = entry.uncompressed_size;
                Box::new(ZipFile::new(self, path, size, reader)) as Box<dyn VirtualFile + 'a>
            })
        }
    }
}

fn make_path(prefix: impl AsRef<Path>, path: impl AsRef<Path>) -> String {
    let mut buf = PathBuf::from(prefix.as_ref());
    buf.push(path.as_ref());
    buf.to_string_lossy().to_string()
}

impl VirtualFileSystem for Zip {
    fn open<'a>(&'a self, path: &dyn AsRef<Path>) -> AResult<Box<dyn VirtualFile + 'a>> {
        // TODO: by_name does not work correctly because (1) case-sensitivity
        // and (2) different character sets in archives
        self.archive
            .by_name(path.as_ref().to_string_lossy())
            .ok_or_else(|| anyhow!("File not found"))
            .and_then(|entry| self.make_file(path, entry))
    }

    fn open_resource_fork<'a>(&'a self, path: &dyn AsRef<Path>) -> AResult<Box<dyn VirtualFile + 'a>> {
        // TODO: by_name does not work correctly because (1) case-sensitivity
        // and (2) different character sets in archives
        self.archive
            .by_name(&make_path("XtraStuf.mac", &path))
            .or_else(|| self.archive.by_name(&make_path("__MACOSX", &path)))
            .ok_or_else(|| anyhow!("File not found"))
            .and_then(|entry| self.make_file(path, entry))
    }
}

struct ZipFile<'a> {
    owner: &'a Zip,
    path: PathBuf,
    size: u64,
    reader: EntryReader<'a, SharedStream<File>>,
    buffer: SpooledTempFile,
    seek: Option<u64>,
}

impl <'a> VirtualFile for ZipFile<'a> {
    fn path(&self) -> &Path {
        &self.path
    }
}

impl <'a> ZipFile<'a> {
    pub fn new(owner: &'a Zip, path: impl AsRef<Path>, size: u64, reader: EntryReader<'a, SharedStream<File>>) -> Self {
        Self {
            owner,
            path: path.as_ref().to_path_buf(),
            reader,
            buffer: SpooledTempFile::new(262_144),
            size,
            seek: None,
        }
    }

    fn fill_buffer_to(&mut self, new_size: u64) -> io::Result<()> {
        let new_size = new_size.min(self.size);
        let cur_size = self.buffer.len()?;
        if new_size > cur_size {
            let cur = self.buffer.pos()?;
            io::copy(&mut self.reader.by_ref().take(new_size - cur_size), &mut self.buffer)?;
            self.buffer.seek(SeekFrom::Start(cur))?;
        }
        Ok(())
    }
}

impl <'a> fmt::Debug for ZipFile<'a> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct(std::any::type_name::<Self>())
            .field("owner", &self.owner)
            .field("path", &self.path)
            .field("reader", &"EntryReader")
            .field("buffer", &self.buffer)
            .field("size", &self.size)
            .field("seek", &self.seek)
            .finish()
    }
}

impl Read for ZipFile<'_> {
    fn read(&mut self, buf: &mut [u8]) -> io::Result<usize> {
        let seeked_pos = if let Some(pos) = self.seek {
            pos
        } else {
            self.buffer.pos()?
        };
        self.fill_buffer_to(seeked_pos + u64::try_from(buf.len()).unwrap())?;
        if self.seek.is_some() {
            self.buffer.seek(SeekFrom::Start(seeked_pos))?;
            self.seek = None;
        }
        self.buffer.read(buf)
    }
}

impl Seek for ZipFile<'_> {
    fn seek(&mut self, style: SeekFrom) -> io::Result<u64> {
        let (base_pos, offset) = match style {
            SeekFrom::Start(n) => {
                return if n < self.buffer.len()? {
                    self.seek = None;
                    self.buffer.seek(style)
                } else {
                    self.seek = Some(n);
                    Ok(n)
                };
            },
            SeekFrom::End(n) => (self.size, n),
            SeekFrom::Current(n) => (self.buffer.pos()?, n),
        };
        let new_pos = if offset >= 0 {
            base_pos.checked_add(offset as u64)
        } else {
            base_pos.checked_sub((offset.wrapping_neg()) as u64)
        };
        match new_pos {
            Some(n) => if n < self.buffer.len()? {
                self.seek = None;
                self.buffer.seek(SeekFrom::Start(n))
            } else {
                self.seek = Some(n);
                Ok(n)
            },
            None => Err(io::Error::new(
                io::ErrorKind::InvalidInput,
                "invalid seek to a negative or overflowing position",
            )),
        }
    }
}

mod tests {
    // TODO
}
