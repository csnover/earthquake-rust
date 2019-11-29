use anyhow::{Result as AResult, anyhow};
use crate::{Endianness, OSType, Reader, ResourceId, os, rsid, collections::{riff, rsrc::MacResourceFile}};
use enum_display_derive::Display;
use std::{fmt::Display, io::SeekFrom};

#[derive(Debug)]
pub struct DetectionInfo {
    pub(crate) os_type_endianness: Endianness,
    pub(crate) data_endianness: Endianness,
    pub(crate) version: MovieVersion,
    pub(crate) kind: MovieType,
    pub size: u32,
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

pub fn detect<T: Reader>(reader: &mut T) -> AResult<DetectionInfo> {
    if let Ok(file_type) = riff::detect(reader) {
        return Ok(file_type);
    }

    reader.seek(SeekFrom::Start(0))?;
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
