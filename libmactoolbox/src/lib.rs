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

mod application_vise;
#[cfg(feature = "dialogs")]
mod dialogs;
mod files;
mod os_type;
pub mod resources;
mod resource_file;
mod resource_id;
mod resource_manager;
#[cfg(feature = "quickdraw")]
mod quickdraw;
pub mod script_manager;
pub mod string;
mod system;

#[deprecated]
pub use files::AppleDouble;
pub use application_vise::*;
#[deprecated]
pub use files::MacBinary;
pub use os_type::*;
pub use resource_file::*;
pub use resource_id::*;
pub use resource_manager::*;
pub use system::System;
use anyhow::Result as AResult;
use byteordered::{ByteOrdered, Endianness};
use libcommon::{Reader, Resource};

#[derive(Clone, Copy, Debug, Default)]
pub struct Point {
    pub x: i16,
    pub y: i16,
}

impl Resource for Point {
    fn load<T: Reader>(input: &mut ByteOrdered<T, Endianness>, size: u32) -> AResult<Self> where Self: Sized {
        assert_eq!(size, 4);
        Ok(Self {
            x: input.read_i16()?,
            y: input.read_i16()?,
        })
    }
}

#[derive(Clone, Copy, Debug, Default)]
pub struct Rect {
    pub top: i16,
    pub left: i16,
    pub bottom: i16,
    pub right: i16,
}

impl Resource for Rect {
    fn load<T: Reader>(input: &mut ByteOrdered<T, Endianness>, size: u32) -> AResult<Self> where Self: Sized {
        assert_eq!(size, 8);
        Ok(Self {
            top: input.read_i16()?,
            left: input.read_i16()?,
            bottom: input.read_i16()?,
            right: input.read_i16()?,
        })
    }
}

// TODO
pub struct TEHandle(u32);
