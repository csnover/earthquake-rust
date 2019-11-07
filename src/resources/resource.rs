use byteordered::{ByteOrdered, Endian};
use crate::{OSType, string::StringReadExt};
use encoding::Encoding;
use std::io::{self, Read};

#[derive(Debug)]
pub enum Resource {
    CastProps {},
    CastInfo {},
    CastMap {},
    ColorLookupTable {},
    Config {},
    Cursor {},
    FileInfo {},
    Frame {},
    FrameLabel {},
    InputMap {},
    Junk,
    KeyMap {},
    LingoContext {},
    LingoNames {},
    LingoScript {},
    MacColorLookupTable {},
    MemoryMap {},
    Score {},
    ScoreOrder {},
    ScoreRef {},
    String {},
    StringList(Vec<String>),
    WinBitmap {},
    Unknown { os_type: OSType, size: usize },
}

pub fn parse_string_list<T, U, V>(mut input: ByteOrdered<T, U>, str_encoding: &V) -> io::Result<Vec<String>>
where
    T: io::Read,
    U: Endian,
    V: Encoding {
    let count = input.read_u16()?;
    let mut strings = Vec::with_capacity(count as usize);
    for _ in 0..count {
        strings.push(input.read_pascal_str(str_encoding)?);
    }
    Ok(strings)
}

pub fn parse_resource<T, U, V>(os_type: OSType, mut input: ByteOrdered<T, U>, str_encoding: Option<&V>) -> io::Result<Resource>
where
    T: io::Read,
    U: Endian,
    V: Encoding {
    use Resource as R;
    Ok(match os_type.as_bytes() {
        b"CAS*" => R::CastMap {},
        // CFTC = ? Looks like mmap, but with less data
        // 00 00 00 00 { <ostype> <size> <dword (id? flags? both?)> <offset> }{..}
        b"CLUT" => R::ColorLookupTable {},
        b"clut" => R::MacColorLookupTable {},
        b"CURS" => R::Cursor {},
        b"DIB " => R::WinBitmap {},
        b"imap" => R::InputMap {},
        b"junk" => R::Junk,
        b"Lctx" => R::LingoContext {},
        b"Lnam" => R::LingoNames {},
        b"Lscr" => R::LingoScript {},
        // McNm = File name? Contains file name without a file extension plus some data path
        // 00 00 00 00 <pascal string, file name> <garbage byte?> <pascal string, data path>
        b"mmap" => R::MemoryMap {},
        b"SCRF" => R::ScoreRef {},
        b"Sord" => R::ScoreOrder {},
        b"STR " => R::String {},
        b"STR#" => R::StringList(parse_string_list(input, str_encoding.expect("String encoding required"))?),
        // VWAC = Accelerator?
        b"VWCF" | b"DRCF" => R::Config {},
        b"VWCI" => R::CastInfo {},
        b"VWCR" | b"CASt" => R::CastProps {},
        // Ver. = ?
        b"VWFI" => R::FileInfo {},
        b"VWFM" => R::Frame {},
        b"VWLB" => R::FrameLabel {},
        b"VWSC" => R::Score {},
        _ => {
            Resource::Unknown { os_type, size: input.bytes().count() }
        }
    })
}
