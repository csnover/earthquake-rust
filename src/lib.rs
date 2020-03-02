#![warn(clippy::pedantic)]
#![allow(
    clippy::cast_possible_truncation,
    clippy::cast_possible_wrap,
    clippy::cast_sign_loss,
    clippy::missing_errors_doc,
    clippy::non_ascii_literal,
    clippy::verbose_bit_mask,
)]

pub mod collections;
pub mod detection;
pub mod encodings;
pub mod macos;
pub mod io;
pub(crate) mod player;
pub mod resources;
pub(crate) mod string;
pub(crate) mod types;

pub(crate) use byteordered::Endianness;
pub(crate) use crate::types::os_type::*;
pub use crate::types::reader::*;
pub(crate) use crate::macos::ResourceId;
pub use crate::player::*;
pub use crate::io::SharedStream;

#[allow(dead_code)]
pub(crate) fn panic_for_sample<T: AsRef<str>>(is_needed: bool, kind: T) {
    if is_needed {
        panic!("{}. Please send this file for analysis.", kind.as_ref());
    }
}
