//! A Director RIFF file.
//!
//! Starting with Director 3 for Windows, movie and cast data are stored in
//! [RIFF files] (earlier versions used [Mac Resource Files]). Special index
//! chunks at the start of the file are used for O(1) lookup of data by chunk
//! index or [`ResourceID`].

use binrw::{BinRead, Endian, io::{Read, Seek, SeekFrom, self}};
use bitflags::bitflags;
use byteorder::{BigEndian, ByteOrder, LittleEndian, ReadBytesExt};
use crate::detection::{movie::{DetectionInfo, Kind as MovieKind}, Version};
use derive_more::{Constructor, Deref, DerefMut, Display};
use libcommon::{Reader, SeekExt, SharedStream, TakeSeekExt, newtype_num};
use libmactoolbox::resources::{OsType, OsTypeReadExt, Error as ResourceError, ResourceId, Result as ResourceResult, Source as ResourceSource};
use std::{any::Any, cell::RefCell, collections::HashMap, convert::{TryFrom, TryInto}, rc::{Rc, Weak}};

pub type Result<T> = core::result::Result<T, Error>;

#[derive(Debug, thiserror::Error)]
#[non_exhaustive]
pub enum Error {
    #[error("unknown i/o error: {0}")]
    Io(#[from] io::Error),
    #[error("not a RIFF file")]
    NotRiff,
    #[error("not a Director RIFF")]
    NotDirectorRiff,
    #[error("RIFF-LE. Please send this file for analysis")]
    UnsupportedRiff,
    #[error("missing resource map; found {0} instead")]
    ResourceMapNotFound(OsType),
    #[error("bad chunk index {0}")]
    BadIndex(ChunkIndex),
    #[error("bad data type for chunk index {0}; OSType is {1}")]
    BadDataType(ChunkIndex, OsType),
    #[error("i/o error seeking to index {0} (offset {1}): {2}")]
    SeekFailure(ChunkIndex, u32, io::Error),
    #[error("i/o error seeking to mmap: {0}")]
    MmapSeekFailure(io::Error),
    #[error("i/o error seeking to KEY*: {0}")]
    KeysSeekFailure(io::Error),
    #[error("bad KEY* offset")]
    BadKeysOffset,
    #[error("bad mmap header size ({0})")]
    BadMmapHeaderSize(u16),
    #[error("bad mmap entry size ({0})")]
    BadMmapEntrySize(u16),
    #[error("bad mmap entry count ({0})")]
    BadMmapEntryCount(u32),
    #[error("bad flags in mmap entry {0}: 0x{1:x}")]
    BadMmapEntryFlags(ChunkIndex, u16),
    #[error("multiple {0} in {1}. Please send this file for analysis")]
    DuplicateResourceId(ResourceId, &'static str),
    #[error("i/o error skipping resource ID of index {0}: {1}")]
    D3ResourceIdSkipIo(ChunkIndex, io::Error),
    #[error("i/o error reading resource name of index {0}: {1}")]
    D3ResourceNameSizeReadIo(ChunkIndex, io::Error),
    #[error("i/o error skipping resource name of index {0}: {1}")]
    D3ResourceNameSkipIo(ChunkIndex, io::Error),
    #[error("can’t load {0} chunk {1} at {2}: {3}")]
    ReadFailure(OsType, ChunkIndex, u32, binrw::Error),
}

impl From<binrw::Error> for Error {
    fn from(error: binrw::Error) -> Self {
        match error {
            binrw::Error::Io(error) => Self::Io(error),
            binrw::Error::Custom { err, .. } => {
                *err.downcast().expect("unexpected error type")
            },
            _ => panic!("unexpected error type"),
        }
    }
}

newtype_num! {
    #[derive(BinRead, Constructor, Debug)]
    pub struct ChunkIndex(i32);
}

#[derive(Debug, Display)]
#[display(fmt = "{} -> chunk {}", id, chunk_index)]
pub struct Iter<'a, T: Reader> {
    id: ResourceId,
    owner: &'a Riff<T>,
    chunk_index: ChunkIndex,
}

impl<'a, T: Reader> Iter<'a, T> {
    #[must_use]
    pub fn id(&self) -> ResourceId {
        self.id
    }

