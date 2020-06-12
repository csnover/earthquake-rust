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
pub mod io;
pub mod macos;
pub mod player;
pub mod resources;
pub(crate) mod string;
pub(crate) mod types;

pub(crate) use byteordered::Endianness;
pub use crate::types::os_type::*;
pub use crate::types::reader::*;
pub(crate) use crate::macos::ResourceId;
pub use crate::io::SharedStream;

#[must_use]
pub fn name(with_version: bool) -> String {
    let mut name = "Earthquake".to_string();
    if with_version {
        let version = version();
        if !version.is_empty() {
            name.push(' ');
            name.push_str(&version);
        }
    }
    name
}

#[must_use]
pub fn version() -> String {
    const SEMVER: Option<&str> = option_env!("VERGEN_SEMVER");
    const VERSION: Option<&str> = option_env!("CARGO_PKG_VERSION");
    const GIT_HASH: Option<&str> = option_env!("VERGEN_SHA_SHORT");

    let mut version = String::new();
    if let Some(semver) = SEMVER.or_else(|| VERSION) {
        if semver == "UNKNOWN" && VERSION.is_some() {
            version += VERSION.unwrap();
        } else {
            version += semver;
        }
    }
    if let Some(hash) = GIT_HASH {
        if !version.is_empty() {
            version.push(' ');
        }
        version.push_str(&format!("({})", hash));
    }
    version
}

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
