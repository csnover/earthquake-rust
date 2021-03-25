use anyhow::{bail, Context, Result as AResult};
use binrw::BinReaderExt;
use crate::script_manager::decode_text;
use libcommon::{SeekExt, SharedStream};

#[derive(Debug)]
pub struct AppleDouble<T: binrw::io::Read + binrw::io::Seek> {
    name: Option<String>,
    /// For AppleSingle these both point to the same thing,
    /// For AppleDouble the data fork points to the data file which contains
    /// pure data and the resource fork points to the hidden AppleDouble file
    data_fork: Option<SharedStream<T>>,
    resource_fork: Option<SharedStream<T>>,
}

impl<T: binrw::io::Read + binrw::io::Seek> AppleDouble<T> {
    pub fn new(data: T, double_data: Option<T>) -> AResult<Self> {
        const DOUBLE_MAGIC: u32 = 0x51607;
        const SINGLE_MAGIC: u32 = 0x51600;

        let found_double = double_data.is_some();
        let data = SharedStream::new(double_data.unwrap_or(data));
        let mut input = data.clone();

        let magic = input.read_be::<u32>().context("Can’t read magic")?;
        if magic != DOUBLE_MAGIC && magic != SINGLE_MAGIC {
            bail!("Bad magic");
        }

        let version = input.read_be::<u32>().context("Can’t read version number")?;
        if version != 0x10000 && version != 0x20000 {
            bail!("Unknown version {:x}", version);
        }

        // In V1 this is an ASCII string, in V2 it is zero-filled, in all cases
        // we do not care about it
        input.skip(16).context("Can’t seek past home file system name")?;

        let num_entries = input.read_be::<u16>().context("Can’t read number of entries")?;

        if num_entries == 0 {
            bail!("No resource entries");
        }

        let mut data_fork = None;
        let mut resource_fork = None;
        let mut name_input = None;
        let mut name_script_code = 0;

        for index in 0..num_entries {
            let entry_id = input.read_be::<u32>().with_context(|| format!("Can’t read ID of entry {}", index))?;
            let offset = input.read_be::<u32>().with_context(|| format!("Can’t read offset of entry {}", index))?;
            let length = input.read_be::<u32>().with_context(|| format!("Can’t read length of entry {}", index))?;

            match entry_id {
                0 => bail!("Invalid ID 0 for entry {}", index),
                1 => {
                    data_fork = Some(input.substream(offset.into(), (offset + length).into()));
                },
                2 => {
                    resource_fork = Some(input.substream(offset.into(), (offset + length).into()));
                },
                3 => {
                    name_input = Some(input.substream(offset.into(), (offset + length).into()));
                },
                9 => {
                    let mut finder_info = input.substream(offset.into(), (offset + length).into());
                    finder_info.skip(26).context("Can’t seek to filename script code")?;
                    name_script_code = finder_info.read_ne::<u8>().context("Can’t read script code")?;
                },
                _ => {},
            };
        }

        if resource_fork.is_none() {
            bail!("Missing resource fork");
        }

        if magic == DOUBLE_MAGIC && data_fork.is_none() && found_double {
            data_fork = Some(data);
        }

        let name = if let Some(mut name_input) = name_input {
            Some(decode_text(&mut name_input, name_script_code))
        } else {
            None
        };

        Ok(Self {
            name,
            data_fork,
            resource_fork,
        })
    }

    #[must_use]
    pub fn data_fork(&self) -> Option<&SharedStream<T>> {
        self.data_fork.as_ref()
    }

    #[must_use]
    pub fn name(&self) -> Option<&String> {
        self.name.as_ref()
    }

    #[must_use]
    pub fn resource_fork(&self) -> Option<&SharedStream<T>> {
        self.resource_fork.as_ref()
    }
}
