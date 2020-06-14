// https://github.com/rust-lang/cargo/issues/5034
#![warn(clippy::pedantic)]
#![allow(
    clippy::cast_possible_truncation,
    clippy::cast_possible_wrap,
    clippy::cast_sign_loss,
    clippy::missing_errors_doc,
    clippy::non_ascii_literal,
    clippy::verbose_bit_mask,
)]
#![warn(rust_2018_idioms)]

pub mod encodings;
mod resource;
mod sharedstream;

pub use resource::Resource;
pub use sharedstream::SharedStream;

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
