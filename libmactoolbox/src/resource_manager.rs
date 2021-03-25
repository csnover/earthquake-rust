use anyhow::{bail, Context, Result as AResult};
use binrw::BinRead;
use crate::{OSType, ResourceError, ResourceFile, ResourceId, ResourceResult, resource_file::{RefNum, ResourceSource}, resources::string_list::StringList, types::{MacString, PString}};
use libcommon::{encodings::DecoderRef, Reader, vfs::{VirtualFile, VirtualFileSystem}};
use std::{convert::TryFrom, io::Cursor, path::Path, rc::Rc};

pub struct ResourceManager<'vfs> {
    fs: Rc<dyn VirtualFileSystem + 'vfs>,
    current_file: usize,
    files: Vec<ResourceFile<Box<dyn VirtualFile + 'vfs>>>,
    system: Option<ResourceFile<Cursor<Vec<u8>>>>,
    decoder: DecoderRef,
}

impl <'vfs> ResourceManager<'vfs> {
    pub fn new(fs: Rc<dyn VirtualFileSystem + 'vfs>, decoder: DecoderRef, system: Option<Vec<u8>>) -> AResult<Self> {
        Ok(Self {
            fs,
            current_file: 0,
            files: Vec::new(),
            system: if let Some(data) = system {
                Some(ResourceFile::new(Cursor::new(data))
                    .context("Can’t create system resource from memory")?)
            } else {
                None
            },
            decoder,
        })
    }

    /// Closes a resource fork.
    ///
    /// `CloseResFile`
    pub fn close_resource_file(&mut self, ref_num: RefNum) -> AResult<()> {
        for (index, file) in self.files.iter().enumerate() {
            if file.reference_number() == ref_num {
                self.files.remove(index);
                return Ok(());
            }
        }

        bail!("Invalid resource file index");
    }

    /// Gets the total number of resources of a given type in the [current]
    /// resource file.
    ///
    /// `Count1Resources`
    #[must_use]
    pub fn count_one_resources(&self, kind: OSType) -> i16 {
        if self.current_file == 0 {
            self.system.as_ref().map_or(0, |file| file.count(kind))
        } else {
            let file = self.files.get(self.current_file - 1).expect("current_file invalid");
            file.count(kind)
        }
    }

    /// Gets the total number of resources of a given type in all open resource
    /// files.
    ///
    /// `CountResources`
    #[must_use]
    pub fn count_resources(&self, kind: OSType) -> i16 {
        self.system.as_ref().map_or(0, |file| file.count(kind))
        + self.files.iter().fold(0, |count, file| count + file.count(kind))
    }

    /// Gets a string from a string (`'STR '`) resource.
    ///
    /// `GetString`
    #[allow(clippy::rc_buffer)]
    pub fn get_string(&self, id: i16) -> Option<MacString> {
        const USERNAME: i16 = -16096;
        const MACHINE_NAME: i16 = -16413;

        match id {
            USERNAME => {
                std::env::var_os("USER")
                    .or_else(|| std::env::var_os("USERNAME"))
                    .map(|username| MacString::Std(username.to_string_lossy().to_string()))
            },

            #[cfg(feature = "sys_info")]
            MACHINE_NAME => {
                Some(MacString::Std(unsafe { qt_core::QSysInfo::machine_host_name() }.to_std_string()))
            },

            _ => {
                self.get_resource::<PString>(ResourceId::new(b"STR ", id), ())
                    .unwrap_or(None)
                    .map(MacString::RawRc)
            },
        }
    }

    /// Gets a string from a string list (`'STR#'`) resource.
    ///
    ///  `GetIndString`
    pub fn get_indexed_string(&self, id: i16, index: i16) -> Option<PString> {
        self.get_resource::<StringList>(ResourceId::new(b"STR#", id), ())
            .unwrap_or(None)
            .and_then(|list| {
                list.get(usize::try_from(index).unwrap()).cloned()
            })
    }

    /// Gets the data for a named resource.
    ///
    ///  `GetNamedResource`
    pub fn get_named_resource<R: BinRead + 'static>(&self, kind: OSType, name: impl AsRef<[u8]>, args: R::Args) -> ResourceResult<Option<Rc<R>>> {
        for file in self.files.iter().take(self.current_file).rev() {
            if let Some(id) = file.id_of_name(kind, &name) {
                return file.load_args::<R>(id, args).map(Some);
            }
        }

