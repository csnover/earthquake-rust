use anyhow::{anyhow, bail, Context, Result as AResult};
use bitflags::bitflags;
use byteorder::{BigEndian, ByteOrder, LittleEndian, ReadBytesExt};
use byteordered::ByteOrdered;
use crate::{
    collections::riff,
    encodings::{DecoderRef, MAC_ROMAN, WIN_ROMAN},
    macos::ResourceFile,
    Reader,
    resources::{
        apple::version::Resource as VersionResource,
        resource,
    },
    rsid,
    SharedStream,
    string::StringReadExt,
};
use enum_display_derive::Display;
use std::{fmt::Display, path::PathBuf, io::{self, Cursor, Read, Seek, SeekFrom}};
use super::projector_settings::*;

#[derive(Debug)]
pub struct DetectionInfo<T: Reader> {
    name: Option<String>,
    string_decoder: Option<DecoderRef>,
    version: Version,
    movie: Movie<T>,
    config: ProjectorSettings,
}

impl<T: Reader> DetectionInfo<T> {
    #[must_use]
    pub fn movie(&self) -> &Movie<T> {
        &self.movie
    }

    #[must_use]
    pub fn name(&self) -> Option<&String> {
        self.name.as_ref()
    }

    #[must_use]
    pub fn version(&self) -> Version {
        self.version
    }

    #[must_use]
    pub fn config(&self) -> &ProjectorSettings {
        &self.config
    }
}

#[derive(Debug)]
pub struct D3WinMovie {
    pub filename: String,
    pub offset: u32,
    pub size: u32,
}

#[derive(Debug)]
pub enum Movie<T: Reader> {
    Embedded(u16),
    D3Win(Vec<D3WinMovie>),
    // The offset of an internal movie needs to be stored separately from the
    // stream because there are offsets inside a RIFF which are absolute to the
    // beginning of the file, not the RIFF block, so the stream needs to be the
    // entire “file”, which might actually be embedded inside of a MacBinary or
    // AppleSingle file.
    Internal { stream: SharedStream<T>, offset: u32, size: u32 },
    External(Vec<String>),
}

#[derive(Debug, Copy, Clone, PartialEq)]
pub enum WinVersion {
    Win3,
    Win95,
}

bitflags! {
    pub struct MacCPU: u8 {
        const M68K = 1;
        const PPC  = 2;
        const ANY  = Self::M68K.bits | Self::PPC.bits;
    }
}

#[derive(Debug, Copy, Clone, PartialEq)]
pub enum Platform {
    Mac(MacCPU),
    Win(WinVersion),
}

#[derive(Debug, Display, Copy, Clone, PartialEq, PartialOrd)]
pub enum Version {
    D3,
    D4,
    D5,
    D6,
    D7,
}

pub fn detect_mac<T: Reader, U: Reader>(resource_fork: &mut T, data_fork: Option<&mut SharedStream<U>>) -> AResult<DetectionInfo<U>> {
    let rom = ResourceFile::new(resource_fork)?;

    let version = if rom.contains(rsid!(b"PJ97", 0)) && rom.contains(rsid!(b"PJst", 0)) {
        Version::D6
    } else if rom.contains(rsid!(b"PJ95", 0)) && rom.contains(rsid!(b"PJst", 0)) {
        Version::D5
    } else if rom.contains(rsid!(b"PJ93", 0)) && rom.contains(rsid!(b"PJst", 0)) {
        Version::D4
    } else if rom.contains(rsid!(b"VWst", 0)) {
        Version::D3
    } else {
        bail!("No Mac projector settings resource");
    };

    let config = {
        let os_type = if version == Version::D3 { b"VWst" } else { b"PJst" };
        let resource_id = rsid!(os_type, 0);
        rom.get(resource_id).unwrap().data()?
    };

    let string_decoder = {
        let id = rsid!(b"vers", 1);
        let vers = rom.get(id)
            .ok_or_else(|| anyhow!("Missing {}", id))?
            .data()?;
        let vers = VersionResource::parse(&mut ByteOrdered::be(std::io::Cursor::new(vers)))?;
        Some(vers.country_code().encoding())
    };

    let (config, movie) = match version {
        Version::D3 => {
            let has_external_data = config[4] != 0;
            let num_movies = BigEndian::read_u16(&config[6..]);
            let config = ProjectorSettings::parse_mac(version, &config)?;
            if has_external_data {
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
                (config, Movie::External(movies))
            } else {
                // Embedded movies start at Resource ID 1024
                (config, Movie::Embedded(num_movies))
            }
        },
        Version::D4 | Version::D5 | Version::D6 => {
            if let Some(data_fork) = data_fork {
                // TODO: Seems like some pre-release version of Director 4
                // created projectors with no PJxx in the data fork. In these
                // ones the CPU flag appears to always be zero. Then before GM
                // they added the CPU flag and PJxx in the data fork. Based on
                // the corresponding structure in the Windows projectors, this
                // extra data is probably:
                //
                // 4 - "PJxx"
                // 4 - RIFF offset
                //
                // and then different by version:
                //
                // D4 (PJ93):
                // 4x9 - fixed driver offsets?
                // <PPC executable>
                //
                // D5+ (PJ95, PJ97, PJ00, etc.):
                // 4  - num drivers
                // 4  - num drivers to skip
                // .. - drivers
                // <PPC executable>
                let has_extended_data_fork = config[7] != 0;
                let config = ProjectorSettings::parse_mac(version, &config)?;
                if has_extended_data_fork {
                    let mut buffer = [ 0; 8 ];
                    data_fork.read_exact(&mut buffer).context("Can’t read Projector header")?;
                    let data_version = data_version(&buffer[0..4]);
                    let mismatch = match data_version {
                        Some(Version::D5) => version != Version::D5 && version != Version::D6,
                        Some(data_version) => version != data_version,
                        None => true,
                    };

                    if mismatch {
                        bail!(
                            "Projector data fork version {} does not match resource fork version {}",
                            data_version.map_or_else(|| "None".to_string(), |v| format!("{}", v)),
                            version
                        );
                    }

                    let riff_offset = BigEndian::read_u32(&buffer[4..]);
                    (config, internal_movie(data_fork, riff_offset)?)
                } else {
                    (config, internal_movie(data_fork, 0)?)
                }
            } else {
                bail!("No data fork; can’t get offset of internal movie");
            }
        },
        Version::D7 => todo!(),
    };

    Ok(DetectionInfo {
        name: rom.name(),
        string_decoder,
        version,
        movie,
        config,
    })
}

