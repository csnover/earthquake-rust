use binread::{BinRead, io};
use bstr::BString;
use std::io::Read;

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
/// This implementation just uses a [`BString`](bstr::BString) for simplicity,
/// since while we’re writing code from the 90s, we don’t have to write it like
/// it’s still the 90s.
///
/// Unfortunately, it is necessary for this implementation to exist
/// [redundantly](libmactoolbox::types::PString), since Mac resources never
/// expected to need to null-terminate 255-byte-long strings.
pub struct WinPString(BString);

impl WinPString {
    /// The maximum possible length of a string stored in this type.
    const MAX: u16 = 261;
}

impl BinRead for WinPString {
    type Args = ();

    fn read_options<R: io::Read + io::Seek>(reader: &mut R, options: &binread::ReadOptions, args: Self::Args) -> binread::BinResult<Self> {
        let size = u8::read_options(reader, options, args)?;
        let mut data = Vec::with_capacity(Self::MAX.into());

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
    }
}
