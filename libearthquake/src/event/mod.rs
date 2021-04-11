use libmactoolbox::{events::EventRecord, intl::ScriptCode, quickdraw::Point, types::{Tick, TickDuration}, windows::CWindowRecord};
use smart_default::SmartDefault;
use std::rc::Rc;

#[derive(Clone, Copy, Debug, SmartDefault)]
pub enum Kind {
    #[default]
    None,
    MouseDown,
    MouseUp,
    KeyDown,
    KeyUp,
    Timeout,
    MouseDoubleClick,
    MouseEnter,
    MouseLeave,
    MouseWithin,
    Idle,
    StartMovie,
    StopMovie,
    StepMovie,
    EnterFrame,
    ExitFrame,
    ActivateWindow,
    DeactivateWindow,
    CloseWindow,
    OpenWindow,
    MoveWindow,
    ZoomWindow,
    ResizeWindow,
    RightMouseDown,
    RightMouseUp,
}

// RE: `g_event_*`
#[derive(Debug, SmartDefault)]
pub struct Manager {
    command_key_down: bool,
    control_key_down: bool,
    #[default(TickDuration::from_millis(333))]
    cpu_hog_ticks: TickDuration,
    enabled: bool,
    find_window_region: i16,
    key_down_handled: bool,
    key_events_disabled_maybe: bool,
    #[default(ScriptCode::Roman)]
    keyboard_script_code: ScriptCode,
    last_key_char: i16,
    last_key_char_0: i16,
    last_key_code: i16,
    #[default(Tick::now())]
    last_key_down_time: Tick,
    #[default(Tick::now())]
    last_mouse_down_time: Tick,
    #[default(Tick::now())]
    last_mouse_move_time: Tick,
    last_mouse_move_where: Point,
    last_mouse_position: Point,
    modal_window: Option<CWindowRecord>,
    mouse_down_event_kind: Kind,
    mouse_up_event_kind: Kind,
    movie_just_started: bool,
    #[default(Tick::now())]
    next_obscure_cursor: Tick,
    #[default(Tick::now())]
    next_tick_to_set_mem_limit: Tick,
    num_text_edit_clicks: i16,
    #[default(Tick::now())]
    old_last_mouse_down_time: Tick,
    option_key_down: bool,
    saved_event: Option<EventRecord>,
    shift_key_down: bool,
    r#where: Point,
    window: Option<Rc<CWindowRecord>>,
}

impl Manager {
    pub fn new() -> Self {
        <_>::default()
    }
}