fn d3_win_movie_info<T: Reader>(input: &mut T, i: u16) -> AResult<(u32, String)> {
    let size = input.read_u32::<LittleEndian>()
        .with_context(|| format!("Can’t read external movie {} size", i))?;
    let filename = {
        // TODO: May need to CHARDET the path if it is non-ASCII
        let filename = input.read_pascal_str(WIN_ROMAN)
            .with_context(|| format!("Can’t read external movie {} filename", i))?;
        let path = input.read_pascal_str(WIN_ROMAN)
            .with_context(|| format!("Can’t read external movie {} path", i))?;

        let mut pathbuf = PathBuf::from(path.replace('\\', "/"));
        pathbuf.push(filename);
        pathbuf.to_string_lossy().to_string()
    };
    Ok((size, filename))
}

pub fn detect_win<T: Reader>(mut input: &mut SharedStream<T>) -> AResult<DetectionInfo<T>> {
    const MZ: u16 = 0x4d5a;
    const HEADER_SIZE: u32 = 8;
    if input.read_u16::<BigEndian>().context("Can’t read magic")? != MZ {
        bail!("Not a Windows executable");
    }

    input.seek(SeekFrom::End(-4)).context("Can’t seek to Director offset")?;
    let offset = input.read_u32::<LittleEndian>().context("Can’t read Director offset")?;
    input.seek(SeekFrom::Start(u64::from(offset))).context("Bad Director data offset")?;

    let mut header = [ 0; HEADER_SIZE as usize ];
    input.read_exact(&mut header).context("Can’t read Projector header")?;

    let mut version = if let Some(version) = data_version(&header[0..4]) {
        version
    } else {
        let checksum: u8 = header[0]
            .wrapping_add(header[1])
            .wrapping_add(header[2])
            .wrapping_add(header[3])
            .wrapping_add(header[4])
            .wrapping_add(header[5])
            .wrapping_add(header[6]);

        if checksum != 0 {
            bail!("Bad Director 3 for Windows checksum");
        }

        Version::D3
    };

    let (platform, name) = get_exe_info(input)?;
    let (config, movie) = if version == Version::D3 {
        input.seek(SeekFrom::Start(u64::from(offset + 7)))?;
        let config = ProjectorSettings::parse_win(version, platform, &header[0..7])?;
        let num_movies = LittleEndian::read_u16(&header);
        let movie = if config.d3().unwrap().use_external_files() {
            let mut movies = Vec::with_capacity(usize::from(num_movies));
            for i in 0..num_movies {
                let (_, filename) = d3_win_movie_info(&mut input, i)?;
                movies.push(filename);
            }
            Movie::External(movies)
        } else {
            let mut movies = Vec::with_capacity(usize::from(num_movies));
            for i in 0..num_movies {
                let (size, filename) = d3_win_movie_info(&mut input, i)?;
                let offset = input.seek(SeekFrom::Current(0))? as u32;
                movies.push(D3WinMovie {
                    filename,
                    offset,
                    size,
                });
            }
            Movie::D3Win(movies)
        };

        (config, movie)
    } else {
        input.seek(SeekFrom::Start(u64::from(offset + 8)))
            .context("Can’t seek to Projector settings")?;

        let settings_offset = match version {
            Version::D3 => unreachable!(),
            Version::D4 => {
                // A Cidade Virtual has more stuff in PJ93 than other samples
                // in the corpus, so it is not possible to just walk forward by
                // a fixed amount
                const SETTINGS_SIZE: u32 = 12;
                let end_offset = input.read_u32::<LittleEndian>()
                    .context("Can’t read offset of first system file")?;
                end_offset - SETTINGS_SIZE
            },
            Version::D5 | Version::D6 => {
                offset + HEADER_SIZE
            },
            Version::D7 => todo!(),
        };

        input.seek(SeekFrom::Start(u64::from(settings_offset)))
            .context("Can’t seek to Projector settings data")?;

        let mut buffer = [ 0; 12 ];
        input.read_exact(&mut buffer).context("Can’t read Projector settings data")?;

        // TODO: Maybe there is a better way to differentiate between D5 and D6,
        // but they use the same data version magic number so the check must
        // occur late in the process. (Not only do they use the same magic
        // number, the Win3 projectors are still named “Release 5.0”)
        if buffer[0] & 0x10 == 0 {
            version = Version::D6;
        }

        (ProjectorSettings::parse_win(version, platform, &buffer)?, internal_movie(input, LittleEndian::read_u32(&header[4..]))?)
    };

    Ok(DetectionInfo {
        name,
        // TODO: Detect the character encoding.
        string_decoder: None,
        version,
        movie,
        config,
    })
}

