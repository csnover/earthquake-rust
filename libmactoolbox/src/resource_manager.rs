use anyhow::{anyhow, bail, Context, Result as AResult};
use crate::{
    OSType,
    resource_file::{RefNum, ResourceSource},
    ResourceFile,
    ResourceId,
    resources::string_list::StringList,
};
use libcommon::{Resource, encodings::DecoderRef, Reader, resource::{StringContext, StringKind}, vfs::{VirtualFile, VirtualFileSystem}};
use std::{convert::TryFrom, io::Cursor, path::Path, rc::Rc};

pub struct ResourceManager<'vfs> {
    fs: Rc<dyn VirtualFileSystem + 'vfs>,
    current_file: usize,
    files: Vec<ResourceFile<Box<dyn VirtualFile + 'vfs>>>,
    system: Option<ResourceFile<Cursor<Vec<u8>>>>,
    decoder: StringContext,
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
            decoder: StringContext(StringKind::PascalStr, decoder),
        })
    }

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

    /// `CountResources`
    #[must_use]
    pub fn count_resources(&self, kind: OSType) -> i16 {
        self.system.as_ref().map_or(0, |file| file.count(kind))
        + self.files.iter().fold(0, |count, file| count + file.count(kind))
    }

    /// `GetString`
    pub fn get_string(&self, id: i16) -> Option<Rc<String>> {
        match id {
            -16096 => std::env::var_os("USER").or_else(|| std::env::var_os("USERNAME")).map(|s| Rc::new(s.to_string_lossy().to_string())),
            #[cfg(feature = "sys_info")]
            -16413 => unsafe { Some(Rc::new(qt_core::QSysInfo::machine_host_name().to_std_string())) },
            _ => self.get_resource::<String>(ResourceId::new(b"STR ", id), &self.decoder).unwrap_or(None),
        }
    }

    /// `GetIndString`
    pub fn get_indexed_string(&self, id: i16, index: i16) -> Option<String> {
        self.get_resource::<StringList>(ResourceId::new(b"STR#", id), &self.decoder.1)
            .unwrap_or(None)
            .map(|list| {
                list.get(usize::try_from(index).unwrap()).unwrap_or(&String::new()).to_owned()
            })
    }

    /// `GetNamedResource`
    pub fn get_named_resource<T: Resource + 'static>(&self, kind: OSType, name: impl AsRef<[u8]>, context: &T::Context) -> AResult<Option<Rc<T>>> {
        for file in self.files.iter().take(self.current_file).rev() {
            if let Some(id) = file.id_of_name(kind, &name) {
                return file.load::<T>(id, context).map(Some);
            }
        }

        if let Some(file) = &self.system {
            if let Some(id) = file.id_of_name(kind, name) {
                return file.load::<T>(id, context).map(Some);
            }
        }

        Ok(None)
    }

    /// `Get1NamedResource`
    pub fn get_one_named_resource<R: Resource + 'static>(&self, os_type: OSType, name: impl AsRef<[u8]>, context: &R::Context) -> AResult<Option<Rc<R>>> {
        self.one_resource::<R, _, _>(
            |file| file.id_of_name(os_type, name.as_ref()),
            |file| file.id_of_name(os_type, name.as_ref()),
            context
        )
    }

    /// `Get1Resource`
    pub fn get_one_resource<R: Resource + 'static>(&self, id: ResourceId, context: &R::Context) -> AResult<Option<Rc<R>>> {
        self.one_resource::<R, _, _>(
            |_| Some(id),
            |_| Some(id),
            context
        )
    }

    /// `Get1IndResource`
    pub fn get_one_indexed_resource<R: Resource + 'static>(&self, kind: OSType, index: i16, context: &R::Context) -> AResult<Option<Rc<R>>> {
        self.one_resource::<R, _, _>(
            |file| file.id_of_index(kind, index),
            |file| file.id_of_index(kind, index),
            context
        )
    }

    /// `GetResource`
    pub fn get_resource<R: Resource + 'static>(&self, id: ResourceId, context: &R::Context) -> AResult<Option<Rc<R>>> {
        for file in self.files.iter().take(self.current_file).rev() {
            if file.contains(id) {
                return file.load::<R>(id, context).map(Some);
            }
        }

        if let Some(file) = &self.system {
            if file.contains(id) {
                return file.load::<R>(id, context).map(Some);
            }
        }

        Ok(None)
    }

    /// `OpenResFile`
    pub fn open_resource_file(&'vfs mut self, path: impl AsRef<Path>) -> AResult<()> {
        let file = self.fs.open_resource_fork(&path)?;
        let res_file = ResourceFile::new(file)?;
        self.files.push(res_file);
        self.current_file = self.files.len();
        Ok(())
    }

    /// `UseResFile`
    pub fn use_resource_file(&mut self, ref_num: RefNum) -> AResult<()> {
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

        bail!("Invalid resource file number {}", ref_num)
    }

    fn one_resource<R, GetSysId, GetUserId>(&self, get_sys_id: GetSysId, get_user_id: GetUserId, context: &R::Context) -> AResult<Option<Rc<R>>>
    where
        R: Resource + 'static,
        GetSysId: Fn(&ResourceFile<Cursor<Vec<u8>>>) -> Option<ResourceId>,
        GetUserId: Fn(&ResourceFile<Box<dyn VirtualFile + 'vfs>>) -> Option<ResourceId>
    {
        if self.current_file == 0 {
            self.system
                .as_ref()
                .ok_or_else(|| anyhow!("no system file"))
                .and_then(|file| load_one(file, get_sys_id, context))
        } else {
            let file = self.files.get(self.current_file - 1).context("current_file invalid")?;
            load_one(file, get_user_id, context)
        }
    }
}

fn load_one<R: Resource + 'static, T: Reader>(file: &ResourceFile<T>, get_id: impl Fn(&ResourceFile<T>) -> Option<ResourceId>, context: &R::Context) -> AResult<Option<Rc<R>>> {
    Ok(if let Some(id) = get_id(file) {
        Some(file.load::<R>(id, context)?)
    } else {
        None
    })
}
