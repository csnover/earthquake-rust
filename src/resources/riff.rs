use byteorder::{BigEndian, ByteOrder, LittleEndian, ReadBytesExt};
use crate::{Endianness, OSType, OSTypeReadExt, Reader, detect::*};
use std::{collections::HashMap, io::{ErrorKind, Result as IoResult, SeekFrom}};

#[derive(Debug)]
pub struct DetectionInfo {
    os_type_endianness: Endianness,
    data_endianness: Endianness,
    version: MovieVersion,
    kind: MovieType,
    size: u32,
}

#[derive(Debug)]
struct OffsetSize {
    offset: u32,
    size: u32,
}

#[derive(Debug)]
pub struct Riff<'a, T: Reader> {
    input: &'a mut T,
    chunk_map: HashMap<OSType, Vec<OffsetSize>>,
    info: DetectionInfo,
}

fn get_riff_attributes(os_type: &OSType, raw_size: &[u8]) -> (Endianness, u32) {
    let endianness = if os_type.as_bytes()[0] == b'M' {
        Endianness::Big
    } else {
        Endianness::Little
    };

    let size = {
        if endianness == Endianness::Big {
            BigEndian::read_u32(&raw_size)
        } else {
            LittleEndian::read_u32(&raw_size)
        }
    };

    (endianness, size)
}

fn detect_subtype(reader: &mut dyn Reader) -> Option<DetectionInfo> {
    let mut chunk_size_raw = [0; 4];
    reader.read_exact(&mut chunk_size_raw).ok()?;

    let sub_type = reader.read_os_type().ok()?;

    match sub_type.as_bytes() {
        b"RMMP" => Some(DetectionInfo {
            os_type_endianness: Endianness::Big,
            data_endianness: Endianness::Little,
            version: MovieVersion::D3,
            kind: MovieType::Normal,
            // This version of Director incorrectly includes the
            // size of the chunk header in the RIFF chunk size
            size: LittleEndian::read_u32(&chunk_size_raw) - 8,
        }),
        b"MV93" | b"39VM" => {
            let (endianness, size) = get_riff_attributes(&sub_type, &chunk_size_raw);
            Some(DetectionInfo {
                os_type_endianness: endianness,
                data_endianness: endianness,
                version: MovieVersion::D4,
                kind: MovieType::Normal,
                size,
            })
        },
        b"MC95" | b"59CM" => {
            let (endianness, size) = get_riff_attributes(&sub_type, &chunk_size_raw);
            Some(DetectionInfo {
                os_type_endianness: endianness,
                data_endianness: endianness,
                version: MovieVersion::D4,
                kind: MovieType::Cast,
                size,
            })
        },
        _ => None
    }
}

impl<'a, 'b, T: Reader> Riff<'a, T> {
    pub fn detect(reader: &mut T) -> Option<DetectionInfo> {
        reader.seek(SeekFrom::Start(0)).ok()?;
        let os_type = reader.read_os_type().ok()?;
        match os_type.as_bytes() {
            b"RIFX" | b"RIFF" | b"XFIR" => detect_subtype(reader),
            b"FFIR" => panic!("RIFF-LE files are not known to exist. Please send a sample of the file you are trying to open."),
            _ => None,
        }
    }

    pub fn new(input: &'a mut T) -> IoResult<Self> {
        const RIFF_HEADER_SIZE: u32 = 4;

        let info = Self::detect(input).ok_or(ErrorKind::InvalidData)?;

        let mut chunk_map = HashMap::new();
        let mut bytes_read = input.seek(SeekFrom::Current(0))? as u32;
        let bytes_to_read = bytes_read + info.size - RIFF_HEADER_SIZE;
        while bytes_read != bytes_to_read {
            let chunk_os_type = if info.os_type_endianness == Endianness::Little {
                input.read_le_os_type()
            } else {
                input.read_os_type()
            }?;

            let chunk_size = if info.data_endianness == Endianness::Little {
                input.read_u32::<LittleEndian>()
            } else {
                input.read_u32::<BigEndian>()
            }?;

            bytes_read += 8;

            chunk_map.entry(chunk_os_type).or_insert(Vec::new()).push(OffsetSize {
                offset: bytes_read,
                size: chunk_size
            });

            // RIFF chunks are always word-aligned (+1 & !1)
            bytes_read = (bytes_read + chunk_size + 1) & !1;
            input.seek(SeekFrom::Start(bytes_read as u64))?;
        }

        Ok(Self { input, chunk_map, info })
    }
}

#[cfg(test)]
mod tests {
    // TODO
}
