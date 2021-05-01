use binrw::{BinRead, io::{Read, self}};
use bstr::ByteSlice;
use crate::resources::config::Platform;
use derive_more::{Deref, DerefMut};
use libcommon::restore_on_error;
use libmactoolbox::types::MacString;

#[derive(Clone, Debug, Default, Eq)]
pub(crate) struct Path {
    platform: Platform,
    path: MacString,
}

impl Path {
    pub(crate) fn new(path: impl Into<MacString>, platform: Platform) -> Self {
        let path = path.into();

        {
            let path = path.as_bstr();
            let is_shell_link = path.get(path.len() - 4..path.len())
                .map(|suffix| suffix.eq_ignore_ascii_case(b".lnk"))
                .unwrap_or(false);

            if is_shell_link {
                todo!("IShellLink support");
            }
        }

        // TODO: Handle the '@' pathname operator
        // TODO: Handle extra normalisation that Director did on the input

        // OD would receive a pascal string and then convert it into internal
        // path data by replacing the length byte at the start with the length
        // of the first path segment, then replacing every path separator with
        // the length of the next segment, terminating with a null byte.

        Self {
            platform,
            path
        }
    }

    fn to_mac_string(&self, platform: Platform) -> MacString {
        // TODO: Normally this path separator fixing happened in the
        // constructor but output requires a full scan of the string anyway to
        // output the proper separators so there is almost no overhead to this
        // approach (an extra branch that normally wouldn’t exist, but which
        // will be correctly handled by branch prediction all of the time).
        // Still, this code is a big mess and probably should be doing character
        // set decoding instead of trying to function in isolation like this.
        let in_path_sep: &[u8] = match platform {
            Platform::Unknown => b":\\/",
            Platform::Mac => b":",
            Platform::Win => b"\\",
        };
        let out_path_sep = match platform {
            Platform::Unknown => b'/',
            Platform::Mac => b':',
            Platform::Win => b'\\',
        };
        let mut s = self.path.clone();
        if in_path_sep != &[out_path_sep] {
            // Safety: The characters being replaced are always ASCII.
            for c in unsafe { s.as_bstr_mut() }.iter_mut() {
                if in_path_sep.contains(c) {
                    *c = out_path_sep;
                }
            }
        }
        s
    }
}

impl PartialEq for Path {
    fn eq(&self, other: &Self) -> bool {
        // Director used OS string comparison functions. On both platforms this
        // meant locale-aware case insensitive comparison, but on Mac OS it also
        // meant diacritics were ignored, and on Windows it also meant
        // full-width CJK forms were ignored. In order to do this, it would be
        // necessary to hold a string with a known encoding, but it’s highly
        // probable that the only desired/predicted behaviour was
        // case-insensitive ASCII, so that is what we’ll do for now.
        self.path.as_bstr().eq_ignore_ascii_case(other.to_mac_string(self.platform).as_bstr())
    }
}

impl PartialEq<Path> for &Path {
    fn eq(&self, other: &Path) -> bool {
        *self == other
    }
}

/// A raw string.
///
/// Identical to a `Vec<u8>` except with string-like output formatting.
#[derive(BinRead, Clone, Default, Deref, DerefMut, Eq, PartialEq)]
pub(crate) struct RawString(Vec<u8>);

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
#[derive(Clone, Default, Deref, DerefMut, Eq, PartialEq)]
pub(crate) struct WinPString(Vec<u8>);

impl WinPString {
    /// The maximum possible length of a string stored in this type.
    const MAX_SIZE: usize = 260;
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

            Ok(Self(data))
        })
    }
}
