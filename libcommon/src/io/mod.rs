mod seek_ext;
mod shared_stream;
mod take_seek;

pub use seek_ext::SeekExt;
pub use shared_stream::SharedStream;
pub use take_seek::{TakeSeek, TakeSeekExt};
use binrw::io;

pub trait Reader: io::Read + io::Seek + core::fmt::Debug {}
impl <T: io::Read + io::Seek + core::fmt::Debug> Reader for T {}

// TODO: Should be generic for all manual read_options implementations
pub fn restore_on_error<R: io::Read + io::Seek, F: Fn(&mut R, u64) -> binrw::BinResult<T>, T>(reader: &mut R, f: F) -> binrw::BinResult<T> {
    let pos = reader.pos()?;
    f(reader, pos).or_else(|err| {
        reader.seek(io::SeekFrom::Start(pos))?;
        Err(err)
    })
}
