use core::convert::TryInto;
use binrw::io;

/// `SeekExt` provides convenience functions for working with seekable streams.
#[allow(clippy::len_without_is_empty)]
pub trait SeekExt: io::Seek {
    /// The number of bytes remaining in the stream.
    fn bytes_left(&mut self) -> io::Result<u64> {
        let pos = self.pos()?;
        let end = self.seek(io::SeekFrom::End(0))?;
        self.seek(io::SeekFrom::Start(pos))?;
        Ok(end - pos)
    }

    /// The total length of the stream, including bytes already read.
    ///
    /// This is the same as the unstable
    /// [`stream_len()`](std::io::Seek::stream_len).
    fn len(&mut self) -> io::Result<u64> {
        let pos = self.pos()?;
        let end = self.seek(io::SeekFrom::End(0))?;
        self.seek(io::SeekFrom::Start(pos))?;
        Ok(end)
    }

    /// The current position of the stream.
    fn pos(&mut self) -> io::Result<u64> {
        self.seek(io::SeekFrom::Current(0))
    }

    /// Reset the stream position to the beginning.
    fn reset(&mut self) -> io::Result<u64> {
        self.seek(io::SeekFrom::Start(0))
    }

    /// Skips ahead `pos` bytes.
    fn skip(&mut self, pos: u64) -> io::Result<u64> {
        self.seek(io::SeekFrom::Current(pos.try_into().unwrap()))
    }
}
impl<T: io::Seek + ?Sized> SeekExt for T {}
