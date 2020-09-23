use std::rc::Rc;

use crate::{EventManager, script_manager::ScriptCode};
use crate::ResourceManager;
use libcommon::{vfs::VirtualFileSystem, encodings::MAC_ROMAN};

pub struct System<'vfs> {
    event_manager: EventManager,
    resource_manager: ResourceManager<'vfs>,
}

impl <'vfs> System<'vfs> {
    pub fn new(fs: Rc<dyn VirtualFileSystem + 'vfs>, script: ScriptCode, system: Option<Vec<u8>>) -> Self {
        let decoder = match script {
            ScriptCode::Roman => MAC_ROMAN,
            _ => unimplemented!(),
        };

        Self {
            event_manager: EventManager::new(),
            resource_manager: ResourceManager::new(fs, decoder, system),
        }
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
