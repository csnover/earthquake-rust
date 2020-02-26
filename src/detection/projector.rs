use anyhow::{anyhow, Context, Result as AResult};
use byteorder::{BigEndian, ByteOrder, LittleEndian, ReadBytesExt};
use byteordered::ByteOrdered;
use crate::{OSType, Reader, ResourceId, collections::riff, macos::MacResourceFile, rsid, resources::resource, string::StringReadExt};
use encoding::all::{MAC_ROMAN, WINDOWS_1252};
use enum_display_derive::Display;
use std::{fmt::Display, path::PathBuf, io::{self, Cursor, SeekFrom}};

#[derive(Debug)]
pub struct DetectionInfo {
    name: Option<String>,
    platform: Platform,
    version: ProjectorVersion,
    movie: Movie,
}

impl DetectionInfo {
    pub fn movie(&self) -> &Movie {
        &self.movie
    }

    pub fn name(&self) -> Option<&String> {
        self.name.as_ref()
    }

    pub fn platform(&self) -> Platform {
        self.platform
    }

    pub fn version(&self) -> ProjectorVersion {
        self.version
    }
}

#[derive(Debug)]
pub enum Movie {
    Embedded(u16),
    Internal { offset: u32, size: u32 },
    External(Vec<String>),
}

#[derive(Debug, Copy, Clone, PartialEq)]
pub enum Platform {
    Windows,
    Mac,
}

#[derive(Debug, Display, Copy, Clone, PartialEq, PartialOrd)]
pub enum ProjectorVersion {
    D3,
    D4,
    D5,
    D7,
}

pub fn detect_mac<T: Reader, U: Reader>(resource_fork: &mut T, data_fork: Option<&mut U>) -> AResult<DetectionInfo> {
    let rom = MacResourceFile::new(resource_fork)?;

    let version = if rom.contains(rsid!(b"PJ95", 0)) && rom.contains(rsid!(b"PJst", 0)) {
        ProjectorVersion::D5
    } else if rom.contains(rsid!(b"PJ93", 0)) && rom.contains(rsid!(b"PJst", 0)) {
        ProjectorVersion::D4
    } else if rom.contains(rsid!(b"VWst", 0)) {
        ProjectorVersion::D3
    } else {
        return Err(anyhow!("No Mac projector settings resource"));
    };

    let config = {
        let os_type = if version == ProjectorVersion::D3 { b"VWst" } else { b"PJst" };
        let resource_id = rsid!(os_type, 0);
        rom.get(resource_id).unwrap().data()?
    };

    let has_external_data = config[4] != 0;
    let num_movies = BigEndian::read_u16(&config[6..]);

    let movie = if has_external_data {
        // TODO: Should parse this in Resource instead of pulling it from
        // the ROM and then pushing it into Resource?
        let resource_id = rsid!(b"STR#", 0);
        let external_files = rom.get(resource_id).ok_or_else(|| anyhow!("Missing external file list"))?;
        let cursor = ByteOrdered::be(Cursor::new(external_files.data().with_context(|| format!("Can’t read {}", resource_id))?));
        // TODO: May need to CHARDET the paths
        let mut movies = Vec::with_capacity(usize::from(num_movies));
        for filename in resource::parse_string_list(cursor, MAC_ROMAN)? {
            movies.push(filename.replace(':', "/"));
        }
        Movie::External(movies)
    } else if version == ProjectorVersion::D3 {
        // Embedded movies start at Resource ID 1024
        Movie::Embedded(num_movies)
    } else if let Some(data_fork) = data_fork {
        // TODO: Figure out WTF is going on with this; AMBER has a non-zero
        // embedded movie value (1 or 2) and a PJ93 chunk in the data fork;
        // others like JMP which have a movie clunt of 0 have no PJ93 in the
        // data fork. This is probably a wrong test since it is a hack guess!
        if num_movies != 0 {
            let mut buffer = [0u8; 8];
            data_fork.read_exact(&mut buffer).context("Can’t read Projector header")?;
            let data_version = data_version(&buffer[0..4]);
            if data_version.is_none() || data_version.unwrap() != version {
                return Err(anyhow!(
                    "Projector data fork version {} does not match resource fork version {}",
                    data_version.map_or_else(|| "None".to_string(), |v| format!("{}", v)),
                    version
                ));
            }
            internal_movie(BigEndian::read_u32(&buffer[4..]), data_fork)?
        } else {
            internal_movie(0, data_fork)?
        }
    } else {
        return Err(anyhow!("Missing data fork; can’t get offset of internal movie"));
    };

    Ok(DetectionInfo {
        name: rom.name(),
        platform: Platform::Mac,
        version,
        movie,
    })
}

