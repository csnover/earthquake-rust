use crate::SeekExt;
use std::{
    cell::RefCell,
    convert::TryFrom,
    io::{Error, ErrorKind, Read, Result, Seek, SeekFrom},
    rc::Rc,
};

type Inner<T> = Rc<RefCell<T>>;

pub struct SharedStream<T: Read + Seek + ?Sized> {
    inner: Inner<T>,
    start_pos: u64,
    current_pos: u64,
    end_pos: u64,
}

// TODO: This is hella questionable
impl<T> From<Inner<T>> for SharedStream<T> where T: Read + Seek {
    fn from(input: Inner<T>) -> Self {
        let (start_pos, end_pos) = {
            let mut input = input.borrow_mut();
            (input.pos().unwrap(), input.len().unwrap())
        };

        Self {
            inner: input,
            start_pos,
            current_pos: start_pos,
            end_pos,
        }
    }
}

// TODO: This is hella questionable
impl<T> From<T> for SharedStream<T> where T: Read + Seek {
    fn from(mut input: T) -> Self {
        let start_pos = input.pos().unwrap();
        let end_pos = input.len().unwrap();

        Self {
            inner: Rc::new(RefCell::new(input)),
            start_pos,
            current_pos: start_pos,
            end_pos,
        }
    }
}

impl<T> SharedStream<T> where T: Read + Seek {
    pub fn new(mut input: T) -> Self {
        Self {
            start_pos: 0,
            current_pos: input.pos().unwrap(),
            end_pos: input.len().unwrap(),
            inner: Rc::new(RefCell::new(input)),
        }
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

impl<T> Clone for SharedStream<T> where T: Read + Seek + ?Sized {
    fn clone(&self) -> Self {
        Self {
            inner: self.inner.clone(),
            start_pos: self.start_pos,
            current_pos: self.current_pos,
            end_pos: self.end_pos,
        }
    }
}

impl<T> Read for SharedStream<T> where T: Read + Seek + ?Sized {
    fn read(&mut self, buf: &mut [u8]) -> Result<usize> {
        let mut inner = match self.inner.try_borrow_mut() {
            Ok(inner) => inner,
            Err(err) => return Err(Error::new(ErrorKind::Other, err))
        };
        inner.seek(SeekFrom::Start(self.current_pos))?;
        let n = if self.current_pos + u64::try_from(buf.len()).unwrap() > self.end_pos {
            inner.read(&mut buf[0..usize::try_from(self.end_pos - self.current_pos).unwrap()])?
        } else {
            inner.read(buf)?
        };
        self.current_pos += u64::try_from(n).unwrap();
        Ok(n)
    }
}

impl<T> Seek for SharedStream<T> where T: Read + Seek + ?Sized {
    fn seek(&mut self, pos: SeekFrom) -> Result<u64> {
        let (base_pos, offset) = match pos {
            SeekFrom::Start(n) => (self.start_pos, i64::try_from(n).unwrap()),
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

impl<T> core::fmt::Debug for SharedStream<T> where T: Read + Seek + core::fmt::Debug {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("SharedStream")
            .field("inner", &self.inner)
            .field("start_pos", &self.start_pos)
            .field("current_pos", &self.current_pos)
            .field("end_pos", &self.end_pos)
            .finish()
    }
}

#[cfg(test)]
mod tests {
    #![allow(clippy::wildcard_imports)]
    use super::*;

    #[test]
    fn test_substream() {
        use std::io::Cursor;
        const IN_START: u16 = 2;
        const OUT_START: u16 = 1;
        const IN_SIZE: u16 = 4;
        let mut data = Cursor::new(vec![ 0, 1, 2, 3, 4, 5, 6, 7, 8, 9 ]);
        let mut out = Vec::with_capacity(IN_SIZE.into());
        let mut out2 = Vec::with_capacity(IN_SIZE.into());

        data.seek(SeekFrom::Start(IN_SIZE.into())).unwrap();

        let mut stream = SharedStream::with_bounds(data, IN_START.into(), (IN_START + IN_SIZE).into());
        stream.seek(SeekFrom::Start(OUT_START.into())).unwrap();
        assert_eq!(stream.seek(SeekFrom::Current(0)).unwrap(), OUT_START.into());

        let mut stream2 = stream.clone();
        let size = stream.read_to_end(&mut out).unwrap();
        let size2 = stream2.read_to_end(&mut out2).unwrap();
        assert_eq!(size, (IN_SIZE - OUT_START).into());
        assert_eq!(size, size2);
        assert_eq!(out[0..(IN_SIZE - OUT_START).into()], [ 3, 4, 5 ]);
        assert_eq!(out, out2);
    }
}
