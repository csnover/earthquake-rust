use bitflags::bitflags;
use byteorder::{ByteOrder, BigEndian};
use byteordered::{ByteOrdered, StaticEndianness};
use encoding::all::MAC_ROMAN;
use crate::{OSType, OSTypeReadExt, Reader, ResourceId, compression::ApplicationVise, rsid, string::StringReadExt};
use std::{cell::RefCell, collections::HashMap, io::{self, Cursor, Read, Seek, SeekFrom}};

#[derive(Debug)]
/// A Macintosh Resource File Format file reader.
pub struct MacResourceFile<T: Reader> {
    input: RefCell<Input<T>>,
    decompressor: RefCell<DecompressorState>,
    resource_map: HashMap<ResourceId, ResourceOffsets>,
}

impl<T: Reader> MacResourceFile<T> {
    /// Makes a new MacResourceFile from a readable stream.
    pub fn new(data: T) -> io::Result<Self> {
        const RESOURCE_MAP_OFFSETS_OFFSET: u64 = 24;
        let mut input = ByteOrdered::be(data);

        let data_offset = input.read_u32()?;
        let map_offset = input.read_u32()?;

        input.seek(SeekFrom::Start(u64::from(map_offset) + RESOURCE_MAP_OFFSETS_OFFSET))?;
        let types_offset = u64::from(map_offset + u32::from(input.read_u16()?));
        let names_offset = map_offset + u32::from(input.read_u16()?);
        let num_types = input.read_u16()? + 1;

        let (mut type_list, mut resource_map) = {
            const TYPE_LIST_ENTRY_SIZE: usize = 8;
            let mut list = Vec::with_capacity(TYPE_LIST_ENTRY_SIZE * num_types as usize);
            let mut num_resources = 0;
            input.as_mut().take(TYPE_LIST_ENTRY_SIZE as u64 * u64::from(num_types)).read_to_end(&mut list)?;
            for i in 0..num_types {
                num_resources += u32::from(BigEndian::read_u16(&list[i as usize * TYPE_LIST_ENTRY_SIZE + 4..]) + 1);
            }
            (ByteOrdered::be(Cursor::new(list)), HashMap::with_capacity(num_resources as usize))
        };

        let mut last_code_id = 0;
        for _ in 0..num_types {
            let os_type = type_list.read_os_type::<BigEndian>()?;
            let num_resources = type_list.read_u16()?;
            let table_offset = types_offset + u64::from(type_list.read_u16()?);

            input.seek(SeekFrom::Start(table_offset))?;

            let mut resource_id = 0;
            for _ in 0..=num_resources {
                resource_id = input.read_i16()?;

                let name_offset = {
                    const NO_NAME: u16 = 0xffff;
                    let value = input.read_u16()?;
                    if value == NO_NAME {
                        None
                    } else {
                        Some(names_offset + u32::from(value))
                    }
                };

                let (data_offset, flags) = {
                    const OFFSET_BITS: u8 = 24;
                    const OFFSET_MASK: u32 = (1 << OFFSET_BITS) - 1;
                    const FLAGS_MASK: u32 = !OFFSET_MASK;

                    let value = input.read_u32()?;
                    let offset = value & OFFSET_MASK;
                    let flags = ((value & FLAGS_MASK) >> OFFSET_BITS) as u8;
                    (data_offset + offset, ResourceFlags::from_bits_truncate(flags))
                };

                input.seek(SeekFrom::Current(4))?; // Reserved

                resource_map.insert(ResourceId(os_type, resource_id), ResourceOffsets {
                    name_offset,
                    data_offset,
                    flags
                });
            }

            if os_type.as_bytes() == b"CODE" {
                last_code_id = resource_id;
            }
        }

        Ok(Self {
            input: RefCell::new(input),
            decompressor: RefCell::new(DecompressorState::Waiting(last_code_id)),
            resource_map,
        })
    }

    /// Returns `true` if the resource file contains the resource with the given
    /// ID.
    pub fn contains(&self, id: ResourceId) -> bool {
        self.resource_map.contains_key(&id)
    }

    /// Returns a handle to retrieve the resource with the given ID.
    pub fn get(&self, id: ResourceId) -> Option<Resource<T>> {
        if let Some(offsets) = self.resource_map.get(&id) {
            Some(Resource {
                id,
                owner: self,
                offsets: *offsets,
            })
        } else {
            None
        }
    }

