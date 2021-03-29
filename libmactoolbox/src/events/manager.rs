use core::convert::TryInto;
use crate::quickdraw::Point;
use libcommon::bitflags::BitFlags;
use qt_core::{KeyboardModifier, MouseButton};
use qt_gui::{QCursor, QGuiApplication};
use smart_default::SmartDefault;
use std::{collections::VecDeque, time::{Duration, Instant}};
use super::{Error, event::{Data as EventData, Kind as EventKind, Modifiers as EventModifiers, Record as EventRecord, Tick}};

#[derive(Debug, SmartDefault)]
pub struct Manager {
    #[default(Instant::now())]
    start: Instant,
    #[default(Instant::now())]
    instance_start: Instant,
    queue: VecDeque<EventRecord>,
}

impl Manager {
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// `Button`
    #[must_use]
    pub fn button(&self) -> bool {
        self.queue.iter().any(|event| event.kind() == EventKind::MouseDown)
    }

    /// `FlushEvents`
    pub fn flush(&mut self) {
        self.queue.clear();
    }

    /// `GetKeys`
    pub fn keys(&self, _map: &[u8]) {
        todo!("keyboard state")
    }

    /// `GetMouse`
    #[must_use]
    pub fn mouse(&self) -> Point {
        // TODO: Supposed to be mouse position within grafport.
        todo!("mouse position")
    }

    /// `PostEvent`
    pub fn post_event(&mut self, kind: EventKind, data: EventData) -> Result<(), Error> {
        let event = match kind {
            EventKind::Null
            | EventKind::MouseDown
            | EventKind::MouseUp => match data {
                EventData::Null | EventData::Mouse(..) => Some(EventRecord {
                    kind,
                    when: Tick::now(),
                    modifiers: self.modifiers(),
                    data: EventData::Mouse(self.mouse_pos()),
                }),
                _ => None
            },
            EventKind::KeyDown
            | EventKind::KeyUp
            | EventKind::AutoKey => if let EventData::Key(_, c, s) = data {
                Some(EventRecord {
                    kind,
                    when: Tick::now(),
                    modifiers: self.modifiers(),
                    data: EventData::Key(self.mouse_pos(), c, s),
                })
            } else {
                None
            },
            EventKind::Update => if let EventData::Window(_, w) = data {
                Some(EventRecord {
                    kind,
                    when: Tick::now(),
                    modifiers: self.modifiers(),
                    data: EventData::Window(self.mouse_pos(), w),
                })
            } else {
                None
            },
            EventKind::Disk => unimplemented!("disk events are not used"),
            EventKind::Activate => if let EventData::ActiveWindow(_, w, a) = data {
                Some(EventRecord {
                    kind,
                    when: Tick::now(),
                    modifiers: self.modifiers(),
                    data: EventData::ActiveWindow(self.mouse_pos(), w, a),
                })
            } else {
                None
            },
            EventKind::Os => todo!("OS events"),
            EventKind::HighLevel => if let EventData::HighLevel(..) = data {
                todo!("high level events")
            } else {
                None
            },
        };

        if let Some(event) = event {
            println!("{:?}", event);
            self.queue.push_back(event);
            Ok(())
        } else {
            Err(Error::BadEventKind)
        }
    }

    /// `StillDown`
    #[must_use]
    pub fn still_down(&self) -> bool {
        self.queue.iter().find(|event| event.kind() == EventKind::MouseUp).is_none()
    }

    /// `WaitMouseUp`
    pub fn wait_mouse_up(&mut self) -> bool {
        if let Some(index) = self.queue.iter().position(|event| event.kind() == EventKind::MouseUp) {
            self.queue.remove(index);
            true
        } else {
            false
        }
    }

    /// `TickCount`
    #[must_use]
    pub fn tick_count(&self) -> Tick {
        self.start + (Instant::now() - self.instance_start)
    }

    fn modifiers(&self) -> EventModifiers {
        let os_modifiers = unsafe { QGuiApplication::keyboard_modifiers() };

        // TODO: Probably need to meddle with this so control is command on
        // non-macOS hosts
        let mut modifiers = EventModifiers::empty();
        if os_modifiers.test_flag(KeyboardModifier::AltModifier) {
            modifiers.insert(EventModifiers::OPTION_KEY);
        }
        if os_modifiers.test_flag(KeyboardModifier::ControlModifier) {
            modifiers.insert(EventModifiers::CONTROL_KEY);
        }
        if os_modifiers.test_flag(KeyboardModifier::MetaModifier) {
            modifiers.insert(EventModifiers::COMMAND_KEY);
        }
        if os_modifiers.test_flag(KeyboardModifier::ShiftModifier) {
            modifiers.insert(EventModifiers::SHIFT_KEY);
        }

        // TODO: OS-specific handlers for right-sided modifiers & caps lock

        let buttons = unsafe { QGuiApplication::mouse_buttons() };
        if buttons.test_flag(MouseButton::LeftButton) {
            modifiers.insert(EventModifiers::BUTTON_STATE);
        }

        modifiers
    }

    fn mouse_pos(&self) -> Point {
        unsafe {
            let p = QCursor::pos_0a();
            Point { x: p.x().try_into().unwrap(), y: p.y().try_into().unwrap() }
        }
    }

    /// Returns the threshold time for considering two clicks to be a double
    /// clicks.
    ///
    /// `GetDblTime`
    ///
    /// # Panics
    ///
    /// Panics if the double click interval from the OS is invalid.
    #[must_use]
    pub fn get_double_time(&self) -> Duration {
        Duration::from_millis(unsafe {
            QGuiApplication::style_hints().mouse_double_click_interval()
        }.try_into().unwrap())
    }
}
