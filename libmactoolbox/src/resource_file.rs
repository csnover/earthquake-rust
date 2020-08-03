use anyhow::{anyhow, bail, Context, ensure, Result as AResult};
use bitflags::bitflags;
use byteorder::{ByteOrder, BigEndian};
use byteordered::{ByteOrdered, Endianness};
use crate::{ApplicationVise, OSType, OSTypeReadExt, ResourceId, rsid, string::ReadExt};
use derive_more::{Constructor, Display};
use libcommon::{encodings::MAC_ROMAN, Reader};
use std::{any::Any, cell::RefCell, collections::HashMap, io::{Cursor, Read, Seek, SeekFrom}, rc::{Weak, Rc}, sync::atomic::{Ordering, AtomicI16}};

#[derive(Clone, Constructor, Copy, Debug, Display, Eq, PartialEq)]
pub struct RefNum(i16);
static REF_NUM: AtomicI16 = AtomicI16::new(1);

#[derive(Debug)]
/// A Macintosh Resource File Format file reader.
pub struct ResourceFile<T: Reader> {
    input: RefCell<Input<T>>,
    decompressor: RefCell<DecompressorState>,
    resource_map: HashMap<ResourceId, ResourceOffsets>,
    counts: HashMap<OSType, u16>,
    reference_number: RefNum,
}

impl<T: Reader> ResourceFile<T> {
    /// Makes a new `ResourceFile` from a readable stream.
    pub fn new(data: T) -> AResult<Self> {
        const RESOURCE_MAP_OFFSETS_OFFSET: u64 = 24;
        let mut input = ByteOrdered::new(data, Endianness::Big);

        let data_offset = input.read_u32().context("Can’t read data offset")?;
        let map_offset = input.read_u32().context("Can’t read map offset")?;

        input.seek(SeekFrom::Start(u64::from(map_offset) + RESOURCE_MAP_OFFSETS_OFFSET))
            .map_err(|_| anyhow!("Bad resource map offset {}", map_offset))?;
        let types_offset = u64::from(map_offset + u32::from(input.read_u16().context("Can’t read types offset")?));
        let names_offset = map_offset + u32::from(input.read_u16().context("Can’t read names offset")?);
        let num_types = input.read_u16()? + 1;

        let (mut type_list, mut resource_map) = {
            const TYPE_LIST_ENTRY_SIZE: usize = 8;
            let mut list = Vec::with_capacity(TYPE_LIST_ENTRY_SIZE * num_types as usize);
            let mut num_resources = 0;
            input.as_mut().take(TYPE_LIST_ENTRY_SIZE as u64 * u64::from(num_types)).read_to_end(&mut list)
                .context("Can’t read types list")?;
            for i in 0..num_types {
                const DOCUMENTED_MAXIMUM: u32 = 2727;
                if num_resources > DOCUMENTED_MAXIMUM {
                    bail!("Bogus number of resources");
                }

                let offset = i as usize * TYPE_LIST_ENTRY_SIZE + 4;
                let entry_slice = &list.get(offset..offset + 2).with_context(|| format!("Premature end of resource list at {}/{}", i, num_types))?;
                num_resources += u32::from(BigEndian::read_u16(entry_slice)) + 1;
            }
            (ByteOrdered::be(Cursor::new(list)), HashMap::with_capacity(num_resources as usize))
        };

        let mut counts = HashMap::with_capacity(num_types as usize);

        let mut last_code_id = 0;
        for i in 0..num_types {
            let os_type = type_list.read_os_type::<BigEndian>()
                .with_context(|| format!("Can’t read OSType of resource table index {}", i))?;
            let num_resources = type_list.read_u16()
                .with_context(|| format!("Can’t read number of resources for {}", os_type))?;
            let table_offset = types_offset + u64::from(type_list.read_u16()
                .with_context(|| format!("Can’t read resource table offset for {}", os_type))?);

            counts.insert(os_type, num_resources + 1);

            input.seek(SeekFrom::Start(table_offset))
                .with_context(|| format!("Bad offset {} for {} resource list", table_offset, os_type))?;

            let mut resource_num = 0;
            for i in 0..=num_resources {
                resource_num = input.read_i16()
                    .with_context(|| format!("Can’t read resource number of {} index {}", os_type, i))?;

                let resource_id = ResourceId(os_type, resource_num);

                let name_offset = {
                    const NO_NAME: u16 = 0xffff;
                    let value = input.read_u16()
                        .with_context(|| format!("Can’t read name offset of {}", resource_id))?;
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

                    let value = input.read_u32()
                        .with_context(|| format!("Can’t read offset of {}", resource_id))?;
                    let offset = value & OFFSET_MASK;
                    let flags = ((value & FLAGS_MASK) >> OFFSET_BITS) as u8;
                    (data_offset + offset, ResourceFlags::from_bits_truncate(flags))
                };

                input.skip(4)?;

                resource_map.insert(resource_id, ResourceOffsets {
                    name_offset,
                    data_offset,
                    flags,
                    data: RefCell::new(None),
                });
            }

            if os_type.as_bytes() == b"CODE" {
                last_code_id = resource_num;
            }
        }

        Ok(Self {
            input: RefCell::new(input),
            decompressor: RefCell::new(DecompressorState::Waiting(last_code_id)),
            resource_map,
            counts,
            reference_number: RefNum(REF_NUM.fetch_add(1, Ordering::Relaxed)),
        })
    }

