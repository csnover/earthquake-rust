// TODO: Finish everything
#![allow(dead_code)]

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
mod macros;
pub(crate) mod cast;
pub(crate) mod collections;
pub mod debug;
pub mod detection;
pub(crate) mod event;
pub(crate) mod fonts;
pub(crate) mod lingo;
pub mod player;
pub mod resources;
pub(crate) mod sound;
pub(crate) mod util;

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
