// https://github.com/rust-lang/cargo/issues/5034
#![warn(clippy::pedantic)]
#![allow(
    clippy::cast_possible_truncation,
    clippy::cast_possible_wrap,
    clippy::cast_sign_loss,
    clippy::missing_errors_doc,
    clippy::non_ascii_literal,
    clippy::option_if_let_else,
    clippy::verbose_bit_mask,
)]
#![warn(rust_2018_idioms)]

pub mod collections;
pub mod detection;
pub mod player;
pub mod resources;

#[must_use]
pub fn name(with_version: bool) -> String {
    let mut name = "Earthquake".to_string();
    if with_version {
        name.push(' ');
        name.push_str(&version());
    }
    name
}

#[must_use]
pub fn version() -> String {
    const SEMVER: Option<&str> = option_env!("VERGEN_SEMVER");
    const GIT_HASH: Option<&str> = option_env!("VERGEN_SHA_SHORT");

    let mut version = String::from(match SEMVER {
        Some(semver) if semver != "UNKNOWN" => semver,
        _ => env!("CARGO_PKG_VERSION"),
    });
    if let Some(hash) = GIT_HASH {
        version.push_str(&format!(" ({})", hash));
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