fn data_version(raw_version: &[u8]) -> Option<Version> {
    match &raw_version[0..4] {
        b"PJ93" | b"39JP" => Some(Version::D4),
        b"PJ95" | b"59JP" => Some(Version::D5),
        // Director 6 has PJ95 data on both Mac and Win, but PJ97 resource on
        // Mac, so this test only works for Mac
        b"PJ97" | b"79JP" => Some(Version::D6),
        b"PJ00" | b"00JP" => Some(Version::D7),
        _ => None
    }
}

fn get_exe_info<T: Reader>(input: &mut T) -> AResult<(Platform, Option<String>)> {
    input.seek(SeekFrom::Start(0x3c))?;
    let exe_header_offset = input.read_u16::<LittleEndian>()?;
    input.seek(SeekFrom::Start(u64::from(exe_header_offset)))?;

    let signature = {
        let mut signature = [ 0; 4 ];
        input.read_exact(&mut signature)?;
        signature
    };

    if signature == *b"PE\0\0" {
        Ok((Platform::Win(WinVersion::Win95), pe::read_product_name(input)))
    } else if signature[0..2] == *b"NE" {
        // 32 bytes from start of NE header, -4 since we consumed 4 bytes of
        // the header already
        input.seek(SeekFrom::Current(32 - 4))?;
        let non_resident_table_size = input.read_u16::<LittleEndian>()?;
        // 44 bytes from start of NE header
        input.seek(SeekFrom::Current(44 - 32 - 2))?;
        let non_resident_table_offset = input.read_u32::<LittleEndian>()?;

        if non_resident_table_size == 0 {
            Ok((Platform::Win(WinVersion::Win3), None))
        } else {
            input.seek(SeekFrom::Start(u64::from(non_resident_table_offset)))?;
            Ok((Platform::Win(WinVersion::Win3), Some(input.read_pascal_str(WIN_ROMAN)?)))
        }
    } else {
        Err(anyhow!("Not a Windows executable"))
    }
}

fn internal_movie<T: Reader>(reader: &mut SharedStream<T>, offset: u32) -> AResult<Movie<T>> {
    reader.seek(SeekFrom::Start(u64::from(offset)))
        .with_context(|| format!("Bad RIFF offset {}", offset))?;
    let info = riff::detect(reader).with_context(|| format!("Can’t detect RIFF at {}", offset))?;
    Ok(Movie::Internal {
        stream: reader.clone(),
        offset,
        size: info.size()
    })
}

mod pe {
    use super::*;

    fn find_resource_segment_offset<T: Reader>(input: &mut T, num_sections: u16) -> Option<(u32, u32)> {
        for _ in 0..num_sections {
            let mut section = [ 0; 40 ];
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
        let value_padding = if value_size & 3 == 0 { 0 } else { 4 - (value_size & 3) };
        let end = start + u64::from(size) + u64::from(if size & 3 == 0 { 0 } else { 4 - (size & 3) });
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
            let mut entry = [ 0; ENTRY_SIZE ];
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
