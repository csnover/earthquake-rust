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

#[cfg(feature = "dialogs")]
mod dialogs;
#[cfg(feature = "events")]
pub mod events;
mod files;
#[cfg(feature = "intl")]
pub mod intl;
pub mod resources;
#[cfg(feature = "quickdraw")]
pub mod quickdraw;
mod system;
pub mod text_edit;
pub mod types;
pub mod vfs;

pub use system::System;
