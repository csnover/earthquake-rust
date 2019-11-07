use byteorder::{BigEndian, ByteOrder, LittleEndian, ReadBytesExt};
use byteordered::{ByteOrdered, StaticEndianness};
use encoding::all::{MAC_ROMAN, WINDOWS_1252};
use crate::{Endianness, OSType, os, collections::{riff, rsrc::MacResourceFile}, Reader, resources::resource, string::StringReadExt};
use std::io::{Cursor, SeekFrom};

// TODO: Create an actual Projector type and do not expost this any more
#[derive(Debug)]
pub struct DetectionInfo {
    pub name: Option<String>,
    endianness: Endianness,
    pub platform: Platform,
    pub version: ProjectorVersion,
    pub movies: Vec<Movie>,
}

#[derive(Debug)]
pub enum Movie {
    Internal {
        offset: u32,
        size: u32
    },
    External {
        filename: String,
        path: String,
        size: u32
    },
}

#[derive(Debug, Copy, Clone, PartialEq)]
pub enum Platform {
    Windows,
    Mac,
}

#[derive(Debug, Copy, Clone, PartialEq, PartialOrd)]
pub enum ProjectorVersion {
    D3,
    D4,
    D5,
    D7,
}

pub fn detect<T: Reader>(reader: &mut T) -> Option<DetectionInfo> {
    detect_win(reader).or_else(|| detect_mac(reader))
}

fn detect_mac<T: Reader>(reader: &mut T) -> Option<DetectionInfo> {
    reader.seek(SeekFrom::Start(0)).ok()?;
    let rom = MacResourceFile::new(reader).ok()?;

    let version = if rom.contains(os!(b"PJ95"), 0) {
        ProjectorVersion::D5
    } else if rom.contains(os!(b"PJ93"), 0) {
        ProjectorVersion::D4
    } else if rom.contains(os!(b"VWst"), 0) {
        ProjectorVersion::D3
    } else {
        return None;
    };

    let has_external_data = {
        let os_type = if version == ProjectorVersion::D3 { os!(b"VWst") } else { os!(b"PJst") };
        rom.get(os_type, 0)?.data().ok()?[4] != 0
    };

    let mut movies = Vec::new();
    if has_external_data {
        // TODO: Should parse this in Resource instead of pulling it from
        // the ROM and then pushing it into Resource?
        let external_files = rom.get(os!(b"STR#"), 0)?;
        let cursor = ByteOrdered::be(Cursor::new(external_files.data().ok()?));
        for filename in resource::parse_string_list(cursor, MAC_ROMAN).ok()? {
            movies.push(Movie::External {
                filename,
                path: String::from("TODO"),
                size: 0,
            });
        }
    } else {
        // TODO
        movies.push(Movie::Internal {
            offset: 0,
            size: 0,
        });
    }

    Some(DetectionInfo {
        name: rom.name(),
        endianness: Endianness::Big,
        platform: Platform::Mac,
        version,
        movies,
    })
}

fn detect_win<T: Reader>(reader: &mut T) -> Option<DetectionInfo> {
    reader.seek(SeekFrom::End(-4)).ok()?;
    let offset = reader.read_u32::<LittleEndian>().ok()?;
    reader.seek(SeekFrom::Start(offset.into())).ok()?;

    let mut buffer = [0u8; 8];
    reader.read_exact(&mut buffer).ok()?;

    let version = match &buffer[0..4] {
        b"PJ93" | b"39JP" => ProjectorVersion::D4,
        b"PJ95" | b"59JP" => ProjectorVersion::D5,
        b"PJ00" | b"00JP" => ProjectorVersion::D7,
        _ => {
            let checksum: u8 = buffer[0]
                .wrapping_add(buffer[1])
                .wrapping_add(buffer[2])
                .wrapping_add(buffer[3])
                .wrapping_add(buffer[4])
                .wrapping_add(buffer[5])
                .wrapping_add(buffer[6]);

            if checksum != 0 {
                return None;
            }

            ProjectorVersion::D3
        },
    };

    // TODO: Turns out that this is a big lie and the data offsets in this file
    // are still little-endian even when the OSType is not.
    // let endianness = if buffer[0] == b'P' {
    //     Endianness::Big
    // } else {
    //     Endianness::Little
    // };
    let endianness = Endianness::Little;

    let movies = match version {
        ProjectorVersion::D3 => {
            let num_movies = LittleEndian::read_u16(&buffer);

            // TODO: Somewhere in the projector header is probably a flag to
            // read movies internally; find it and then set the movie list
            // appropriately. For now the only corpus available has a single
            // external movie.
            let mut movies: Vec<Movie> = Vec::new();
            for _ in 0..num_movies {
                let size = reader.read_u32::<LittleEndian>().ok()?;
                // TODO: Probably need to try to use CHARDET for this
                let filename = reader.read_pascal_str(WINDOWS_1252).ok()?;
                let path = reader.read_pascal_str(WINDOWS_1252).ok()?;

                movies.push(Movie::External {
                    filename,
                    path,
                    size,
                });
            }

            movies
        },
        _ => {
            let offset = if endianness == Endianness::Big {
                BigEndian::read_u32(&buffer[4..])
            } else {
                LittleEndian::read_u32(&buffer[4..])
            };

            reader.seek(SeekFrom::Start(u64::from(offset))).ok()?;
            let info = riff::detect(reader).unwrap_or_else(|| panic!("Could not parse embedded RIFF at {}", offset));

            let mut movies = Vec::new();
            movies.push(Movie::Internal {
                offset,
                size: info.size
            });
            movies
        }
    };

    Some(DetectionInfo {
        name: Some(String::from("TODO")),
        endianness,
        platform: Platform::Windows,
        version,
        movies,
    })
}
