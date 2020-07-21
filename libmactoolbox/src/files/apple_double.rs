use anyhow::{bail, Context, Result as AResult};
use byteordered::ByteOrdered;
use crate::script_manager::decode_text;
use libcommon::{Reader, SharedStream};
use std::{ffi::OsString, fs::File, io, path::{Path, PathBuf}};

#[derive(Debug)]
pub struct AppleDouble<T: Reader> {
    name: Option<String>,
    /// For AppleSingle these both point to the same thing,
    /// For AppleDouble the data fork points to the data file which contains
    /// pure data and the resource fork points to the hidden AppleDouble file
    data_fork: Option<SharedStream<T>>,
    resource_fork: Option<SharedStream<T>>,
}

impl<T: Reader> AppleDouble<T> {
    pub fn new(data: T, double_data: Option<T>) -> AResult<Self> {
        const DOUBLE_MAGIC: u32 = 0x51607;
        const SINGLE_MAGIC: u32 = 0x51600;

        let found_double = double_data.is_some();
        let data = SharedStream::new(double_data.unwrap_or(data));
        let mut input = ByteOrdered::be(data.clone());

        let magic = input.read_u32().context("Could not read magic")?;
        if magic != DOUBLE_MAGIC && magic != SINGLE_MAGIC {
            bail!("Bad magic");
        }

        let version = input.read_u32().context("Could not read version number")?;
        if version != 0x10000 && version != 0x20000 {
            bail!("Unknown version {:x}", version);
        }

        // In V1 this is an ASCII string, in V2 it is zero-filled, in all cases
        // we do not care about it
        input.skip(16).context("Could not seek past home file system name")?;

        let num_entries = input.read_u16().context("Could not read number of entries")?;

        if num_entries == 0 {
            bail!("No resource entries");
        }

        let mut data_fork = None;
        let mut resource_fork = None;
        let mut name_input = None;
        let mut name_script_code = 0;

        for index in 0..num_entries {
            let entry_id = input.read_u32().with_context(|| format!("Could not read ID of entry {}", index))?;
            let offset = input.read_u32().with_context(|| format!("Could not read offset of entry {}", index))?;
            let length = input.read_u32().with_context(|| format!("Could not read length of entry {}", index))?;

            match entry_id {
                0 => bail!("Invalid ID 0 for entry {}", index),
                1 => {
                    data_fork = Some(input.inner_mut().substream(u64::from(offset), u64::from(offset + length)));
                },
                2 => {
                    resource_fork = Some(input.inner_mut().substream(u64::from(offset), u64::from(offset + length)));
                },
                3 => {
                    name_input = Some(input.inner_mut().substream(u64::from(offset), u64::from(offset + length)));
                },
                9 => {
                    let mut finder_info = ByteOrdered::be(input.inner_mut().substream(u64::from(offset), u64::from(offset + length)));
                    finder_info.skip(26).context("Could not seek to filename script code")?;
                    name_script_code = finder_info.read_u8().context("Could not read script code")?;
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

impl AppleDouble<File> {
    pub fn open(path: impl AsRef<Path>) -> AResult<Self> {
        Self::new(File::open(&path)?, open_apple_double(&path).ok())
    }
}

fn open_apple_double(path: impl AsRef<Path>) -> io::Result<File> {
    let mut path = PathBuf::from(path.as_ref());
    path.set_file_name({
        let mut file_name = OsString::from("._");
        file_name.push(path.file_name().unwrap());
        file_name
    });
    File::open(path)
}
