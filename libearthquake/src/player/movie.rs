use crate::{fonts::Map as FontMap, lingo::types::Actor, resources::{cast::{LibNum, MemberId}, movie::{Cast, CastScoreOrder, FileInfo}, tile::Tiles}, util::RawString};
use binrw::{BinRead, derive_binread};
use libcommon::{Unk16, Unk32, Unk8, UnkHnd, UnkPtr, bitflags};
use libmactoolbox::{quickdraw::{Point, Rect}, resources::{RefNum, ResNum}, typed_resource, types::{Tick, TickDuration}};
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
#[derive(Clone, Copy, Debug)]
struct SpriteCursor;

/// The amount of time that each frame of the movie should be displayed.
///
/// In the original code, durations are stored in milliseconds.
///
/// OsType: `'VWtc'`
#[derive(Clone, Debug, Default)]
struct Timecodes(Vec<std::time::Duration>);
typed_resource!(Timecodes => b"VWtc");

/// Score frame labels.
///
/// OsType: `'VWLB'`
#[derive_binread]
#[derive(Clone, Debug, Default)]
struct Labels {
    #[br(temp)]
    num_labels: u16,
    #[br(count(num_labels))]
    indexes: Vec<(FrameNum, u16)>,
    #[br(temp)]
    labels_size: u32,
    #[br(count(labels_size))]
    data: RawString,
}
typed_resource!(Labels => b"VWLB");

#[derive(Debug, SmartDefault)]
pub struct Movie {
    /// The list of cast libraries loaded by this movie. The first entry is
    /// always the movie’s own internal cast.
    libraries: Vec<Cast>,

    own_lib_num_maybe: LibNum,

    /// The number of the cast library which was most recently activated.
    ///
    /// Lingo: `activeCastLib`
    active_lib_num: LibNum,

    /// The resource number of the movie.
    ///
    /// This is unused in Director 4+ (always 1024). It is normally an i32.
    mac_res_id: ResNum,

    /// Platform font remapping data for the movie.
    font_map: FontMap,

    /// The order in which cast members appear in the movie score.
    score_order: Option<Rc<CastScoreOrder>>,

    frame_timing: Movie14,

    /// Movie score frame labels.
    ///
    /// Originally this was three fields (i16, handle to indexes, handle to
    /// label data), but can be collapsed into one here.
    frame_labels: Labels,

    /// Movie pattern tiles.
    tiles: Tiles,

    field_34: UnkHnd,

    /// Cached movie score frame durations.
    time_codes: Timecodes,

    /// The last generated base resource number for a cast library.
    ///
    /// This is normally an i32.
    #[default(1024)]
    last_used_mac_res_id: ResNum,

    /// This value is always -1 on at least Windows because the function that
    /// would normally open the file is nulled out.
    #[default(-1)]
    some_res_file_ref_num: RefNum,

    is_loaded: bool,

    modified_flags: ModifiedFlags,

    field_44: bool,

    /// Whether or not palette remapping is enabled for the movie.
    ///
    /// Lingo: `the paletteMapping`
    palette_mapping: bool,

    field_46: bool,

    some_legacy_flag: bool, /* score color and flags related */

    /// Lingo: `the updateMovieEnabled`
    update_movie_enabled: bool,

    /// Lingo: `the preloadEventAbort`
    pre_load_event_abort: bool,

    field_4a: Unk8,

    /// If true, the movie was exported from Director as a protected movie.
    ///
    /// This flag is not used by projectors.
    protected: bool,

    field_4c: Unk8,

    field_4d: Unk8,

    field_4e: Unk8,

    vwci_entry_5: i16,

    vwci_entry_6: i16,

    vwci_entry_7: i16,

    // TODO: Do not use magic numbers.
    #[default(MemberId::new(0, -101))]
    default_palette: MemberId,

    field_5a: Unk32,

    /// Metadata about the movie.
    file_info: Option<Rc<FileInfo>>,

    /// The maximum color depth of any resource in the movie?
    max_maybe_color_depth: i16,

    /// The preferred color depth for the screen displaying the movie, according
    /// to its [`Config`].
    default_color_depth: i16,

    /// The score for this movie.
    score: Option<Rc<Score>>,

    /// The window for this movie. Used only by sub-movies.
    window: Option<Rc<Window>>,

    /// The dimensions of the stage for this movie according to its [`Config`].
    default_stage_rect: Rect,

    /// The output graphics port for the movie.
    graf_port: Option<UnkPtr>,

    /// A pointer to a previous movie object.
    ///
    /// This is used when the previous global movie is replaced by this movie.
    /// (The original Director code really liked using globals directly instead
    /// of passing objects.)
    old_movie: Option<Rc<Movie>>,

    /// The frame number on which `pauseState` was set to `true` by Lingo, or a
    /// sentinel value if the `pauseState` was set in an `exitFrame` handler.
    ///
    /// Lingo: `pauseState`
    lingo_paused_frame: PausedFrame,

    delayed_frame_num: FrameNum,

    delayed_until_tick: Option<Tick>,

    field_86: UnkHnd,

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

    per_frame_hook: Option<UnkHnd>,

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