    #[deprecated(note = "TODO: For debugging only")]
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.owner.memory_map.get(self.chunk_index).unwrap().size == 0
    }

    #[deprecated(note = "TODO: For debugging only")]
    #[must_use]
    pub fn len(&self) -> u32 {
        self.owner.memory_map.get(self.chunk_index).unwrap().size
    }

    pub fn load<R: BinRead + 'static>(&self, args: R::Args) -> Result<Rc<R>> {
        self.owner.load_chunk_args(self.chunk_index, args)
    }

    #[deprecated(note = "TODO: For debugging only")]
    #[must_use]
    pub fn offset(&self) -> u32 {
        self.owner.memory_map.get(self.chunk_index).unwrap().offset
    }
}

#[derive(Clone, Debug)]
pub struct Riff<T: Reader> {
    // TODO: This is needed to convert a Riff chunk back to its owner filename,
    // but not enough information is recorded currently to actually do this.
    id: Identity,

    input: RefCell<SharedStream<T>>,

    endianness: binrw::Endian,

    /// Index for O(1) access of RIFF chunks by index.
    memory_map: MemoryMap,

    /// Mac resource types to RIFF chunk indexes.
    resource_map: ResourceMap,

    info: DetectionInfo,
}

impl<T: Reader> Riff<T> {
    pub fn new(input: T) -> Result<Self> {
        Self::with_identity(Identity::Parent, SharedStream::new(input))
    }

    #[must_use]
    pub fn first_of_kind(&self, kind: impl Into<OsType>) -> ChunkIndex {
        let kind = kind.into();
        for (index, item) in self.memory_map.iter().enumerate() {
            if item.os_type == kind {
                return ChunkIndex::new(index.try_into().unwrap());
            }
        }
        ChunkIndex::new(-1)
    }

