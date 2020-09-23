use anyhow::{Context, Result as AResult};
use cpp_core::{NullPtr, Ptr};
use fluent_ergonomics::FluentErgo;
use libcommon::vfs::VirtualFileSystem;
use libearthquake::detection::FileType;
use libmactoolbox::{EventKind, EventData, Point, script_manager::ScriptCode};
use std::{path::PathBuf, rc::{Weak, Rc}};
use qt_core::{q_event::Type as QEventType, QBox, QCoreApplication, QEvent, QObject};
use qt_core_custom_events::custom_event_filter::CustomEventFilter;
use qt_gui::QKeyEvent;
use qt_widgets::{QApplication, QMessageBox};
use crate::{player::Player, qtr};

/*
for each file in file list:
    set up the correct environment for input file (resource manager always, riff
    container sometimes, global file list), then:
        - show the "made with mm" logo if the projector configuration says so
        - load the next movie from the RIFF container, then run the movie loop:
            this loop runs forever until the player sends a quit signal
            it prevents the player from advancing if the window is backgrounded
            otherwise it just tells the player to run its loop
        - check to see if the user quit or if playback just ended, and if
        playback just ended and there is another movie in the RIFF container,
        load and play it
    - finally, show the "made with mm" logo if the projector configuration says so
*/

// Run each file and then jump to the next one; each one should be
// treated fully independently.
pub(crate) struct Engine<'vfs> {
    app: Ptr<QApplication>,
    localizer: FluentErgo,

    charset: Option<ScriptCode>,
    event_filter: QBox<CustomEventFilter>,

    data_dir: Option<PathBuf>,

    files: <Vec<(String, FileType)> as IntoIterator>::IntoIter,

    init_event_kind: QEventType,

    player: Option<Player<'vfs>>,

    // IT SHOULD NOT BE NECESSARY TO USE AN Rc HERE.
    vfs: Rc<dyn VirtualFileSystem + 'vfs>,
}

impl <'vfs> Engine<'vfs> {
    pub fn new(
        localizer: FluentErgo,
        vfs: Rc<dyn VirtualFileSystem + 'vfs>,
        app: Ptr<QApplication>,
        charset: Option<ScriptCode>,
        data_dir: Option<PathBuf>,
        files: Vec<(String, FileType)>
    ) -> Self {
        let mut engine = Self {
            app,
            charset,
            data_dir,
            event_filter: unsafe { QBox::null() },
            files: files.into_iter(),
            player: None,
            localizer,
            init_event_kind: unsafe { QEventType::from(QEvent::register_event_type_0a()) },
            vfs,
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
                self.load_next().unwrap_or_else(|ref e| self.show_error(e));
                true
            },

            QEventType::ZeroTimerEvent => {
                println!("ZTE");
                false
            },

            QEventType::Quit => {
                println!("Quit?");
                false
            },

            QEventType::Timer => {
                true
            },

            QEventType::WindowActivate | QEventType::WindowDeactivate => {
                if let Some(player) = self.player.as_mut() {
                    player.post_event(
                        EventKind::Activate,
                        EventData::ActiveWindow(Point::default(), Weak::new(), e == QEventType::WindowActivate),
                    ).unwrap_or_else(|ref e| self.show_error(e));
                }
                true
            },

            QEventType::MouseButtonPress => {
                if let Some(player) = self.player.as_mut() {
                    player.post_event(
                        EventKind::MouseDown,
                        EventData::Null,
                    ).unwrap_or_else(|ref e| self.show_error(e));
                }
                true
            },

            QEventType::MouseButtonRelease => {
                if let Some(player) = self.player.as_mut() {
                    player.post_event(
                        EventKind::MouseUp,
                        EventData::Null,
                    ).unwrap_or_else(|ref e| self.show_error(e));
                }
                true
            },

            QEventType::MouseMove => {
                // TODO: As OS event
                true
            },

            QEventType::KeyPress | QEventType::KeyRelease => {
                if let Some(player) = self.player.as_mut() {
                    let event = &*(event as *mut QEvent as *mut QKeyEvent);
                    let char = event.text().to_std_string().chars().next().unwrap_or('\0');
                    let key = event.key();
                    player.post_event(if e == QEventType::KeyRelease {
                        EventKind::KeyUp
                    } else if event.is_auto_repeat() {
                        EventKind::AutoKey
                    } else {
                        EventKind::KeyDown
                    }, EventData::Key(Point::default(), char, key))
                    .unwrap_or_else(|ref e| self.show_error(e));
                }
                true
            },

            QEventType::Paint => {
                if let Some(player) = self.player.as_mut() {
                    player.post_event(
                        EventKind::Update,
                        EventData::Window(Point::default(), Weak::new())
                    ).unwrap_or_else(|ref e| self.show_error(e));
                }
                true
            },

            _ => {
                false
            },
        }
    }

    pub fn exec(&mut self) -> i32 {
        unsafe {
            self.app.install_event_filter(&self.event_filter);
            QCoreApplication::post_event_2a(self.app, QEvent::new(self.init_event_kind).into_ptr());
            QApplication::exec()
        }
    }

    fn load_next(&mut self) -> AResult<()> {
        if let Some((file_name, file_info)) = self.files.next() {
            self.player = Some(Player::new(self.vfs.clone(), self.charset, &file_name, file_info).with_context(|| {
                format!("Can’t create player for {}", file_name)
            })?);
        } else {
            println!("Thank you for playing Wing Commander!");
            unsafe { QCoreApplication::quit() }
        }

        Ok(())
    }

    fn show_error(&self, error: &anyhow::Error) {
        unsafe {
            // TODO: i18n
            QMessageBox::warning_q_widget2_q_string(NullPtr, qtr!(self.localizer, "Oops!"), qtr!(self.localizer, format!("{:#}", error).as_ref()));
        }
    }
}
