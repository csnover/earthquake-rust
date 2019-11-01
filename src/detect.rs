use byteorder::{ByteOrder, LittleEndian, ReadBytesExt};
use crate::{m68k::M68k, Reader, string::StringReadExt};
use std::io::SeekFrom;

#[derive(Debug)]
pub enum Endianness {
    Little,
    Big,
    Unknown,
}

#[derive(Debug)]
pub enum Platform {
    Windows,
    Mac,
}

#[derive(Debug)]
pub enum Movie {
    Internal { offset: u32, size: u32 },
    External { filename: String, size: u32 },
}

#[derive(Debug, PartialEq, PartialOrd)]
pub enum MovieVersion {
    Unknown,
    D3,
    D4,
}

#[derive(Debug, PartialEq, PartialOrd)]
pub enum ProjectorVersion {
    Unknown,
    D3,
    D4,
    D5,
    D7,
}

#[derive(Debug)]
pub enum FileType {
    Projector {
        name: String,
        endianness: Endianness,
        platform: Platform,
        version: ProjectorVersion,
        data_dirname: String,
        movies: Vec<Movie>,
    },
    Movie {
        os_type_endianness: Endianness,
        data_endianness: Endianness,
        version: MovieVersion,
    }
}

fn detect_movie(reader: &mut dyn Reader) -> Option<FileType> {
    let mut buffer = [0u8; 8];
    reader.read_exact(&mut buffer).ok()?;
    let size = reader.seek(SeekFrom::End(0)).ok()?;

    match &buffer[0..4] {
        b"RIFX" | b"RIFF" => {
            const RIFF_HEADER_SIZE: u64 = 8;
            let riff_size = LittleEndian::read_u32(&buffer[4..]) as u64;

            // Only D3Win includes the RIFF header in the RIFF length.
            let has_le_data = size == riff_size + RIFF_HEADER_SIZE || size == riff_size;

            Some(FileType::Movie {
                os_type_endianness: Endianness::Big,
                data_endianness: if has_le_data { Endianness::Little } else { Endianness::Big },
                version: if buffer[3] == b'X' { MovieVersion::D4 } else { MovieVersion::D3 },
            })
        },
        b"XFIR" => Some(FileType::Movie {
            os_type_endianness: Endianness::Little,
            // LE data here is an assumption, since the only reason why OSType
            // would be LE is because it is generated on Windows
            data_endianness: Endianness::Little,
            version: if buffer[0] == b'X' { MovieVersion::D4 } else { MovieVersion::D3 },
        }),
        b"FFIR" => panic!("RIFF-LE files are not known to exist. Please send a sample of the file you are trying to open."),
        _ => None,
    }
}

fn detect_win(reader: &mut dyn Reader) -> Option<FileType> {
    reader.seek(SeekFrom::End(-4)).ok()?;
    let offset = reader.read_u32::<LittleEndian>().ok()?;
    reader.seek(SeekFrom::Start(offset.into())).ok()?;

    let mut buffer = [0u8; 7];
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

    let endianness = if buffer[0] == b'P' {
        Endianness::Big
    } else {
        Endianness::Little
    };

    let (data_dirname, movies) = match version {
        ProjectorVersion::D3 => {
            let num_movies = LittleEndian::read_u16(&buffer);

            let mut movies: Vec<Movie> = Vec::new();
            for _ in 0..num_movies {
                // TODO: Read flag for whether or not we are doing embedded RIFF;
                // this is for non-embedded only
                let movie_size = reader.read_u32::<LittleEndian>().ok()?;
                let movie_filename = reader.read_pascal_str().ok()?;

                movies.push(Movie::External {
                    filename: movie_filename,
                    size: movie_size,
                });
            }

            (reader.read_pascal_str().ok()?, movies)
        },
        _ => {
            // TODO
            (String::new(), Vec::new())
        }
    };

    Some(FileType::Projector {
        name: String::from("TODO"),
        endianness,
        platform: Platform::Windows,
        version,
        data_dirname,
        movies,
    })
}

fn detect_mac(reader: &mut dyn Reader) -> Option<FileType> {
    // TODO: Maybe do not parse the entire thing into memory to do this…
    reader.seek(SeekFrom::Start(0)).ok()?;
    let mut rom_data = Vec::new();
    reader.read_to_end(&mut rom_data).ok()?;
    let rom = M68k::new(rom_data).ok()?;

    let version = if rom.get_resource("PJ95", 0).is_some() {
        ProjectorVersion::D5
    } else if rom.get_resource("PJ93", 0).is_some() {
        ProjectorVersion::D4
    } else if rom.get_resource("VWst", 0).is_some() {
        ProjectorVersion::D3
    } else {
        return None;
    };

    let has_external_data = {
        let os_type = if version == ProjectorVersion::D3 { "VWst" } else { "PJst" };
        rom.get_resource(os_type, 0)?.data[4] != 0
    };

    let mut movies = Vec::new();
    if has_external_data {
        // TODO
    } else {
        // TODO
        movies.push(Movie::Internal {
            offset: 0,
            size: 0,
        });
    }

    reader.seek(SeekFrom::Start(0x30)).ok()?;
    let name = reader.read_pascal_str().ok()?;

    Some(FileType::Projector {
        name,
        endianness: Endianness::Big,
        platform: Platform::Mac,
        version: version,
        data_dirname: String::new(),
        movies,
    })
}

fn detect_projector(reader: &mut dyn Reader) -> Option<FileType> {
    if let Some(file_type) = detect_win(reader) {
        return Some(file_type);
    }

    if let Some(file_type) = detect_mac(reader) {
        return Some(file_type);
    }

    None
}

pub fn detect_type(reader: &mut dyn Reader) -> Option<FileType> {
    if let Some(file_type) = detect_movie(reader) {
        return Some(file_type);
    }

    if let Some(file_type) = detect_projector(reader) {
        return Some(file_type);
    }

    None
}
