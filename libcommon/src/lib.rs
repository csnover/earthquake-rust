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

#[macro_use]
pub mod bitflags;
pub mod encodings;
pub mod error;
// TODO: use positioned_io crate?
mod shared_stream;
pub mod string;
pub mod vfs;

pub use shared_stream::SharedStream;

use anyhow::{anyhow, Context, Error as AError, Result as AResult};
use binrw::BinRead;
use core::{cmp, convert::{TryFrom, TryInto}};
use std::io;

pub fn flatten_errors<T>(mut result: AResult<T>, chained_error: &AError) -> AResult<T> {
    for error in chained_error.chain() {
        result = result.context(anyhow!("{}", error));
    }
    result
}

// TODO: Should be generic for all manual read_options implementations
pub fn restore_on_error<R: binrw::io::Read + binrw::io::Seek, F: Fn(&mut R, u64) -> binrw::BinResult<T>, T>(reader: &mut R, f: F) -> binrw::BinResult<T> {
    let pos = reader.pos()?;
    f(reader, pos).or_else(|err| {
        reader.seek(binrw::io::SeekFrom::Start(pos))?;
        Err(err)
    })
}

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
        let limit = usize::try_from(self.end.saturating_sub(self.pos)).unwrap();

        // Don't call into inner reader at all at EOF because it may still block
        if limit == 0 {
            return Ok(0);
        }

        let max = cmp::min(buf.len(), limit);
        let n = self.inner.read(&mut buf[0..max])?;
        self.pos += u64::try_from(n).unwrap();
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
            base_pos.checked_add(<_>::try_from(offset).unwrap())
        } else {
            base_pos.checked_sub(<_>::try_from(offset.wrapping_neg()).unwrap())
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

impl <T> core::fmt::Debug for TakeSeek<T> where T: io::Read + io::Seek + core::fmt::Debug {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        f.debug_struct("TakeSeek")
            .field("inner", &self.inner)
            .field("pos", &self.pos)
            .field("end", &self.end)
            .finish()
    }
}

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
        Ok(end - pos)
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

// TODO:
// All of this newtype stuff seems like it absolutely should be easier to do.
// 1. Sending types like `__newtype_num_impl!($ident, $ty)` does not work
//    because the type will not match the `$ty` token so every type must be
//    explicitly enumerated in the main `newtype_num` pattern.
// 2. Using generics for the conversion impls does not work because
//    specialization is not stable as of rustc 1.48.0, and core includes a
//    generic `impl <T> From<T> for T`, which conflicts with
//    `impl <T: std::convert::From<$ty>> From<T> for $ident`.
// 3. `derive_more::From` + `#[from(forward)]` (as of 0.99.11) does not work
//    correctly with `TryInto` for some reason.

#[doc(hidden)]
#[macro_export]
macro_rules! __newtype_num_from {
    ($ident:ident, $($from_ty:ty)+) => {
        $(impl ::std::convert::From<$from_ty> for $ident {
            fn from(value: $from_ty) -> Self {
                Self(value.into())
            }
        })+
    }
}

#[doc(hidden)]
#[macro_export]
macro_rules! __newtype_num_into {
    ($ident:ident, $($into_ty:ty)+) => {
        $(impl ::std::convert::From<$ident> for $into_ty {
            fn from(value: $ident) -> Self {
                <$into_ty>::from(value.0)
            }
        })+
    }
}

#[doc(hidden)]
#[macro_export]
macro_rules! __newtype_num_try_from {
    ($ident:ident, $ty:ty, $($try_ty:ty)+) => {
        $(impl ::std::convert::TryFrom<$try_ty> for $ident {
            type Error = <$ty as ::std::convert::TryFrom<$try_ty>>::Error;
            fn try_from(value: $try_ty) -> ::std::result::Result<Self, Self::Error> {
                Ok(Self(::std::convert::TryFrom::try_from(value)?))
            }
        })+
    }
}

#[doc(hidden)]
#[macro_export]
macro_rules! __newtype_num_decl {
    ([$($meta:meta),*], $vis:vis, $ident:ident, $ty_vis:vis, $ty:ty) => {
        #[derive(
            ::std::clone::Clone,
            ::std::marker::Copy,
            ::std::default::Default,
            ::std::cmp::Eq, ::std::cmp::Ord, ::std::cmp::PartialEq, ::std::cmp::PartialOrd,
            ::derive_more::Display,
            ::derive_more::Binary, ::derive_more::Octal,
            ::derive_more::LowerHex, ::derive_more::UpperHex,
            ::derive_more::Add, ::derive_more::Sub,
            ::derive_more::BitAnd, ::derive_more::BitOr, ::derive_more::BitXor,
            ::derive_more::Mul, ::derive_more::Div, ::derive_more::Rem,
            ::derive_more::Shr, ::derive_more::Shl,
            ::derive_more::AddAssign, ::derive_more::SubAssign,
            ::derive_more::BitAndAssign, ::derive_more::BitOrAssign, ::derive_more::BitXorAssign,
            ::derive_more::MulAssign, ::derive_more::DivAssign, ::derive_more::RemAssign,
            ::derive_more::ShrAssign, ::derive_more::ShlAssign
        )]
        $(#[$meta])*
        #[mul(forward)]
        $vis struct $ident($ty_vis $ty);
    }
}

