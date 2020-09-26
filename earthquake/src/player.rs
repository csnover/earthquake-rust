use anyhow::{Context, Result as AResult, bail};
use libcommon::vfs::{VirtualFile, VirtualFileSystem};
use libearthquake::{collections::riff::Riff, collections::riff_container::RiffContainer, detection::{Detection, FileType, Version, projector::{
            D3WinMovie,
            Movie as ProjectorMovie,
        }}, player::{movie::Movie, score::Score}};
use libmactoolbox::{EventData, EventKind, ResourceFile, System, rsid, script_manager::ScriptCode};
use std::{io::SeekFrom, rc::Rc};
use qt_core::{QBox, QEventLoop};
use qt_widgets::QWidget;

#[derive(Debug)]
enum MovieList<'vfs> {
    RiffContainer(RiffContainer<Box<dyn VirtualFile + 'vfs>>),
    SingleRiff(Riff<Box<dyn VirtualFile + 'vfs>>),
    D3Win(Vec<D3WinMovie>),
    D3Mac(Vec<String>),
    Embeds(ResourceFile<Box<dyn VirtualFile + 'vfs>>, u16),
}

impl <'vfs> MovieList<'vfs> {
    fn len(&self) -> usize {
        match self {
            MovieList::RiffContainer(c) => c.len(),
            MovieList::SingleRiff(_) => 1,
            MovieList::D3Win(l) => l.len(),
            MovieList::D3Mac(l) => l.len(),
            MovieList::Embeds(_, c) => *c as usize
        }
    }
}

pub struct Player<'vfs> {
    system: System<'vfs>,
    movies: MovieList<'vfs>,
    current_index: usize,
    paused: bool,

    root_movie: Movie,
    root_score: Score,

    windows: Vec<QBox<QWidget>>,
    vfs: Rc<dyn VirtualFileSystem + 'vfs>,
}

impl <'vfs> std::fmt::Debug for Player<'vfs> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Player")
            .field("movies", &self.movies)
            .field("current_index", &self.current_index)
            .field("paused", &self.paused)
            .finish()
    }
}

impl <'vfs> Player<'vfs> {
    pub fn new(vfs: Rc<dyn VirtualFileSystem + 'vfs>, charset: Option<ScriptCode>, path: impl AsRef<str>, mut d: Detection<'vfs>) -> AResult<Self> {
        let (script_code, system_resources, movies) = match d.info {
            FileType::Projector(p) => (
                p.charset().unwrap_or_else(|| charset.unwrap_or(ScriptCode::Roman)),
                p.system_resources().cloned(),
                match p.movie() {
                    ProjectorMovie::Embedded(count) => {
                        if let Some(resource_fork) = d.resource_fork.take() {
                            let resource_file = ResourceFile::new(resource_fork)
                                .context("Can’t create resource file for projector")?;
                            MovieList::Embeds(resource_file, *count)
                        } else {
                            bail!("Missing resource fork for projector");
                        }
                    },
                    ProjectorMovie::D3Win(movies) => MovieList::D3Win(movies.clone()),
                    ProjectorMovie::Internal(offset) => MovieList::RiffContainer({
                        if let Some(mut input) = d.data_fork.take() {
                            input.seek(SeekFrom::Start(u64::from(*offset))).context("Can’t seek to RIFF container")?;
                            RiffContainer::new(input).context("Can’t create RIFF container from data fork")?
                        } else {
                            bail!("Missing data fork for RIFF container");
                        }
                    }),
                    ProjectorMovie::External(files) => MovieList::D3Mac(files.clone()),
                },
            ),
            FileType::Movie(m) => (
                charset.unwrap_or(ScriptCode::Roman),
                None,
                if m.version() == Version::D3 {
                    if let Some(resource_fork) = d.resource_fork.take() {
                        let resource_file = ResourceFile::new(resource_fork)
                            .context("Can’t create resource file for movie")?;
                        MovieList::Embeds(resource_file, 1)
                    } else {
                        bail!("Missing resource fork for movie");
                    }
                } else {
                    MovieList::SingleRiff(
                        Riff::new(
                            d.data_fork.take().context("Missing data fork for movie")?
                        ).context("Can’t create RIFF from data fork")?
                    )
                }
            ),
        };

        Ok(Self {
            system: System::new(vfs.clone(), script_code, system_resources),
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
        todo!();
    }

    pub fn next(&mut self) -> bool {
        if self.current_index == self.movies.len() {
            return false;
        }

        todo!();

        self.current_index += 1;
    }

    pub fn post_event(&mut self, kind: EventKind, data: EventData) -> AResult<()> {
        self.system.event_manager_mut().post_event(kind, data)
    }
}
