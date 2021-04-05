use anyhow::{Context, Result as AResult, anyhow, bail, ensure};
use binrw::{BinRead, io::{Cursor, Read, SeekFrom}};
use bitflags::bitflags;
use bstr::ByteSlice;
use byteorder::{BigEndian, ByteOrder, LittleEndian, ReadBytesExt};
use crate::{
    collections::riff::detect as detect_riff,
    panic_sample,
};
use derive_more::Display;
use libcommon::SeekExt;
use libmactoolbox::{resources::{File as ResourceFile, ResourceId, Source as ResourceSource, kinds::StringList as StringListResource}, intl::ScriptCode, types::{MacString, PString}};
use std::{convert::TryInto, rc::Rc};
use super::{projector_settings::ProjectorSettings, Version};

#[derive(Clone)]
pub struct DetectionInfo {
    name: Option<MacString>,
    charset: Option<ScriptCode>,
    version: Version,
    movie: Movie,
    system_resources: Option<Vec<u8>>,
    config: ProjectorSettings,
}

impl std::fmt::Debug for DetectionInfo {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct(std::any::type_name::<Self>())
            .field("name", &self.name)
            .field("charset", &self.charset)
            .field("version", &self.version)
            .field("movie", &self.movie)
            .field("system_resources", &self.system_resources.as_ref().map(Vec::len))
            .field("config", &self.config)
            .finish()
    }
}

impl DetectionInfo {
    #[must_use]
    pub fn config(&self) -> &ProjectorSettings {
        &self.config
    }

    #[must_use]
    pub fn charset(&self) -> Option<ScriptCode> {
        self.charset
    }

    #[must_use]
    pub fn is_mac_embedded(&self) -> bool {
        matches!(self.movie, Movie::Embedded(_))
    }

    #[must_use]
    pub fn movie(&self) -> &Movie {
        &self.movie
    }

    #[must_use]
    pub fn name(&self) -> Option<&MacString> {
        self.name.as_ref()
    }

    #[must_use]
    pub fn system_resources(&self) -> Option<&Vec<u8>> {
        self.system_resources.as_ref()
    }

    #[must_use]
    pub fn version(&self) -> Version {
        self.version
    }
}

#[derive(Clone, Debug)]
pub struct D3WinMovie {
    pub filename: MacString,
    pub offset: u32,
    pub size: u32,
}

#[derive(Clone, Debug)]
pub enum Movie {
    /// The number of movies embedded as resources in a Director 3 Mac
    /// projector.
    Embedded(u16),
    /// Movies embedded in a Director 3 Windows projector.
    D3Win(Vec<D3WinMovie>),
    /// The offset of a RIFF container embedded in a Director 4+ projector.
    Internal(u32),
    /// External movies referenced by a Director 3 projector.
    External(Vec<MacString>),
}

#[derive(Clone, Copy, Debug, Display, PartialEq)]
pub enum WinVersion {
    #[display(fmt = "3")]
    Win3,
    #[display(fmt = "95")]
    Win95,
}

bitflags! {
    pub struct MacCPU: u8 {
        const M68K = 1;
        const PPC  = 2;
        const ANY  = Self::M68K.bits | Self::PPC.bits;
    }
}

impl std::fmt::Display for MacCPU {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match *self {
            MacCPU::M68K => write!(f, "68000"),
            MacCPU::PPC => write!(f, "PowerPC"),
            MacCPU::ANY => write!(f, "68000/PowerPC"),
            _ => unreachable!("there are no other CPU types"),
        }
    }
}

#[derive(Debug, Display, Clone, Copy, PartialEq)]
pub enum Platform {
    #[display(fmt = "Mac {}", _0)]
    Mac(MacCPU),
    #[display(fmt = "Windows {}", _0)]
    Win(WinVersion),
}

