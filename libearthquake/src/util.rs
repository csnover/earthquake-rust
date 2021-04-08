use binrw::{BinRead, io::{Read, self}};
use bstr::ByteSlice;
use derive_more::{Deref, DerefMut};
use libcommon::restore_on_error;

/// A raw string.
///
/// Identical to a `Vec<u8>` except with string-like output formatting.
#[derive(BinRead, Clone, Deref, DerefMut)]
pub struct RawString(Vec<u8>);

impl core::fmt::Debug for RawString {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        self.0.as_bstr().fmt(f)
    }
}

impl core::fmt::Display for RawString {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        self.0.as_bstr().fmt(f)
    }
}

/// A hybrid between a Pascal string and a C string.
///
/// This string type was used in place of normal `Str255` strings in Mac2Win.
///
/// Like `Str255`, it used a fixed-length buffer. Unlike `Str255`, it actually
/// stored up to 261 characters, using a null terminator for lengths >= 255.
///
/// This weird string format almost certainly exists entirely for compatibility:
///
/// 1. `MAX_PATH` on Windows has a maximum of 260 characters (including the null
///    terminator), and strings had to be able to hold an entire Windows file
///    path.
/// 2. Win32 uses null-terminated strings, so ensuring all strings are already
///    null-terminated makes it trivial to pass them in.
///
/// This implementation just uses a [`Vec`] for simplicity, since while we’re
/// writing code from the 90s, we don’t have to write it like it’s still the
/// 90s.
///
/// Unfortunately, it is necessary for this implementation to exist
/// [redundantly](libmactoolbox::types::PString), since Mac resources never
/// expected to need to null-terminate 255-byte-long strings.
#[derive(Clone, Deref, DerefMut)]
pub struct WinPString(Vec<u8>);

impl WinPString {
    /// The maximum possible length of a string stored in this type.
    const MAX_SIZE: usize = 261;
}

impl core::fmt::Debug for WinPString {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        self.0.as_bstr().fmt(f)
    }
}

impl core::fmt::Display for WinPString {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        self.0.as_bstr().fmt(f)
    }
}

impl BinRead for WinPString {
    type Args = ();

    fn read_options<R: io::Read + io::Seek>(reader: &mut R, options: &binrw::ReadOptions, args: Self::Args) -> binrw::BinResult<Self> {
        restore_on_error(reader, |reader, _| {
            let size = u8::read_options(reader, options, args)?;
            let mut data = Vec::with_capacity(Self::MAX_SIZE);

            if reader.take(size.into()).read_to_end(&mut data)? != size.into() {
                return Err(io::Error::from(io::ErrorKind::UnexpectedEof).into());
            }

            if size == u8::MAX {
                for byte in reader.bytes() {
                    match byte {
                        Ok(0) => break,
                        Ok(byte) => data.push(byte),
                        Err(error) => return Err(error.into()),
                    }
                }
            }

            Ok(Self(data.into()))
        })
    }
}