#[doc(hidden)]
#[macro_export]
macro_rules! __newtype_num_impl {
    ($ident:ident, i8) => {
        $crate::__newtype_num_from!($ident, i8);
        $crate::__newtype_num_into!($ident, i8 i16 i32 i64 i128);
        $crate::__newtype_num_try_from!($ident, i8, u8 i16 u16 i32 u32 i64 u64 i128 u128);
    };
    ($ident:ident, u8) => {
        $crate::__newtype_num_from!($ident, u8);
        $crate::__newtype_num_into!($ident, u8 i16 u16 i32 u32 i64 u64 i128 u128);
        $crate::__newtype_num_try_from!($ident, u8, i8 i16 u16 i32 u32 i64 u64 i128 u128);
    };
    ($ident:ident, i16) => {
        $crate::__newtype_num_from!($ident, i8 u8 i16);
        $crate::__newtype_num_into!($ident, i16 i32 i64 i128);
        $crate::__newtype_num_try_from!($ident, i16, u16 i32 u32 i64 u64 i128 u128);
    };
    ($ident:ident, u16) => {
        $crate::__newtype_num_from!($ident, u8 u16);
        $crate::__newtype_num_into!($ident, u16 u32 u64 u128);
        $crate::__newtype_num_try_from!($ident, u16, i8 i16 i32 u32 i64 u64 i128 u128);
    };
    ($ident:ident, i32) => {
        $crate::__newtype_num_from!($ident, i8 u8 i16 u16 i32);
        $crate::__newtype_num_into!($ident, i32 i64 i128);
        $crate::__newtype_num_try_from!($ident, i32, u32 i64 u64 i128 u128);
    };
    ($ident:ident, u32) => {
        $crate::__newtype_num_from!($ident, u8 u16 u32);
        $crate::__newtype_num_into!($ident, u32 u64 i128 u128);
        $crate::__newtype_num_try_from!($ident, u32, i8 i16 i32 i64 u64 i128 u128);
    };
    ($ident:ident, i64) => {
        $crate::__newtype_num_from!($ident, i8 u8 i16 u16 i32 u32 i64);
        $crate::__newtype_num_into!($ident, i64 i128);
        $crate::__newtype_num_try_from!($ident, i64, u64 i128 u128);
    };
    ($ident:ident, u64) => {
        $crate::__newtype_num_from!($ident, u8 u16 u32 u64);
        $crate::__newtype_num_into!($ident, u64 u128);
        $crate::__newtype_num_try_from!($ident, u64, i8 i16 i32 i64 i128 u128);
    };
    ($ident:ident, i128) => {
        $crate::__newtype_num_from!($ident, i8 u8 i16 u16 i32 u32 i64 u64 i128);
        $crate::__newtype_num_into!($ident, i128);
        $crate::__newtype_num_try_from!($ident, i128, u128);
    };
    ($ident:ident, u128) => {
        $crate::__newtype_num_from!($ident, u8 u16 u32 u64 u128);
        $crate::__newtype_num_into!($ident, u128);
        $crate::__newtype_num_try_from!($ident, u128, i8 i16 i32 i64 i128);
    };
    ($ident:ident, isize) => {
        #[cfg(target_pointer_width = "16")]
        $crate::__newtype_num_impl!($ident, i16);
        #[cfg(target_pointer_width = "32")]
        $crate::__newtype_num_impl!($ident, i32);
        #[cfg(target_pointer_width = "64")]
        $crate::__newtype_num_impl!($ident, i64);
    };
    ($ident:ident, usize) => {
        #[cfg(target_pointer_width = "16")]
        $crate::__newtype_num_impl!($ident, u16);
        #[cfg(target_pointer_width = "32")]
        $crate::__newtype_num_impl!($ident, u32);
        #[cfg(target_pointer_width = "64")]
        $crate::__newtype_num_impl!($ident, u64);
    };
}

