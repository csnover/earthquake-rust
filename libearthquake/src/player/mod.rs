// TODO: You know, finish this file and then remove these overrides
#![allow(dead_code)]

pub(super) mod movie;
pub(super) mod score;
mod window;

use anyhow::Result as AResult;
use crate::{collections::{riff::Riff, riff_container::RiffContainer}, detection::projector::D3WinMovie, event::Manager as EventManager, sound::Manager as SoundManager};
use libcommon::vfs::{VirtualFile, VirtualFileSystem};
use libmactoolbox::{System, events::{EventData, EventKind}, quickdraw::Rect, resources::File as ResourceFile, types::MacString};
use std::path::Path;

#[derive(Debug)]
enum MovieList<'vfs> {
    RiffContainer(RiffContainer<Box<dyn VirtualFile + 'vfs>>),
    SingleRiff(Riff<Box<dyn VirtualFile + 'vfs>>),
    D3Win(Vec<D3WinMovie>),
    D3Mac(Vec<MacString>),
    Embeds(ResourceFile<Box<dyn VirtualFile + 'vfs>>, u16),
}

impl <'vfs> MovieList<'vfs> {
    fn len(&self) -> usize {
        match self {
            MovieList::RiffContainer(c) => c.len(),
            MovieList::SingleRiff(_) => 1,
            MovieList::D3Win(l) => l.len(),
            MovieList::D3Mac(l) => l.len(),
            MovieList::Embeds(_, c) => usize::from(*c)
        }
    }
}

pub struct Player<'vfs> {
    system: &'vfs mut System<'vfs>,
    movies: Option<MovieList<'vfs>>,
    // RE: `g_event`
    event: EventManager,
    // RE: `g_sound`
    sound: SoundManager,
    gray_rgn: Rect,
}

impl <'vfs> Player<'vfs> {
    // RE: `OVWD_InitWorld`
    pub fn new(system: &'vfs mut System<'vfs>) -> Self {
        // TODO: OVWDWorld_Init
        // TODO: Set original projector path

        // TODO: Technically this is supposed to be inset by 4px
        let gray_rgn = system.window_manager().gray_region().bounding_box;

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
            movies: None,
            event: EventManager::new(),
            sound: SoundManager::new(),
            // TODO: Technically this is supposed to be inset by 4px
            gray_rgn,
        }
    }

    pub fn open(mut self, fs: &impl VirtualFileSystem, path: impl AsRef<Path>) -> AResult<Self> {
        // let (script_code, system_resources, movies) = match Some(detect(fs, &path)?.info() {
        //     FileType::Projector(p) => (
        //         p.charset().unwrap_or_else(|| charset.unwrap_or(ScriptCode::Roman)),
        //         p.system_resources().cloned(),
        //         match p.movie() {
        //             &ProjectorMovie::Embedded(count) => {
        //                 if let Some(resource_fork) = file.resource_fork.take() {
        //                     let resource_file = ResourceFile::new(resource_fork)
        //                         .context("Can’t create resource file for projector")?;
        //                     MovieList::Embeds(resource_file, count)
        //                 } else {
        //                     bail!("Missing resource fork for projector");
        //                 }
        //             },
        //             ProjectorMovie::D3Win(movies) => MovieList::D3Win(movies.clone()),
        //             &ProjectorMovie::Internal(offset) => MovieList::RiffContainer({
        //                 if let Some(mut input) = file.data_fork.take() {
        //                     input.seek(SeekFrom::Start(offset.into())).context("Can’t seek to RIFF container")?;
        //                     RiffContainer::new(input).context("Can’t create RIFF container from data fork")?
        //                 } else {
        //                     bail!("Missing data fork for RIFF container");
        //                 }
        //             }),
        //             ProjectorMovie::External(files) => MovieList::D3Mac(files.clone()),
        //         },
        //     ),
        //     FileType::Movie(m) => (
        //         charset.unwrap_or(ScriptCode::Roman),
        //         None,
        //         if m.version() == Version::D3 {
        //             if let Some(resource_fork) = file.resource_fork.take() {
        //                 let resource_file = ResourceFile::new(resource_fork)
        //                     .context("Can’t create resource file for movie")?;
        //                 MovieList::Embeds(resource_file, 1)
        //             } else {
        //                 bail!("Missing resource fork for movie");
        //             }
        //         } else {
        //             MovieList::SingleRiff(
        //                 Riff::new(
        //                     file.data_fork.take().context("Missing data fork for movie")?
        //                 ).context("Can’t create RIFF from data fork")?
        //             )
        //         }
        //     ),
        // };
        todo!()
    }

    pub fn post_event(&mut self, kind: EventKind, data: EventData) -> AResult<()> {
        self.system.event_manager_mut().post_event(kind, data).map_err(anyhow::Error::new)
    }
}
