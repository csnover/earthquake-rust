use crate::Reader;
use derive_more::Deref;
use std::{
    cell::RefCell,
    fs::File,
    io::{BufReader, Error, ErrorKind, Read, Result, Seek, SeekFrom},
    path::{Path, PathBuf},
    rc::Rc,
};

type Inner<T> = Rc<RefCell<T>>;

#[derive(Debug)]
pub struct SharedStream<T: Reader + ?Sized> {
    inner: Inner<T>,
    start_pos: u64,
    current_pos: u64,
    end_pos: u64,
}

impl<T> From<Inner<T>> for SharedStream<T> where T: Reader {
    fn from(input: Inner<T>) -> Self {
        let (start_pos, end_pos) = input_bounds(&mut *input.borrow_mut()).unwrap();

        Self {
            inner: input,
            start_pos,
            current_pos: start_pos,
            end_pos,
        }
    }
}

impl<T> From<T> for SharedStream<T> where T: Reader {
    fn from(mut input: T) -> Self {
        let (start_pos, end_pos) = input_bounds(&mut input).unwrap();

        Self {
            inner: Rc::new(RefCell::new(input)),
            start_pos,
            current_pos: start_pos,
            end_pos,
        }
    }
}

impl<T> SharedStream<T> where T: Reader {
    pub fn new(input: T) -> Self {
        Self::from(input)
    }

    pub fn with_bounds(input: T, start_pos: u64, end_pos: u64) -> Self {
        Self {
            inner: Rc::new(RefCell::new(input)),
            start_pos,
            current_pos: start_pos,
            end_pos,
        }
    }

    #[must_use]
    pub fn substream(&self, start_pos: u64, end_pos: u64) -> Self {
        assert!(end_pos <= self.end_pos);
        Self {
            inner: self.inner.clone(),
            start_pos: start_pos + self.start_pos,
            current_pos: start_pos + self.start_pos,
            end_pos,
        }
    }
}

impl<T> Clone for SharedStream<T> where T: Reader {
    fn clone(&self) -> Self {
        Self {
            inner: self.inner.clone(),
            start_pos: self.start_pos,
            current_pos: self.current_pos,
            end_pos: self.end_pos,
        }
    }
}

impl<T> Read for SharedStream<T> where T: Reader {
    fn read(&mut self, buf: &mut [u8]) -> Result<usize> {
        let mut inner = match self.inner.try_borrow_mut() {
            Ok(inner) => inner,
            Err(err) => return Err(Error::new(ErrorKind::Other, err))
        };
        inner.seek(SeekFrom::Start(self.current_pos))?;
        let n = if self.current_pos + buf.len() as u64 > self.end_pos {
            inner.read(&mut buf[0..(self.end_pos - self.current_pos) as usize])?
        } else {
            inner.read(buf)?
        };
        self.current_pos += n as u64;
        Ok(n)
    }
}

impl<T> Seek for SharedStream<T> where T: Reader {
    fn seek(&mut self, pos: SeekFrom) -> Result<u64> {
        let (base_pos, offset) = match pos {
            SeekFrom::Start(n) => (self.start_pos, n as i64),
            SeekFrom::End(n) => (self.end_pos, n),
            SeekFrom::Current(n) => (self.current_pos, n),
        };
        let new_pos = if offset >= 0 {
            base_pos.checked_add(offset as u64)
        } else {
            base_pos.checked_sub((offset.wrapping_neg()) as u64)
        };
        match new_pos {
            Some(n) if n >= self.start_pos && n <= self.end_pos => {
                self.current_pos = n;
                Ok(n - self.start_pos)
            },
            _ => Err(Error::new(ErrorKind::InvalidInput, "invalid seek to a negative or overflowing position"))
        }
    }
}

fn input_bounds<T>(input: &mut T) -> Result<(u64, u64)> where T: Reader {
    let start_pos = input.pos()?;
    let end_pos = input.seek(SeekFrom::End(0))?;
    input.seek(SeekFrom::Start(start_pos))?;
    Ok((start_pos, end_pos))
}

#[derive(Clone, Deref)]
pub struct SharedFile {
    #[deref]
    inner: SharedStream<BufReader<File>>,
    path: PathBuf,
}

impl SharedFile {
    pub fn new(file: File, path: impl AsRef<Path>) -> Self {
        Self {
            inner: SharedStream::new(BufReader::new(file)),
            path: path.as_ref().into(),
        }
    }

    pub fn open(path: impl AsRef<Path>) -> Result<Self> {
        Ok(Self::new(File::open(&path)?, path))
    }
}

#[cfg(test)]
mod tests {
    #![allow(clippy::wildcard_imports)]
    use super::*;

    #[test]
    fn test_substream() {
        use std::io::Cursor;
        const IN_START: usize = 2;
        const OUT_START: usize = 1;
        const IN_SIZE: usize = 4;
        let mut data = Cursor::new(vec![ 0, 1, 2, 3, 4, 5, 6, 7, 8, 9 ]);
        let mut out = Vec::with_capacity(IN_SIZE);
        let mut out2 = Vec::with_capacity(IN_SIZE);

        data.seek(SeekFrom::Start(IN_SIZE as u64)).unwrap();

        let mut stream = SharedStream::with_bounds(data, IN_START as u64, IN_START as u64 + IN_SIZE as u64);
        stream.seek(SeekFrom::Start(OUT_START as u64)).unwrap();
        assert_eq!(stream.seek(SeekFrom::Current(0)).unwrap(), OUT_START as u64);

        let mut stream2 = stream.clone();
        let size = stream.read_to_end(&mut out).unwrap();
        let size2 = stream2.read_to_end(&mut out2).unwrap();
        assert_eq!(size, IN_SIZE - OUT_START);
        assert_eq!(size, size2);
        assert_eq!(out[0..IN_SIZE - OUT_START], [ 3, 4, 5 ]);
        assert_eq!(out, out2);
    }
}