pub fn detect_mac<R1, R2>(mut resource_fork: R1, data_fork: Option<R2>) -> AResult<DetectionInfo>
where
    R1: binrw::io::Read + binrw::io::Seek,
    R2: binrw::io::Read + binrw::io::Seek,
{
    let resource_fork_offset = resource_fork.pos().context("Can’t read resource fork position")?;
    let rom = ResourceFile::new(resource_fork)?;

    let version = if rom.contains(ResourceId::new(b"PJ97", 0)) && rom.contains(ResourceId::new(b"PJst", 0)) {
        Version::D6
    } else if rom.contains(ResourceId::new(b"PJ95", 0)) && rom.contains(ResourceId::new(b"PJst", 0)) {
        Version::D5
    } else if rom.contains(ResourceId::new(b"PJ93", 0)) && rom.contains(ResourceId::new(b"PJst", 0)) {
        Version::D4
    } else if rom.contains(ResourceId::new(b"VWst", 0)) {
        Version::D3
    } else {
        bail!("No Mac projector settings resource");
    };

    let config = {
        let os_type = if version == Version::D3 { b"VWst" } else { b"PJst" };
        let resource_id = ResourceId::new(os_type, 0);
        rom.load_args::<ProjectorSettings>(resource_id, (version, Platform::Mac(MacCPU::ANY)))?
    };

    let movie = match version {
        Version::D3 => {
            if config.use_external_files() {
                let movies = rom.load::<StringListResource>(ResourceId::new(b"STR#", 0))
                    .context("Missing external file list")?;
                let mut movies = Rc::try_unwrap(movies)
                    .map_err(|_| anyhow!("Could not take ownership of movie list"))?;
                for filename in &mut movies {
                    *filename = filename.replace(b":", b"/").into();
                }
                Movie::External(movies.into_iter().map(MacString::Raw).collect::<Vec::<_>>())
            } else {
                // Embedded movies start at Resource ID 1024
                Movie::Embedded(config.num_movies())
            }
        },
        Version::D4 | Version::D5 | Version::D6 => {
            if let Some(mut data_fork) = data_fork {
                if config.has_extended_data_fork() {
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
                            data_version.map_or_else(|| "None".to_string(), |v| v.to_string()),
                            version
                        );
                    }

                    let riff_offset = BigEndian::read_u32(&buffer[4..]);
                    internal_movie(&mut data_fork, riff_offset)?
                } else {
                    internal_movie(&mut data_fork, 0)?
                }
            } else {
                bail!("No data fork; can’t get offset of internal movie");
            }
        },
        Version::D7 => todo!("D7Mac projector detection"),
    };

    let name = rom.name();

    let system_resources = if version == Version::D3 {
        None
    } else {
        let mut rom_data = rom.into_inner();
        rom_data.seek(SeekFrom::Start(resource_fork_offset)).context("Can’t rewind resource fork for system resource data")?;
        let mut data = Vec::with_capacity(rom_data.bytes_left()?.try_into().unwrap());
        rom_data.read_to_end(&mut data).context("Can’t read system resource fork data")?;
        Some(data)
    };

    Ok(DetectionInfo {
        name,
        // TODO: Detect the character encoding. Reading the file creator name
        // from VWFI might be the best way to do this.
        charset: None,
        version,
        movie,
        system_resources,
        config: *config,
    })
}

fn d3_win_movie_info<R: binrw::io::Read + binrw::io::Seek>(input: &mut R, i: u16) -> AResult<(u32, MacString)> {
    let size = input.read_u32::<LittleEndian>()
        .with_context(|| format!("Can’t read movie {} size", i))?;
    let filename = {
        let filename = PString::read(input)
            .with_context(|| format!("Can’t read movie {} filename", i))?;
        let path = PString::read(input)
            .with_context(|| format!("Can’t read movie {} path", i))?;

        let mut path = path.replace(b"\\", b"/");
        path.push(b'/');
        path.extend_from_slice(filename.as_bytes());
        MacString::Raw(PString::from(path))
    };
    Ok((size, filename))
}

const HEADER_SIZE: u32 = 8;
const SETTINGS_SIZE: u32 = 12;