#[macro_export]
macro_rules! newtype_num {
    ($(#[$meta: meta])* $vis:vis struct $ident:ident($ty_vis:vis i8);) => {
        $crate::__newtype_num_decl!([$($meta),*], $vis, $ident, $ty_vis, i8);
        $crate::__newtype_num_impl!($ident, i8);
    };
    ($(#[$meta: meta])* $vis:vis struct $ident:ident($ty_vis:vis u8);) => {
        $crate::__newtype_num_decl!([$($meta),*], $vis, $ident, $ty_vis, u8);
        $crate::__newtype_num_impl!($ident, u8);
    };
    ($(#[$meta: meta])* $vis:vis struct $ident:ident($ty_vis:vis i16);) => {
        $crate::__newtype_num_decl!([$($meta),*], $vis, $ident, $ty_vis, i16);
        $crate::__newtype_num_impl!($ident, i16);
    };
    ($(#[$meta: meta])* $vis:vis struct $ident:ident($ty_vis:vis u16);) => {
        $crate::__newtype_num_decl!([$($meta),*], $vis, $ident, $ty_vis, u16);
        $crate::__newtype_num_impl!($ident, u16);
    };
    ($(#[$meta: meta])* $vis:vis struct $ident:ident($ty_vis:vis i32);) => {
        $crate::__newtype_num_decl!([$($meta),*], $vis, $ident, $ty_vis, i32);
        $crate::__newtype_num_impl!($ident, i32);
    };
    ($(#[$meta: meta])* $vis:vis struct $ident:ident($ty_vis:vis u32);) => {
        $crate::__newtype_num_decl!([$($meta),*], $vis, $ident, $ty_vis, u32);
        $crate::__newtype_num_impl!($ident, u32);
    };
    ($(#[$meta: meta])* $vis:vis struct $ident:ident($ty_vis:vis i64);) => {
        $crate::__newtype_num_decl!([$($meta),*], $vis, $ident, $ty_vis, i64);
        $crate::__newtype_num_impl!($ident, i64);
    };
    ($(#[$meta: meta])* $vis:vis struct $ident:ident($ty_vis:vis u64);) => {
        $crate::__newtype_num_decl!([$($meta),*], $vis, $ident, $ty_vis, u64);
        $crate::__newtype_num_impl!($ident, u64);
    };
    ($(#[$meta: meta])* $vis:vis struct $ident:ident($ty_vis:vis i128);) => {
        $crate::__newtype_num_decl!([$($meta),*], $vis, $ident, $ty_vis, i128);
        $crate::__newtype_num_impl!($ident, i128);
    };
    ($(#[$meta: meta])* $vis:vis struct $ident:ident($ty_vis:vis u128);) => {
        $crate::__newtype_num_decl!([$($meta),*], $vis, $ident, $ty_vis, u128);
        $crate::__newtype_num_impl!($ident, u128);
    };
    ($(#[$meta: meta])* $vis:vis struct $ident:ident($ty_vis:vis isize);) => {
        $crate::__newtype_num_decl!([$($meta),*], $vis, $ident, $ty_vis, isize);
        $crate::__newtype_num_impl!($ident, isize);
    };
    ($(#[$meta: meta])* $vis:vis struct $ident:ident($ty_vis:vis usize);) => {
        $crate::__newtype_num_decl!([$($meta),*], $vis, $ident, $ty_vis, usize);
        $crate::__newtype_num_impl!($ident, usize);
    };
}

newtype_num! {
    #[derive(BinRead, Debug)]
    pub struct UnkHnd(pub u32);
}

newtype_num! {
    #[derive(BinRead, Debug)]
    pub struct UnkPtr(pub u32);
}

newtype_num! {
    #[derive(BinRead, Debug)]
    pub struct Unk32(pub i32);
}

newtype_num! {
    #[derive(BinRead, Debug)]
    pub struct Unk16(pub i16);
}

newtype_num! {
    #[derive(BinRead, Debug)]
    pub struct Unk8(i8);
}

#[doc(hidden)]
pub mod __private {
    pub use paste::paste;
}

#[macro_export]
macro_rules! binrw_enum {
    ($name: ident, $size: ty) => {
        impl ::binrw::BinRead for $name {
            type Args = ();

            fn read_options<R: ::binrw::io::Read + ::binrw::io::Seek>(reader: &mut R, options: &::binrw::ReadOptions, args: Self::Args) -> ::binrw::BinResult<Self> {
                use $crate::SeekExt;
                $crate::restore_on_error(reader, |reader, pos| {
                    let value = <$size>::read_options(reader, options, args)?;
                    $crate::__private::paste! {
                        Self::[<from_ $size>](value).ok_or_else(|| ::binrw::Error::AssertFail {
                            pos,
                            message: format!(concat!("Invalid ", stringify!($name), " value 0x{:x}"), value),
                        })
                    }
                })
            }
        }
    }
}
