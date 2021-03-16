//! Basic Macintosh data types
//!
//! MacTypes.h

use binread::{BinRead, io};
use bstr::{ByteSlice, ByteVec};
use derive_more::{Deref, DerefMut, Display, From};
use std::{io::Read, rc::Rc};

/// A string which may be stored in one of several forms depending upon its
/// origin.
///
/// In Macintosh Toolbox, as with most legacy systems, strings were defined as
/// simply a bunch of bytes and interpreted according to the system’s
/// [script code] at runtime. We would like to be able to do the same thing, but
/// Mac OS components like [`ResourceManager`] retrieve strings from the host
/// operating system using [hard-coded string resource values], and those values
/// will be encoded differently since they are coming from a modern host.
///
/// To handle this situation, the `String` type exists to operate against
/// several different underlying string representations. Theoretically it would
/// be possible to decode everything into [`std::string::String`] directly from
/// data, but there is no guarantee that all data can safely be transformed into
/// UTF-8, nor that the correct script code will be available at the time data
/// is read out of the resource manager.
#[derive(Clone, Debug, Display, From)]
pub enum MacString {
    Std(String),
    Raw(PString),
    RawRc(Rc<PString>),
}

impl MacString {
    #[must_use]
    pub fn to_path_lossy(&self) -> std::borrow::Cow<'_, std::path::Path> {
        match self {
            MacString::Std(s) => std::borrow::Cow::Borrowed(std::path::Path::new(s)),
            MacString::Raw(s) => s.to_path_lossy(),
            MacString::RawRc(s) => s.to_path_lossy(),
        }
    }

    #[must_use]
    pub fn to_str_lossy(&self) -> std::borrow::Cow<'_, str> {
        match self {
            MacString::Std(s) => std::borrow::Cow::Borrowed(s),
            MacString::Raw(s) => s.to_str_lossy(),
            MacString::RawRc(s) => s.to_str_lossy(),
        }
    }

    #[must_use]
    pub fn into_string_lossy(self) -> String {
        match self {
            MacString::Std(s) => s,
            MacString::Raw(s) => s.0.into_string_lossy(),
            MacString::RawRc(s) => s.0.clone().into_string_lossy(),
        }
    }
}

/// A binary [Pascal string](https://en.wikipedia.org/wiki/String_(computer_science)#Length-prefixed).
///
/// In Macintosh Toolbox, string types are defined in several lengths with
/// fixed-size buffers: `Str15`, `Str27`, `Str31`, `Str32`, `Str63`, and
/// `Str255`. Most APIs use `Str255`.
///
/// For the moment, only a single type with heap-backed storage is defined for
/// simplicity. This should be fine, since Pascal strings inside
/// [resources](crate::resources) were not stored with padding, so there is no
/// reason to have 6 distinct string types that differ only by length.
#[derive(Clone, Default, Deref, DerefMut, Eq, From, PartialEq)]
#[from(forward)]
#[must_use]
pub struct PString(Vec<u8>);

impl core::fmt::Debug for PString {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        core::fmt::Debug::fmt(self.0.as_bstr(), f)
    }
}

impl core::fmt::Display for PString {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        core::fmt::Display::fmt(self.0.as_bstr(), f)
    }
}

impl PString {
    pub fn new() -> Self {
        Self::default()
    }

    #[must_use]
    pub fn into_inner(self) -> Vec<u8> {
        self.0
    }
}

impl BinRead for PString {
    type Args = ();

    fn read_options<R: io::Read + io::Seek>(reader: &mut R, options: &binread::ReadOptions, args: Self::Args) -> binread::BinResult<Self> {
        let size = u8::read_options(reader, options, args)?;
        let mut data = Vec::with_capacity(size.into());
        if reader.take(size.into()).read_to_end(&mut data)? == size.into() {
            Ok(Self(data))
        } else {
            Err(io::Error::from(io::ErrorKind::UnexpectedEof).into())
        }
    }
}
