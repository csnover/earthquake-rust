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
pub mod resources;
pub(crate) mod string;
pub(crate) mod types;

pub(crate) use byteordered::Endianness;
pub use crate::types::os_type::*;
pub use crate::types::reader::*;
pub(crate) use crate::macos::ResourceId;
pub use crate::io::SharedStream;

#[macro_export]
macro_rules! assert_sample(
    ($test:expr, $($arg:tt)+) => (
        if !$test {
            $crate::panic_sample!($($arg)+)
        }
    )
);

#[macro_export]
macro_rules! bail_sample(
    ($msg:expr) => ({
        ::anyhow::bail!("{}. Please send this file for analysis.", $msg)
    });
    ($msg:expr,) => ({
        $crate::bail_sample!($msg)
    });
    ($fmt:expr, $($arg:tt)+) => ({
        $crate::bail_sample!(format_args!($fmt, $($arg)+))
    });
);

#[macro_export]
macro_rules! ensure_sample(
    ($test:expr, $msg:expr) => ({
        ::anyhow::ensure!($test, "{}. Please send this file for analysis.", $msg)
    });
    ($test:expr, $msg:expr,) => ({
        $crate::ensure_sample!($test, $msg)
    });
    ($test:expr, $fmt:expr, $($arg:tt)+) => ({
        $crate::ensure_sample!($test, format_args!($fmt, $($arg)+))
    });
);

#[macro_export]
macro_rules! panic_sample(
    ($msg:expr) => ({
        panic!("{}. Please send this file for analysis.", $msg)
    });
    ($msg:expr,) => ({
        $crate::panic_sample!($msg)
    });
    ($fmt:expr, $($arg:tt)+) => ({
        $crate::panic_sample!(format_args!($fmt, $($arg)+))
    });
);