    pub fn iter(&self) -> impl Iterator<Item = Iter<'_, T>> {
        self.resource_map.iter().map(move |(k, v)| Iter {
            id: *k,
            owner: self,
            chunk_index: *v,
        })
    }

    #[must_use]
    pub fn kind(&self) -> MovieKind {
        self.info.kind()
    }

    pub fn load_chunk<R>(&self, index: ChunkIndex) -> Result<Rc<R>>
    where
        R: BinRead + 'static,
        R::Args: Default + Sized
    {
        self.load_chunk_args::<R>(index, R::Args::default())
    }

    pub fn load_chunk_args<R: BinRead + 'static>(&self, index: ChunkIndex, args: R::Args) -> Result<Rc<R>> {
        let entry = self.memory_map.get(index).ok_or(Error::BadIndex(index))?;

        if let Some(data) = entry.data.borrow().as_data().and_then(Weak::upgrade) {
            return data.downcast::<R>()
                .map_err(|_| Error::BadDataType(index, entry.os_type));
        }

        let mut input = self.input.borrow_mut();
        input.seek(SeekFrom::Start((entry.offset + Self::CHUNK_HEADER_SIZE).into()))
            .map_err(|error| Error::SeekFailure(index, entry.offset, error))?;

        let mut entry_size = entry.size;

        if self.info.version() == Version::D3 {
            input.skip(4).map_err(|error| Error::D3ResourceIdSkipIo(index, error))?;
            let mut name_size = input
                .read_u8()
                .map_err(|error| Error::D3ResourceNameSizeReadIo(index, error))?;
            // padding byte; check appears reversed, but is correct, because of
            // the odd-sized fixed offset later
            if name_size & 1 == 0 {
                // padding byte
                name_size += 1;
            }
            entry_size -= u32::from(name_size) + 5;
            input.skip(name_size.into()).map_err(|error|
                Error::D3ResourceNameSkipIo(index, error))?;
        }

        let mut options = binrw::ReadOptions::default();
        options.endian = self.endianness;
        // TODO: Figure out how to get entry size into it
        R::read_options(&mut input.clone().take_seek(entry_size.into()), &options, args)
            .map(|resource| {
                let resource = Rc::new(resource);
                *entry.data.borrow_mut() = ChunkData::Loaded(Rc::downgrade(&(Rc::clone(&resource) as _)));
                resource
            })
            .map_err(|error| Error::ReadFailure(entry.os_type, index, entry.offset, error))
    }

    pub fn load_riff(&self, index: ChunkIndex) -> Result<Self> {
        let entry = self.memory_map.get(index).ok_or(Error::BadIndex(index))?;

        let mut input = self.input.borrow_mut().clone();
        input.seek(SeekFrom::Start(entry.offset.into()))
            .map_err(|error| Error::SeekFailure(index, entry.offset, error))?;
        Self::with_identity(Identity::Child(index), input)
    }

    pub fn make_free(&mut self, index: ChunkIndex) {
        let next_free_index = self.memory_map.next_free_index;
        let entry = self.memory_map.get_mut(index).unwrap();
        entry.os_type = b"free".into();
        entry.size = 0;
        entry.offset = 0;
        entry.flags = MemoryMapFlags::VALID | MemoryMapFlags::FREE;
        entry.field_e = 0;
        entry.data.replace(ChunkData::Free { next_free: next_free_index });
        self.memory_map.next_free_index = index;
    }

    pub fn make_junk(&mut self, index: ChunkIndex) {
        let next_junk_index = self.memory_map.next_junk_index;
        let entry = self.memory_map.get_mut(index).unwrap();
        entry.os_type = b"junk".into();
        entry.flags = MemoryMapFlags::VALID;
        entry.field_e = 0;
        entry.data.replace(ChunkData::Free { next_free: next_junk_index });
        self.memory_map.next_junk_index = index;
    }

    // TODO: For debugging
    pub fn metadata(&self, index: ChunkIndex) -> Option<&MemoryMapItem> {
        self.memory_map.get(index)
    }

    #[must_use]
    pub fn size(&self) -> u32 {
        self.info.size()
    }

    #[must_use]
    pub fn version(&self) -> Version {
        self.info.version()
    }

    const CHUNK_HEADER_SIZE: u32 = 8;
    const KEYS_HEADER_SIZE: u32 = 12;
    const MMAP_HEADER_SIZE: u32 = 24;
    const MMAP_ENTRY_SIZE: u32 = 20;
    // n.b. Number of entries is limited to 3275 so the mmap always fits
    // in a 64k page (`(0xffff - header_size) / entry_size`)
    const MMAP_MAX_ENTRIES: u32 = ((0xFFFF - Self::MMAP_HEADER_SIZE) / Self::MMAP_ENTRY_SIZE);

    fn init<R: Read + Seek, OE: ByteOrder, DE: ByteOrder>(input: &mut R) -> Result<(MemoryMap, ResourceMap)> {
        let os_type = input.read_os_type::<OE>()?;
        match os_type.as_bytes() {
            b"CFTC" => Self::read_cftc::<R, OE, DE>(input),
            // n.b. Director actually seeks through every RIFF chunk until it
            // finds an imap, but it is always the first chunk in a
            // well-authored file, so for implementation simplicity we do not do
            // that.
            b"imap" => Self::read_imap::<R, OE, DE>(input),
            _ => Err(Error::ResourceMapNotFound(os_type)),
        }
    }

    fn read_cftc<R: Read + Seek, OE: ByteOrder, DE: ByteOrder>(input: &mut R) -> Result<(MemoryMap, ResourceMap)> {
        const ENTRY_SIZE: u32 = 16;

        let mut bytes_to_read = input.read_u32::<DE>()?;

        // This value is ignored in at least D3Win version 36 (the version # is
        // the resource ID of the Ver. resource) and appears to always be zero.
        input.skip(4)?;
        bytes_to_read -= 4;

        let (mut memory_map_items, mut resource_map) = {
            let num_entries = usize::try_from(bytes_to_read / ENTRY_SIZE - 1).unwrap();
            (Vec::with_capacity(num_entries), HashMap::with_capacity(num_entries))
        };

        while bytes_to_read != 0 {
            let os_type = input.read_os_type::<OE>()?;
            if os_type.as_bytes() == b"\0\0\0\0" {
                break;
            }
            let size = input.read_u32::<DE>()?;
            let id = i16::try_from(input.read_i32::<DE>()?).unwrap();
            let offset = input.read_u32::<DE>()?;

            let mmap_index = ChunkIndex(memory_map_items.len().try_into().unwrap());
            memory_map_items.push(MemoryMapItem {
                os_type,
                size,
                offset,
                flags: MemoryMapFlags::empty(),
                field_e: 0,
                data: RefCell::new(ChunkData::Free { next_free: ChunkIndex::new(-1) }),
            });

            let res_id = ResourceId::new(os_type, id);
            if resource_map.insert(res_id, mmap_index).is_some() {
                return Err(Error::DuplicateResourceId(res_id, "CFTC"));
            }

            bytes_to_read -= ENTRY_SIZE;
        }

        Ok((MemoryMap {
            items: memory_map_items,
            next_free_index: ChunkIndex::new(-1),
            next_junk_index: ChunkIndex::new(-1),
        }, resource_map))
    }

    fn read_imap<R: Read + Seek, OE: ByteOrder, DE: ByteOrder>(input: &mut R) -> Result<(MemoryMap, ResourceMap)> {
        let _imap_size = input.skip(4)?;
        let _num_maps = input.skip(4)?;
        let map_offset = input.read_u32::<DE>()?;
        // imap contains the reference to the active mmap chunk for the file
        // along with some other unknown data which we can ignore for now
        input.seek(SeekFrom::Start(map_offset.into()))
            .map_err(Error::MmapSeekFailure)?;

        let os_type = input.read_os_type::<OE>()?;
        if os_type.as_bytes() != b"mmap" {
            return Err(Error::ResourceMapNotFound(os_type));
        }

        let _chunk_size = input.skip(4)?;

        let header_size = input.read_u16::<DE>()?;
        if header_size != 0x18 {
            return Err(Error::BadMmapHeaderSize(header_size));
        }
        let entry_size = input.read_u16::<DE>()?;
        if entry_size != 0x14 {
            return Err(Error::BadMmapEntrySize(entry_size));
        }
        let _table_capacity = input.skip(4)?;
        let num_entries = input.read_u32::<DE>()?;
        if num_entries > Self::MMAP_MAX_ENTRIES {
            return Err(Error::BadMmapEntryCount(num_entries));
        }
        let next_junk_index = ChunkIndex::new(input.read_i32::<DE>()?);
        let _garbage = input.skip(4)?;
        let next_free_index = ChunkIndex::new(input.read_i32::<DE>()?);
        input.skip((u32::from(header_size) - Self::MMAP_HEADER_SIZE).into())?;

        let mut memory_map_items = Vec::with_capacity(num_entries.try_into().unwrap());

        let mut resource_map_offset = None;
        for index in 0..num_entries {
            let os_type = input.read_os_type::<OE>()?;
            let size = input.read_u32::<DE>()?;
            let offset = input.read_u32::<DE>()?;
            let flags_bits = input.read_u16::<DE>()?;
            let flags = MemoryMapFlags::from_bits(flags_bits)
                .ok_or_else(|| Error::BadMmapEntryFlags(ChunkIndex(i32::try_from(index).unwrap()), flags_bits))?;
            let field_e = input.read_u16::<DE>()?;
            let next_free_index = ChunkIndex::new(input.read_i32::<DE>()?);
            input.skip((u32::from(entry_size) - Self::MMAP_ENTRY_SIZE).into())?;
            memory_map_items.push(MemoryMapItem {
                os_type,
                size,
                offset,
                flags,
                field_e,
                data: RefCell::new(ChunkData::Free { next_free: next_free_index }),
            });

            // Director built the memory map, then looked for the first
            // valid KEY* chunk; since we are building the map anyway, might
            // as well just get the offset now.
            if resource_map_offset.is_none()
                && os_type.as_bytes() == b"KEY*"
                && !flags.contains(MemoryMapFlags::VALID)
            {
                resource_map_offset = Some(offset);
            }
        }

        // It is valid to have no KEY*—if it is a RIFF container; Director
        // normally would not try to find and parse this chunk in the cases
        // where it was not expected, but we just always look since there is no
        // reason to have a separate code path for container RIFFs
        let resource_map = if let Some(offset) = resource_map_offset {
            input.seek(SeekFrom::Start(offset.into()))
                .map_err(Error::KeysSeekFailure)?;
            Self::read_keys::<R, OE, DE>(input)?
        } else {
            ResourceMap::new()
        };

        Ok((MemoryMap {
            items: memory_map_items,
            next_free_index,
            next_junk_index,
        }, resource_map))
    }

    fn read_keys<R: Read + Seek, OE: ByteOrder, DE: ByteOrder>(input: &mut R) -> Result<ResourceMap> {
        if input.read_os_type::<OE>()?.as_bytes() != b"KEY*" {
            return Err(Error::BadKeysOffset);
        }

        let _chunk_size = input.skip(4)?;
        let header_size = input.read_u16::<DE>()?;
        let _item_size = input.skip(2)?;
        let _capacity = input.skip(4)?;
        let num_entries = input.read_u32::<DE>()?;
        input.skip((u32::from(header_size) - Self::KEYS_HEADER_SIZE).into())?;

        let mut resource_map = HashMap::with_capacity(num_entries.try_into().unwrap());

        for _ in 0..num_entries {
            let riff_index = ChunkIndex::new(input.read_i32::<DE>()?);
            let id = i16::try_from(input.read_i32::<DE>()?).unwrap();
            let os_type = input.read_os_type::<DE>()?;

            let res_id = ResourceId::new(os_type, id);
            if resource_map.insert(res_id, riff_index).is_some() {
                return Err(Error::DuplicateResourceId(res_id, "KEY*"));
            }
        }

        Ok(resource_map)
    }

    fn with_identity(id: Identity, mut input: SharedStream<T>) -> Result<Self> {
        let info = detect(&mut input)?;

        let (memory_map, resource_map) = {
            if info.os_type_endianness() == Endian::Little && info.data_endianness() == Endian::Little {
                Self::init::<_, LittleEndian, LittleEndian>(&mut input)?
            } else if info.os_type_endianness() == Endian::Big && info.data_endianness() == Endian::Little {
                Self::init::<_, BigEndian, LittleEndian>(&mut input)?
            } else if info.os_type_endianness() == Endian::Big && info.data_endianness() == Endian::Big {
                Self::init::<_, BigEndian, BigEndian>(&mut input)?
            } else {
                unreachable!("big endian data with little endian OSType does not exist");
            }
        };

        Ok(Self {
            id,
            input: RefCell::new(input),
            endianness: info.data_endianness(),
            memory_map,
            resource_map,
            info
        })
    }
}

