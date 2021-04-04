use crate::{lingo::types::Actor, resources::{cast::{LibNum, MemberId}, movie::Cast}};
use libcommon::{Unk16, Unk32, Unk8, UnkPtr, bitflags};
use libmactoolbox::{quickdraw::{Point, Rect}, resources::RefNum, types::{Tick, TickDuration}};
use smart_default::SmartDefault;
use std::{collections::BTreeSet, rc::Rc};
use super::{score::{ChannelNum, Fps, FrameNum, NUM_SPRITES, Score, SpriteBitmask}, window::Window};

bitflags! {
    #[derive(Default)]
    pub struct ModifiedFlags: u8 {
        /// A new cast library was added.
        const ADDED_CAST    = 1;
        /// An existing cast library was modified.
        ///
        /// This may be a modification of a setting of the cast library, or
        /// of a cast member inside the library.
        const MODIFIED_CAST = 2;
    }
}

#[derive(Debug, SmartDefault)]
enum PausedFrame {
    #[default]
    NotPaused,
    InExitFrame,
    Paused(FrameNum),
}

#[derive(Debug, SmartDefault)]
enum ButtonStyle {
    #[default]
    ListStyle = 0,
    DialogStyle,
}

/// Determines what happens when a user clicks a checkbox or radio button.
#[derive(Debug, SmartDefault)]
enum CheckBoxAccess {
    /// Lets the user toggle checkboxes and radio buttons on and off.
    #[default]
    All = 0,
    /// Lets the user toggle checkboxes and radio buttons on, but not off.
    OnOnly,
    /// Only scripts are allowed to set the state of checkboxes and radio
    /// buttons.
    ScriptOnly,
}

/// The style of a checkbox when it is checked.
///
/// Lingo: `checkBoxType`
#[derive(Debug, SmartDefault)]
enum CheckBoxStyle {
    /// Draw an ‘x’ in a checked box.
    #[default]
    Standard = 0,
    /// Draw a black rectangle in a checked box.
    BlackRect,
    /// Completely fill a checked box.
    Filled,
}

#[derive(Debug, Default)]
struct Menu;
#[derive(Debug, Default)]
struct MenuBar;
#[derive(Debug, Default)]
struct Movie14;
#[derive(Copy, Clone, Debug)]
struct SpriteCursor;

#[derive(Debug, SmartDefault)]
pub struct Movie {
    libraries: Vec<Cast>,
    own_lib_num_maybe: LibNum,
    active_lib_num: LibNum,
    mac_res_id: i32,
    font_map: UnkPtr, /* FXmp */
    /// Comes from Sord resource.
    score_order: Vec<MemberId>,
    frame_timing: Movie14,
    max_frame_num_maybe: i16,
    frame_label_indexes: Vec<(i16, i16)>,
    frame_labels: Vec<u8>,
    tiles: UnkPtr, /* VWTL */
    field_34: UnkPtr,
    time_codes: UnkPtr, /* VWtc */
    #[default(1024)]
    last_used_mac_res_id: i32,
    some_res_file_ref_num: RefNum,
    is_loaded: bool,
    modified_flags: ModifiedFlags,
    field_44: bool,
    palette_mapping: bool,
    field_46: bool,
    some_legacy_flag: bool, /* score color and flags related */
    update_movie_enabled: bool,
    pre_load_event_abort: bool,
    field_4a: Unk8,
    protected: bool,
    field_4c: Unk8,
    field_4d: Unk8,
    field_4e: Unk8,
    vwci_entry_5: i16,
    vwci_entry_6: i16,
    vwci_entry_7: i16,
    default_palette: MemberId,
    field_5a: Unk32,
    file_info: UnkPtr, /* VWFI */
    /// The maximum color depth of a resource in the movie?
    max_maybe_color_depth: i16,
    default_color_depth: i16,
    score: Option<Rc<Score>>,
    window: Option<Rc<Window>>,
    default_stage_rect: Rect,
    graf_port: Option<UnkPtr>,
    old_movie: Option<Rc<Movie>>,
    lingo_paused_frame: PausedFrame,
    delayed_frame_num: FrameNum,
    delayed_until_tick: Option<Tick>,
    field_86: UnkPtr,
    cast_member_hilites: BTreeSet<MemberId>,
    field_8e: Unk32,
    some_score: Option<Rc<Score>>,
    inverted_maybe_sprite_num: ChannelNum,
    /// The `buttonStyle` from Lingo.
    button_style: ButtonStyle,
    menu: Option<Rc<Menu>>,
    menu_bar: Option<Rc<MenuBar>>,
    timeout_lapsed: Unk32,
    timeout: Unk32,
    timer: Unk32,
    field_ae: Unk16,
    is_paused: Unk16,
    cursored_sprites: SpriteBitmask,
    #[default([ SpriteCursor; NUM_SPRITES + 2 ])]
    sprite_cursors: [ SpriteCursor; NUM_SPRITES + 2 ],
    last_clicked_sprite: ChannelNum,
    last_some_event_sprite: ChannelNum,
    mouse_over_sprite: ChannelNum,
    mouse_over_where: Point,
    mouse_over_score: Option<Rc<Score>>,
    check_box_style: CheckBoxStyle,
    check_box_access: CheckBoxAccess,
    click_loc: Point,
    last_key_char_size: u8,
    last_key_char_low: u8,
    last_key_char_high: u8,
    frame_exited_maybe: Unk8,
    field_390: Unk8,
    field_391: Unk8,
    is_mouse_down: bool,
    is_movie_unloading_maybe: bool,
    field_394: Unk8,
    quit_all_movies: bool,
    quit_this_movie: bool,
    dont_pass_event: bool, /* :-( */
    beep_on: bool,
    timeout_play: bool,
    timeout_mouse: bool,
    timeout_keydown: bool,
    exit_lock: bool,
    paused_in_exit_frame_event: bool,
    is_stopped_maybe: bool,
    quit_lingo_maybe: bool,
    vwtc_init_state: Unk16,
    bg_color_index: i16, /* TODO: type? this is the stage colour. */
    actors: Vec<Actor>,
    per_frame_hook: Option<UnkPtr>,
    idle_handler_period: TickDuration,
    #[default(Tick::now())]
    idle_handler_next_tick: Tick,
    list_51b69c_num: Unk16, /* own 51b69c num? */
    field_3b8: Unk16,
    frame_state: FrameState,
    in_exit_frame_event: bool,
    mouse_up_event_sent: bool,
    #[default(Fps(60))]
    digital_video_time_scale: Fps,
    in_paint_proc: bool,
}

#[derive(Debug, SmartDefault)]
enum FrameState {
    /// The movie is not playing.
    #[default]
    NotPlaying = 0,
    /// The movie state is in frame.
    EnteredFrame,
    /// The movie state is between frames.
    ExitedFrame,
}
