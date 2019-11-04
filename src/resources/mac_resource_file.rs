use byteorder::{ByteOrder, BigEndian, ReadBytesExt};
use crate::{Reader, compression::ApplicationVise, string::StringReadExt};
use std::{collections::HashMap, io::{ErrorKind, Result as IoResult, Read, SeekFrom}};

const RES_TABLE_ENTRY_SIZE: u16 = 12;

type Offset = u32;
pub(crate) type OSType = u32;

#[derive(Debug)]
pub(crate) struct Resource {
    pub name: Option<String>,
    pub data: Vec<u8>,
    pub flags: u8,
}

#[derive(Debug, Copy, Clone)]
pub struct OffsetCount {
    offset: Offset,
    count: u16,
}

#[derive(Debug)]
pub(crate) struct MacResourceFile<'a, T: Reader> {
    input: &'a mut T,
    decompressor: Option<ApplicationVise>,
    data_offset: Offset,
    names_offset: Offset,
    resource_tables: HashMap<OSType, OffsetCount>,
}

impl<'a, T: Reader> MacResourceFile<'a, T> {
    pub fn new(data: &'a mut T) -> IoResult<Self> {
        data.seek(SeekFrom::Start(0))?;
        let data_offset = data.read_u32::<BigEndian>()?;
        let map_offset = data.read_u32::<BigEndian>()?;

        data.seek(SeekFrom::Start(map_offset as u64 + 24))?;
        let types_offset = map_offset + data.read_u16::<BigEndian>()? as u32;
        let names_offset = map_offset + data.read_u16::<BigEndian>()? as u32;
        let num_types = data.read_u16::<BigEndian>()?;

        let mut resource_tables = HashMap::with_capacity(num_types as usize);
        for _ in 0..=num_types {
            let os_type = data.read_u32::<BigEndian>()? as OSType;
            let count = data.read_u16::<BigEndian>()?;
            let offset = types_offset + data.read_u16::<BigEndian>()? as Offset;
            resource_tables.insert(os_type, OffsetCount { offset, count });
        }

        Ok(Self {
            input: data,
            decompressor: None,
            data_offset,
            names_offset,
            resource_tables,
        })
    }

    fn decompress(&mut self, data: &[u8]) -> IoResult<Vec<u8>> {
        if self.decompressor.is_none() {
            let iter = self.iter_by_type("CODE").expect("Missing CODE table");
            // It is impossible to not get an item from this iterator, since
            // there is no way to have a zero-element resource table
            let last_code = iter.last().unwrap();
            let resource = self.build_resource(&last_code)?;
            let data = ApplicationVise::find_shared_data(&resource.data).ok_or(ErrorKind::InvalidData)?;
            self.decompressor = Some(ApplicationVise::new(data.to_vec()));
        }

        self.decompressor.as_ref().unwrap().decompress(&data)
    }

    fn build_resource(&mut self, entry: &ResourceEntry) -> IoResult<Resource> {
        const NO_NAME: u16 = 0xffff;

        let name = if entry.name_offset == NO_NAME {
            None
        } else {
            self.input.seek(SeekFrom::Start((self.names_offset + entry.name_offset as u32) as u64)).and_then(|_| {
                self.input.read_pascal_str()
            }).ok()
        };

        let (data, flags) = {
            const OFFSET_BITS: u8 = 24;
            const OFFSET_MASK: u32 = (1 << OFFSET_BITS) - 1;
            const FLAGS_MASK: u32 = !OFFSET_MASK;

            let offset = entry.data_offset & OFFSET_MASK;
            let flags = ((entry.data_offset & FLAGS_MASK) >> OFFSET_BITS) as u8;

            self.input.seek(SeekFrom::Start((self.data_offset + offset) as u64))?;
            let size = self.input.read_u32::<BigEndian>()?;
            let mut data = Vec::with_capacity(size as usize);
            self.input.take(size as u64).read_to_end(&mut data)?;

            if ApplicationVise::is_compressed(&data) {
                data = self.decompress(&data)?;
            }

            (data, flags)
        };

        Ok(Resource {
            name,
            data,
            flags,
        })
    }

    pub fn get_name(&mut self) -> Option<String> {
        self.input.seek(SeekFrom::Start(0x30)).ok()?;
        self.input.read_pascal_str().ok()
    }

    fn get_resource_table(&self, os_type: &str) -> Option<OffsetCount> {
        self.resource_tables.get(&BigEndian::read_u32(os_type.as_bytes())).copied()
    }

    fn iter_by_type(&mut self, os_type: &str) -> Option<ResourceTableIter> {
        let resource_table = self.get_resource_table(os_type)?;
        self.input.seek(SeekFrom::Start(resource_table.offset as u64)).ok()?;
        let table_size = (resource_table.count + 1) * RES_TABLE_ENTRY_SIZE;
        let mut table = Vec::with_capacity(table_size as usize);
        self.input.take(table_size as u64).read_to_end(&mut table).ok()?;
        Some(ResourceTableIter { table, offset: 0 })
    }

    pub fn get_resource(&mut self, os_type: &str, id: u16) -> Option<Resource> {
        for entry in self.iter_by_type(os_type)? {
            if entry.id == id {
                return Some(self.build_resource(&entry).expect("Error building resource"));
            }
        }

        None
    }
}

pub struct ResourceEntry {
    id: u16,
    name_offset: u16,
    data_offset: u32,
}

pub struct ResourceTableIter {
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
