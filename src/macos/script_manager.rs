use crate::Reader;
use encoding::{all as encodings, types::{DecoderTrap, Encoding}};
use num_derive::FromPrimitive;
use num_traits::FromPrimitive;

#[derive(FromPrimitive)]
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
pub fn decode_text<T: Reader>(input: &mut T, script_code: u8) -> String {
    match ScriptCode::from_u8(script_code) {
        Some(ScriptCode::Roman) | None       => decode_with_decoder(input, encodings::MAC_ROMAN),
        Some(ScriptCode::Japanese)           => decode_with_decoder(input, encodings::WINDOWS_31J),
        Some(ScriptCode::ChineseTraditional) => decode_with_decoder(input, encodings::BIG5_2003),
        Some(ScriptCode::Korean)             => decode_with_decoder(input, encodings::WINDOWS_949),
        Some(ScriptCode::Russian)            => decode_with_decoder(input, encodings::MAC_CYRILLIC),
        _ => unimplemented!(),
    }
}

fn decode_with_decoder<T: Reader, D>(input: &mut T, decoder: &D) -> String
where D: Encoding {
    let mut raw_text = Vec::new();
    input.read_to_end(&mut raw_text).unwrap();
    decoder.decode(raw_text.as_slice(), DecoderTrap::Replace).unwrap()
}
