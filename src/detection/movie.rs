use anyhow::{Result as AResult, anyhow};
use crate::{Endianness, macos::MacResourceFile, OSType, Reader, os};
use enum_display_derive::Display;
use std::fmt::Display;

#[derive(Debug)]
pub struct DetectionInfo {
    pub(crate) os_type_endianness: Endianness,
    pub(crate) data_endianness: Endianness,
    pub(crate) version: MovieVersion,
    pub(crate) kind: MovieType,
    pub(crate) size: u32,
}

impl DetectionInfo {
    pub fn data_endianness(&self) -> Endianness {
        self.data_endianness
    }

    pub fn os_type_endianness(&self) -> Endianness {
        self.os_type_endianness
    }

    pub fn kind(&self) -> MovieType {
        self.kind
    }

    pub fn size(&self) -> u32 {
        self.size
    }

    pub fn version(&self) -> MovieVersion {
        self.version
    }
}

#[derive(Debug, Display, Copy, Clone, PartialEq)]
pub enum MovieType {
    Accelerator,
    Embedded,
    Movie,
    Cast,
}

#[derive(Debug, Display, Copy, Clone, PartialEq, PartialOrd)]
pub enum MovieVersion {
    D3,
    D4,
}

pub fn detect_mac<T: Reader>(reader: &mut T) -> AResult<DetectionInfo> {
    let rom = MacResourceFile::new(reader)?;

    if rom.contains_type(os!(b"VWCF")) {
        Ok(DetectionInfo {
            data_endianness: Endianness::Big,
            os_type_endianness: Endianness::Big,
            version: MovieVersion::D3,
            kind: MovieType::Embedded,
            size: 0,
        })
    } else if rom.contains_type(os!(b"EMPO")) {
        Ok(DetectionInfo {
            data_endianness: Endianness::Big,
            os_type_endianness: Endianness::Big,
            version: MovieVersion::D3,
            kind: MovieType::Accelerator,
            size: 0,
        })
    } else {
        Err(anyhow!("Missing Director movie configuration resource"))
    }
}
