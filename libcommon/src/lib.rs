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
pub mod resource;
// TODO: use positioned_io crate?
mod shared_stream;
pub mod string;
pub mod vfs;

pub use resource::Resource;
pub use shared_stream::SharedStream;

use anyhow::{anyhow, Context, Error as AError, Result as AResult};
use std::{fmt, io};

pub fn flatten_errors<T>(mut result: AResult<T>, chained_error: &AError) -> AResult<T> {
    for error in chained_error.chain() {
        result = result.context(anyhow!("{}", error));
    }
    result
}

pub trait Reader: io::Read + io::Seek + fmt::Debug {
    fn is_empty(&mut self) -> io::Result<bool> {
        Ok(self.len()? == 0)
    }

    fn len(&mut self) -> io::Result<u64> {
        let pos = self.pos()?;
        let end = self.seek(io::SeekFrom::End(0))?;
        self.seek(io::SeekFrom::Start(pos))?;
        Ok(end)
    }

    fn pos(&mut self) -> io::Result<u64> {
        self.seek(io::SeekFrom::Current(0))
    }

    fn reset(&mut self) -> io::Result<u64> {
        self.seek(io::SeekFrom::Start(0))
    }

    fn skip(&mut self, pos: u64) -> io::Result<u64> {
        self.seek(io::SeekFrom::Current(pos as i64))
    }
}
impl<T: io::Read + io::Seek + ?Sized + fmt::Debug> Reader for T {}

#[derive(Clone, Copy, Debug, Default)]
pub struct UnkHnd(u32);

#[derive(Clone, Copy, Debug, Default)]
pub struct UnkPtr(u32);

#[derive(Clone, Copy, Debug, Default)]
pub struct Unk32(u32);

#[derive(Clone, Copy, Debug, Default)]
pub struct Unk16(u16);

#[derive(Clone, Copy, Debug, Default)]
pub struct Unk8(u8);
