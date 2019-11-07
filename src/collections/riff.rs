use byteorder::{BigEndian, ByteOrder, LittleEndian, ReadBytesExt};
use byteordered::ByteOrdered;
use crate::{Endianness, OSType, OSTypeReadExt, Reader};
use enum_display_derive::Display;
use std::{cell::RefCell, collections::HashMap, fmt::Display, io::{self, ErrorKind, Read, Result as IoResult, Seek, SeekFrom}};

#[derive(Debug)]
pub struct DetectionInfo {
    os_type_endianness: Endianness,
    data_endianness: Endianness,
    version: MovieVersion,
    kind: MovieType,
    pub size: u32,
}

#[derive(Debug, Display, Copy, Clone, PartialEq)]
pub enum MovieType {
    Embedded,
    Movie,
    Cast,
}

#[derive(Debug, Display, Copy, Clone, PartialEq, PartialOrd)]
pub enum MovieVersion {
    D3,
    D4,
}

#[derive(Copy, Clone, Debug)]
struct OffsetSize {
    offset: u32,
    size: u32,
}

type ResourceMap = HashMap<(OSType, u16), OffsetSize>;
type Input<T> = RefCell<ByteOrdered<T, byteordered::Endianness>>;

#[derive(Debug)]
pub struct Riff<T: Reader> {
    input: Input<T>,
    resource_map: ResourceMap,
    info: DetectionInfo,
}

impl<T: Reader> Riff<T> {
    pub fn new(mut input: T) -> IoResult<Self> {
        let info = detect(&mut input).ok_or_else(|| io::Error::new(ErrorKind::InvalidData, "Failed to detect a valid RIFF file"))?;

        let resource_map = {
            if info.os_type_endianness == Endianness::Little && info.data_endianness == Endianness::Little {
                build_resource_map::<T, LittleEndian, LittleEndian>(&mut input)?
            } else if info.os_type_endianness == Endianness::Big && info.data_endianness == Endianness::Little {
                build_resource_map::<T, BigEndian, LittleEndian>(&mut input)?
            } else if info.os_type_endianness == Endianness::Big && info.data_endianness == Endianness::Big {
                build_resource_map::<T, BigEndian, BigEndian>(&mut input)?
            } else {
                panic!("Big endian data with little endian OSType is impossible.");
            }
        };

        Ok(Self {
            input: RefCell::new(ByteOrdered::runtime(input, info.data_endianness)),
            resource_map,
            info
        })
    }

    pub fn kind(&self) -> MovieType {
        self.info.kind
    }

    pub fn size(&self) -> u32 {
        self.info.size
    }

    pub fn version(&self) -> MovieVersion {
        self.info.version
    }

    pub fn iter(&self) -> RiffIterator<T> {
        RiffIterator {
            input: &self.input,
            map_iter: self.resource_map.iter()
        }
    }
}

pub struct RiffIterator<'a, T: Reader> {
    input: &'a Input<T>,
    map_iter: std::collections::hash_map::Iter<'a, (OSType, u16), OffsetSize>,
}

impl<'a, T: Reader> Iterator for RiffIterator<'a, T> {
    type Item = RiffData<'a, T>;

    fn next(&mut self) -> Option<Self::Item> {
        match self.map_iter.next() {
            Some(item) => {
                Some(RiffData {
                    id: *item.0,
                    input: self.input,
                    offset_size: *item.1,
                })
            },
            None => None
        }
    }
}

impl<'a, T: Reader> ExactSizeIterator for RiffIterator<'a, T> {
    fn len(&self) -> usize {
        self.map_iter.len()
    }
}

impl<'a, T: Reader> std::iter::FusedIterator for RiffIterator<'a, T> {}

impl<'a, T: Reader> IntoIterator for &'a Riff<T> {
    type Item = RiffData<'a, T>;
    type IntoIter = RiffIterator<'a, T>;

    #[inline]
    fn into_iter(self) -> Self::IntoIter {
        self.iter()
    }
}

pub struct RiffData<'a, T: Reader> {
    pub id: (OSType, u16),
    input: &'a Input<T>,
    offset_size: OffsetSize,
}

impl<'a, T: Reader> RiffData<'a, T> {
    pub fn data(&self) -> IoResult<Vec<u8>> {
        self.input.borrow_mut().seek(SeekFrom::Start(u64::from(self.offset_size.offset)))?;
        let mut data = Vec::new();
        self.input.borrow_mut().as_mut().take(u64::from(self.offset_size.size)).read_to_end(&mut data)?;
        Ok(data)
    }
}

pub fn detect<T: Reader>(reader: &mut T) -> Option<DetectionInfo> {
    let os_type = reader.read_os_type::<BigEndian>().ok()?;
    match os_type.as_bytes() {
        b"RIFX" | b"RIFF" | b"XFIR" => detect_subtype(reader),
        b"FFIR" => panic!("RIFF-LE files are not known to exist. Please send a sample of the file you are trying to open."),
        _ => None,
    }
}

