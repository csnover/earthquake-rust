use bitflags::bitflags;
use byteorder::{ByteOrder, BigEndian};
use byteordered::{ByteOrdered, StaticEndianness};
use encoding::all::MAC_ROMAN;
use crate::{os, OSType, OSTypeReadExt, Reader, compression::ApplicationVise, string::StringReadExt};
use std::{cell::RefCell, collections::HashMap, io::{ErrorKind, Result as IoResult, Read, Seek, SeekFrom}};

bitflags! {
    /// Flags set on a resource.
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

        /// Internal flag used by the Resource Manager.
        const CHANGED             = 0x02;

        /// The resource data is compressed.
        const COMPRESSED          = 0x01;
    }
}

#[derive(Debug)]
/// A resource from a Resource File.
pub struct Resource {
    pub name: Option<String>,
    pub data: Vec<u8>,
    pub flags: ResourceFlags,
}

type Input<T> = ByteOrdered<T, StaticEndianness<BigEndian>>;

#[derive(Debug)]
/// MacResourceFile is used to read Macintosh Resource File Format files.
/// These are the resource forks of all Mac OS Classic executables.
pub struct MacResourceFile<T: Reader> {
    input: RefCell<Input<T>>,
    decompressor: RefCell<Option<ApplicationVise>>,
    data_offset: Offset,
    names_offset: Offset,
    resource_tables: HashMap<OSType, OffsetCount>,
}

impl<T: Reader> MacResourceFile<T> {
    /// Creates a new MacResourceFile from a readable data stream.
    pub fn new(data: T) -> IoResult<Self> {
        let mut input = ByteOrdered::be(data);

        input.seek(SeekFrom::Start(0))?;
        let data_offset = input.read_u32()?;
        let map_offset = input.read_u32()?;

        input.seek(SeekFrom::Start(u64::from(map_offset) + 24))?;
        let types_offset = map_offset + u32::from(input.read_u16()?);
        let names_offset = map_offset + u32::from(input.read_u16()?);
        let num_types = input.read_u16()?;

        let mut resource_tables = HashMap::with_capacity(num_types as usize);
        for _ in 0..=num_types {
            let os_type = input.read_os_type::<BigEndian>()?;
            let count = input.read_u16()?;
            let offset = types_offset + Offset::from(input.read_u16()?);
            resource_tables.insert(os_type, OffsetCount { offset, count });
        }

        Ok(Self {
            input: RefCell::new(input),
            decompressor: RefCell::new(None),
            data_offset,
            names_offset,
            resource_tables,
        })
    }

    /// Tests whether the given resource exists in the file.
    pub fn contains(&self, os_type: OSType, id: u16) -> bool {
        for entry in self.iter_by_type(os_type) {
            if entry.id == id {
                return true;
            }
        }

        false
    }

    /// Gets a resource from the file.
    pub fn get(&self, os_type: OSType, id: u16) -> Option<Resource> {
        for entry in self.iter_by_type(os_type) {
            if entry.id == id {
                return Some(self.build_resource(&entry).expect("Error building resource"));
            }
        }

        None
    }

    /// Gets the name of the Resource File itself, if one exists. For Mac
    /// applications, this is the original name of the application.
    pub fn name(&self) -> Option<String> {
        let mut input = self.input.borrow_mut();
        input.seek(SeekFrom::Start(0x30)).ok()?;
        input.read_pascal_str(MAC_ROMAN).ok()
    }

    fn build_resource(&self, entry: &ResourceEntry) -> IoResult<Resource> {
        const NO_NAME: u16 = 0xffff;

        let mut input = self.input.borrow_mut();

        let name = if entry.name_offset == NO_NAME {
            None
        } else {
            input.seek(SeekFrom::Start(u64::from(self.names_offset + u32::from(entry.name_offset)))).and_then(|_| {
                input.read_pascal_str(MAC_ROMAN)
            }).ok()
        };

        let (data, flags) = {
            const OFFSET_BITS: u8 = 24;
            const OFFSET_MASK: u32 = (1 << OFFSET_BITS) - 1;
            const FLAGS_MASK: u32 = !OFFSET_MASK;

            let offset = entry.data_offset & OFFSET_MASK;
            let flags = ((entry.data_offset & FLAGS_MASK) >> OFFSET_BITS) as u8;

            input.seek(SeekFrom::Start(u64::from(self.data_offset + offset)))?;
            let size = input.read_u32()?;
            let mut data = Vec::with_capacity(size as usize);
            input.as_mut().take(u64::from(size)).read_to_end(&mut data)?;

            if ApplicationVise::is_compressed(&data) {
                data = self.decompress(&data)?;
            }

            (data, ResourceFlags::from_bits_truncate(flags))
        };

        Ok(Resource {
            name,
            data,
            flags,
        })
    }

    fn decompress(&self, data: &[u8]) -> IoResult<Vec<u8>> {
        if self.decompressor.borrow().is_none() {
            let iter = self.iter_by_type(os!(b"CODE"));
            let last_code = iter.last().expect("Missing CODE table");
            let resource = self.build_resource(&last_code)?;
            let data = ApplicationVise::find_shared_data(&resource.data).ok_or(ErrorKind::InvalidData)?;
            self.decompressor.replace(Some(ApplicationVise::new(data.to_vec())));
        }

        self.decompressor.borrow().as_ref().unwrap().decompress(&data)
    }

    fn resource_table(&self, os_type: OSType) -> Option<OffsetCount> {
        self.resource_tables.get(&os_type).copied()
    }

    fn iter_by_type(&self, os_type: OSType) -> ResourceTableIter {
        let table = if let Some(resource_table) = self.resource_table(os_type) {
            read_raw_resource_table(self.input.borrow_mut().as_mut(), resource_table).unwrap_or_else(|_| Vec::new())
        } else {
            Vec::new()
        };

        ResourceTableIter { table, offset: 0 }
    }
}

fn read_raw_resource_table<T: Reader>(mut input: T, resource_table: OffsetCount) -> IoResult<Vec<u8>> {
    input.seek(SeekFrom::Start(u64::from(resource_table.offset)))?;
    let table_size = (resource_table.count + 1) * RES_TABLE_ENTRY_SIZE;
    let mut table = Vec::with_capacity(table_size as usize);
    input.take(u64::from(table_size)).read_to_end(&mut table)?;
    Ok(table)
}

const RES_TABLE_ENTRY_SIZE: u16 = 12;

type Offset = u32;

#[derive(Debug, Copy, Clone)]
struct OffsetCount {
    offset: Offset,
    count: u16,
}

struct ResourceEntry {
    id: u16,
    name_offset: u16,
    data_offset: u32,
}

struct ResourceTableIter {
    table: Vec<u8>,
    offset: usize,
}

impl Iterator for ResourceTableIter {
    type Item = ResourceEntry;

    fn next(&mut self) -> Option<Self::Item> {
        if self.offset == self.table.len() {
            None
        } else {
            let id = BigEndian::read_u16(&self.table[self.offset..]);
            let name_offset = BigEndian::read_u16(&self.table[self.offset + 2..]);
            let data_offset = BigEndian::read_u32(&self.table[self.offset + 4..]);
            self.offset += RES_TABLE_ENTRY_SIZE as usize;
            Some(ResourceEntry { id, name_offset, data_offset })
        }
    }
}

#[cfg(test)]
mod tests {
    // TODO
}
