//! Type definitions for converting from legacy character encodings to UTF-8.

// TODO: The actual Mac mappings for unimplemented character sets can be found
// at https://www.unicode.org/Public/MAPPINGS/VENDORS/

use core::convert::TryInto;
use encoding::{DecoderTrap, label::encoding_from_windows_code_page};
use super::{Error, ScriptCode};

/// Tries to convert a legacy text encoding identified by script code to UTF-8.
///
/// Note that script codes are not always sufficient to transform text to
/// Unicode, so this API is not suitable for use with Turkish, Croatian,
/// Icelandic, Romanian, Celtic, Gaelic, Greek, or Farsi.
///
/// Similar to `TECConvertText`.
pub(crate) fn convert_text(text: impl AsRef<[u8]>, script_code: impl TryInto<ScriptCode>) -> Result<String, Error> {
    let script_code = script_code.try_into().map_err(|_| Error::BadScriptCode)?;
    let code_page = match script_code {
        ScriptCode::Roman => 10000,
        ScriptCode::Japanese => 10001,
        ScriptCode::ChineseTraditional => 10002,
        ScriptCode::Korean => 10003,
        ScriptCode::Arabic => 10004,
        ScriptCode::Hebrew => 10005,
        ScriptCode::Greek => 10006,
        ScriptCode::Russian => 10007,
        ScriptCode::Thai => 10021,
        ScriptCode::ChineseSimplified => 10008,
        _ => return Err(Error::NoEncoder(script_code)),
    };

    if let Some(encoder) = encoding_from_windows_code_page(code_page) {
        encoder.decode(text.as_ref(), DecoderTrap::Strict).map_err(|_| Error::InvalidInput(script_code))
    } else {
        Err(Error::NoEncoder(script_code))
    }
}
