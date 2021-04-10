use crate::{fonts::Map as FontMap, lingo::types::Actor, resources::{cast::{LibNum, MemberId}, movie::{Cast, CastScoreOrder, FileInfo}, tile::Tiles}, util::RawString};
use binrw::derive_binread;
use libcommon::{Unk16, Unk32, Unk8, UnkHnd, UnkPtr, bitflags};
use libmactoolbox::{quickdraw::{Point, Rect}, resources::{RefNum, ResNum}, typed_resource, types::{Tick, TickDuration}};
use smart_default::SmartDefault;
use std::{collections::BTreeSet, rc::Rc};
use super::{score::{ChannelNum, Fps, FrameNum, NUM_SPRITES, Palette, Score, SpriteBitmask}, window::Window};

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

#[derive(Clone, Copy, Debug, SmartDefault)]
enum TimecodesState {
    #[default]
    Zero = 0,
    Two = 2,
    Three,
}

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
    /// Lingo: `the activeCastLib`
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
    #[default(1024_i16.into())]
    last_used_mac_res_id: ResNum,

    /// This value is always -1 on at least Windows because the function that
    /// would normally open the file is nulled out.
    #[default(RefNum(-1))]
    some_res_file_ref_num: RefNum,

    /// Whether or not the movie data has been successfully loaded from disk.
    is_loaded: bool,

    /// Flags used to indicate that some movie data has been modified in memory.
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

    vwfi_flag_2000h: Unk8,

    vwfi_flag_1000h: Unk8,

    vwfi_entry_5: i16,

    vwfi_entry_6: i16,

    vwfi_entry_7: i16,

    /// The initial palette to use for the movie.
    #[default(Palette::SYSTEM_WIN_DIR_4)]
    default_palette: MemberId,

    legacy_maybe_movie_script: MemberId,

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
    /// Lingo: `the pauseState`
    lingo_paused_frame: PausedFrame,

    /// The frame number where the last delay command was received.
    ///
    /// Lingo: `delay`
    delayed_frame_num: FrameNum,

    /// The time at which playback should resume after being delayed.
    ///
    /// Lingo: `delay`
    delayed_until_tick: Option<Tick>,

    /// The set of check boxes and radio buttons which are currently selected.
    ///
    /// Lingo: `the hilite of member`
    cast_member_hilites: BTreeSet<MemberId>,

    /// The score that `inverted_maybe_sprite_num` belongs to.
    inverted_sprite_score: Option<Rc<Score>>,

    // Maybe the number of the sprite currently being rendered inverted because
    // the mouse is down on the sprite and auto-hilite is on.
    inverted_maybe_sprite_num: ChannelNum,

    /// The visual response type when a user presses a button and then hovers to
    /// another button without first releasing the mouse.
    ///
    /// Lingo: `the buttonStyle`
    button_style: ButtonStyle,

    /// The native OS menu objects for the movie’s menu bar.
    menu: Option<Rc<Menu>>,

    /// The menu bar contents for this movie.
    ///
    /// Lingo: `menu` and `installMenu`
    menu_bar: Option<Rc<MenuBar>>,

    /// The amount of time since the last timeout event.
    ///
    /// Lingo: `the timeoutLapsed`
    timeout_lapsed: TickDuration,

    /// The amount of time between timeout events.
    ///
    /// Lingo: `the timeoutLength`
    #[default(TickDuration::from_secs(180))]
    timeout: TickDuration,

    /// Lingo: `the timer`
    timer: TickDuration,

    /// The starting character of a selection, one-indexed.
    ///
    /// Lingo: `the selStart`
    selection_start: i16,

    /// The ending character of a selection, one-indexed.
    ///
    /// Lingo: `the selEnd`
    selection_end: i16,

    /// A bitmap describing which sprites have cursors set by Lingo.
    ///
    /// Lingo: `the cursor of sprite`
    cursored_sprites: SpriteBitmask,

    /// The cursors set by Lingo for each sprite. The fourth channel defines the
    /// default cursor.
    ///
    /// Lingo: `the cursor` and `the cursor of sprite`
    #[default([ SpriteCursor; NUM_SPRITES + 2 ])]
    sprite_cursors: [ SpriteCursor; NUM_SPRITES + 2 ],

    /// The last active (i.e. has a script) sprite clicked by the user.
    ///
    /// Lingo: `the clickOn`
    last_clicked_sprite: ChannelNum,

    /// The penultimate active (i.e. has a script) sprite clicked by the user.
    last_some_event_sprite: ChannelNum,

    /// The sprite that the mouse is currently hovered over, if any.
    /// One-indexed. Used for event management.
    mouse_over_sprite: ChannelNum,

    /// The position on the stage of the mouse cursor. Used for event
    /// management.
    mouse_over_where: Point,

    /// The score which corresponds to the sprite being hovered.
    mouse_over_score: Option<Rc<Score>>,

    /// The style of a checkbox when it is checked.
    ///
    /// Lingo: `the checkBoxType`
    check_box_style: CheckBoxStyle,

    /// The behaviour to use when a user clicks a checkbox or radio button.
    ///
    /// Lingo: `the checkBoxAccess`
    check_box_access: CheckBoxAccess,

    /// The last position on the stage where the user clicked.
    ///
    /// Lingo: `the clickLoc`
    click_loc: Point,

    /// The size of the last pressed key, as a character.
    ///
    /// Despite its existence here, this value is normally taken from a separate
    /// global in Director code.
    last_key_char_size: u8,

    /// The low byte of the last pressed key, as a character.
    ///
    /// Despite its existence here, this value is normally taken from a separate
    /// global in Director code.
    last_key_char_low: u8,

    /// The high byte of the last pressed key, as a multi-byte character.
    ///
    /// Despite its existence here, this value is normally taken from a separate
    /// global in Director code.
    last_key_char_high: u8,

    /// An flag that the movie is in the process of transitioning to the next
    /// frame.
    ///
    /// This flag is set just prior to the `exitFrame` event trigger.
    frame_exited_maybe: bool,

    field_390: Unk8,

    field_391: Unk8,

    /// If true, get the mouse position from an event global. Otherwise, get it
    /// from the Macintosh Toolbox.
    is_mouse_down: bool,

    is_movie_unloading_maybe: bool,

    field_394: Unk8,

    /// If set, the player should terminate playback of all movies and quit.
    quit_all_movies: bool,

    /// If set, the player should terminate playback of this movie and start
    /// playing the next movie in the playlist.
    quit_this_movie: bool,

    /// If set inside of an event handler, stops propagation of an event to
    /// subsequent locations in the message hierarchy.
    ///
    /// This property affects only the currently dispatched event.
    ///
    ///  Lingo: `dontPassEvent`
    dont_pass_event: bool, /* :-( */

    /// Causes the computer to emit an error noise when clicking outside of
    /// active (i.e. has a script) sprites.
    ///
    /// Lingo: `the beepOn`
    beep_on: bool,

    /// If true, reset the `timeoutLapsed` property when a movie is played.
    ///
    /// Lingo: `the timeoutPlay`
    timeout_play: bool,

    /// If true, reset the `timeoutLapsed` property when a `mouseDown` event
    /// occurs.
    ///
    /// Lingo: `the timeoutMouse`
    timeout_mouse: bool,

    /// If true, reset the `timeoutLapsed` property when a `keyDown` event
    /// occurs.
    ///
    /// Lingo: `the timeoutKeyDown`
    timeout_keydown: bool,

    /// If true, users cannot quit the projector using normal keyboard
    /// shortcuts.
    ///
    ///  Lingo: `the exitLock`
    exit_lock: bool,

    /// If true, the movie was paused from inside an `exitFrame` event handler.
    paused_in_exit_frame_event: bool,

    is_stopped_maybe: bool,

    quit_lingo_maybe: bool,

    /// The initialisation state of the cached timecode data for this movie.
    vwtc_init_state: TimecodesState,

    /// The background colour of the stage.
    bg_color_index: i16,

    /// A list of Lingo objects which receive `mouseHitTest` messages.
    actors: Vec<Actor>,

    /// An `XObject` which is called every frame with a `mAtFrame` message.
    ///
    /// In Director 4, this property was deprecated and replaced with the
    /// `the actorList` and `on enterFrame` (D5) or `on stepFrame` (D6)
    /// handlers.
    ///
    /// Lingo: `the perFrameHook`
    per_frame_hook: Option<UnkHnd>,

    /// The maximum number of ticks that should pass until an `idle` message is
    /// sent to Lingo.
    ///
    /// Lingo: `the idleHandlerPeriod`
    idle_handler_period: TickDuration,

    /// The next tick when an `idle` message should be sent.
    #[default(Tick::now())]
    idle_handler_next_tick: Tick,

    list_51b69c_num: Unk16, /* own 51b69c num? */

    /// A counter for limiting reentry into `enterFrame` events.
    in_enter_frame_count: i16,

    /// The interframe playback state of the movie.
    ///
    /// Because transitions can be defined to occur “between” frames, the
    /// movie can be playing, but not any particular frame.
    frame_state: FrameState,

    /// A flag to prevent reentry into `exitFrame` events.
    in_exit_frame_event: bool,

    /// A flag to ensure `mouseDown` and `mouseUp` events are sent only once
    /// and in the correct order.
    mouse_up_event_sent: bool,

    /// The time scale to use when tracking digital video cast members so that
    /// the system’s time unit for video is a multiple of the actual video’s
    /// time unit.
    ///
    /// Lingo: `the digitalVideoTimeScale`
    #[default(Fps(60))]
    digital_video_time_scale: Fps,

    /// A flag used to prevent reentry into the score painting routine.
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
