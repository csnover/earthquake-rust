use crate::Reader;
use std::{
    cell::RefCell,
    io::{Error, ErrorKind, Read, Result, Seek, SeekFrom},
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

// TODO: This is hella questionable
impl<T> From<Inner<T>> for SharedStream<T> where T: Reader {
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
impl<T> From<T> for SharedStream<T> where T: Reader {
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

impl<T> SharedStream<T> where T: Reader {
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
