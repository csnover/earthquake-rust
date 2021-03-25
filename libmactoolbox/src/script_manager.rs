use libcommon::{
    encodings::{Decoder, DecoderRef, MAC_CYRILLIC, MAC_JAPANESE, MAC_ROMAN},
};
use num_derive::FromPrimitive;
use num_traits::FromPrimitive;
use strum_macros::EnumVariantNames;

#[derive(Copy, Clone, Debug, FromPrimitive)]
pub enum CountryCode {
    USA               = 0,   // en_US
    France            = 1,   // fr_FR
    Britain           = 2,   // en_GB
    Germany           = 3,   // de_DE
    Italy             = 4,   // it_IT
    Netherlands       = 5,   // nl_NL
    Flemish           = 6,   // nl_BE
    Sweden            = 7,   // sv_SE
    Spain             = 8,   // es_ES
    Denmark           = 9,   // da_DK
    Portugal          = 10,  // pt_PT
    FrCanada          = 11,  // fr_CA
    Norway            = 12,  // no_NO
    Israel            = 13,  // iw_IL
    Japan             = 14,  // ja_JP
    Australia         = 15,  // en_AU
    Arabic            = 16,  // ar
    Finland           = 17,  // fi_FI
    FrSwiss           = 18,  // fr_CH
    GrSwiss           = 19,  // de_CH
    Greece            = 20,  // el_GR
    Iceland           = 21,  // is_IS
    Malta             = 22,  // mt_MT
    Cyprus            = 23,  //   _CY
    Turkey            = 24,  // tr_TR
    YugoCroatian      = 25,  // deprecated: use Croatia
    NetherlandsComma  = 26,
    BelgiumLuxPoint   = 27,
    CanadaComma       = 28,
    CanadaPoint       = 29,  // unused
    VariantPortugal   = 30,  // unused
    VariantNorway     = 31,  // unused
    VariantDenmark    = 32,
    IndiaHindi        = 33,  // hi_IN
    PakistanUrdu      = 34,  // ur_PK
    TurkishModified   = 35,
    ItalianSwiss      = 36,  // it_CH
    International     = 37,  // en

    Romania           = 39,  // ro_RO
    GreecePoly        = 40,
    Lithuania         = 41,  // lt_LT
    Poland            = 42,  // pl_PL
    Hungary           = 43,  // hu_HU
    Estonia           = 44,  // et_EE
    Latvia            = 45,  // lv_LV
    Sami              = 46,  // se
    FaroeIsl          = 47,  // fo_FO
    Iran              = 48,  // fa_IR
    Russia            = 49,  // ru_RU
    Ireland           = 50,  // ga_IE
    Korea             = 51,  // ko_KR
    China             = 52,  // zh_CN
    Taiwan            = 53,  // zh_TW
    Thailand          = 54,  // th_TH
    ScriptGeneric     = 55,
    Czech             = 56,  // cs_CZ
    Slovak            = 57,  // sk_SK
    FarEastGeneric    = 58,
    Magyar            = 59,  // unused: see Hungary
    Bengali           = 60,  // bn
    ByeloRussian      = 61,  // be_BY
    Ukraine           = 62,  // uk_UA

    GreeceAlt         = 64,  // unused
    Serbian           = 65,  // sr_YU, sh_YU
    Slovenian         = 66,  // sl_SI
    Macedonian        = 67,  // mk_MK
    Croatia           = 68,  // hr_HR, sh_HR

    GermanReformed    = 70,  // de_DE
    Brazil            = 71,  // pt_BR
    Bulgaria          = 72,  // bg_BG
    Catalonia         = 73,  // ca_ES
    Multilingual      = 74,
    ScottishGaelic    = 75,  // gd
    ManxGaelic        = 76,  // gv
    Breton            = 77,  // br
    Nunavut           = 78,  // iu_CA
    Welsh             = 79,  // cy

    IrishGaelicScript = 81,  // ga_IE
    EngCanada         = 82,  // en_CA
    Bhutan            = 83,  // dz_BT
    Armenian          = 84,  // hy_AM
    Georgian          = 85,  // ka_GE
    SpLatinAmerica    = 86,  // es

    Tonga             = 88,  // to_TO

    FrenchUniversal   = 91,  // fr
    Austria           = 92,  // de_AT

    Gujarati          = 94,  // gu_IN
    Punjabi           = 95,  // pa
    IndiaUrdu         = 96,  // ur_IN
    Vietnam           = 97,  // vi_VN
    FrBelgium         = 98,  // fr_BE
    Uzbek             = 99,  // uz_UZ
    Singapore         = 100, //
    Nynorsk           = 101, //   _NO
    Afrikaans         = 102, // af_ZA
    Esperanto         = 103, // eo
    Marathi           = 104, // mr_IN
    Tibetan           = 105, // bo
    Nepal             = 106, // ne_NP
    Greenland         = 107  // kl
}

impl CountryCode {
    #[must_use]
    pub fn encoding(self) -> DecoderRef {
        // This translation matrix is based on the list at
        // https://www.unicode.org/Public/MAPPINGS/VENDORS/APPLE/ReadMe.txt.
        // Trying to get away with just doing a mapping from the country code
        // instead of also needing a script code because country codes can be
        // found in any 'vers' resource but script codes are generally not
        // available in the resource fork.
        #![allow(clippy::match_same_arms)]
        match self {
            Self::Turkey | Self::Croatia | Self::Slovenian | Self::YugoCroatian |
            Self::Iceland | Self::FaroeIsl | Self::Ireland | Self::ScottishGaelic |
            Self::ManxGaelic | Self::Breton | Self::Welsh | Self::IrishGaelicScript |
            Self::Greece => unimplemented!(),
            Self::Japan => MAC_JAPANESE as &dyn Decoder,
            Self::China => unimplemented!(),
            Self::Korea => unimplemented!(),
            Self::Arabic => unimplemented!(),
            Self::Iran => unimplemented!(),
            Self::Israel => unimplemented!(),
            Self::Russia => MAC_CYRILLIC as &dyn Decoder,
            Self::Ukraine => unimplemented!(),
            Self::IndiaHindi => unimplemented!(),
            Self::Thailand => unimplemented!(),
            Self::Taiwan => unimplemented!(),
            Self::Tibetan => unimplemented!(),
            Self::Nunavut => unimplemented!(),
            Self::Poland | Self::Czech | Self::Slovak | Self::Hungary |
            Self::Estonia | Self::Latvia | Self::Lithuania => unimplemented!(),
            _ => MAC_ROMAN as &dyn Decoder,
        }
    }
}

#[derive(Clone, Copy, Debug, EnumVariantNames, FromPrimitive)]
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

// TODO: This is not sufficient; region codes are needed in addition to the
// script code for correct decoding of Turkish, Croatian, Icelandic, Romanian,
// Celtic, Gaelic, Greek, and Farsi.
pub fn decode_text<T: binrw::io::Read + binrw::io::Seek>(input: &mut T, script_code: u8) -> String {
    #![allow(clippy::match_same_arms)]
    match ScriptCode::from_u8(script_code) {
        Some(ScriptCode::Roman) | None       => MAC_ROMAN.decode_stream(input),
        Some(ScriptCode::Japanese)           => MAC_JAPANESE.decode_stream(input),
        Some(ScriptCode::ChineseTraditional) => todo!("Chinese traditional decoder"),
        Some(ScriptCode::Korean)             => todo!("Korean decoder"),
        Some(ScriptCode::Russian)            => MAC_CYRILLIC.decode_stream(input),
        _ => unimplemented!("no currently known uses of other script codes"),
    }
}
