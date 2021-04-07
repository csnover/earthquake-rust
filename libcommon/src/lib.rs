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
    clippy::struct_excessive_bools,
    clippy::verbose_bit_mask,
)]
#![warn(rust_2018_idioms)]

#[macro_use]
pub mod bitflags;
#[macro_use]
mod newtype;
pub mod convert;
mod error;
pub mod prelude;
pub mod io;
pub mod vfs;

pub use error::{flatten_errors, ReasonsExt};
pub use io::*;

newtype_num! {
    #[derive(binrw::BinRead, Debug)]
    pub struct UnkHnd(pub u32);
}

newtype_num! {
    #[derive(binrw::BinRead, Debug)]
    pub struct UnkPtr(pub u32);
}

newtype_num! {
    #[derive(binrw::BinRead, Debug)]
    pub struct Unk32(pub i32);
}

newtype_num! {
    #[derive(binrw::BinRead, Debug)]
    pub struct Unk16(pub i16);
}

newtype_num! {
    #[derive(binrw::BinRead, Debug)]
    pub struct Unk8(i8);
}
