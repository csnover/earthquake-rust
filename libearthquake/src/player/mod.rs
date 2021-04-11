// TODO: You know, finish this file and then remove these overrides
#![allow(dead_code)]

pub mod movie;
pub mod score;
mod window;

use anyhow::Result as AResult;
use crate::{detection::{detect, FileType}, event::Manager as EventManager, sound::Manager as SoundManager};
use libcommon::vfs::VirtualFileSystem;
use libmactoolbox::{System, quickdraw::Rect};
use std::path::{Path, PathBuf};

pub struct Player<'system> {
    system: &'system System<'system>,
    // RE: `g_event`
    event: EventManager,
    // RE: `g_sound`
    sound: SoundManager,
    gray_rgn: Rect,
    file_type: Option<FileType>,
    path: Option<PathBuf>,
}

impl <'system> Player<'system> {
    // RE: `OVWD_InitWorld`
    pub fn new(system: &'system System<'system>) -> Self {
        // TODO: OVWDWorld_Init
        // TODO: Set original projector path

        // OD: A whole lot of irrelevant legacy stuff is elided here, like
        // making sure the Mac OS version is new enough, checking whether it has
        // support for Color QuickDraw, true colour graphics, etc. Also kept out
        // a whole lot of unnecessary work which was probably unnecessary in OD,
        // and initialisations which instead happen using the magic of modern
        // programming languages that have cool things called ‘constructors’.

        // TODO: For some reason this tries to set the score’s editable sprite
        // rect to the gray region. Why? Is this necessary?
        Self {
            system,
            event: EventManager::new(),
            sound: SoundManager::new(),
            // TODO: Technically this is supposed to be inset by 4px
            gray_rgn: system.window_manager().gray_region().bounding_box,
            file_type: <_>::default(),
            path: <_>::default(),
        }
    }

    pub fn open(mut self, fs: &impl VirtualFileSystem, path: impl AsRef<Path>) -> AResult<Self> {
        self.file_type = Some(detect(fs, &path)?.info);
        self.path = Some(path.as_ref().to_path_buf());
        Ok(self)
    }
}