impl <T: Reader> ResourceSource for Riff<T> {
    fn contains(&self, id: impl Into<ResourceId>) -> bool {
        self.resource_map.get(&id.into()).is_some()
    }

    fn load_args<R: BinRead + 'static>(&self, id: ResourceId, args: R::Args) -> ResourceResult<Rc<R>> {
        if let Some(&chunk_index) = self.resource_map.get(&id) {
            Self::load_chunk_args::<R>(self, chunk_index, args).map_err(|error| {
                // TODO: Losing a lot of data here.
                match error {
                    Error::Io(error) => ResourceError::Io(error),
                    Error::BadIndex(_) => ResourceError::NotFound(id),
                    Error::BadDataType(_, _) => ResourceError::BadDataType(id),
                    Error::SeekFailure(_, _, error)
                    | Error::D3ResourceIdSkipIo(_, error)
                    | Error::D3ResourceNameSkipIo(_, error) => ResourceError::SeekFailure(id, error),
                    Error::D3ResourceNameSizeReadIo(_, error) => ResourceError::ReadSizeFailure(id, error),
                    Error::ReadFailure(_, _, _, error) => ResourceError::ResourceReadFailure(id, error),
                    _ => todo!("split RiffError to distinguish between resource loads and riff loads"),
                }
            })
        } else {
            Err(ResourceError::NotFound(id))
        }
    }
}

