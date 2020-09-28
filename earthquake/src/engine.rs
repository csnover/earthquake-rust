use anyhow::{Context, Result as AResult};
use cpp_core::{NullPtr, Ptr};
use crate::{player::Player, qt::EQApplication, qt::EventReceiver, qtr, tr};
use fluent_ergonomics::FluentErgo;
use libcommon::{error::ReasonsExt, vfs::VirtualFileSystem};
use libearthquake::detection::Detection;
use libmactoolbox::{EventKind, EventData, Point, script_manager::ScriptCode};
use std::{path::PathBuf, rc::{Weak, Rc}};
use qt_core::{QCoreApplication, QEvent, q_event::Type as QEventType, qs};
use qt_gui::QKeyEvent;
use qt_widgets::{QApplication, QMessageBox, q_message_box::{
        Icon as MBIcon,
        StandardButton as MBButton,
    }};

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
    app: Ptr<EQApplication>,
    localizer: FluentErgo,

    charset: Option<ScriptCode>,

    data_dir: Option<PathBuf>,

    files: <Vec<(String, Detection<'vfs>)> as IntoIterator>::IntoIter,

    init_event_kind: QEventType,
    next_movie_event_kind: QEventType,

    player: Option<Player<'vfs>>,

    // IT SHOULD NOT BE NECESSARY TO USE AN Rc HERE.
    vfs: Rc<dyn VirtualFileSystem + 'vfs>,
}

impl <'vfs> Engine<'vfs> {
    pub fn new(
        localizer: FluentErgo,
        vfs: Rc<dyn VirtualFileSystem + 'vfs>,
        app: Ptr<EQApplication>,
        charset: Option<ScriptCode>,
        data_dir: Option<PathBuf>,
        files: Vec<(String, Detection<'vfs>)>
    ) -> Self {
        Self {
            app,
            charset,
            data_dir,
            files: files.into_iter(),
            player: None,
            localizer,
            init_event_kind: unsafe { QEvent::register_event_type_0a().into() },
            next_movie_event_kind: unsafe { QEvent::register_event_type_0a().into() },
            vfs,
        }
    }

    pub fn exec(&mut self) -> i32 {
        unsafe {
            self.app.set_event_receiver(self as &dyn EventReceiver);
            QCoreApplication::post_event_2a(self.app, QEvent::new(self.init_event_kind).into_ptr());
            QApplication::exec()
        }
    }

    fn load_next(&mut self) -> AResult<()> {
        if let Some((path, detection)) = self.files.next() {
            self.player = Some(Player::new(self.vfs.clone(), self.charset, detection, self.next_movie_event_kind).with_context(|| {
                tr!(self.localizer, "engine-load_next_error", [ "file_path" => path ])
            })?);
            self.player.as_mut().unwrap().exec()?;
        } else {
            println!("Thank you for playing Wing Commander!");
            unsafe { QCoreApplication::quit(); }
        }

        Ok(())
    }

    fn show_error(&self, error: &anyhow::Error) {
        unsafe {
            let message_box = QMessageBox::from_icon2_q_string_q_flags_standard_button_q_widget(
                MBIcon::Critical,
                qtr!(self.localizer, "engine-error"),
                &qs(error.to_string()),
                MBButton::Ok.into(),
                NullPtr,
            );

            message_box.set_detailed_text(&qs(error.reasons()));

            if let Some(url) = option_env!("CARGO_PKG_REPOSITORY") {
                message_box.set_informative_text(qtr!(
                    self.localizer,
                    "engine-failed_message-html",
                    [ "url" => format!("{}/issues", url) ]
                ));
            }

            message_box.exec();
        }
    }
}

impl <'vfs> EventReceiver for Engine<'vfs> {
    fn event(&mut self, event: &QEvent) -> bool {
        let e = unsafe { event.type_() };
        match e {
            _ if e == self.init_event_kind => {
                self.load_next().unwrap_or_else(|ref e| self.show_error(e));
                true
            },

            _ if e == self.next_movie_event_kind => {
                if let Some(player) = &mut self.player {
                    let loaded = player.next()
                        .unwrap_or_else(|ref e| { self.show_error(e); false });
                    if !loaded {
                        self.load_next().unwrap_or_else(|ref e| self.show_error(e));
                    }
                    true
                } else {
                    false
                }
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
                    let event = unsafe { &*(event as *const QEvent as *const QKeyEvent) };
                    let char = unsafe { event.text() }.to_std_string().chars().next().unwrap_or('\0');
                    let key = unsafe { event.key() };
                    player.post_event(if e == QEventType::KeyRelease {
                        EventKind::KeyUp
                    } else if unsafe { event.is_auto_repeat() } {
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
}
