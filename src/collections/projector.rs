use byteorder::{BigEndian, ByteOrder, LittleEndian, ReadBytesExt};
use byteordered::ByteOrdered;
use crate::{OSType, Reader, ResourceId, collections::{riff, rsrc::MacResourceFile}, rsid, resources::resource, string::StringReadExt};
use encoding::all::{MAC_ROMAN, WINDOWS_1252};
use std::{path::PathBuf, io::{self, Cursor, SeekFrom}};

// TODO: Create an actual Projector type and do not expost this any more
#[derive(Debug)]
pub struct DetectionInfo {
    pub name: Option<String>,
    pub platform: Platform,
    pub version: ProjectorVersion,
    pub movies: Vec<Movie>,
}

#[derive(Debug)]
pub enum Movie {
    Embedded,
    Internal {
        offset: u32,
        size: u32
    },
    External(String),
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

    let version = if rom.contains(rsid!(b"PJ95", 0)) {
        ProjectorVersion::D5
    } else if rom.contains(rsid!(b"PJ93", 0)) {
        ProjectorVersion::D4
    } else if rom.contains(rsid!(b"VWst", 0)) {
        ProjectorVersion::D3
    } else {
        return None;
    };

    let has_external_data = {
        let os_type = if version == ProjectorVersion::D3 { b"VWst" } else { b"PJst" };
        rom.get(rsid!(os_type, 0))?.data().ok()?[4] != 0
    };

    let mut movies = Vec::new();
    if has_external_data {
        // TODO: Should parse this in Resource instead of pulling it from
        // the ROM and then pushing it into Resource?
        let external_files = rom.get(rsid!(b"STR#", 0))?;
        let cursor = ByteOrdered::be(Cursor::new(external_files.data().ok()?));
        for filename in resource::parse_string_list(cursor, MAC_ROMAN).ok()? {
            movies.push(Movie::External(filename.replace(':', "/")));
        }
    } else {
        // Embedded movies start at Resource ID 1024
        movies.push(Movie::Embedded);
    }

    Some(DetectionInfo {
        name: rom.name(),
        platform: Platform::Mac,
        version,
        movies,
    })
}

fn detect_win<T: Reader>(reader: &mut T) -> Option<DetectionInfo> {
    const MZ: u16 = 0x4d5a;
    reader.seek(SeekFrom::Start(0)).ok()?;
    if reader.read_u16::<BigEndian>().ok()? != MZ {
        return None;
    }

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

    let movies = match version {
        ProjectorVersion::D3 => {
            // Since we read 8 bytes above to make RIFF subtype detection easier
            // and the D3 header is actually 7 bytes, rewind once to get
            // realigned to the rest of the data
            reader.seek(SeekFrom::Current(-1))?;

            let num_movies = LittleEndian::read_u16(&buffer);

            // TODO: Somewhere in the projector header is probably a flag to
            // read movies internally; find it and then set the movie list
            // appropriately. For now the only corpus available has a single
            // external movie.
            let mut movies = Vec::new();
            for _ in 0..num_movies {
                let _size = reader.read_u32::<LittleEndian>().ok()?;
                let filename = {
                    // TODO: May need to CHARDET the path if it is non-ASCII
                    let filename = reader.read_pascal_str(WINDOWS_1252).ok()?;
                    let path = reader.read_pascal_str(WINDOWS_1252).ok()?;

                    let mut pathbuf = PathBuf::from(path);
                    pathbuf.push(filename);
                    pathbuf.to_string_lossy().to_string()
                };

                movies.push(Movie::External(filename));
            }

            movies
        },
        _ => {
            let offset = LittleEndian::read_u32(&buffer[4..]);
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
        name: get_exe_filename(reader),
        platform: Platform::Windows,
        version,
        movies,
    })
}

fn get_exe_filename<T: Reader>(input: &mut T) -> Option<String> {
    input.seek(SeekFrom::Start(0x3c)).ok()?;
    let exe_header_offset = input.read_u16::<LittleEndian>().ok()?;
    input.seek(SeekFrom::Start(u64::from(exe_header_offset))).ok()?;

    let signature = {
        let mut signature = [0u8; 4];
        input.read_exact(&mut signature).ok()?;
        signature
    };

    if signature == *b"PE\0\0" {
        pe::read_product_name(input)
    } else if signature[0..2] == *b"NE" {
        // 32 bytes from start of NE header, -4 since we consumed 4 bytes of
        // the header already
        input.seek(SeekFrom::Current(32 - 4)).ok()?;
        let non_resident_table_size = input.read_u16::<LittleEndian>().ok()?;
        // 44 bytes from start of NE header
        input.seek(SeekFrom::Current(44 - 32 - 2)).ok()?;
        let non_resident_table_offset = input.read_u32::<LittleEndian>().ok()?;

        if non_resident_table_size == 0 {
            None
        } else {
            input.seek(SeekFrom::Start(u64::from(non_resident_table_offset))).ok()?;
            Some(input.read_pascal_str(WINDOWS_1252).ok()?)
        }
    } else {
        None
    }
}

mod pe {
    use super::*;