    pub fn iter(&self) -> impl Iterator<Item = Resource<T>> {
        self.resource_map.iter().map(move |(k, v)| Resource {
            id: *k,
            owner: self,
            offsets: *v,
        })
    }

    /// Returns the name embedded in the Resource File. For applications, this
    /// is the name of the application.
    pub fn name(&self) -> Option<String> {
        let mut input = self.input.borrow_mut();
        input.seek(SeekFrom::Start(0x30)).ok()?;
        input.read_pascal_str(MAC_ROMAN).ok()
    }

    fn build_resource_data(&self, offsets: &ResourceOffsets) -> io::Result<Vec<u8>> {
        let mut input = self.input.borrow_mut();

        input.seek(SeekFrom::Start(u64::from(offsets.data_offset)))?;
        let size = input.read_u32()?;
        let mut data = Vec::with_capacity(size as usize);
        input.as_mut().take(u64::from(size)).read_to_end(&mut data)?;

        if ApplicationVise::is_compressed(&data) {
            data = self.decompress(&data)?;
        }

        Ok(data)
    }

    fn decompress(&self, data: &[u8]) -> io::Result<Vec<u8>> {
        if let DecompressorState::Waiting(resource_id) = *self.decompressor.borrow() {
            let resource = self.get(rsid!(b"CODE", resource_id)).unwrap();
            let resource_data = resource.data()?;
            let shared_data = ApplicationVise::find_shared_data(&resource_data)
                .ok_or_else(|| io::Error::new(io::ErrorKind::InvalidData, "Could not find the Application VISE shared dictionary"))?;
            self.decompressor.replace(DecompressorState::Loaded(ApplicationVise::new(shared_data.to_vec())));
        }

        if let DecompressorState::Loaded(decompressor) = &*self.decompressor.borrow() {
            decompressor.decompress(&data)
        } else {
            unreachable!();
        }
    }
}

#[derive(Debug)]
/// A resource from a Resource File.
pub struct Resource<'a, T: Reader> {
    id: ResourceId,
    owner: &'a MacResourceFile<T>,
    offsets: ResourceOffsets,
}

impl<'a, T: Reader> Resource<'a, T> {
    /// Returns the resource’s data.
    pub fn data(&self) -> io::Result<Vec<u8>> {
        self.owner.build_resource_data(&self.offsets)
    }

    /// Returns the resource’s flags.
    pub fn flags(&self) -> ResourceFlags {
        self.offsets.flags
    }

    /// Returns the resources’s ID.
    pub fn id(&self) -> ResourceId {
        self.id
    }

    /// Returns the resource’s name.
    pub fn name(&self) -> Option<String> {
        if let Some(name_offset) = self.offsets.name_offset {
            let mut input = self.owner.input.borrow_mut();
            input.seek(SeekFrom::Start(u64::from(name_offset))).ok()?;
            Some(input.read_pascal_str(MAC_ROMAN).ok()?)
        } else {
            None
        }
    }
}

bitflags! {
    /// The flags set on a resource from a Resource File.
    pub struct ResourceFlags: u8 {
        /// Reserved; unused.
        const RESERVED            = 0x80;

        /// The resource should be loaded in the system heap instead of the
        /// application heap.
        const LOAD_TO_SYSTEM_HEAP = 0x40;

        /// The resource may be paged out of memory.
        const PURGEABLE           = 0x20;

        /// The resource may not be moved in memory.
        const LOCKED              = 0x10;

        /// The resource is read-only.
        const READ_ONLY           = 0x08;

        /// The resource should be loaded as soon as the file is opened.
        const PRELOAD             = 0x04;

        /// An internal flag used by the Resource Manager.
        const CHANGED             = 0x02;

        /// The resource data is compressed.
        const COMPRESSED          = 0x01;
    }
}

#[derive(Debug)]
enum DecompressorState {
    Waiting(i16),
    Loaded(ApplicationVise),
}

type Input<T> = ByteOrdered<T, StaticEndianness<BigEndian>>;

#[derive(Copy, Clone, Debug)]
struct ResourceOffsets {
    name_offset: Option<u32>,
    data_offset: u32,
    flags: ResourceFlags,
}

#[cfg(test)]
mod tests {
    // TODO
}
