use byteorder::{ByteOrder, BigEndian, ReadBytesExt};
use std::{collections::HashMap, io::{Cursor, Seek, SeekFrom}, mem::size_of};

pub(crate) type OSType = u32;
pub(crate) type ResourcesOfType = HashMap<u16, Resource>;
pub(crate) type Resources = HashMap<OSType, ResourcesOfType>;

// TODO: Maybe this should just store the offsets and the raw data is kept
// all together in the M68k struct. Or just don’t even bother to make a map
// since the number of entries is so tiny.
#[derive(Debug)]
pub(crate) struct Resource {
    pub name: String,
    pub data: Vec<u8>,
    pub flags: u8,
}

#[derive(Debug)]
pub(crate) struct M68k {
    pub extra_data: Vec<u8>,
    resources: Resources,
}

impl M68k {
    pub fn get_resource(&self, os_type: &str, id: u16) -> Option<&Resource> {
        self.resources.get(&BigEndian::read_u32(os_type.as_bytes()))?.get(&id)
    }
}

fn build_resource(data_section: &[u8], resource_entry: &[u8], name_list: &[u8]) -> Resource {
    const NO_NAME: usize = 0xffff;

    let name_offset = BigEndian::read_u16(&resource_entry[2..]) as usize;

    let (data, flags) = {
        const OFFSET_BITS: u8 = 24;
        const OFFSET_MASK: u32 = (1 << OFFSET_BITS) - 1;
        const FLAGS_MASK: u32 = !OFFSET_MASK;
        let e = BigEndian::read_u32(&resource_entry[4..]);

        let offset = (e & OFFSET_MASK) as usize;
        let flags = ((e & FLAGS_MASK) >> OFFSET_BITS) as u8;

        let size = BigEndian::read_u32(&data_section[offset..]) as usize;
        let offset = offset + size_of::<u32>();
        (data_section[offset..offset + size].to_vec(), flags)
    };

    let name = if name_offset == NO_NAME {
        String::new()
    } else {
        let size = name_list[name_offset] as usize;
        let offset = name_offset + size_of::<u8>();
        String::from_utf8_lossy(&name_list[offset..offset + size]).into_owned()
    };

    Resource {
        name,
        data,
        flags,
    }
}

fn build_resource_list(data: &[u8], num_types: usize, type_list: &[u8], name_list: &[u8]) -> Resources {
    const TYPE_ENTRY_SIZE: usize = 8;
    const RESOURCE_ENTRY_SIZE: usize = 12;

    let mut resources = HashMap::with_capacity(num_types.into());

    let mut type_entry = type_list;
    for _ in 0..num_types {
        let os_type: OSType = BigEndian::read_u32(&type_entry);
        let num_resources = BigEndian::read_u16(&type_entry[4..]) as usize + 1;

        let mut resources_of_type = ResourcesOfType::with_capacity(num_resources);

        let mut resource_entry = {
            let offset = BigEndian::read_u16(&type_entry[6..]) as usize - 2;
            &type_list[offset..]
        };

        for _ in 0..num_resources {
            let id = BigEndian::read_u16(&resource_entry);
            resources_of_type.insert(id, build_resource(data, resource_entry, name_list));
            resource_entry = &resource_entry[RESOURCE_ENTRY_SIZE..];
        }

        resources.insert(os_type, resources_of_type);

        type_entry = &type_entry[TYPE_ENTRY_SIZE..];
    }

    resources
}

impl M68k {
    pub fn new(data: Vec<u8>) -> std::io::Result<M68k> {
        const ROM_HEADER_SIZE: usize = 16;

        let mut reader = Cursor::new(&data);
        let data_offset = reader.read_u32::<BigEndian>()? as usize;
        let map_offset = reader.read_u32::<BigEndian>()? as usize;
        let data_end = data_offset + reader.read_u32::<BigEndian>()? as usize;
        let map_end = map_offset + reader.read_u32::<BigEndian>()? as usize;

        reader.seek(SeekFrom::Start(map_offset as u64 + 24))?;
        let type_list_offset = reader.read_u16::<BigEndian>()? as usize + 2;
        let name_list_offset = reader.read_u16::<BigEndian>()? as usize;
        let num_types = reader.read_u16::<BigEndian>()? as usize;

        let resources = {
            let data_section = &data[data_offset..data_end];
            let type_list = &data[map_offset + type_list_offset..map_end];
            let name_list = &data[map_offset + name_list_offset..map_end];
            build_resource_list(&data_section, num_types, type_list, name_list)
        };

        let extra_data = if data_offset != ROM_HEADER_SIZE {
            data[ROM_HEADER_SIZE..data_offset].to_vec()
        } else {
            Vec::new()
        };

        Ok(M68k {
            extra_data,
            resources,
        })
    }
}

#[cfg(test)]
mod tests {
    // TODO
}
