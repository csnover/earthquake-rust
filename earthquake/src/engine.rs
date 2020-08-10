use anyhow::{anyhow, Context, Result as AResult};
use cpp_core::{NullPtr, Ptr, StaticUpcast, CppBox};
use derive_more::Display;
use fluent_ergonomics::FluentErgo;
use libcommon::{Reader, SharedStream, Resource, vfs::{VirtualFileSystem, VirtualFile}, UnkPtr};
use libearthquake::{collections::riff_container::RiffContainer, detection::{projector::{Movie as MovieKind, Platform as ProjectorPlatform, Version as ProjectorVersion}, FileType, movie::Version as MovieVersion}};
use libmactoolbox::{os, ResourceFile, ResourceManager, script_manager::ScriptCode, System, ResourceId, vfs::HostFileSystem, EventManager, Point, EventModifiers, EventKind, EventData};
use std::{fs::File, io::Cursor, path::PathBuf, rc::{Weak, Rc}, cell::Cell};
use qt_core::{q_event::Type as QEventType, QBox, QCoreApplication, QEvent, QObject, qs, QTimer, TimerType, KeyboardModifier, MouseButton};
use qt_core_custom_events::custom_event_filter::CustomEventFilter;
use qt_gui::{QGuiApplication, QMouseEvent, QCursor, QKeyEvent};
use qt_widgets::{QApplication, QMessageBox, QWidget};
use crate::qtr;

#[derive(Display, Eq, PartialEq)]
enum PlayResult {
    Terminate = 7,
}

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

// Run each file and then jump to the next one; each one should be
// treated fully independently.
pub(crate) struct Engine<'a> {
    app: Ptr<QApplication>,
    charset: Option<ScriptCode>,
    event_manager: EventManager,
    event_filter: QBox<CustomEventFilter>,
    data_dir: Option<PathBuf>,
    resource_manager: Option<ResourceManager<'a>>,
    files: Vec<(String, FileType)>,
    localizer: FluentErgo,
    movies: Option<MovieList>,
    current_file_index: usize,
    current_movie_index: usize,
    init_event_kind: QEventType,
    last_error: i16,
    vfs: Box<dyn VirtualFileSystem>,
    windows: Vec<QBox<QWidget>>,
}

impl <'a> Engine<'a> {
    pub fn new(localizer: FluentErgo, app: Ptr<QApplication>, charset: Option<ScriptCode>, data_dir: Option<PathBuf>, files: Vec<(String, FileType)>) -> Self {
        let mut engine = Self {
            app,
            charset,
            data_dir,
            event_manager: EventManager::default(),
            event_filter: unsafe { QBox::null() },
            files,
            movies: None,
            localizer,
            resource_manager: None,
            current_file_index: 0,
            current_movie_index: 0,
            init_event_kind: unsafe { QEventType::from(QEvent::register_event_type_0a()) },
            last_error: 0,
            vfs: Box::new(HostFileSystem::new()),
            windows: Vec::new(),
        };

        engine.event_filter = CustomEventFilter::new(|object, event| unsafe {
            engine.handle_event(object, event)
        });

        engine
    }

    unsafe fn handle_event<'o, 'e>(&mut self, _target: &'o mut QObject, event: &'e mut QEvent) -> bool {
        let e = event.type_();
        match e {
            _ if e == self.init_event_kind => {
                // TODO
                println!("Init!");
                true
            },

            QEventType::WindowActivate | QEventType::WindowDeactivate => {
                self.event_manager.post_event(
                    EventKind::Activate,
                    EventData::ActiveWindow(Point::default(), Weak::new(), e == QEventType::WindowActivate),
                ).unwrap();
                true
            },

            QEventType::MouseButtonPress => {
                self.event_manager.post_event(
                    EventKind::MouseDown,
                    EventData::Null,
                ).unwrap();
                true
            },

            QEventType::MouseButtonRelease => {
                self.event_manager.post_event(
                    EventKind::MouseUp,
                    EventData::Null,
                ).unwrap();
                true
            },

            QEventType::MouseMove => {
                // TODO: As OS event
                true
            },

            QEventType::KeyPress | QEventType::KeyRelease => {
                let event = &*(event as *mut QEvent as *mut QKeyEvent);
                let char = event.text().to_std_string().chars().next().unwrap_or('\0');
                let key = event.key();
                self.event_manager.post_event(if e == QEventType::KeyRelease {
                    EventKind::KeyUp
                } else if event.is_auto_repeat() {
                    EventKind::AutoKey
                } else {
                    EventKind::KeyDown
                }, EventData::Key(Point::default(), char, key)).unwrap();
                true
            },

            QEventType::Paint => {
                self.event_manager.post_event(
                    EventKind::Update,
                    EventData::Window(Point::default(), Weak::new())
                ).unwrap();
                true
            },
            _ => {
                false
            },
        }
    }

    pub fn exec(&mut self) -> i32 {
        unsafe {
            let timer = QTimer::new_1a(self.app);
            timer.set_timer_type(TimerType::PreciseTimer);
            timer.start_1a(16);
            self.app.install_event_filter(&self.event_filter);
            QCoreApplication::post_event_2a(self.app, QEvent::new(self.init_event_kind).into_ptr());
            QApplication::exec()
        }
    }

    fn play(&mut self) -> AResult<PlayResult> {
        let load_next_file = match &self.movies {
            Some(movies) => self.current_movie_index == movies.len() - 1,
            None => false,
        };

        if load_next_file {
            if self.current_file_index == self.files.len() - 1 {
                return Ok(PlayResult::Terminate);
            } else {
                self.current_file_index += 1;
                self.current_movie_index = 0;
                let file = self.files.get(self.current_file_index).context("Invalid current_file_index")?;
                match &file.1 {
                    FileType::Projector(info) => {
                        // set up the correct resource manager
                        // info.version() == ProjectorVersion::D3 {

                        // }
                    },
                    FileType::Movie(info) => {
                        // set up the correct resource manager
                    },
                }
            }
        } else {
            self.current_movie_index += 1;
        }

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
}
