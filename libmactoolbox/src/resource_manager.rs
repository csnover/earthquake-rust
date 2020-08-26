use anyhow::{anyhow, bail, Context, Result as AResult};
use crate::{
    OSType,
    ResourceFile,
    ResourceId,
    resource_file::RefNum, rsid, resources::string_list::StringList,
};
use libcommon::{Resource, vfs::{VirtualFile, VirtualFileSystem}};
use std::{io::Cursor, path::Path, rc::Rc};

pub struct ResourceManager<'vfs> {
    fs: &'vfs dyn VirtualFileSystem,
    current_file: usize,
    files: Vec<ResourceFile<Box<dyn VirtualFile + 'vfs>>>,
    system: Option<ResourceFile<Cursor<Vec<u8>>>>,
}

impl <'vfs> ResourceManager<'vfs> {
    #[must_use]
    pub fn new(fs: &'vfs dyn VirtualFileSystem, system: Option<Vec<u8>>) -> Self {
        Self {
            fs,
            current_file: 0,
            files: Vec::new(),
            system: system.map(|data| ResourceFile::new(Cursor::new(data)).unwrap()),
        }
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
    pub fn count_one_resources(&self, kind: OSType) -> u16 {
        if self.current_file == 0 {
            self.system.as_ref().map_or(0, |file| file.count(kind))
        } else {
            let file = self.files.get(self.current_file - 1).expect("current_file invalid");
            file.count(kind)
        }
    }

    /// `CountResources`
    #[must_use]
    pub fn count_resources(&self, kind: OSType) -> u16 {
        self.system.as_ref().map_or(0, |file| file.count(kind))
        + self.files.iter().fold(0, |count, file| count + file.count(kind))
    }

    /// `GetString`
    pub fn get_string(&self, id: i16) -> Option<Rc<String>> {
        // TODO: User Information Resources
        self.get_resource::<String>(rsid!(b"STR ", id), &Default::default()).unwrap_or(None)
    }

    /// `GetIndString`
    pub fn get_indexed_string(&self, id: i16, index: i16) -> Option<String> {
        self.get_resource::<StringList>(rsid!(b"STR#", id), &Default::default())
            .unwrap_or(None)
            .map(|list| {
                list.get(index as usize).unwrap_or(&String::new()).to_owned()
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
    pub fn get_one_named_resource<T: Resource + 'static>(&self, kind: OSType, name: impl AsRef<[u8]>, context: &T::Context) -> AResult<Option<Rc<T>>> {
        if self.current_file == 0 {
            self.system
                .as_ref()
                .ok_or_else(|| anyhow!("no system file"))
                .and_then(|file| Ok({
                    if let Some(id) = file.id_of_name(kind, name) {
                        Some(file.load::<T>(id, context)?)
                    } else {
                        None
                    }
                }))
        } else {
            let file = self.files.get(self.current_file - 1).context("current_file invalid")?;
            Ok(if let Some(id) = file.id_of_name(kind, name) {
                Some(file.load::<T>(id, context)?)
            } else {
                None
            })
        }
    }

    /// `Get1Resource`
    pub fn get_one_resource<T: Resource + 'static>(&self, id: ResourceId, context: &T::Context) -> AResult<Option<Rc<T>>> {
        if self.current_file == 0 {
            self.system
                .as_ref()
                .ok_or_else(|| anyhow!("no system file"))
                .and_then(|file| Ok({
                    if file.contains(id) {
                        Some(file.load::<T>(id, context)?)
                    } else {
                        None
                    }
                }))
        } else {
            let file = self.files.get(self.current_file - 1).context("current_file invalid")?;
            Ok(if file.contains(id) {
                Some(file.load::<T>(id, context)?)
            } else {
                None
            })
        }
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
    pub fn open_resource_file(&mut self, path: impl AsRef<Path>) -> AResult<()> {
        self.fs.open_resource_fork(&path)
            .and_then(|file| {
                let res_file = ResourceFile::new(file)?;
                self.files.push(res_file);
                self.current_file = self.files.len();
                Ok(())
            })
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
}
