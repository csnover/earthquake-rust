use binrw::io::{Error, ErrorKind, Read, Result, Seek, SeekFrom};
use core::cell::RefCell;
use crate::convert::UnwrapFrom;
use std::rc::Rc;
use super::SeekExt;

pub struct SharedStream<T: Read + Seek + ?Sized> {
    inner: Rc<RefCell<T>>,
    start_pos: u64,
    current_pos: u64,
    end_pos: u64,
}

impl<T> SharedStream<T> where T: Read + Seek {
    /// Consumes this stream, returning the inner stream.
    ///
    /// # Panics
    ///
    /// Panics if the inner stream has more than one strong reference.
    #[must_use]
    pub fn into_inner(self) -> T {
        Rc::try_unwrap(self.inner)
            .map_err(|_| "could not unwrap SharedStream Rc")
            .unwrap()
            .into_inner()
    }
}

impl<T> SharedStream<T> where T: Read + Seek {
    /// Creates a new `SharedStream` from the given input, using the full range
    /// of the input stream and setting the current position to the input’s
    /// current position.
    ///
    /// # Panics
    ///
    /// Panics if the getting the position or length of the input stream fails.
    pub fn new(mut input: T) -> Self {
        let current_pos = input.pos().unwrap();
        let end_pos = input.len().unwrap();

        Self {
            inner: Rc::new(RefCell::new(input)),
            start_pos: 0,
            current_pos,
            end_pos,
        }
    }

    /// Creates a new `SharedStream` from the given input, bounding the new
    /// stream using the current position of the input stream as the start
    /// position.
    ///
    /// # Panics
    ///
    /// Panics if the getting the position or length of the input stream fails.
    pub fn substream_from(mut input: T) -> Self {
        let start_pos = input.pos().unwrap();
        let end_pos = input.len().unwrap();

        Self {
            inner: Rc::new(RefCell::new(input)),
            start_pos,
            current_pos: start_pos,
            end_pos,
        }
    }

    /// Creates a new `SharedStream` from the given input, bounding the new
    /// stream with the given start and end position.
    pub fn with_bounds(input: T, start_pos: u64, end_pos: u64) -> Self {
        Self {
            inner: Rc::new(RefCell::new(input)),
            start_pos,
            current_pos: start_pos,
            end_pos,
        }
    }

    /// Creates a new `SharedStream` from the this stream with the given start
    /// and end positions.
    ///
    /// # Panics
    ///
    /// Panics if the given `end_pos` is beyond the end of this stream.
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
        let limit = usize::unwrap_from(self.end_pos.saturating_sub(self.current_pos));

        // Don't call into inner reader at all at EOF because it may still block
        if limit == 0 {
            return Ok(0);
        }

        let max = buf.len().min(limit);
        let n = inner.read(&mut buf[0..max])?;
        self.current_pos += u64::unwrap_from(n);
        Ok(n)
    }
}

impl<T> Seek for SharedStream<T> where T: Read + Seek + ?Sized {
    fn seek(&mut self, pos: SeekFrom) -> Result<u64> {
        let (base_pos, offset) = match pos {
            SeekFrom::Start(n) => (self.start_pos, i64::unwrap_from(n)),
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
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        f.debug_struct("SharedStream")
            .field("inner", &self.inner)
            .field("start_pos", &self.start_pos)
            .field("current_pos", &self.current_pos)
            .field("end_pos", &self.end_pos)
            .finish()
    }
}
