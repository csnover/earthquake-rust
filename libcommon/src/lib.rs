// https://github.com/rust-lang/cargo/issues/5034
#![warn(clippy::pedantic)]
#![allow(
    clippy::cast_possible_truncation,
    clippy::cast_possible_wrap,
    clippy::cast_sign_loss,
    clippy::map_err_ignore,
    clippy::missing_errors_doc,
    clippy::non_ascii_literal,
    clippy::option_if_let_else,
    clippy::verbose_bit_mask,
)]
#![warn(rust_2018_idioms)]

pub mod encodings;
pub mod error;
pub mod resource;
// TODO: use positioned_io crate?
mod shared_stream;
pub mod string;
pub mod vfs;

pub use resource::Resource;
pub use shared_stream::SharedStream;

use anyhow::{anyhow, Context, Error as AError, Result as AResult};
use binread::BinRead;
use std::{convert::TryInto, fmt, io};

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
        self.seek(io::SeekFrom::Current(pos.try_into().unwrap()))
    }
}
impl<T: io::Read + io::Seek + ?Sized + fmt::Debug> Reader for T {}

#[derive(BinRead, Clone, Copy, Debug, Default)]
pub struct UnkHnd(pub u32);

#[derive(BinRead, Clone, Copy, Debug, Default)]
pub struct UnkPtr(pub u32);

#[derive(BinRead, Clone, Copy, Debug, Default)]
pub struct Unk32(pub u32);

#[derive(BinRead, Clone, Copy, Debug, Default)]
pub struct Unk16(pub u16);

#[derive(BinRead, Clone, Copy, Debug, Default)]
pub struct Unk8(pub u8);

#[macro_export]
macro_rules! binread_flags {
    ($name: ident, $size: ty) => {
        impl BinRead for $name {
            type Args = ();

            fn read_options<R: binread::io::Read + binread::io::Seek>(reader: &mut R, options: &binread::ReadOptions, _: Self::Args) -> binread::BinResult<Self> {
                use binread::BinReaderExt;
                let last_pos = reader.seek(SeekFrom::Current(0))?;
                let value = reader.read_type::<$size>(options.endian)?;
                Self::from_bits(value).ok_or_else(|| binread::Error::AssertFail {
                    pos: last_pos.try_into().unwrap(),
                    message: format!(concat!("Invalid ", stringify!($name), " flags 0x{:x}"), value),
                })
            }
        }
    }
}

#[macro_export]
macro_rules! binread_enum {
    ($name: ident, $size: ty) => {
        impl BinRead for $name {
            type Args = ();

            fn read_options<R: binread::io::Read + binread::io::Seek>(reader: &mut R, options: &binread::ReadOptions, _: Self::Args) -> binread::BinResult<Self> {
                use binread::BinReaderExt;
                use paste::paste;
                let last_pos = reader.seek(SeekFrom::Current(0))?;
                let value = reader.read_type::<$size>(options.endian)?;
                paste! {
                    Self::[<from_ $size>](value).ok_or_else(|| binread::Error::AssertFail {
                        pos: last_pos.try_into().unwrap(),
                        message: format!(concat!("Invalid ", stringify!($name), " value 0x{:x}"), value),
                    })
                }
            }
        }
    }
}
