use std::{fmt, io};

pub trait Reader: io::Read + io::Seek + fmt::Debug {
    fn skip(&mut self, pos: u64) -> io::Result<u64> {
        self.seek(io::SeekFrom::Current(pos as i64))
    }

    fn pos(&mut self) -> io::Result<u64> {
        self.seek(io::SeekFrom::Current(0))
    }
}
impl<T: io::Read + io::Seek + ?Sized + fmt::Debug> Reader for T {}
