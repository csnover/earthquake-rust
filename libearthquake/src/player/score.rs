// TODO: You know, finish this file and then remove these overrides
#![allow(clippy::struct_excessive_bools)]
#![allow(dead_code)]

use libcommon::{
    Unk16,
    Unk32,
    Unk8,
    UnkPtr,
};
use libmactoolbox::{Point, Rect, TEHandle};
use crate::resources::cast::{MemberId, MemberKind};

// TODO: Will need to be a bit_field::BitArray for >64 sprites
type SpriteBitmask = u64;

#[derive(Clone, Copy, Debug, Default, Eq, Ord, PartialEq, PartialOrd)]
pub struct Tempo(pub i16);

type FrameNum = i16;
type SpriteNum = i16;

// TODO: Different sizes for different Director versions:
// D3: 24
// D4: 48
// D5: 48
// D6: 120
// D7: 150
const NUM_SPRITES: usize = 32;

#[derive(Clone, Copy, Debug)]
enum Transition {
    Cast(MemberId),
    Custom { chunk_size: u8, which_transition: u8, time: u8, change_area: bool }
}

impl Default for Transition {
    fn default() -> Self {
        Transition::Cast(MemberId::default())
    }
}

#[derive(Default)]
pub struct Score {
    current_frame_vwsc_position: u32,
    next_frame_vwsc_position: u32,
    vwsc: Vec<u8>,
    score_header: Vec<u8>,
    vwsc_frame_data_maybe_start_pos: u32,
    vwsc_frame_data_maybe_end_pos: u32,
    next_frame: SpriteFrame,
    current_frame: SpriteFrame,
    inserted_frame_maybe: SpriteFrame,
    vwsc_channels_used: SpriteBitmask,
    field_12b4: Unk32,
    field_12b8: Unk32,
    current_frame_palette: Palette,
    puppet_sprites: SpriteBitmask,
    maybe_scaled_rect: Rect,
    maybe_unscaled_rect: Rect,
    score_sprites: SpriteBitmask,
    sprites_to_paint0: SpriteBitmask,
    sprites_to_paint1: SpriteBitmask,
    sprite_origins: [ Point; NUM_SPRITES ],
    moveable_sprites: SpriteBitmask,
    immediate_sprites: SpriteBitmask,
    interactive_sprites: SpriteBitmask,
    editable_sprites: SpriteBitmask,
    visible_sprites: SpriteBitmask,
    xtra_maybe_mouse_event_sprites: SpriteBitmask,
    mouse_down_sprites: SpriteBitmask,
    key_up_sprites: SpriteBitmask,
    xtra_maybe_movie_event_sprites: SpriteBitmask,
    xtra_146c_sprites: SpriteBitmask,
    mouse_hover_event_sprites: SpriteBitmask,
    overlay_video_sprites: SpriteBitmask,
    composited_video_sprites: SpriteBitmask,
    vwtk: UnkPtr,
    puppet_transition: Transition,
    field_1494: [ Score1494; NUM_SPRITES ],
    field_16d4: Unk16,
    field_16d6: Unk16,
    field_16d8: Unk16,
    editable_sprite: TextEditor,
    last_maybe_editable_sprite_num: i16,
    maybe_current_editable_sprite_num: i16,
    field_16f2: Unk16,
    current_frame_num: FrameNum,
    current_tempo: Tempo,
    flags: u16,
    palette_mapping: bool,
    maybe_has_current_frame: bool,
    maybe_wrote_frame_delta: bool,
    maybe_error_writing_delta: bool,
    some_pause_state: Unk8,
    field_16ff: Unk8,
    maybe_unscaled_rect_is_not_empty: bool,
    not_paused: bool,
    should_loop: bool,
    maybe_rewind_to_first_frame: bool,
    maybe_has_moveable_sprites: bool,
}

#[derive(Default)]
struct Palette {
    frame_palette_maybe: MemberId,
    field_4: Unk8,
    field_5: Unk8,
    field_6: Unk8,
    field_7: Unk8,
    field_8: Unk16,
    field_a: Unk16,
    field_c: Unk8,
    field_d: Unk8,
    field_e: Unk8,
}

#[derive(Default)]
struct Score1494 {
    data: UnkPtr,
    id: MemberId,
    field_8: Unk8,
    field_9: Unk8,
    flags: u8,
    cast_member_kind: MemberKind,
}

#[derive(Default)]
struct TextEditor {
    h_te: TEHandle,
    rect: Rect,
    sprite_num: SpriteNum,
    id: MemberId,
    field_12: Unk8,
    field_13: Unk8,
}

#[derive(Default)]
struct SpriteFrame {
    frame: Frame,
    rects: [ Rect; NUM_SPRITES ],
}

#[derive(Default)]
struct Frame {
    script: MemberId,
    sound_1: MemberId,
    sound_2: MemberId,
    transition: Transition,
    tempo_related: Unk8,
    sound_1_related: Unk8,
    sound_2_related: Unk8,
    script_related: Unk8,
    transition_related: Unk8,
    tempo: Tempo,
    palette: Palette,
    sprites: [ FrameCell; NUM_SPRITES ],
}

#[derive(Default)]
struct FrameCell {
    kind: Unk8,
    ink_and_flags: Unk8,
    id: MemberId,
    script: MemberId,
    fore_color_index: u8,
    back_color_index: u8,
    origin: Point,
    height: i16,
    width: i16,
    score_color_and_flags: u8,
    blend_amount: u8,
    line_size_and_flags: u8,
}