    fn find_resource_segment_offset<T: Reader>(input: &mut T, num_sections: u16) -> Option<(u32, u32)> {
        for _ in 0..num_sections {
            let mut section = [0u8; 40];
            input.read_exact(&mut section).ok()?;
            if section[0..8] == *b".rsrc\0\0\0" {
                return Some((LittleEndian::read_u32(&section[12..]), LittleEndian::read_u32(&section[20..])))
            }
        }

        None
    }

    pub(super) fn read_product_name<T: Reader>(input: &mut T) -> Option<String> {
        const VERSION_INFO_TYPE: u32 = 0x10;
        const VERSION_INFO_ID: u32 = 1;
        const VERSION_INFO_LANG: u32 = 1033;

        let (virtual_address, from_offset) = seek_to_resource_segment(input).ok()?;
        seek_to_directory_entry(input, from_offset, VERSION_INFO_TYPE).ok()?;
        seek_to_directory_entry(input, from_offset, VERSION_INFO_ID).ok()?;
        seek_to_directory_entry(input, from_offset, VERSION_INFO_LANG).ok()?;
        seek_to_resource_data(input, virtual_address, from_offset).ok()?;
        read_version_struct(input).ok()?
    }

    fn read_version_struct<T: Reader>(input: &mut T) -> io::Result<Option<String>> {
        const FIXED_HEADER_WORD_SIZE: usize = 3;
        let start = input.seek(SeekFrom::Current(0))?;
        let size = input.read_u16::<LittleEndian>()?;
        let mut value_size = input.read_u16::<LittleEndian>()?;
        let is_text_data = input.read_u16::<LittleEndian>()? == 1;
        if is_text_data {
            value_size *= 2;
        }
        let value_padding = if value_size & 3 != 0 { 4 - (value_size & 3) } else { 0 };
        let end = start + u64::from(size) + u64::from(if size & 3 != 0 { 4 - (size & 3) } else { 0 });
        let key = input.read_utf16_c_str::<LittleEndian>()?;

        let key_padding_size = ((FIXED_HEADER_WORD_SIZE + key.len() + 1) & 1) * 2;
        if key_padding_size != 0 {
            input.seek(SeekFrom::Current(key_padding_size as i64))?;
        }

        let is_string_table = key == "StringFileInfo" || (key.len() == 8 && &key[4..8] == "04b0");

        match key.as_ref() {
            "ProductName" => Ok(Some(input.read_utf16_c_str::<LittleEndian>()?)),
            "VS_VERSION_INFO" => {
                input.seek(SeekFrom::Current(i64::from(value_size + value_padding)))?;
                read_version_struct(input)
            },
            _ if is_string_table => {
                while input.seek(SeekFrom::Current(0))? != end {
                    if let Ok(Some(value)) = read_version_struct(input) {
                        return Ok(Some(value));
                    }
                }
                Ok(None)
            },
            _ => {
                input.seek(SeekFrom::Start(end))?;
                Ok(None)
            }
        }
    }

    fn seek_to_directory_entry<T: Reader>(input: &mut T, from_offset: u32, id: u32) -> io::Result<()> {
        const ENTRY_SIZE: usize = 8;
        input.seek(SeekFrom::Current(12))?;
        let skip_entries = input.read_u16::<LittleEndian>()?;
        let num_entries = input.read_u16::<LittleEndian>()?;
        input.seek(SeekFrom::Current(i64::from(ENTRY_SIZE as u16 * skip_entries)))?;
        for _ in 0..num_entries {
            let mut entry = [0u8; ENTRY_SIZE];
            input.read_exact(&mut entry)?;
            let found_id = LittleEndian::read_u32(&entry);
            if found_id == id {
                const HAS_CHILDREN_FLAG: u32 = 0x8000_0000;
                let offset = LittleEndian::read_u32(&entry[4..]) & !HAS_CHILDREN_FLAG;
                input.seek(SeekFrom::Start(u64::from(from_offset + offset)))?;
                return Ok(());
            }
        }

        Err(io::ErrorKind::InvalidData.into())
    }

    fn seek_to_resource_data<T: Reader>(input: &mut T, virtual_address: u32, raw_offset: u32) -> io::Result<()> {
        let offset = input.read_u32::<LittleEndian>()?;
        input.seek(SeekFrom::Start(u64::from(offset - virtual_address + raw_offset)))?;
        Ok(())
    }

    fn seek_to_resource_segment<T: Reader>(input: &mut T) -> io::Result<(u32, u32)> {
        input.seek(SeekFrom::Current(2))?;
        let num_sections = input.read_u16::<LittleEndian>()?;
        input.seek(SeekFrom::Current(12))?;
        let optional_header_size = input.read_u16::<LittleEndian>()?;
        input.seek(SeekFrom::Current(2 + i64::from(optional_header_size)))?;
        let (virtual_address, offset) = find_resource_segment_offset(input, num_sections).ok_or_else(|| io::Error::from(io::ErrorKind::InvalidData))?;
        input.seek(SeekFrom::Start(u64::from(offset)))?;
        Ok((virtual_address, offset))
    }
}
