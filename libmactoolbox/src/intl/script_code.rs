use num_derive::FromPrimitive;
use num_traits::FromPrimitive;
use strum_macros::{Display, EnumVariantNames};

#[derive(Clone, Copy, Debug, Display, EnumVariantNames, FromPrimitive)]
#[strum(serialize_all = "kebab-case")]
pub enum ScriptCode {
    Roman = 0,
    Japanese,
    ChineseTraditional,
    Korean,
    Arabic,
    Hebrew,
    Greek,
    Russian,
    RightLeftSymbols,
    Devanagari,
    Gurmukhi,
    Oriya,
    Bengali,
    Tamil,
    Telugu,
    Kannada,
    Malayalam,
    Sinhalese,
    Burmese,
    Cambodian,
    Thai,
    Laotian,
    Georgian,
    Armenian,
    ChineseSimplified,
    Tibetan,
    Mongolian,
    Ethiopian,
    NonCyrillicSlavic,
    Vietnamese,
    Sindhi,
    UninterpretedSymbols,
}

impl core::convert::TryFrom<u8> for ScriptCode {
    type Error = super::Error;

    fn try_from(value: u8) -> Result<Self, Self::Error> {
        Self::from_u8(value).ok_or(super::Error::BadScriptCode)
    }
}
