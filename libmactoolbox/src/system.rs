use anyhow::{Context, Result as AResult};
use crate::{EventManager, intl::ScriptCode};
use crate::resources::Manager as ResourceManager;
use libcommon::vfs::VirtualFileSystem;
use std::rc::Rc;

pub struct System<'vfs> {
    event_manager: EventManager,
    resource_manager: ResourceManager<'vfs>,
}

impl <'vfs> System<'vfs> {
    pub fn new(fs: Rc<dyn VirtualFileSystem + 'vfs>, script: ScriptCode, system: Option<Vec<u8>>) -> AResult<Self> {
        Ok(Self {
            event_manager: EventManager::new(),
            resource_manager: ResourceManager::new(fs, system)
                .context("Can’t create resource manager")?,
        })
    }

    pub fn event_manager(&self) -> &EventManager {
        &self.event_manager
    }

    pub fn event_manager_mut(&mut self) -> &mut EventManager {
        &mut self.event_manager
    }

    pub fn resource_manager(&self) -> &ResourceManager<'vfs> {
        &self.resource_manager
    }

    pub fn resource_manager_mut(&mut self) -> &mut ResourceManager<'vfs> {
        &mut self.resource_manager
    }
}
