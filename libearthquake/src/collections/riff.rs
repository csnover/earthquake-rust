use anyhow::{anyhow, bail, Context, Result as AResult};
use bitflags::bitflags;
use byteorder::{BigEndian, ByteOrder, LittleEndian, ReadBytesExt};
use byteordered::{ByteOrdered, Endianness};
use crate::{
    bail_sample,
    detection::movie::{
        DetectionInfo,
        Kind as MovieKind,
        Version as MovieVersion,
    },
    ensure_sample,
};
use derive_more::{Constructor, Deref, DerefMut, Display};
use libcommon::{
    Reader,
    Resource,
    SharedStream,
};
use libmactoolbox::{
    os,
    OSType,
    OSTypeReadExt,
    ResourceId,
};
use std::{
    any::Any,
    cell::RefCell,
    collections::HashMap,
    io::{Seek, SeekFrom},
    rc::{Rc, Weak},
};

#[derive(Clone, Copy, Constructor, Debug, Display, Eq, Ord, PartialEq, PartialOrd)]
pub struct ChunkIndex(i32);

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

    // TODO: For debugging only
    pub fn len(&self) -> u32 {
        self.owner.memory_map.get(self.chunk_index).unwrap().size
    }

    pub fn load<R: Resource + 'static>(&self, context: &R::Context) -> AResult<Rc<R>> {
        self.owner.load(self.chunk_index, context)
    }

    // TODO: For debugging only
    pub fn offset(&self) -> u32 {
        self.owner.memory_map.get(self.chunk_index).unwrap().offset
    }
}

#[derive(Clone, Debug)]
pub struct Riff<T: Reader> {
    // TODO: This is needed to convert a Riff chunk back to its owner filename,
    // but not enough information is recorded currently to actually do this.
    id: Identity,

    input: RefCell<ByteOrdered<SharedStream<T>, byteordered::Endianness>>,

    /// Index for O(1) access of RIFF chunks by index.
    memory_map: MemoryMap,

    /// Mac resource types to RIFF chunk indexes.
    resource_map: ResourceMap,

    info: DetectionInfo,
}

impl<T: Reader> Riff<T> {
    pub fn new(input: T) -> AResult<Self> {
        Self::with_identity(Identity::Parent, SharedStream::new(input))
    }

    #[must_use]
    pub fn first_of_kind(&self, kind: OSType) -> ChunkIndex {
        for (index, item) in self.memory_map.iter().enumerate() {
            if item.os_type == kind {
                return ChunkIndex::new(index as i32);
            }
        }
        ChunkIndex::new(-1)
    }

    pub fn has_id(&self, id: ResourceId) -> bool {
        self.resource_map.get(&id).is_some()
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

    pub fn load<R: 'static + Resource>(&self, index: ChunkIndex, context: &R::Context) -> AResult<Rc<R>> {
        let entry = self.memory_map.get(index)
            .with_context(|| format!("Invalid RIFF index {}", index))?;

        if let Some(data) = entry.data.borrow().as_data().and_then(Weak::upgrade) {
            return data.downcast::<R>()
                .map_err(|_| anyhow!("Invalid data type for index {}", index));
        }

        let mut input = self.input.borrow_mut();
        input.seek(SeekFrom::Start(u64::from(entry.offset) + Self::CHUNK_HEADER_SIZE))
            .with_context(|| format!("Can’t seek to RIFF index {}", index))?;

        R::load(&mut input, entry.size, context).map(|resource| {
            let resource = Rc::new(resource);
            *entry.data.borrow_mut() = ChunkData::Loaded(Rc::downgrade(&(Rc::clone(&resource) as Rc<dyn Any>)));
            resource
        })
    }

