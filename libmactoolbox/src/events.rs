use anyhow::{bail, Result as AResult};
use bitflags::bitflags;
use crate::{OSType, Point};
use libcommon::UnkPtr;
use std::{collections::VecDeque, rc::Weak, time::Duration};
use qt_core::{MouseButton, KeyboardModifier};
use qt_gui::{QCursor, QGuiApplication};

type Tick = std::time::Instant;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum EventKind {
    Null      = 0,
    MouseDown = 1,
    MouseUp   = 2,
    KeyDown   = 3,
    KeyUp     = 4,
    AutoKey   = 5,
    Update    = 6,
    Disk      = 7,
    Activate  = 8,
    OS        = 15,
    HighLevel = 23,
}

bitflags! {
    pub struct EventMask: u16 {
        const NULL       = 1 << 0;
        const MOUSE_DOWN = 1 << 1;
        const MOUSE_UP   = 1 << 2;
        const KEY_DOWN   = 1 << 3;
        const KEY_UP     = 1 << 4;
        const AUTO_KEY   = 1 << 5;
        const UPDATE     = 1 << 6;
        const DISK       = 1 << 7;
        const ACTIVATE   = 1 << 8;
        const HIGH_LEVEL = 1 << 10;
        const OS         = 1 << 15;
    }
}

bitflags! {
    pub struct EventModifiers: u16 {
        const ACTIVE            = 1 << 0;
        const BUTTON_STATE      = 1 << 7;
        const COMMAND_KEY       = 1 << 8;
        const SHIFT_KEY         = 1 << 9;
        const CAPS_LOCK         = 1 << 10;
        const OPTION_KEY        = 1 << 11;
        const CONTROL_KEY       = 1 << 12;
        const RIGHT_SHIFT_KEY   = 1 << 13;
        const RIGHT_OPTION_KEY  = 1 << 14;
        const RIGHT_CONTROL_KEY = 1 << 15;
    }
}

#[derive(Debug)]
pub struct EventRecord {
    kind: EventKind,
    when: Tick,
    modifiers: EventModifiers,
    data: EventData,
}

impl EventRecord {
    pub fn activate(&self) -> Option<bool> {
        match self.data {
            EventData::ActiveWindow(_, _, a) => Some(a),
            _ => None,
        }
    }

    pub fn char_code(&self) -> Option<char> {
        match self.data {
            EventData::Key(_, c, _) => Some(c),
            _ => None,
        }
    }

    pub fn high_level_kind(&self) -> Option<OSType> {
        match self.data {
            EventData::HighLevel(o) => Some(o),
            _ => None,
        }
    }

    pub fn kind(&self) -> EventKind {
        self.kind
    }

    pub fn modifiers(&self) -> EventModifiers {
        self.modifiers
    }

    pub fn mouse(&self) -> Option<Point> {
        match self.data {
            EventData::Null
            | EventData::HighLevel(_) => None,
            EventData::Mouse(p)
            | EventData::Key(p, _, _)
            | EventData::Window(p, _)
            | EventData::ActiveWindow(p, _, _) => Some(p),
        }
    }

    pub fn scan_code(&self) -> Option<i32> {
        match self.data {
            EventData::Key(_, _, s) => Some(s),
            _ => None,
        }
    }

    pub fn when(&self) -> Tick {
        self.when
    }

    pub fn window(&self) -> Option<Weak<UnkPtr>> {
        match &self.data {
            EventData::Window(_, w)
            | EventData::ActiveWindow(_, w, _) => Some(w.clone()),
            _ => None,
        }
    }
}

#[derive(Debug)]
pub enum EventData {
    Null,
    Mouse(Point),
    Key(Point, char, i32),
    Window(Point, Weak<UnkPtr>),
    ActiveWindow(Point, Weak<UnkPtr>, bool),
    HighLevel(OSType),
}

#[derive(Debug, Default)]
pub struct EventManager {
    queue: VecDeque<EventRecord>,
}

impl EventManager {
    pub fn new() -> Self {
        Self::default()
    }

    /// `Button`
    pub fn button(&self) -> bool {
        self.queue.iter().any(|event| event.kind() == EventKind::MouseDown)
    }

    /// `FlushEvents`
    pub fn flush(&mut self) {
        self.queue.clear();
    }

    /// `GetKeys`
    pub fn keys(&self, map: &[u8]) {
        todo!()
    }

    /// `GetMouse`
    pub fn mouse(&self) -> Point {
        // TODO: Supposed to be mouse position within grafport.
        todo!()
    }

    /// `PostEvent`
    pub fn post_event(&mut self, kind: EventKind, data: EventData) -> AResult<()> {
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
            EventKind::Disk => unimplemented!(),
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
            EventKind::OS => todo!(),
            EventKind::HighLevel => if let EventData::HighLevel(data) = data {
                todo!()
            } else {
                None
            },
        };

        if let Some(event) = event {
            println!("{:?}", event);
            self.queue.push_back(event);
            Ok(())
        } else {
            bail!("Invalid event type")
        }
    }

    /// `StillDown`
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
    pub fn tick_count(&self) -> Tick {
        todo!()
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
            Point { x: p.x() as i16, y: p.y() as i16 }
        }
    }

    pub fn get_double_time(&self) -> Duration {
        Duration::from_millis(unsafe {
            QGuiApplication::style_hints().mouse_double_click_interval()
        } as u64)
    }
}