pub fn detect_win<R: binrw::io::Read + binrw::io::Seek>(input: &mut R) -> AResult<DetectionInfo> {
    const MZ: u16 = 0x4d5a;

    if input.read_u16::<BigEndian>().context("Can’t read magic")? != MZ {
        bail!("Not a Windows executable");
    }

    input.seek(SeekFrom::End(-4)).context("Can’t seek to Director offset")?;
    let offset = input.read_u32::<LittleEndian>().context("Can’t read Director offset")?;
    input.seek(SeekFrom::Start(offset.into())).context("Bad Director data offset")?;

    let mut header = [ 0; HEADER_SIZE as usize ];
    input.read_exact(&mut header).context("Can’t read Projector header")?;

    let mut version = if let Some(version) = data_version(&header[0..4]) {
        version
    } else {
        let checksum = header[0..7].iter().fold(0_u8, |c, &v| c.wrapping_add(v));

        if checksum != 0 {
            bail!("Bad Director 3 for Windows checksum");
        }

        Version::D3
    };

    let (platform, name) = get_exe_info(input)?;
    let (config, movie, system_resources) = if version == Version::D3 {
        input.seek(SeekFrom::Start((offset + 7).into()))?;
        let config = ProjectorSettings::read_args(
            &mut Cursor::new(header),
            (version, Platform::Win(WinVersion::Win3))
        )?;
        let num_movies = config.num_movies();
        let movie = if config.use_external_files() {
            let mut movies = Vec::with_capacity(num_movies.into());
            for i in 0..num_movies {
                let (_, filename) = d3_win_movie_info(input.by_ref(), i)?;
                movies.push(filename);
            }
            Movie::External(movies)
        } else {
            let mut movies = Vec::with_capacity(num_movies.into());
            for i in 0..num_movies {
                let (size, filename) = d3_win_movie_info(input.by_ref(), i)?;
                let offset: u32 = input.pos()?.try_into()?;
                movies.push(D3WinMovie {
                    filename,
                    offset,
                    size,
                });
                input.skip(size.into())
                    .with_context(|| format!("Can’t skip to internal movie {}", i + 1))?;
            }
            Movie::D3Win(movies)
        };

        (config, movie, None)
    } else {
        input.seek(SeekFrom::Start((offset + HEADER_SIZE).into()))
            .context("Can’t seek to Projector settings")?;

        let settings_offset = match version {
            Version::D3 => unreachable!("D3 has incompatible projector settings and is parsed separately"),
            Version::D4 => {
                // A Cidade Virtual has more stuff in PJ93 than other samples
                // in the corpus, so it is not possible to just walk forward by
                // a fixed amount
                let end_offset = input.read_u32::<LittleEndian>()
                    .context("Can’t read offset of first system file")?;
                end_offset - SETTINGS_SIZE
            },
            Version::D5 | Version::D6 | Version::D7 => {
                offset + HEADER_SIZE
            },
        };

        input.seek(SeekFrom::Start(settings_offset.into()))
            .context("Can’t seek to Projector settings data")?;

        // SETTINGS_SIZE for D7 is actually only 8
        let mut buffer = [ 0; SETTINGS_SIZE as usize ];
        input.read_exact(&mut buffer).context("Can’t read Projector settings data")?;

        // TODO: Maybe there is a better way to differentiate between D5 and D6,
        // but they use the same data version magic number so the check must
        // occur late in the process. (Not only do they use the same magic
        // number, the Win3 projectors are still named “Release 5.0”)
        if version == Version::D5 && buffer[0] & 0x10 == 0 {
            version = Version::D6;
        }

        (
            ProjectorSettings::read_args(&mut Cursor::new(buffer), (version, platform))?,
            internal_movie(input, LittleEndian::read_u32(&header[4..]))?,
            get_projector_rsrc(input, offset, version)?
        )
    };

    Ok(DetectionInfo {
        name,
        // TODO: Detect the character encoding.
        charset: None,
        version,
        movie,
        system_resources,
        config,
    })
}

fn data_version(raw_version: &[u8]) -> Option<Version> {
    match &raw_version[0..4] {
        b"PJ93" | b"39JP" => Some(Version::D4),
        b"PJ95" | b"59JP" => Some(Version::D5),
        // Director 6 uses "PJ95" for the data version on both Mac and Win,
        // even though it has incompatible settings
        b"PJ97" | b"79JP" => panic_sample!("PJ97 in data fork"),
        b"PJ00" | b"00JP" => Some(Version::D7),
        _ => None
    }
}