pub fn detect<R: Read + Seek>(reader: &mut R) -> Result<DetectionInfo> {
    let os_type = reader.read_os_type::<BigEndian>()?;
    match os_type.as_bytes() {
        b"RIFX" | b"RIFF" | b"XFIR" => detect_subtype(reader).ok_or(Error::NotDirectorRiff),
        b"FFIR" => Err(Error::UnsupportedRiff),
        _ => Err(Error::NotRiff),
    }
}

#[derive(Clone, Debug)]
enum ChunkData {
    Free { next_free: ChunkIndex },
    Loaded(Weak<dyn Any>)
}

impl ChunkData {
    pub fn as_data(&self) -> Option<&Weak<dyn Any>> {
        match &self {
            Self::Free { .. } => None,
            Self::Loaded(weak_ref) => Some(weak_ref)
        }
    }
}

#[derive(Clone, Copy, Debug)]
enum Identity {
    Parent,
    Child(ChunkIndex)
}

#[derive(Clone, Debug, Deref, DerefMut)]
struct MemoryMap {
    #[deref]
    #[deref_mut]
    items: Vec<MemoryMapItem>,
    next_junk_index: ChunkIndex,
    next_free_index: ChunkIndex,
}

impl MemoryMap {
    fn get(&self, index: ChunkIndex) -> Option<&MemoryMapItem> {
        self.items.get(usize::try_from(index.0).unwrap())
    }