pub fn detect_win<T: Reader>(reader: &mut T) -> AResult<DetectionInfo> {
    const MZ: u16 = 0x4d5a;
    if reader.read_u16::<BigEndian>().context("Can’t read magic")? != MZ {
        return Err(anyhow!("Not a Windows executable"));
    }

    reader.seek(SeekFrom::End(-4)).context("Can’t seek to Director offset")?;
    let offset = reader.read_u32::<LittleEndian>().context("Can’t read Director offset")?;
    reader.seek(SeekFrom::Start(offset.into())).context("Bad Director data offset")?;

    let mut buffer = [0u8; 8];
    reader.read_exact(&mut buffer).context("Can’t read Projector header")?;

    let version = match data_version(&buffer[0..4]) {
        Some(version) => version,
        _ => {
            let checksum: u8 = buffer[0]
                .wrapping_add(buffer[1])
                .wrapping_add(buffer[2])
                .wrapping_add(buffer[3])
                .wrapping_add(buffer[4])
                .wrapping_add(buffer[5])
                .wrapping_add(buffer[6]);

            if checksum != 0 {
                return Err(anyhow!("Bad Director 3 for Windows checksum"));
            }

            ProjectorVersion::D3
        },
    };

    let movie = match version {
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
            let mut movies = Vec::with_capacity(usize::from(num_movies));
            for i in 0..num_movies {
                let _size = reader.read_u32::<LittleEndian>()
                    .with_context(|| format!("Can’t read external movie {} size", i))?;
                let filename = {
                    // TODO: May need to CHARDET the path if it is non-ASCII
                    let filename = reader.read_pascal_str(WINDOWS_1252)
                        .with_context(|| format!("Can’t read external movie {} filename", i))?;
                    let path = reader.read_pascal_str(WINDOWS_1252)
                        .with_context(|| format!("Can’t read external movie {} path", i))?;

                    let mut pathbuf = PathBuf::from(path.replace('\\', "/"));
                    pathbuf.push(filename);
                    pathbuf.to_string_lossy().to_string()
                };

                movies.push(filename);
            }

            Movie::External(movies)
        },
        _ => internal_movie(LittleEndian::read_u32(&buffer[4..]), reader)?
    };

    Ok(DetectionInfo {
        name: get_exe_filename(reader),
        platform: Platform::Windows,
        version,
        movie,
    })
}

fn data_version(raw_version: &[u8]) -> Option<ProjectorVersion> {
    match &raw_version[0..4] {
        b"PJ93" | b"39JP" => Some(ProjectorVersion::D4),
        b"PJ95" | b"59JP" => Some(ProjectorVersion::D5),
        b"PJ00" | b"00JP" => Some(ProjectorVersion::D7),
        _ => None
    }
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

fn internal_movie<T: Reader>(offset: u32, reader: &mut T) -> AResult<Movie> {
    reader.seek(SeekFrom::Start(u64::from(offset)))
        .with_context(|| format!("Bad RIFF offset {}", offset))?;
    let info = riff::detect(reader).with_context(|| format!("Can’t detect RIFF at {}", offset))?;

    Ok(Movie::Internal {
        offset,
        size: info.size()
    })
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

    fn read_version_struct<T: Reader>(input: &mut T) -> AResult<Option<String>> {
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
