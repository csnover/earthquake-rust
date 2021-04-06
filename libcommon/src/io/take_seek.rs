use binrw::io;
use core::fmt;
use crate::convert::{UnwrapFrom, UnwrapInto};
use super::SeekExt;

// TODO: Lots of redundancy with SharedStream here, the only real difference is
// that this one does has no `start_pos` and does not shove `inner` into a
// RefCell
pub struct TakeSeek<T: io::Read + io::Seek> {
    inner: T,
    pos: u64,
    end: u64,
}

impl <T> io::Read for TakeSeek<T> where T: io::Read + io::Seek {
    fn read(&mut self, buf: &mut [u8]) -> io::Result<usize> {
        let limit = usize::unwrap_from(self.end.saturating_sub(self.pos));

        // Don't call into inner reader at all at EOF because it may still block
        if limit == 0 {
            return Ok(0);
        }

        let max = buf.len().min(limit);
        let n = self.inner.read(&mut buf[0..max])?;
        self.pos += u64::unwrap_from(n);
        Ok(n)
    }
}

impl <T> io::Seek for TakeSeek<T> where T: io::Read + io::Seek {
    fn seek(&mut self, style: io::SeekFrom) -> io::Result<u64> {
        let (base_pos, offset) = match style {
            io::SeekFrom::Start(n) => {
                self.inner.seek(io::SeekFrom::Start(n))?;
                self.pos = n;
                return Ok(n);
            }
            io::SeekFrom::End(n) => (self.end, n),
            io::SeekFrom::Current(n) => (self.pos, n),
        };
        let new_pos = if offset >= 0 {
            base_pos.checked_add(offset.unwrap_into())
        } else {
            base_pos.checked_sub(offset.wrapping_neg().unwrap_into())
        };
        match new_pos {
            Some(n) => {
                self.inner.seek(io::SeekFrom::Start(n))?;
                self.pos = n;
                Ok(n)
            }
            None => Err(io::Error::new(
                io::ErrorKind::InvalidInput,
                "invalid seek to a negative or overflowing position",
            )),
        }
    }
}

impl <T> fmt::Debug for TakeSeek<T> where T: io::Read + io::Seek + fmt::Debug {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("TakeSeek")
            .field("inner", &self.inner)
            .field("pos", &self.pos)
            .field("end", &self.end)
            .finish()
    }
}

#[allow(clippy::module_name_repetitions)]
pub trait TakeSeekExt: io::Read + io::Seek {
    fn take_seek(self, limit: u64) -> TakeSeek<Self> where Self: Sized;
}

impl <T: io::Read + io::Seek> TakeSeekExt for T {
    fn take_seek(mut self, limit: u64) -> TakeSeek<Self> where Self: Sized {
        let pos = self.pos().expect("cannot get position for `take_seek`");
        TakeSeek {
            inner: self,
            pos,
            end: pos + limit,
        }
    }
}
