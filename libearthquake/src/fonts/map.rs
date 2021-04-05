use binrw::BinRead;

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
pub struct Map(Vec<u8>);
