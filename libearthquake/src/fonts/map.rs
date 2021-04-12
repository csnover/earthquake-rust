use binrw::BinRead;
use crate::{resources::{SerializedDict, config::Platform}, util::RawString};
use derive_more::{Deref, DerefMut};
use libcommon::prelude::*;
use libmactoolbox::{resources::ResNum, typed_resource};
use num_traits::FromPrimitive;

/// An extended font map.
///
/// Director copies the content of `FONTMAP.TXT` into this resource.
///
/// The font map is used to convert fonts and character sets between Mac and
/// Windows.
///
/// The grammar looks roughly like:
///
/// ```text
/// NonCR        = [^\r]
/// NonWS        = [^\s]
/// NonQuote     = [^"]
/// EndOfLine    = "\r" "\n"?
/// Number       = ([0-9])+
/// Platform     = i"Mac" | i"Win"
/// CommentStart = ";" | "--"
/// MapModifier  = i"map none" | i"map all"
///
/// FontName     = '"' (NonQuote)* '"' | (NonWS)*
/// SizeMap      = Number "=>" Number
///
/// Comment      = CommentStart (NonCR)* EndOfLine
/// FontMap      = Platform ":" FontName "=>" Platform ":" FontName MapModifier? (SizeMap)* EndOfLine
/// CharMap      = Platform ":" "=>" Platform ":" (SizeMap)+ EndOfLine
/// ```
///
/// OsType: `'FXmp'`
#[derive(BinRead, Clone, Debug, Default)]
pub struct Source(RawString);
typed_resource!(Source => b"FXmp");

// RE: R_FXmpMBListChar
// The values could be font sizes or characters. So, probably: TODO don’t be so
// sloppy.
#[derive(Clone, Copy, Debug)]
pub struct Value {
    from: u16,
    to: u16,
}

// RE: R_FXmpMBList
#[derive(Clone, Debug, Default)]
pub struct Map(Vec<Value>);

/// The font families used in Mac Styled Text resources in the movie.
///
/// This information is used along with the `FXmp` map to convert the fonts used
/// in Mac Styled Text resources between platforms.
#[derive(BinRead, Clone, Debug, Deref, DerefMut)]
pub struct Fmap(SerializedDict<FontFamily>);
typed_resource!(Fmap => b"Fmap");

/// Identifying information about a font family.
#[derive(Clone, Copy, Debug)]
pub struct FontFamily {
    /// The original platform.
    platform: Platform,
    /// The resource number of the font.
    id: ResNum,
}

#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error("bad platform '{0}' in font map")]
    BadPlatform(i16)
}

// Since this data was stored as if it were an `i32`, it has to be converted
// with bit shifts and reinterpreting casts after it has been read out as an
// `i32` (since otherwise endianness conversion would be wrong and the Dict API
// would not be usable).
impl TryFrom<i32> for FontFamily {
    type Error = Error;

    fn try_from(value: i32) -> Result<Self, Self::Error> {
        let platform = (value >> 16) as i16;
        Ok(Self {
            platform: Platform::from_i16(platform).ok_or(Error::BadPlatform(platform))?,
            id: ResNum::from((value & 0xffff) as i16),
        })
    }
}