fn get_exe_info<R: binrw::io::Read + binrw::io::Seek>(input: &mut R) -> AResult<(Platform, Option<MacString>)> {
    input.seek(SeekFrom::Start(0x3c))?;
    let exe_header_offset = input.read_u16::<LittleEndian>()?;
    input.seek(SeekFrom::Start(exe_header_offset.into()))?;

    let signature = {
        let mut signature = [ 0; 4 ];
        input.read_exact(&mut signature)?;
        signature
    };

    if signature == *b"PE\0\0" {
        Ok((Platform::Win(WinVersion::Win95), pe::read_product_name(input).map(<_>::into)))
    } else if signature[0..2] == *b"NE" {
        // 32 bytes from start of NE header, -4 since we consumed 4 bytes of
        // the header already
        input.skip(32 - 4)?;
        let non_resident_table_size = input.read_u16::<LittleEndian>()?;
        // 44 bytes from start of NE header
        input.skip(44 - 32 - 2)?;
        let non_resident_table_offset = input.read_u32::<LittleEndian>()?;

        if non_resident_table_size == 0 {
            Ok((Platform::Win(WinVersion::Win3), None))
        } else {
            input.seek(SeekFrom::Start(non_resident_table_offset.into()))?;
            Ok((Platform::Win(WinVersion::Win3), Some(PString::read(input)?.into())))
        }
    } else {
        bail!("Not a Windows executable")
    }
}

fn get_projector_rsrc<R: binrw::io::Read + binrw::io::Seek>(input: &mut R, offset: u32, version: Version) -> AResult<Option<Vec<u8>>> {
    let (rsrc_offset, rsrc_size) = match version {
        Version::D3 => unreachable!("D3 does not include a system resource file"),
        Version::D4 => {
            input.seek(SeekFrom::Start((offset + HEADER_SIZE + 8).into()))
                .context("Can’t seek to PROJECTR.RSR offset")?;
            let rsrc_offset = input.read_u32::<LittleEndian>()
                .context("Can’t read PROJECTR.RSR offset")?;
            let next_offset = input.read_u32::<LittleEndian>()
                .context("Can’t read fourth system file offset")?;
            let size = next_offset - rsrc_offset;
            (rsrc_offset, size)
        },
        Version::D5 | Version::D6 => {
            const DRIVERS_HEADER_SIZE: u32 = 12;
            let driver_entry_size = if version == Version::D5 { 0x204 } else { 0x208 };

            input.seek(SeekFrom::Start((offset + HEADER_SIZE + SETTINGS_SIZE + DRIVERS_HEADER_SIZE + driver_entry_size * 2).into()))
                .context("Can’t seek to PROJECTR.RSR offset")?;
            let rsrc_offset = input.read_u32::<LittleEndian>()
                .context("Can’t read PROJECTOR.RSR offset")?;
            let size = if version == Version::D5 {
                input.skip((driver_entry_size - 4).into())
                    .context("Can’t skip to fourth system file offset")?;
                let next_offset = input.read_u32::<LittleEndian>()
                    .context("Can’t read fourth system file offset")?;
                next_offset - rsrc_offset
            } else {
                input.read_u32::<LittleEndian>()
                    .context("Can’t read PROJECTOR.RSR size")?
            };
            (rsrc_offset, size)
        },
        Version::D7 => {
            #[allow(dead_code)]
            const DRIVERS_HEADER_SIZE: u32 = 8;
            // SETTINGS_SIZE here is actually only 8

            // Driver entries here are:
            // 0x4 offset, 0x4 size, 0x21 basename, 0xb ext
            //
            // There is no PROJECTR.RSR in D7 -- resources are now native PE
            // resources inside the embedded DLLs. So figure out how to make
            // that work, ha ha ugh.
            #[allow(dead_code)]
            const DRIVER_ENTRY_SIZE: u32 = 0x3c;
            todo!("D7 projector system resource loading")
        },
    };

    let mut system_resources = Vec::with_capacity(rsrc_size.try_into().unwrap());
    input.seek(SeekFrom::Start(rsrc_offset.into()))
        .context("Can’t seek to PROJECTR.RSR")?;
    let actual = input.take(rsrc_size.into()).read_to_end(&mut system_resources)
        .context("Can’t read PROJECTR.RSR")?;
    ensure!(actual == rsrc_size.try_into().unwrap(), "Expected {} bytes, read {} bytes", rsrc_size, actual);

    Ok(Some(system_resources))
}

fn internal_movie<R: binrw::io::Read + binrw::io::Seek>(reader: &mut R, offset: u32) -> AResult<Movie> {
    reader.seek(SeekFrom::Start(offset.into()))
        .with_context(|| format!("Bad RIFF offset {}", offset))?;

    detect_riff(reader)
        .with_context(|| format!("Can’t detect RIFF at {}", offset))?;

    Ok(Movie::Internal(offset))
}

