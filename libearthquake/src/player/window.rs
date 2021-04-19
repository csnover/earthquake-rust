use libmactoolbox::{quickdraw::Rect, types::MacString, windows::{CWindowRecord, Kind}};
use std::rc::Rc;
use super::{movie::Movie, score::Score};

#[derive(Debug, Default)]
pub(super) struct LingoWindow;

#[derive(Debug)]
pub(super) struct Window {
    // Original Director allowed different window procedure and event handlers
    // but always used the same one.
    // A pointer to the window record is normally here, but it’s not needed
    // since the window record is also owned by this object and storing the
    // pointer seems to have just been an optimisation.
    // There was also an unreferenced 32-bit field.
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
    movie_loaded: bool,
    owns_score_maybe: bool,
    movie_started: bool,
    is_modal: bool,
    // padding byte
}

impl Window {
    // RE: Window_New
    pub(super) fn new(title: Option<MacString>, lingo_window: LingoWindow) -> Self {
        // OD: Hack here to return whatever window was in the global Movie
        // object if no title, but the only caller that does that has its own
        // guard, so it is not necessary.

        // OD: After construction, if there was no title, the code would set the
        // pointer to the window record to the global movie’s graphics port and
        // would set the draw rect to the default stage rect.
        Self {
            window_record: <_>::default(),
            lingo_window,
            movie: None,
            score: None,
            file_name: <_>::default(),
            window_title: title.unwrap_or_default(),
            clip_rect_maybemaybe: <_>::default(),
            source_rect: <_>::default(),
            draw_rect: <_>::default(),
            window_kind: Kind::new(-1),
            title_visible: true,
            movie_loaded: false,
            owns_score_maybe: false,
            movie_started: false,
            is_modal: false,
        }
    }
}
