use crate::{quickdraw::Point, resources::OsType, types::Tick};
use libcommon::{UnkPtr, bitflags};
use std::rc::Weak;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Kind {
    Null      = 0,
    MouseDown = 1,
    MouseUp   = 2,
    KeyDown   = 3,
    KeyUp     = 4,
    AutoKey   = 5,
    Update    = 6,
    Disk      = 7,
    Activate  = 8,
    Os        = 15,
    HighLevel = 23,
}

bitflags! {
    pub struct Mask: u16 {
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
    pub struct Modifiers: u16 {
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
pub struct Record {
    pub(super) kind: Kind,
    pub(super) when: Tick,
    pub(super) modifiers: Modifiers,
    pub(super) data: Data,
}

impl Record {
    #[must_use]
    pub fn activate(&self) -> Option<bool> {
        match self.data {
            Data::ActiveWindow(_, _, a) => Some(a),
            _ => None,
        }
    }

    #[must_use]
    pub fn char_code(&self) -> Option<char> {
        match self.data {
            Data::Key(_, c, _) => Some(c),
            _ => None,
        }
    }

    #[must_use]
    pub fn high_level_kind(&self) -> Option<OsType> {
        match self.data {
            Data::HighLevel(o) => Some(o),
            _ => None,
        }
    }

    #[must_use]
    pub fn kind(&self) -> Kind {
        self.kind
    }

    #[must_use]
    pub fn modifiers(&self) -> Modifiers {
        self.modifiers
    }

    #[must_use]
    pub fn mouse(&self) -> Option<Point> {
        match self.data {
            Data::Null
            | Data::HighLevel(_) => None,
            Data::Mouse(p)
            | Data::Key(p, _, _)
            | Data::Window(p, _)
            | Data::ActiveWindow(p, _, _) => Some(p),
        }
    }

    #[must_use]
    pub fn scan_code(&self) -> Option<i32> {
        match self.data {
            Data::Key(_, _, s) => Some(s),
            _ => None,
        }
    }

    #[must_use]
    pub fn when(&self) -> Tick {
        self.when
    }

    #[must_use]
    pub fn window(&self) -> Option<Weak<UnkPtr>> {
        match &self.data {
            Data::Window(_, w)
            | Data::ActiveWindow(_, w, _) => Some(w.clone()),
            _ => None,
        }
    }
}

#[derive(Debug)]
pub enum Data {
    Null,
    Mouse(Point),
    Key(Point, char, i32),
    Window(Point, Weak<UnkPtr>),
    ActiveWindow(Point, Weak<UnkPtr>, bool),
    HighLevel(OsType),
}