        if let Some(file) = &self.system {
            if let Some(id) = file.id_of_name(kind, name) {
                return file.load_args::<R>(id, args).map(Some);
            }
        }

        Ok(None)
    }

    /// Gets the data for a named resource in the [current] resource file.
    ///
    /// `Get1NamedResource`
    pub fn get_one_named_resource<R: BinRead + 'static>(&self, os_type: OSType, name: impl AsRef<[u8]>, args: R::Args) -> ResourceResult<Option<Rc<R>>> {
        self.one_resource::<R, _, _>(
            |file| file.id_of_name(os_type, name.as_ref()),
            |file| file.id_of_name(os_type, name.as_ref()),
            args
        )
    }

    /// Gets the data for a resource in the [current] resource file.
    ///
    ///  `Get1Resource`
    pub fn get_one_resource<R: BinRead + 'static>(&self, id: ResourceId, args: R::Args) -> ResourceResult<Option<Rc<R>>> {
        self.one_resource::<R, _, _>(
            |_| Some(id),
            |_| Some(id),
            args
        )
    }

    /// Gets the data for a resource by its index in the resource map of the
    /// [current] resource file.
    ///
    /// `Get1IndResource`
    pub fn get_one_indexed_resource<R: BinRead + 'static>(&self, kind: OSType, index: i16, args: R::Args) -> ResourceResult<Option<Rc<R>>> {
        self.one_resource::<R, _, _>(
            |file| file.id_of_index(kind, index),
            |file| file.id_of_index(kind, index),
            args
        )
    }

    /// Gets the data for a resource.
    ///
    /// `GetResource`
    pub fn get_resource<R: BinRead + 'static>(&self, id: ResourceId, args: R::Args) -> ResourceResult<Option<Rc<R>>> {
        for file in self.files.iter().take(self.current_file).rev() {
            if file.contains(id) {
                return file.load_args::<R>(id, args).map(Some);
            }
        }

        if let Some(file) = &self.system {
            if file.contains(id) {
                return file.load_args::<R>(id, args).map(Some);
            }
        }

        Ok(None)
    }

    /// Opens a file’s resource fork.
    ///
    /// `OpenResFile`
    pub fn open_resource_file(&'vfs mut self, path: impl AsRef<Path>) -> ResourceResult<()> {
        let file = self.fs.open_resource_fork(&path).map_err(ResourceError::VFSError)?;
        let res_file = ResourceFile::new(file)?;
        self.files.push(res_file);
        self.current_file = self.files.len();
        Ok(())
    }

    /// Sets the [current] resource file.
    ///
    ///  `UseResFile`
    pub fn use_resource_file(&mut self, ref_num: RefNum) -> ResourceResult<()> {
        if ref_num == RefNum(0) {
            self.current_file = 0;
            return Ok(());
        }

        for (index, file) in self.files.iter().enumerate() {
            if file.reference_number() == ref_num {
                self.current_file = index;
                return Ok(());
            }
        }

        Err(ResourceError::BadRefNum(ref_num))
    }

    fn one_resource<R, GetSysId, GetUserId>(&self, get_sys_id: GetSysId, get_user_id: GetUserId, args: R::Args) -> ResourceResult<Option<Rc<R>>>
    where
        R: BinRead + 'static,
        GetSysId: Fn(&ResourceFile<Cursor<Vec<u8>>>) -> Option<ResourceId>,
        GetUserId: Fn(&ResourceFile<Box<dyn VirtualFile + 'vfs>>) -> Option<ResourceId>
    {
        if self.current_file == 0 {
            self.system
                .as_ref()
                .ok_or(ResourceError::NoSystemFile)
                .and_then(|file| load_one(file, get_sys_id, args))
        } else {
            let file = self.files.get(self.current_file - 1)
                .ok_or_else(|| ResourceError::BadCurrentFile(
                    self.current_file,
                    self.files.len()
                ))?;
            load_one(file, get_user_id, args)
        }
    }
}

fn load_one<R: BinRead + 'static, T: Reader>(file: &ResourceFile<T>, get_id: impl Fn(&ResourceFile<T>) -> Option<ResourceId>, args: R::Args) -> ResourceResult<Option<Rc<R>>> {
    Ok(if let Some(id) = get_id(file) {
        Some(file.load_args::<R>(id, args)?)
    } else {
        None
    })
}