    pub fn load_id<R: 'static + Resource>(&self, id: ResourceId, context: &R::Context) -> AResult<Rc<R>> {
        if let Some(&chunk_index) = self.resource_map.get(&id) {
            Self::load::<R>(self, chunk_index, context)
        } else {
            bail!("Invalid resource ID {}", id)
        }
    }

    pub fn load_riff(&self, index: ChunkIndex) -> AResult<Self> {
        let entry = self.memory_map.get(index)
            .with_context(|| format!("Invalid RIFF index {}", index))?;

        let mut input = self.input.borrow_mut().inner_mut().clone();
        input.seek(SeekFrom::Start(u64::from(entry.offset)))
            .with_context(|| format!("Invalid RIFF offset {} for index {}", entry.offset, index))?;
        Self::with_identity(Identity::Child(index), input)
    }

    pub fn make_free(&mut self, index: ChunkIndex) {
        let next_free_index = self.memory_map.next_free_index;
        let entry = self.memory_map.get_mut(index).unwrap();
        entry.os_type = os!(b"free");
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
        entry.os_type = os!(b"junk");
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
    pub fn version(&self) -> MovieVersion {
        self.info.version()
    }

    const CHUNK_HEADER_SIZE: u64 = 8;
    const KEYS_HEADER_SIZE: u64 = 12;
    const MMAP_HEADER_SIZE: u64 = 24;
    const MMAP_ENTRY_SIZE: u64 = 20;
    // n.b. Number of entries is limited to 3275 so the mmap always fits
    // in a 64k page (`(0xffff - header_size) / entry_size`)
    const MMAP_MAX_ENTRIES: usize = ((0xFFFF - Self::MMAP_HEADER_SIZE) / Self::MMAP_ENTRY_SIZE) as usize;

    fn init<R: Reader, OE: ByteOrder, DE: ByteOrder>(input: &mut R) -> AResult<(MemoryMap, ResourceMap)> {
        let os_type = input.read_os_type::<OE>()?;
        match os_type.as_bytes() {
            b"CFTC" => Self::read_cftc::<R, OE, DE>(input),
            // n.b. Director actually seeks through every RIFF chunk until it
            // finds an imap, but it is always the first chunk in a
            // well-authored file, so for implementation simplicity we do not do
            // that.
            b"imap" => Self::read_imap::<R, OE, DE>(input),
            _ => bail!("Could not find a valid resource map; found {} instead", os_type),
        }
    }

    fn read_cftc<R: Reader, OE: ByteOrder, DE: ByteOrder>(input: &mut R) -> AResult<(MemoryMap, ResourceMap)> {
        // CFTC map was not RCE'd so may not be correct.
        const ENTRY_SIZE: u32 = 16;

        let mut bytes_to_read = input.read_u32::<DE>()?;

        // TODO: Is this value important? It seems to always be 0.
        input.skip(4)?;
        bytes_to_read -= 4;

        let (mut memory_map_items, mut resource_map) = {
            let num_entries = (bytes_to_read / ENTRY_SIZE - 1) as usize;
            (Vec::with_capacity(num_entries), HashMap::with_capacity(num_entries))
        };

        while bytes_to_read != 0 {
            let os_type = input.read_os_type::<OE>()?;
            if os_type == os!(b"\0\0\0\0") {
                break;
            }
            let size = input.read_u32::<DE>()?;
            let id = input.read_i32::<DE>()? as i16;
            let offset = input.read_u32::<DE>()?;

            let mmap_index = ChunkIndex(memory_map_items.len() as i32);
            memory_map_items.push(MemoryMapItem {
                os_type,
                size,
                offset,
                flags: MemoryMapFlags::empty(),
                field_e: 0,
                data: RefCell::new(ChunkData::Free { next_free: ChunkIndex::new(-1) }),
            });

            if resource_map.insert(ResourceId(os_type, id), mmap_index).is_some() {
                bail_sample!("Multiple {} {} in CFTC", os_type, id);
            }

            bytes_to_read -= ENTRY_SIZE;
        }

        Ok((MemoryMap {
            items: memory_map_items,
            next_free_index: ChunkIndex::new(-1),
            next_junk_index: ChunkIndex::new(-1),
        }, resource_map))
    }

    fn read_imap<R: Reader, OE: ByteOrder, DE: ByteOrder>(input: &mut R) -> AResult<(MemoryMap, ResourceMap)> {
        let _imap_size = input.skip(4)?;
        let _num_maps = input.skip(4)?;
        let map_offset = input.read_u32::<DE>()?;
        // imap contains the reference to the active mmap chunk for the file
        // along with some other unknown data which we can ignore for now
        input.seek(SeekFrom::Start(u64::from(map_offset)))
            .context("Can’t seek to mmap")?;

        let os_type = input.read_os_type::<OE>()?;
        if os_type != os!(b"mmap") {
            bail!("Can’t find a valid resource map; found {} instead", os_type);
        }

        let _chunk_size = input.skip(4)?;

        let header_size = input.read_u16::<DE>()?;
        ensure_sample!(header_size == 0x18, "Unexpected mmap header size {}", header_size);
        let entry_size = input.read_u16::<DE>()?;
        ensure_sample!(entry_size == 0x14, "Unexpected mmap entry size {}", entry_size);
        let _table_capacity = input.skip(4)?;
        let num_entries = input.read_u32::<DE>()? as usize;
        ensure_sample!(num_entries <= Self::MMAP_MAX_ENTRIES, "Invalid number of mmap entries {}", num_entries);
        let next_junk_index = ChunkIndex::new(input.read_i32::<DE>()?);
        let _garbage = input.skip(4)?;
        let next_free_index = ChunkIndex::new(input.read_i32::<DE>()?);
        input.skip(u64::from(header_size) - Self::MMAP_HEADER_SIZE)?;

        let mut memory_map_items = Vec::with_capacity(num_entries);

        let mut resource_map_offset = None;
        for index in 0..num_entries {
            let os_type = input.read_os_type::<OE>()?;
            let size = input.read_u32::<DE>()?;
            let offset = input.read_u32::<DE>()?;
            let flags_bits = input.read_u16::<DE>()?;
            let flags = MemoryMapFlags::from_bits(flags_bits).with_context(|| {
                format!("Invalid flags in mmap entry {}: {:x}", index, flags_bits)
            })?;
            let field_e = input.read_u16::<DE>()?;
            let next_free_index = ChunkIndex::new(input.read_i32::<DE>()?);
            input.skip(u64::from(entry_size) - Self::MMAP_ENTRY_SIZE)?;
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
                && os_type == os!(b"KEY*")
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
            input.seek(SeekFrom::Start(u64::from(offset)))
                .context("Can’t seek to KEY*")?;
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

    fn read_keys<R: Reader, OE: ByteOrder, DE: ByteOrder>(input: &mut R) -> AResult<ResourceMap> {
        if input.read_os_type::<OE>()? != os!(b"KEY*") {
            bail!("Bad KEY* offset");
        }

        let _chunk_size = input.skip(4)?;
        let header_size = input.read_u16::<DE>()?;
        let _item_size = input.skip(2)?;
        let _capacity = input.skip(4)?;
        let num_entries = input.read_u32::<DE>()?;
        input.skip(u64::from(header_size) - Self::KEYS_HEADER_SIZE)?;

        let mut resource_map = HashMap::with_capacity(num_entries as usize);

        for _ in 0..num_entries {
            let riff_index = ChunkIndex::new(input.read_i32::<DE>()?);
            let id = input.read_i32::<DE>()? as i16;
            let os_type = input.read_os_type::<DE>()?;

            if resource_map.insert(ResourceId(os_type, id), riff_index).is_some() {
                bail_sample!("Multiple {} {} in KEY*", os_type, id);
            }
        }

        Ok(resource_map)
    }

    fn with_identity(id: Identity, mut input: SharedStream<T>) -> AResult<Self> {
        let info = detect(&mut input)?;

        let (memory_map, resource_map) = {
            if info.os_type_endianness() == Endianness::Little && info.data_endianness() == Endianness::Little {
                Self::init::<_, LittleEndian, LittleEndian>(&mut input)?
            } else if info.os_type_endianness() == Endianness::Big && info.data_endianness() == Endianness::Little {
                Self::init::<_, BigEndian, LittleEndian>(&mut input)?
            } else if info.os_type_endianness() == Endianness::Big && info.data_endianness() == Endianness::Big {
                Self::init::<_, BigEndian, BigEndian>(&mut input)?
            } else {
                unreachable!("big endian data with little endian OSType does not exist");
            }
        };

        Ok(Self {
            id,
            input: RefCell::new(ByteOrdered::runtime(input, info.data_endianness())),
            memory_map,
            resource_map,
            info
        })
    }
}

pub fn detect(reader: &mut impl Reader) -> AResult<DetectionInfo> {
    let os_type = reader.read_os_type::<BigEndian>()?;
    match os_type.as_bytes() {
        b"RIFX" | b"RIFF" | b"XFIR" => detect_subtype(reader).context("Not a Director RIFF"),
        b"FFIR" => bail_sample!("RIFF-LE"),
        _ => bail!("Not a RIFF file"),
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
        self.items.get(index.0 as usize)
    }

    fn get_mut(&mut self, index: ChunkIndex) -> Option<&mut MemoryMapItem> {
        self.items.get_mut(index.0 as usize)
    }
}

impl ::core::ops::Index<ChunkIndex> for MemoryMap {
    type Output = MemoryMapItem;
    fn index(&self, index: ChunkIndex) -> &Self::Output {
        &self.items[index.0 as usize]
    }
}

impl ::core::ops::IndexMut<ChunkIndex> for MemoryMap {
    fn index_mut(&mut self, index: ChunkIndex) -> &mut Self::Output {
        &mut self.items[index.0 as usize]
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
    os_type: OSType,
    size: u32,
    offset: u32,
    flags: MemoryMapFlags,
    field_e: u16,
    data: RefCell<ChunkData>,
}

type ResourceMap = HashMap<ResourceId, ChunkIndex>;

fn detect_subtype<T: Reader>(reader: &mut T) -> Option<DetectionInfo> {
    let mut chunk_size_raw = [ 0; 4 ];
    reader.read_exact(&mut chunk_size_raw).ok()?;

    let sub_type = reader.read_os_type::<BigEndian>().ok()?;

    match sub_type.as_bytes() {
        b"RMMP" => Some(DetectionInfo {
            os_type_endianness: Endianness::Big,
            data_endianness: Endianness::Little,
            version: MovieVersion::D3,
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
                version: MovieVersion::D4,
                kind: MovieKind::Movie,
                size,
            })
        },
        b"MC95" | b"59CM" => {
            let (endianness, size) = get_riff_attributes(sub_type, &chunk_size_raw);
            Some(DetectionInfo {
                os_type_endianness: endianness,
                data_endianness: endianness,
                version: MovieVersion::D4,
                kind: MovieKind::Cast,
                size,
            })
        },
        b"APPL" | b"LPPA" => {
            let (endianness, size) = get_riff_attributes(sub_type, &chunk_size_raw);
            Some(DetectionInfo {
                os_type_endianness: endianness,
                data_endianness: endianness,
                version: MovieVersion::D4,
                kind: MovieKind::Embedded,
                size,
            })
        },
        _ => None
    }
}

fn get_riff_attributes(os_type: OSType, raw_size: &[u8]) -> (Endianness, u32) {
    // Director checks endianness based on the main RIFX OSType, but this works
    // just as well and simplifies support for the special D3Win format
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