    fn get_mut(&mut self, index: ChunkIndex) -> Option<&mut MemoryMapItem> {
        self.items.get_mut(usize::try_from(index.0).unwrap())
    }
}

impl ::core::ops::Index<ChunkIndex> for MemoryMap {
    type Output = MemoryMapItem;
    fn index(&self, index: ChunkIndex) -> &Self::Output {
        &self.items[usize::try_from(index.0).unwrap()]
    }
}

impl ::core::ops::IndexMut<ChunkIndex> for MemoryMap {
    fn index_mut(&mut self, index: ChunkIndex) -> &mut Self::Output {
        &mut self.items[usize::try_from(index.0).unwrap()]
    }
}

bitflags! {
    struct MemoryMapFlags: u16 {
        const DIRTY = 1;
        const VALID = 4;
        const FREE  = 8;
        const FLAG_20 = 0x20;
        const FLAG_40 = 0x40;
        const ALLOCATED = 0x80;
        const FLAG_8000 = 0x8000;
    }
}

#[derive(Clone, Debug)]
pub struct MemoryMapItem {
    os_type: OsType,
    size: u32,
    offset: u32,
    flags: MemoryMapFlags,
    field_e: u16,
    data: RefCell<ChunkData>,
}

type ResourceMap = HashMap<ResourceId, ChunkIndex>;

fn detect_subtype<T: Read + Seek>(reader: &mut T) -> Option<DetectionInfo> {
    let mut chunk_size_raw = [ 0; 4 ];
    reader.read_exact(&mut chunk_size_raw).ok()?;

    let sub_type = reader.read_os_type::<BigEndian>().ok()?;

    match sub_type.as_bytes() {
        b"RMMP" => Some(DetectionInfo {
            os_type_endianness: Endian::Big,
            data_endianness: Endian::Little,
            version: Version::D3,
            kind: MovieKind::Movie,
            // This version of Director incorrectly includes the
            // size of the chunk header in the RIFF chunk size
            size: LittleEndian::read_u32(&chunk_size_raw) - 8,
        }),
        b"MV93" | b"39VM" => {
            let (endianness, size) = get_riff_attributes(sub_type, &chunk_size_raw);
            Some(DetectionInfo {
                os_type_endianness: endianness,
                data_endianness: endianness,
                version: Version::D4,
                kind: MovieKind::Movie,
                size,
            })
        },
        b"MC95" | b"59CM" => {
            let (endianness, size) = get_riff_attributes(sub_type, &chunk_size_raw);
            Some(DetectionInfo {
                os_type_endianness: endianness,
                data_endianness: endianness,
                version: Version::D4,
                kind: MovieKind::Cast,
                size,
            })
        },
        b"APPL" | b"LPPA" => {
            let (endianness, size) = get_riff_attributes(sub_type, &chunk_size_raw);
            Some(DetectionInfo {
                os_type_endianness: endianness,
                data_endianness: endianness,
                version: Version::D4,
                kind: MovieKind::Embedded,
                size,
            })
        },
        _ => None
    }
}

fn get_riff_attributes(os_type: OsType, raw_size: &[u8]) -> (Endian, u32) {
    // Director checks endianness based on the main RIFX OSType, but this works
    // just as well and simplifies support for the special D3Win format
    let endianness = match os_type.as_bytes()[0] {
        b'M' | b'A' => Endian::Big,
        _ => Endian::Little
    };

    let size = {
        if endianness == Endian::Big {
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
