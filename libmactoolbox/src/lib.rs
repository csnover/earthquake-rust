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

mod application_vise;
#[cfg(feature = "dialogs")]
mod dialogs;
mod errors;
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
pub mod types;
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
use binrw::BinRead;
use quickdraw::Pixels;

#[derive(BinRead, Clone, Copy, Debug, Default)]
pub struct Point {
    pub x: Pixels,
    pub y: Pixels,
}

#[derive(BinRead, Clone, Copy, Default)]
pub struct Rect {
    pub top: Pixels,
    pub left: Pixels,
    pub bottom: Pixels,
    pub right: Pixels,
}

impl Rect {
    #[inline]
    #[must_use]
    pub fn height(self) -> Pixels {
        self.bottom - self.top
    }

    #[inline]
    #[must_use]
    pub fn width(self) -> Pixels {
        self.right - self.left
    }
}

impl std::fmt::Debug for Rect {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct(std::any::type_name::<Self>())
            .field("top", &self.top)
            .field("left", &self.left)
            .field("bottom", &self.bottom)
            .field("right", &self.right)
            .field("(width)", &self.width())
            .field("(height)", &self.height())
            .finish()
    }
}

// TODO
#[derive(Clone, Copy, Debug, Default)]
pub struct TEHandle(u32);
