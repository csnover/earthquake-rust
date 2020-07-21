use anyhow::{anyhow, bail, Result as AResult};
use crate::{
    files::{AppleDouble, MacBinary, open_resource_fork},
    OSType,
    ResourceFile,
    ResourceId
};
use libcommon::{Reader, Resource, SharedFile};
use std::{fs::File, path::Path, rc::Rc};

#[derive(Default)]
pub struct ResourceManager {
    current_file: usize,
    files: Vec<ResourceFile<Box<dyn Reader>>>,
}

impl ResourceManager {
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    pub fn add_resource<T: Resource>(&mut self, resource: T) -> AResult<()> {
        todo!();
    }

    pub fn add_resource_file(&mut self, file: ResourceFile<Box<dyn Reader>>) {
        self.files.push(file);
        self.current_file = self.files.len();
    }

    pub fn close_resource_file(&mut self, index: i16) -> AResult<()> {
        if index >= 0 && self.files.len() < (index as usize) {
            self.files.remove(index as usize);
            Ok(())
        } else {
            bail!("Invalid resource file index");
        }
    }

    /// `Count1Resources`
    #[must_use]
    pub fn count_one_resources(&self, kind: OSType) -> u16 {
        // Setting current file to 0 normally searches the System file, but
        // there is no System file
        if self.current_file == 0 {
            return 0;
        }

        let file = self.files.get(self.current_file - 1).expect("current_file invalid");
        file.count(kind)
    }

    /// `CountResources`
    #[must_use]
    pub fn count_resources(&self, kind: OSType) -> u16 {
        self.files.iter().fold(0, |count, file| count + file.count(kind))
    }

    pub fn get_string(&self, id: i16) -> Option<String> {
        todo!();
    }

    pub fn get_indexed_string(&self, id: i16, index: i16) -> Option<String> {
        todo!();
    }

    pub fn get_indexed_resource<T: Resource>(&self, index: i16) -> Option<T> {
        todo!();
    }

    pub fn get_named_resource<T: Resource>(&self, name: impl AsRef<str>) -> Option<T> {
        todo!();
    }

    pub fn get_one_indexed_resource<T: Resource>(&self, index: i16) -> Option<T> {
        todo!();
    }

    pub fn get_one_named_resource<T: Resource>(&self, name: impl AsRef<str>) -> Option<T> {
        todo!();
    }

    /// `Get1Resource`
    pub fn get_one_resource<T: Resource + 'static>(&self, id: ResourceId) -> AResult<Option<Rc<T>>> {
        // Setting current file to 0 normally searches the System file, but
        // there is no System file
        if self.current_file == 0 {
            return Ok(None)
        }

        let file = self.files.get(self.current_file - 1).expect("current_file invalid");
        Ok(if file.contains(id) {
            Some(file.load::<T>(id)?)
        } else {
            None
        })
    }

    /// `GetResource`
    pub fn get_resource<R: Resource + 'static>(&self, id: ResourceId) -> AResult<Option<Rc<R>>> {
        for file in self.files.iter().take(self.current_file).rev() {
            if file.contains(id) {
                return file.load::<R>(id).map(Some);
            }
        }

        Ok(None)
    }

    /// `OpenResFile`
    pub fn open_resource_file(&mut self, filename: impl AsRef<Path>) -> AResult<()> {
        // let file = open_resource_fork(&filename)
        //     .map(|file| SharedFile::new(file, &filename))
        //     .or_else(|_|
        //         AppleDouble::open(&filename)?
        //             .resource_fork()
        //             .ok_or_else(|| anyhow!("missing resource fork"))
        //             .map(|s| s.clone())
        //     )
        //     .or_else(|_|
        //         MacBinary::open(&filename)?
        //             .resource_fork()
        //             .ok_or_else(|| anyhow!("missing resource fork"))
        //             .map(|s| s.clone())
        //     )?;

        // self.files.push(ResourceFile::new(Box::new(file) as Box<dyn Reader>)?);
        // self.current_file = self.files.len();
        Ok(())
    }

    /// `UseResFile`
    pub fn use_resource_file(&mut self, index: i16) -> AResult<()> {
        if index >= 0 && (index as usize) <= self.files.len() {
            self.current_file = index as usize;
            Ok(())
        } else {
            bail!("Invalid resource file number {}", index)
        }
    }
}
