use anyhow::{bail, Result as AResult};
use byteordered::Endianness;
use derive_more::Display;
use libcommon::Reader;
use libmactoolbox::{
    ResourceFile,
    rsid,
};

#[derive(Debug)]
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

#[derive(Debug, Display, Copy, Clone, PartialEq)]
pub enum Kind {
    Accelerator,
    Embedded,
    Movie,
    Cast,
}

#[derive(Debug, Display, Copy, Clone, PartialEq, PartialOrd)]
pub enum Version {
    #[display(fmt = "3")]
    D3,
    #[display(fmt = "4")]
    D4,
}

pub fn detect_mac<T: Reader>(reader: &mut T) -> AResult<DetectionInfo> {
    let rom = ResourceFile::new(reader)?;

    if rom.contains(rsid!(b"VWCF", 1024)) {
        Ok(DetectionInfo {
            data_endianness: Endianness::Big,
            os_type_endianness: Endianness::Big,
            version: Version::D3,
            kind: Kind::Embedded,
            size: 0,
        })
    } else if rom.contains(rsid!(b"EMPO", 256)) {
        Ok(DetectionInfo {
            data_endianness: Endianness::Big,
            os_type_endianness: Endianness::Big,
            version: Version::D3,
            kind: Kind::Accelerator,
            size: 0,
        })
    } else {
        bail!("No Director 3 movie configuration resource")
    }
}