use binrw::{BinReaderExt, io};
use crate::types::MacString;
use libcommon::{io::prelude::*, prelude::*};

#[derive(Debug)]
pub struct AppleDouble<T: Read + Seek> {
    name: Option<MacString>,
    // For AppleSingle these both point to the same thing,
    // For AppleDouble the data fork points to the data file which contains
    // pure data and the resource fork points to the hidden AppleDouble file
    data_fork: Option<SharedStream<T>>,
    resource_fork: Option<SharedStream<T>>,
}

#[derive(Debug, thiserror::Error)]
#[non_exhaustive]
pub enum Error {
    #[error("unknown i/o error: {0}")]
    Io(#[from] io::Error),
    #[error("i/o error reading magic: {0}")]
    MagicReadIo(binrw::Error),
    #[error("bad magic 0x{0:x}")]
    BadMagic(u32),
    #[error("i/o error reading version: {0}")]
    VersionReadIo(binrw::Error),
    #[error("unknown version 0x{0:x}")]
    UnknownVersion(u32),
    #[error("i/o error seeking past home file system name: {0}")]
    HomeFileSystemSeekIo(io::Error),
    #[error("i/o error reading number of entries: {0}")]
    EntryCountReadIo(binrw::Error),
    #[error("no resource entries")]
    NoResourceEntries,
    #[error("invalid ID 0 for entry {0}")]
    InvalidEntryId(u16),
    #[error("i/o error seeking to filename script code: {0}")]
    FileNameScriptCodeSeekIo(io::Error),
    #[error("i/o error reading filename script code: {0}")]
    FileNameScriptCodeReadIo(binrw::Error),
    #[error("i/o error reading ID of entry {0}: {1}")]
    EntryIdReadIo(u16, binrw::Error),
    #[error("i/o error reading offset of entry {0}: {1}")]
    EntryOffsetReadIo(u16, binrw::Error),
    #[error("i/o error reading length of entry {0}: {1}")]
    EntryLengthReadIo(u16, binrw::Error),
    #[error("missing resource fork")]
    MissingResourceFork,
    #[error("i/o error reading filename: {0}")]
    NameReadIo(io::Error),
}

impl<T: Read + Seek> AppleDouble<T> {
    pub fn new(data: T, double_data: Option<T>) -> Result<Self, Error> {
        const DOUBLE_MAGIC: u32 = 0x51607;
        const SINGLE_MAGIC: u32 = 0x51600;

        let found_double = double_data.is_some();
        let data = SharedStream::new(double_data.unwrap_or(data));
        let mut input = data.clone();

        let magic = input.read_be::<u32>().map_err(Error::MagicReadIo)?;
        if magic != DOUBLE_MAGIC && magic != SINGLE_MAGIC {
            return Err(Error::BadMagic(magic));
        }

        let version = input.read_be::<u32>().map_err(Error::VersionReadIo)?;
        if version != 0x10000 && version != 0x20000 {
            return Err(Error::UnknownVersion(version));
        }

        // In V1 this is an ASCII string, in V2 it is zero-filled, in all cases
        // we do not care about it
        input.skip(16).map_err(Error::HomeFileSystemSeekIo)?;

        let num_entries = input.read_be::<u16>().map_err(Error::EntryCountReadIo)?;

        if num_entries == 0 {
            return Err(Error::NoResourceEntries);
        }

        let mut data_fork = None;
        let mut resource_fork = None;
        let mut name_input = None;
        let mut name_script_code = None;

        for index in 0..num_entries {
            let entry_id = input.read_be::<u32>()
                .map_err(|error| Error::EntryIdReadIo(index, error))?;
            let offset = u64::from(input.read_be::<u32>()
                .map_err(|error| Error::EntryOffsetReadIo(index, error))?);
            let length = u64::from(input.read_be::<u32>()
                .map_err(|error| Error::EntryLengthReadIo(index, error))?);

            match entry_id {
                0 => return Err(Error::InvalidEntryId(index)),
                1 => {
                    data_fork = Some(input.substream(offset, offset + length));
                },
                2 => {
                    resource_fork = Some(input.substream(offset, offset + length));
                },
                3 => {
                    name_input = Some(input.substream(offset, offset + length));
                },
                #[cfg(feature = "intl")]
                9 => {
                    let mut finder_info = input.substream(offset, offset + length);
                    finder_info.skip(26).map_err(Error::FileNameScriptCodeSeekIo)?;
                    name_script_code = Some(finder_info.read_ne::<u8>().map_err(Error::FileNameScriptCodeReadIo)?);
                },
                _ => {},
            };
        }

        if resource_fork.is_none() {
            return Err(Error::MissingResourceFork);
        }

        if magic == DOUBLE_MAGIC && data_fork.is_none() && found_double {
            data_fork = Some(data);
        }

        let name = if let Some(mut name_input) = name_input {
            let mut name = Vec::with_capacity(name_input.len()?.unwrap_into());
            name_input.read_to_end(&mut name).map_err(Error::NameReadIo)?;
            let mut name = MacString::Raw(name.into());

            #[cfg(feature = "intl")]
            if let Some(script_code) = name_script_code {
                name.decode(script_code).ok();
            }

            Some(name)
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
    pub fn name(&self) -> Option<&MacString> {
        self.name.as_ref()
    }

    #[must_use]
    pub fn resource_fork(&self) -> Option<&SharedStream<T>> {
        self.resource_fork.as_ref()
    }
}
