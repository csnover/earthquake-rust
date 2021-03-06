use anyhow::{bail, Result as AResult};
use byteordered::Endianness;
use derive_more::Display;
use libcommon::Reader;
use libmactoolbox::{ResourceFile, ResourceId, ResourceSource};
use super::Version;

#[derive(Clone, Debug)]
pub struct DetectionInfo {
    pub(crate) os_type_endianness: Endianness,
    pub(crate) data_endianness: Endianness,
    pub(crate) version: Version,
    pub(crate) kind: Kind,
    pub(crate) size: u32,
}

impl DetectionInfo {
    #[must_use]
    pub fn data_endianness(&self) -> Endianness {
        self.data_endianness
    }

    #[must_use]
    pub fn os_type_endianness(&self) -> Endianness {
        self.os_type_endianness
    }

    #[must_use]
    pub fn kind(&self) -> Kind {
        self.kind
    }

    #[must_use]
    pub fn size(&self) -> u32 {
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

pub fn detect_mac<T: Reader>(reader: &mut T) -> AResult<DetectionInfo> {
    let rom = ResourceFile::new(reader)?;

    if rom.contains(ResourceId::new(b"EMPO", 256)) {
        Ok(DetectionInfo {
            data_endianness: Endianness::Big,
            os_type_endianness: Endianness::Big,
            version: Version::D3,
            kind: Kind::Accelerator,
            size: 0,
        })
    } else if rom.count(b"VWCF") > 1 || (rom.count(b"VWCF") == 1 && rom.id_of_name(b"VWCF", b"Tiles").is_none()) {
        Ok(DetectionInfo {
            data_endianness: Endianness::Big,
            os_type_endianness: Endianness::Big,
            version: Version::D3,
            kind: Kind::Embedded,
            size: 0,
        })
    } else {
        bail!("No Director 3 movie configuration resource")
    }
}
