use libcommon::{
    Unk16,
    Unk32,
    Unk8,
    UnkPtr,
};
use libmactoolbox::{
    Rect,
    TEHandle,
};
use crate::resources::cast::MemberId;

// TODO: Will need to be a bit_field::BitArray for >64 sprites
type SpriteBitmask = u64;

#[derive(Clone, Copy, Debug, Default, Eq, Ord, PartialEq, PartialOrd)]
pub struct Tempo(pub u16);

type FrameNum = u16;
type SpriteNum = u16;

// TODO: Different sizes for different Director versions:
// D3: 24
// D4: 48
// D5: 48
// D6: 120
// D7: 150
const NUM_SPRITES: usize = 48;

struct Score {
    field_0: Unk32,
    field_4: Unk32,
    field_8: Unk32,
    field_c: UnkPtr,
    field_10: Unk32,
    field_14: Unk32,
    next_frame: SpriteFrame,
    current_frame: SpriteFrame,
    inserted_frame_maybe: SpriteFrame,
    field_12a8: Unk32,
    field_12ac: Unk32,
    field_12b4: Unk32,
    current_frame_field_18_maybe: Score12BC,
    puppet_sprites: SpriteBitmask,
    stage_maybe_rect: Rect,
    field_12e4: Rect,
    field_12ec: Unk32,
    field_12f0: Unk32,

    field_1304: [ Score1304; NUM_SPRITES ],
    field_13c4: [ u16; NUM_SPRITES ],
    field_1424: SpriteBitmask,
    immediate_sprites: SpriteBitmask,

    field_147c: SpriteBitmask,

    field_1494: [ Score1494; NUM_SPRITES ],
    field_16d4: Unk16,
    field_16d6: Unk16,
    field_16d8: Unk16,
    field_16da: Score16DA,
    field_16ee: Unk16,
    field_16f0: Unk16,
    field_16f2: Unk16,
    current_frame_num: FrameNum,
    current_tempo: Tempo,
    field_16f8: Unk16,
    field_16fa: Unk8,
    field_c_is_init: bool,
    field_16fc: Unk8,
    field_16fd: Unk8,
    field_16fe: Unk8,
    field_16ff: Unk8,
    some_field_12e4_rect_is_not_empty: bool,
    pause_state: bool,
    field_1702: Unk8,
    field_1703: Unk8,
    field_1704: Unk8,
    field_1705: Unk8,
}

struct Score12BC {
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

struct Score1304 {
    field_0: Unk16,
    field_2: Unk16,
}

struct Score1494 {
    // size 0xc
}

struct Score16DA {
    h_te: TEHandle,
    rect: Rect,
    sprite_num: SpriteNum,
    id: MemberId,
    field_12: Unk8,
    field_13: Unk8,
}

struct SpriteFrame {
    frame: Frame,
    rects: [ Rect; NUM_SPRITES ],
}

struct Frame {
    script: MemberId,
    field_4: Unk32,
    field_8: Unk32,
    field_c: Unk16,
    field_e: Unk16,

    tempo: Tempo,
    field_18: Score12BC,
    cells: [ FrameCell; NUM_SPRITES ],
}

struct FrameCell {
    field_0: Unk8,
    field_1: Unk8,
    id: MemberId,
    field_6: Unk32,

    field_10: Unk16,
    field_12: Unk16,
    field_14: Unk8,
}
