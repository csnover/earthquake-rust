use encoding::{all as encodings, types::{DecoderTrap, Encoding as _}};

// Mappings: https://www.unicode.org/Public/MAPPINGS/VENDORS/

pub trait Decoder {
    /// Decodes a byte slice into a string. Invalid code sequences are replaced
    /// by the Unicode replacement character.
    fn decode(&self, text: &[u8]) -> String;

    /// Decodes a stream into a string. Invalid code sequences are replaced
    /// by the Unicode replacement character.
    fn decode_stream(&self, text: &mut dyn std::io::Read) -> String {
        let mut buffer = Vec::new();
        text.read_to_end(&mut buffer).expect("Failed to read stream into memory");
        self.decode(&buffer)
    }
}

pub struct MacJapanese;
impl Decoder for MacJapanese {
    fn decode(&self, text: &[u8]) -> String {
        todo!()
    }
}
pub const MAC_JAPANESE: &MacJapanese = &MacJapanese;

macro_rules! encodings_decoder(
    ($name:ident, $id:ident, $($module:ident)::+) => (
        pub struct $id;
        impl Decoder for $id {
            fn decode(&self, text: &[u8]) -> String {
                $($module)::+.decode(text.as_ref(), DecoderTrap::Replace).unwrap()
            }
        }
        pub const $name: &$id = &$id;
    );
);

encodings_decoder!(MAC_CYRILLIC, MacCyrillic, encodings::MAC_CYRILLIC);
encodings_decoder!(MAC_ROMAN, MacRoman, encodings::MAC_ROMAN);
encodings_decoder!(WIN_CYRILLIC, WinCyrillic, encodings::WINDOWS_1251);
encodings_decoder!(WIN_JAPANESE, WinJapanese, encodings::WINDOWS_31J);
encodings_decoder!(WIN_ROMAN, WinRoman, encodings::WINDOWS_1252);

pub type DecoderRef = &'static dyn Decoder;