mod pe {
    use binrw::{BinReaderExt, NullWideString, io};
    use core::convert::TryInto;
    use super::{
        AResult,
        ByteOrder,
        LittleEndian,
        ReadBytesExt,
        SeekExt,
        SeekFrom,
    };

    fn find_resource_segment_offset<R: binrw::io::Read + binrw::io::Seek>(input: &mut R, num_sections: u16) -> Option<(u32, u32)> {
        for _ in 0..num_sections {
            let mut section = [ 0; 40 ];
            input.read_exact(&mut section).ok()?;
            if section[0..8] == *b".rsrc\0\0\0" {
                return Some((LittleEndian::read_u32(&section[12..]), LittleEndian::read_u32(&section[20..])))
            }
        }

        None
    }

    pub(super) fn read_product_name<R: binrw::io::Read + binrw::io::Seek>(input: &mut R) -> Option<String> {
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

    fn read_version_struct<R: binrw::io::Read + binrw::io::Seek>(input: &mut R) -> AResult<Option<String>> {
        const FIXED_HEADER_WORD_SIZE: usize = 3;
        let start = input.pos()?;
        let size = input.read_u16::<LittleEndian>()?;
        let mut value_size = input.read_u16::<LittleEndian>()?;
        let is_text_data = input.read_u16::<LittleEndian>()? == 1;
        if is_text_data {
            value_size *= 2;
        }
        let value_padding = if value_size & 3 == 0 { 0 } else { 4 - (value_size & 3) };
        let end = start + u64::from(size) + u64::from(if size & 3 == 0 { 0 } else { 4 - (size & 3) });
        let key = input.read_le::<NullWideString>()?.into_string();

        let key_padding_size = ((FIXED_HEADER_WORD_SIZE + key.len() + 1) & 1) * 2;
        if key_padding_size != 0 {
            input.skip(key_padding_size.try_into().unwrap())?;
        }

        let is_string_table = key == "StringFileInfo" || (key.len() == 8 && &key[4..8] == "04b0");

        match key.as_ref() {
            "ProductName" => Ok(Some(input.read_le::<NullWideString>()?.into_string())),
            "VS_VERSION_INFO" => {
                input.skip((value_size + value_padding).into())?;
                read_version_struct(input)
            },
            _ if is_string_table => {
                while input.pos()? != end {
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

    fn seek_to_directory_entry<R: binrw::io::Read + binrw::io::Seek>(input: &mut R, from_offset: u32, id: u32) -> io::Result<()> {
        const ENTRY_SIZE: usize = 8;
        input.skip(12)?;
        let skip_entries = input.read_u16::<LittleEndian>()?;
        let num_entries = input.read_u16::<LittleEndian>()?;
        input.skip((ENTRY_SIZE * usize::from(skip_entries)).try_into().unwrap())?;
        for _ in 0..num_entries {
            let mut entry = [ 0; ENTRY_SIZE ];
            input.read_exact(&mut entry)?;
            let found_id = LittleEndian::read_u32(&entry);
            if found_id == id {
                const HAS_CHILDREN_FLAG: u32 = 0x8000_0000;
                let offset = LittleEndian::read_u32(&entry[4..]) & !HAS_CHILDREN_FLAG;
                input.seek(SeekFrom::Start((from_offset + offset).into()))?;
                return Ok(());
            }
        }

        Err(io::ErrorKind::InvalidData.into())
    }

    fn seek_to_resource_data<R: binrw::io::Read + binrw::io::Seek>(input: &mut R, virtual_address: u32, raw_offset: u32) -> io::Result<()> {
        let offset = input.read_u32::<LittleEndian>()?;
        input.seek(SeekFrom::Start(u64::from(offset - virtual_address + raw_offset)))?;
        Ok(())
    }

    fn seek_to_resource_segment<R: binrw::io::Read + binrw::io::Seek>(input: &mut R) -> io::Result<(u32, u32)> {
        input.skip(2)?;
        let num_sections = input.read_u16::<LittleEndian>()?;
        input.skip(12)?;
        let optional_header_size = input.read_u16::<LittleEndian>()?;
        input.skip(2 + u64::from(optional_header_size))?;
        let (virtual_address, offset) = find_resource_segment_offset(input, num_sections).ok_or_else(|| io::Error::from(io::ErrorKind::InvalidData))?;
        input.seek(SeekFrom::Start(offset.into()))?;
        Ok((virtual_address, offset))
    }
}
