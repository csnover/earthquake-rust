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
#[cfg(feature = "events")]
mod events;
mod files;
mod os_type;
pub mod resources;
mod resource_file;
mod resource_id;
mod resource_manager;
#[cfg(feature = "quickdraw")]
pub mod quickdraw;
pub mod script_manager;
mod system;
pub mod vfs;

#[deprecated]
pub use files::AppleDouble;
pub use application_vise::*;
pub use events::*;
#[deprecated]
pub use files::MacBinary;
pub use os_type::*;
pub use resource_file::*;
pub use resource_id::*;
pub use resource_manager::*;
pub use system::System;
use anyhow::Result as AResult;
use byteordered::{ByteOrdered, Endianness};
use libcommon::{Reader, Resource, resource::Input};

#[derive(Clone, Copy, Debug, Default)]
pub struct Point {
    pub x: i16,
    pub y: i16,
}

impl Point {
    pub const SIZE: u32 = 4;
}

impl Resource for Point {
    type Context = ();
    fn load(input: &mut ByteOrdered<impl Reader, Endianness>, size: u32, _: &Self::Context) -> AResult<Self> where Self: Sized {
        assert_eq!(size, Self::SIZE);
        Ok(Self {
            x: input.read_i16()?,
            y: input.read_i16()?,
        })
    }
}

#[derive(Clone, Copy, Default)]
pub struct Rect {
    pub top: i16,
    pub left: i16,
    pub bottom: i16,
    pub right: i16,
}

impl Rect {
    pub const SIZE: u32 = 8;

    #[inline]
    #[must_use]
    pub fn height(self) -> i16 {
        self.bottom - self.top
    }

    #[inline]
    #[must_use]
    pub fn width(self) -> i16 {
        self.right - self.left
    }
}

impl std::fmt::Debug for Rect {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Rect")
            .field("top", &self.top)
            .field("left", &self.left)
            .field("bottom", &self.bottom)
            .field("right", &self.right)
            .field("(width)", &self.width())
            .field("(height)", &self.height())
            .finish()
    }
}

impl Resource for Rect {
    type Context = ();
    fn load(input: &mut Input<impl Reader>, size: u32, _: &Self::Context) -> AResult<Self> where Self: Sized {
        assert_eq!(size, Self::SIZE);
        Ok(Self {
            top: input.read_i16()?,
            left: input.read_i16()?,
            bottom: input.read_i16()?,
            right: input.read_i16()?,
        })
    }
}

// TODO
#[derive(Default)]
pub struct TEHandle(u32);
