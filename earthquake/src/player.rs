use anyhow::Result as AResult;
use libcommon::vfs::{VirtualFile, VirtualFileSystem};
use libearthquake::{collections::riff_container::RiffContainer, detection::{FileType, projector::Movie as ProjectorMovie}, detection::movie::Kind, player::movie::Movie, player::score::Score};
use libmactoolbox::{System, EventKind, EventData, script_manager::ScriptCode, rsid};
use std::{borrow::Borrow, rc::Rc};
use qt_core::QBox;
use qt_widgets::QWidget;

enum MovieList {
    RiffContainer(RiffContainer<Box<dyn VirtualFile>>),
    Files(Vec<String>),
    Embeds(i16),
}

impl MovieList {
    fn len(&self) -> usize {
        match self {
            MovieList::RiffContainer(container) => container.len(),
            MovieList::Files(files) => files.len(),
            &MovieList::Embeds(count) => count as usize,
        }
    }
}

pub struct Player<'vfs> {
    system: System<'vfs>,
    riff_container: Option<RiffContainer<Box<dyn VirtualFile + 'vfs>>>,
    movies: MovieList,
    current_index: usize,
    paused: bool,

    root_movie: Movie,
    root_score: Score,

    windows: Vec<QBox<QWidget>>,
    vfs: Rc<dyn VirtualFileSystem + 'vfs>,
}

impl <'vfs> Player<'vfs> {
    pub fn new(vfs: Rc<dyn VirtualFileSystem + 'vfs>, charset: Option<ScriptCode>, path: impl AsRef<str>, info: FileType) -> AResult<Self> {
        let (script_code, system_resources, movies) = match info {
            FileType::Projector(p) => (
                p.charset().unwrap_or_else(|| charset.unwrap_or(ScriptCode::Roman)),
                p.system_resources().cloned(),
                match p.movie() {
                    ProjectorMovie::Embedded(_) |
                    ProjectorMovie::D3Win(_) |
                    ProjectorMovie::Internal(_) |
                    ProjectorMovie::External(_) => todo!(),
                }
            ),
            FileType::Movie(_) => (
                charset.unwrap_or(ScriptCode::Roman),
                None,
                MovieList::Files(vec![path.as_ref().to_string()]),
            ),
        };

        Ok(Self {
            system: System::new(vfs.clone(), script_code, system_resources),
            riff_container: None,
            movies,
            current_index: 0,
            paused: false,
            root_movie: Movie,
            root_score: Score::default(),
            windows: Vec::new(),
            vfs,
        })
    }

    pub fn exec(&mut self) {
        self.system.resource_manager().get_resource::<Vec<u8>>(rsid!(b"VWCF", 1024), &());
    }

    pub fn post_event(&mut self, kind: EventKind, data: EventData) -> AResult<()> {
        self.system.event_manager_mut().post_event(kind, data)
    }
}
