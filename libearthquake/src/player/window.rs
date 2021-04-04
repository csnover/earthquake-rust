use libcommon::{Unk32, Unk8};
use libmactoolbox::{events::EventRecord, quickdraw::{Rect, Region}, types::MacString, windows::{CWindowRecord, Kind}};
use std::rc::Rc;
use super::{movie::Movie, score::Score};

#[derive(Debug, Default)]
struct LingoXObjectArg;
#[derive(Debug, Default)]
struct LingoWindow;

type WindowProc = fn(i32, &[LingoXObjectArg], i16, &Window);
type EventHandlerProc = fn(&Window, &EventRecord, &Region);

pub struct Window {
    proc: &'static WindowProc,
    event_handler: &'static EventHandlerProc,
    // A pointer to the window record is normally here, but it’s not needed
    // since the window record is also owned by this object and storing the
    // pointer seems to have just been an optimisation.
    field_c: Unk32,
    /// The OS window record for this window.
    window_record: Option<CWindowRecord>,
    /// The Lingo window object for this window.
    lingo_window: LingoWindow,
    movie: Option<Rc<Movie>>,
    score: Option<Rc<Score>>,
    file_name: MacString,
    window_title: MacString,
    // movie ref count is normally here, but that’s not necessary
    clip_rect_maybemaybe: Rect,
    source_rect: Rect,
    draw_rect: Rect,
    /// The window definition ID.
    ///
    /// While Director documentation says that this can be any value, in
    /// practice it can only be 0–4, 8, 12, 16, or 49. 49 is a Director-specific
    /// value for a floating palette. All other values are internally converted
    /// to use the plain dialogue box ID.
    window_kind: Kind,
    title_visible: bool,
    field_47: Unk8,
    owns_score_maybe: bool,
    field_49: Unk8,
    is_modal: bool,
    // padding byte
}

// Cannot derive due to the function pointers:
// https://github.com/rust-lang/rust/issues/70263
impl core::fmt::Debug for Window {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Window")
            .field("proc", &(*self.proc as *const ()))
            .field("event_handler", &(*self.event_handler as *const ()))
            .field("field_c", &self.field_c)
            .field("window_record", &self.window_record)
            .field("lingo_window", &self.lingo_window)
            .field("movie", &self.movie)
            .field("score", &self.score)
            .field("file_name", &self.file_name)
            .field("window_title", &self.window_title)
            .field("clip_rect_maybemaybe", &self.clip_rect_maybemaybe)
            .field("source_rect", &self.source_rect)
            .field("draw_rect", &self.draw_rect)
            .field("window_kind", &self.window_kind)
            .field("title_visible", &self.title_visible)
            .field("field_47", &self.field_47)
            .field("owns_score_maybe", &self.owns_score_maybe)
            .field("field_49", &self.field_49)
            .field("is_modal", &self.is_modal)
            .finish()
    }
}
