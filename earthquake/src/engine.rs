use anyhow::{anyhow, Context, Result as AResult};
use cpp_core::{NullPtr, Ptr};
use libcommon::{Reader, SharedStream, Resource};
use libearthquake::detection::{projector::{Movie as MovieKind, Platform as ProjectorPlatform, Version as ProjectorVersion}, FileType, movie::Version as MovieVersion};
use libmactoolbox::{os, ResourceFile, ResourceManager, script_manager::ScriptCode, System, ResourceId};
use std::{fs::File, io::Cursor, path::PathBuf, rc::Rc};
use qt_core::{QBox, qs};
use qt_widgets::{QApplication, QMessageBox, QWidget};

#[derive(Eq, PartialEq)]
enum PlayResult {
    Terminate = 7,
}

enum MacResource {
    Embedded {
        owner_path: PathBuf,
        resource_num: i16,
    },
    External,
}

enum Riff<T: Reader> {
    Embedded3 {
        owner_path: PathBuf,
        owner_stream: SharedStream<T>,
        offset: u32,
        size: u32,
    },
    Embedded4 {
        owner_path: PathBuf,
        owner_stream: SharedStream<T>,
        owner_index: i32,
    },
    External,
}

enum MovieMeta<T: Reader> {
    MacResource(MacResource),
    Riff(Riff<T>),
}

struct Movie<T: Reader> {
    path: PathBuf,
    stream: Option<SharedStream<T>>,
    meta: MovieMeta<T>,
}

struct EngineResourceManager;
impl EngineResourceManager {
    pub fn load<R: Resource + 'static>(id: ResourceId) -> AResult<Rc<R>> {
        todo!()
    }

    pub fn set_current_file(index: i16) -> AResult<()> {
        todo!()
    }
}

pub(crate) struct Engine<'a> {
    app: Ptr<QApplication>,
    charset: Option<ScriptCode>,
    data_dir: Option<PathBuf>,
    resource_manager: ResourceManager<'a>,
    movies: Vec<Movie<SharedStream<File>>>,
    current_movie_index: usize,
    last_error: i16,
    windows: Vec<QBox<QWidget>>,
}

impl <'a> Engine<'a> {
    pub fn new(app: Ptr<QApplication>, charset: Option<ScriptCode>, data_dir: Option<PathBuf>, files: Vec<(String, FileType)>) -> Self {
        todo!()
        // Self {
        //     app,
        //     charset,
        //     data_dir,
        //     movies: files.map(|(path, kind)| {
        //         // if d3mac, macresource always
        //         // if d3win projector, riffs always
        //         // if d4 projector, system file plus riffs
        //         // if d4 movie, riffs
        //         Movie {
        //             path,
        //             stream: None,
        //             meta: match kind {
        //                 FileType::Projector(detection_info, stream) => {
        //                     match detection_info.version() => {
        //                         ProjectorVersion::D3 => {
        //                             match detection_info.config().platform() {
        //                                 ProjectorPlatform::Mac(_) => MacResource::Embedded {

        //                                 }
        //                             }
        //                         }
        //                     }
        //                 },
        //                 FileType::Movie(detection_info, stream) => {
        //                     match detection_info.version() {
        //                         MovieVersion::D3 => MacResource::External,
        //                         MovieVersion::D4 => Riff::External,
        //                     }
        //                 },
        //                 D3MacProjector => {
        //                     // read the STR#
        //                     // read the embedded flag
        //                     // for each, MacResource::External or MacResource::Embedded
        //                     MacResource::External
        //                 },
        //                 D3MacMovie => {
        //                     MacResource::External
        //                 },
        //                 D3WinProjector => {
        //                     // read the embedded flag
        //                     // for each, Riff::External or Riff::Embedded3
        //                 },
        //                 D4Projector => {
        //                     // read the RiffContainer
        //                     // for each, Riff::Embedded4
        //                     // also, pluck projectr.rsr
        //                 },
        //                 D4Movie => {
        //                     Riff::External
        //                 }
        //             }
        //         }
        //     }),
        //     resource_manager: ResourceManager::new(),
        //     current_movie_index: 0,
        //     last_error: 0,
        //     windows: Vec::new(),
        // }
    }

    fn init(&mut self) {
        todo!()
    }

    fn play(&mut self) -> AResult<PlayResult> {
        todo!()
        // let file = self.files.get(self.current_file_index).ok_or_else(|| anyhow!("Invalid current_file_index"))?;
        // match &file.1 {
        //     FileType::Projector(info, stream) => {
        //         if let Some(resource_data) = info.system_resources() {
        //             let res_file = ResourceFile::new(Box::new(Cursor::new(resource_data.clone())) as Box<dyn Reader>)
        //                 .context("Invalid system resources file")?;
        //             self.resource_manager.add_resource_file(res_file);
        //         }

        //         match info.movie() {
        //             MovieKind::Embedded(_id) => {
        //                 let res_file = ResourceFile::new(Box::new(stream.clone()) as Box<dyn Reader>)
        //                     .context("Invalid movie data")?;
        //                 self.resource_manager.add_resource_file(res_file);
        //             },
        //             _ => todo!(),
        //         }
        //     },

        //     FileType::Movie(info, stream) => todo!(),
        // }

        // Ok(PlayResult::Terminate)
    }

    fn show_alert(&self, alert_id: i16, stop_flag: bool) {

    }

    fn show_error(&self, error_id: i16, text_param: Option<&str>) {
        let alert_num = match self.last_error {
            -35 | -43 | -120 => 1040, // File Not Found
            10 => 1020, // Old Format
            18 => 1060, // Color Depth
            _ if self.last_error > -108 && self.last_error < -116 => 1070, // Low Memory
            _ => 1030, // File Problem
        };

        self.show_alert(alert_num, false);
    }

    pub fn exec(&mut self) -> i32 {
        self.init();
        loop {
            match self.play() {
                Ok(PlayResult::Terminate) => {
                    break;
                },
                Err(e) => {
                    // TODO: Window
                    unsafe { QMessageBox::critical_q_widget2_q_string(NullPtr, &qs("TODO"), &qs(e.to_string())); }
                },
            }
        }
        unsafe { QApplication::exec() }
    }
}
