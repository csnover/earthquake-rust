use byteorder::{ByteOrder, BigEndian, ReadBytesExt};
use crate::{Reader, string::StringReadExt};
use std::{collections::HashMap, io::{Result as IoResult, Read, SeekFrom}};

type Offset = u32;
pub(crate) type OSType = u32;

#[derive(Debug)]
pub(crate) struct Resource {
    pub name: Option<String>,
    pub data: Vec<u8>,
    pub flags: u8,
}

#[derive(Debug)]
pub struct OffsetCount {
    offset: Offset,
    count: u16,
}

#[derive(Debug)]
pub(crate) struct MacResourceFile<'a, T: Reader> {
    input: &'a mut T,
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
            data_offset,
            names_offset,
            resource_tables,
        })
    }

    fn build_resource(&mut self) -> Option<Resource> {
        const NO_NAME: u16 = 0xffff;

        let name_offset = self.input.read_u16::<BigEndian>().ok()?;
        let data_offset = self.input.read_u32::<BigEndian>().ok()?;

        let name = if name_offset == NO_NAME {
            None
        } else {
            self.input.seek(SeekFrom::Start((self.names_offset + name_offset as u32) as u64)).and_then(|_| {
                self.input.read_pascal_str()
            }).ok()
        };

        let (data, flags) = {
            const OFFSET_BITS: u8 = 24;
            const OFFSET_MASK: u32 = (1 << OFFSET_BITS) - 1;
            const FLAGS_MASK: u32 = !OFFSET_MASK;

            let offset = data_offset & OFFSET_MASK;
            let flags = ((data_offset & FLAGS_MASK) >> OFFSET_BITS) as u8;

            self.input.seek(SeekFrom::Start((self.data_offset + offset) as u64)).ok()?;
            let size = self.input.read_u32::<BigEndian>().ok()?;
            let mut data = Vec::with_capacity(size as usize);
            self.input.take(size as u64).read_to_end(&mut data).ok()?;
            (data, flags)
        };

        Some(Resource {
            name,
            data,
            flags,
        })
    }

    pub fn get_name(&mut self) -> Option<String> {
        self.input.seek(SeekFrom::Start(0x30)).ok()?;
        self.input.read_pascal_str().ok()
    }

    pub fn get_resource(&mut self, os_type: &str, id: u16) -> Option<Resource> {
        let resource_table = self.resource_tables.get(&BigEndian::read_u32(os_type.as_bytes()))?;

        self.input.seek(SeekFrom::Start(resource_table.offset as u64)).ok()?;
        for _ in 0..=resource_table.count {
            let found_id = self.input.read_u16::<BigEndian>().ok()?;
            if id != found_id {
                self.input.seek(SeekFrom::Current(10)).ok()?;
            } else {
                return self.build_resource();
            }
        }

        None
    }
}

#[cfg(test)]
mod tests {
    // TODO
}
