use anyhow::{bail, Result as AResult};
use binrw::Endian;
use crate::resources::config::Platform;
use derive_more::Display;
use libmactoolbox::resources::{File as ResourceFile, ResourceId, Source as ResourceSource};
use super::Version;

#[derive(Clone, Debug)]
pub struct DetectionInfo {
    pub(crate) os_type_endianness: Endian,
    pub(crate) data_endianness: Endian,
    pub(crate) version: Version,
    pub(crate) kind: Kind,
    pub(crate) size: u32,
}

impl DetectionInfo {
    #[must_use]
    pub(crate) fn data_endianness(&self) -> Endian {
        self.data_endianness
    }

    #[must_use]
    pub(crate) fn os_type_endianness(&self) -> Endian {
        self.os_type_endianness
    }

    #[must_use]
    pub fn kind(&self) -> Kind {
        self.kind
    }

    // This might be a bad inference.
    #[must_use]
    pub(crate) fn platform(&self) -> Platform {
        if self.data_endianness == Endian::Big && self.os_type_endianness == Endian::Big {
            Platform::Mac
        } else {
            Platform::Win
        }
    }

    #[must_use]
    pub(crate) fn size(&self) -> u32 {
        self.size
    }

    #[must_use]
    pub fn version(&self) -> Version {
        self.version
    }
}

#[derive(Clone, Copy, Debug, Display, PartialEq)]
pub enum Kind {
    Accelerator,
    Embedded,
    Movie,
    Cast,
}

pub(super) fn detect_mac<T: binrw::io::Read + binrw::io::Seek>(reader: &mut T) -> AResult<DetectionInfo> {
    let rom = ResourceFile::new(reader)?;

    if rom.contains(ResourceId::new(b"EMPO", 256_i16)) {
        Ok(DetectionInfo {
            data_endianness: Endian::Big,
            os_type_endianness: Endian::Big,
            version: Version::D3,
            kind: Kind::Accelerator,
            size: 0,
        })
    } else if rom.count(b"VWCF") > 1 || (rom.count(b"VWCF") == 1 && rom.id_of_name(b"VWCF", b"Tiles").is_none()) {
        Ok(DetectionInfo {
            data_endianness: Endian::Big,
            os_type_endianness: Endian::Big,
            version: Version::D3,
            kind: Kind::Embedded,
            size: 0,
        })
    } else {
        bail!("No Director 3 movie configuration resource")
    }
}