    /// Returns `true` if the resource file contains the resource with the given
    /// ID.
    pub fn contains(&self, id: ResourceId) -> bool {
        self.resource_map.contains_key(&id)
    }

    /// Returns `true` if the resource file contains the resource with the given
    /// ID.
    pub fn contains_type(&self, os_type: OSType) -> bool {
        self.counts.get(&os_type).is_some()
    }

    pub fn count(&self, os_type: OSType) -> u16 {
        *self.counts.get(&os_type).unwrap_or(&0)
    }

    pub fn load<R: 'static + libcommon::Resource>(&self, id: ResourceId) -> AResult<Rc<R>> {
        let entry = self.resource_map.get(&id)
            .with_context(|| format!("Resource {} not found", id))?;

        ensure!(!entry.flags.contains(ResourceFlags::COMPRESSED), "Resource {} uses unsupported compression", id);

        if let Some(data) = entry.data.borrow().as_ref().and_then(Weak::upgrade) {
            return data.downcast::<R>()
                .map_err(|_| anyhow!("Invalid data type for resource {}", id));
        }

        let mut input = self.input.try_borrow_mut()?;
        input.seek(SeekFrom::Start(u64::from(entry.data_offset)))
            .with_context(|| format!("Can’t seek to resource {}", id))?;

        let size = input.read_u32()
            .with_context(|| format!("Can’t read size of resource {}", id))?;

        let is_vise_compressed = {
            let mut sig = [ 0; 4 ];
            input.read_exact(&mut sig).ok();
            input.seek(SeekFrom::Start(u64::from(entry.data_offset) + 4))
                .with_context(|| format!("Can’t seek to resource {}", id))?;
            ApplicationVise::is_compressed(&sig)
        };

        if is_vise_compressed {
            let data = {
                let mut compressed_data = Vec::with_capacity(size as usize);
                input.as_mut().take(u64::from(size)).read_to_end(&mut compressed_data)?;
                self.decompress(&compressed_data)
                    .with_context(|| format!("Can’t decompress resource {}", id))?
            };
            let decompressed_size = data.len() as u32;
            R::load(&mut ByteOrdered::new(Cursor::new(data), Endianness::Big), decompressed_size)
        } else {
            R::load(&mut input.as_mut(), size)
        }.map(|resource| {
            let resource = Rc::new(resource);
            *entry.data.borrow_mut() = Some(Rc::downgrade(&(Rc::clone(&resource) as Rc<dyn Any>)));
            resource
        })
    }

    pub fn iter<'a>(&'a self) -> impl Iterator<Item = ResourceId> + 'a {
        self.resource_map.keys().copied()
    }

    /// Returns the name embedded in the Resource File. For applications, this
    /// is the name of the application.
    pub fn name(&self) -> Option<String> {
        let mut input = self.input.borrow_mut();
        input.seek(SeekFrom::Start(0x30)).ok()?;
        input.read_pascal_str(MAC_ROMAN).ok()
    }

    pub fn reference_number(&self) -> RefNum {
        self.reference_number
    }

    fn decompress(&self, data: &[u8]) -> AResult<Vec<u8>> {
        // https://stackoverflow.com/questions/33495933/how-to-end-a-borrow-in-a-match-or-if-let-expression
        let resource_id = if let DecompressorState::Waiting(resource_id) = *self.decompressor.borrow() {
            Some(resource_id)
        } else {
            None
        };

        if let Some(resource_id) = resource_id {
            let resource_data = self.load::<Vec<u8>>(rsid!(b"CODE", resource_id))
                .context("Can’t find the Application VISE CODE resource")?;
            let shared_data = ApplicationVise::find_shared_data(&resource_data)
                .context("Can’t find the Application VISE shared dictionary")?;
            self.decompressor.replace(DecompressorState::Loaded(ApplicationVise::new(shared_data.to_vec())));
        }

        if let DecompressorState::Loaded(decompressor) = &*self.decompressor.borrow() {
            decompressor.decompress(&data).context("Decompression failure")
        } else {
            unreachable!();
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

type Input<T> = ByteOrdered<T, Endianness>;

#[derive(Clone, Debug)]
struct ResourceOffsets {
    name_offset: Option<u32>,
    data_offset: u32,
    flags: ResourceFlags,
    data: RefCell<Option<Weak<dyn Any>>>,
}

#[cfg(test)]
mod tests {
    // TODO
}
