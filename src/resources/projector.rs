use byteorder::{ByteOrder, LittleEndian, ReadBytesExt};
use encoding::all::MAC_ROMAN;
use crate::{Endianness, OSType, os, resources::mac_resource_file::MacResourceFile, Reader, string::StringReadExt};
use std::io::SeekFrom;

#[derive(Debug)]
pub struct DetectionInfo {
    name: Option<String>,
    endianness: Endianness,
    platform: Platform,
    version: ProjectorVersion,
    data_dirname: String,
    movies: Vec<Movie>,
}

#[derive(Debug)]
pub enum Movie {
    Internal { offset: u32, size: u32 },
    External { filename: String, size: u32 },
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
    let mut rom = MacResourceFile::new(reader).ok()?;

    let version = if rom.contains(os!(b"PJ95"), 0) {
        ProjectorVersion::D5
    } else if rom.contains(os!(b"PJ95"), 0) {
        ProjectorVersion::D4
    } else if rom.contains(os!(b"PJ95"), 0) {
        ProjectorVersion::D3
    } else {
        return None;
    };

    let has_external_data = {
        let os_type = if version == ProjectorVersion::D3 { os!(b"VWst") } else { os!(b"PJst") };
        rom.get(os_type, 0)?.data[4] != 0
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

    Some(DetectionInfo {
        name: rom.name(),
        endianness: Endianness::Big,
        platform: Platform::Mac,
        version,
        data_dirname: String::new(),
        movies,
    })
}

fn detect_win<T: Reader>(reader: &mut T) -> Option<DetectionInfo> {
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
                let movie_filename = reader.read_pascal_str(MAC_ROMAN).ok()?;

                movies.push(Movie::External {
                    filename: movie_filename,
                    size: movie_size,
                });
            }

            (reader.read_pascal_str(MAC_ROMAN).ok()?, movies)
        },
        _ => {
            // TODO
            (String::new(), Vec::new())
        }
    };

    Some(DetectionInfo {
        name: Some(String::from("TODO")),
        endianness,
        platform: Platform::Windows,
        version,
        data_dirname,
        movies,
    })
}