fn build_resource_map<T: Reader, OE: ByteOrder, DE: ByteOrder>(input: &mut T) -> IoResult<ResourceMap> {
    let map_os_type = input.read_os_type::<OE>()?;
    let mut bytes_to_read = input.read_u32::<DE>()?;
    let mut resource_map = HashMap::new();

    match map_os_type.as_bytes() {
        b"CFTC" => {
            // TODO: Is this value important? It seems to always be 0.
            input.seek(SeekFrom::Current(4))?;

            bytes_to_read -= 4;
            while bytes_to_read != 0 {
                let os_type = input.read_os_type::<OE>()?;
                if os_type.as_bytes() == b"\0\0\0\0" {
                    break;
                }
                let size = input.read_u32::<DE>()?;
                let id = input.read_i32::<DE>()?;
                let offset = input.read_u32::<DE>()?;

                let result = resource_map.insert((os_type, id as u16), OffsetSize { offset, size });
                if result.is_some() {
                    panic!(format!("Multiple {} {} in mmap", os_type, id));
                }

                bytes_to_read -= 16;
            }
        },
        b"imap" => {
            let _num_maps = input.read_u32::<DE>()?;
            let map_offset = input.read_u32::<DE>()?;
            input.seek(SeekFrom::Start(u64::from(map_offset)))?;
            let map_os_type = input.read_os_type::<OE>()?;
            if map_os_type.as_bytes() != b"mmap" {
                return Err(io::Error::new(ErrorKind::InvalidData, "Could not find a valid resource map"));
            }
            let _chunk_size = input.read_u32::<DE>()?;

            const MMAP_HEADER_BYTES_READ: i64 = 12;
            let header_size = input.read_u16::<DE>()?;
            let table_entry_size = input.read_u16::<DE>()?;
            let _table_entry_count_max = input.read_u32::<DE>()?;
            let table_entry_count = input.read_u32::<DE>()?;
            input.seek(SeekFrom::Current(i64::from(header_size) - MMAP_HEADER_BYTES_READ))?;
            // TODO: Do not actually know that the index is taken as the ID, but
            // there seems to be no other identifier for chunks in the index
            for id in 0..table_entry_count {
                const ENTRY_BYTES_READ: i64 = 12;
                let os_type = input.read_os_type::<OE>()?;
                let size = input.read_u32::<DE>()?;
                let offset = input.read_u32::<DE>()? + 8;
                input.seek(SeekFrom::Current(i64::from(table_entry_size) - ENTRY_BYTES_READ))?;
                resource_map.insert((os_type, id as u16), OffsetSize { offset, size });
            }
        },
        _ => return Err(io::Error::new(ErrorKind::InvalidData, "Could not find a valid resource map"))
    }

    Ok(resource_map)
}

fn detect_subtype<T: Reader>(reader: &mut T) -> Option<DetectionInfo> {
    let mut chunk_size_raw = [0; 4];
    reader.read_exact(&mut chunk_size_raw).ok()?;

    let sub_type = reader.read_os_type::<BigEndian>().ok()?;

    match sub_type.as_bytes() {
        b"RMMP" => Some(DetectionInfo {
            os_type_endianness: Endianness::Big,
            data_endianness: Endianness::Little,
            version: MovieVersion::D3,
            kind: MovieType::Movie,
            // This version of Director incorrectly includes the
            // size of the chunk header in the RIFF chunk size
            size: LittleEndian::read_u32(&chunk_size_raw) - 8,
        }),
        b"MV93" | b"39VM" => {
            let (endianness, size) = get_riff_attributes(sub_type, &chunk_size_raw);
            Some(DetectionInfo {
                os_type_endianness: endianness,
                data_endianness: endianness,
                version: MovieVersion::D4,
                kind: MovieType::Movie,
                size,
            })
        },
        b"MC95" | b"59CM" => {
            let (endianness, size) = get_riff_attributes(sub_type, &chunk_size_raw);
            Some(DetectionInfo {
                os_type_endianness: endianness,
                data_endianness: endianness,
                version: MovieVersion::D4,
                kind: MovieType::Cast,
                size,
            })
        },
        b"APPL" | b"LPPA" => {
            let (endianness, size) = get_riff_attributes(sub_type, &chunk_size_raw);
            Some(DetectionInfo {
                os_type_endianness: endianness,
                data_endianness: endianness,
                version: MovieVersion::D4,
                kind: MovieType::Embedded,
                size,
            })
        },
        _ => None
    }
}

fn get_riff_attributes(os_type: OSType, raw_size: &[u8]) -> (Endianness, u32) {
    let endianness = match os_type.as_bytes()[0] {
        b'M' | b'A' => Endianness::Big,
        _ => Endianness::Little
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

#[cfg(test)]
mod tests {
    // TODO
}
