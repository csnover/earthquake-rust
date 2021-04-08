use anyhow::Result as AResult;
use binrw::{BinRead, io::Cursor};
use crate::resources::config::Version as ConfigVersion;
use libcommon::{io::prelude::*, prelude::*, bitflags};
use libmactoolbox::quickdraw::{Point, Rect};
use smart_default::SmartDefault;
use super::{Fps, Frame, FrameNum, NUM_SPRITES, Palette, Score1494, Stream, Sprite, SpriteBitmask, Tempo, TextEditor, Transition, Version};

bitflags! {
    #[derive(Default)]
    pub struct Flags: u16 {
        /// Any updates to the score during authoring should not be displayed
        /// on the stage.
        const UPDATE_LOCK              = 1;
        const MAYBE_STAGE_NEEDS_UPDATE = 2;
        /// Score frame data has been modified in authoring mode.
        const FRAME_MODIFIED           = 4;
        const FLAG_8                   = 8;
        /// Score
        const SCORE_MODIFIED           = 0x10;
        const WAIT_FOR_EVENT           = 0x20;
        const WAIT_TICKS               = 0x40;
    }
}

#[allow(clippy::struct_excessive_bools)]
#[derive(Clone, Debug, SmartDefault)]
pub struct Score {
    #[default(ScoreHeaderV5::SIZE)]
    current_frame_vwsc_position: u32,
    next_frame_vwsc_position: u32,
    vwsc: Stream,
    score_header: Vec<u8>,
    #[default(ScoreHeaderV5::SIZE)]
    vwsc_frame_data_maybe_start_pos: u32,
    #[default(ScoreHeaderV5::SIZE)]
    vwsc_frame_data_maybe_end_pos: u32,
    next_frame: SpriteFrame,
    current_frame: SpriteFrame,
    inserted_frame_maybe: SpriteFrame,
    vwsc_channels_used: SpriteBitmask,
    field_12b4: Unk32,
    wait_for: Unk32,
    current_frame_palette: Palette,
    puppet_sprites: SpriteBitmask,
    maybe_scaled_rect: Rect,
    maybe_unscaled_rect: Rect,
    score_sprites: SpriteBitmask,
    sprites_to_paint0: SpriteBitmask,
    sprites_to_paint1: SpriteBitmask,
    #[default([ Point { x: (-0x8000_i16).into(), y: 0_i16.into() }; NUM_SPRITES ])]
    sprite_origins: [ Point; NUM_SPRITES ],
    moveable_sprites: SpriteBitmask,
    immediate_sprites: SpriteBitmask,
    interactive_sprites: SpriteBitmask,
    editable_sprites: SpriteBitmask,
    hidden_sprites: SpriteBitmask,
    xtra_maybe_mouse_event_sprites: SpriteBitmask,
    mouse_down_sprites: SpriteBitmask,
    key_up_sprites: SpriteBitmask,
    xtra_maybe_movie_event_sprites: SpriteBitmask,
    xtra_146c_sprites: SpriteBitmask,
    mouse_hover_event_sprites: SpriteBitmask,
    overlay_video_sprites: SpriteBitmask,
    composited_video_sprites: SpriteBitmask,
    vwtk: UnkHnd,
    puppet_transition: Transition,
    #[default([ <_>::default(); NUM_SPRITES ])]
    field_1494: [ Score1494; NUM_SPRITES ],
    #[default(Unk16(-0x8000))]
    field_16d4: Unk16,
    #[default(Unk16(-0x8000))]
    field_16d6: Unk16,
    #[default(Unk16(-0x8000))]
    field_16d8: Unk16,
    editable_sprite: TextEditor,
    last_maybe_editable_sprite_num: i16,
    maybe_current_editable_sprite_num: i16,
    field_16f2: Unk16,
    current_frame_num: FrameNum,
    #[default(Tempo::Fps(Fps(15)))]
    current_tempo: Tempo,
    flags: Flags,
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

// TODO: This is just for debugging
impl Iterator for Score {
    type Item = AResult<Frame>;

    fn next(&mut self) -> Option<Self::Item> {
        match self.vwsc.next(self.puppet_sprites) {
            Ok(Some(frame)) => {
                self.current_frame.frame = frame.clone();
                self.current_frame_num += FrameNum(1);
                Some(Ok(frame))
            },
            Ok(None) => None,
            Err(e) => Some(Err(e)),
        }
    }
}

fn frame_size_in_cells(version: Version) -> u16 {
    if version < Version::V5 {
        Frame::V0_SIZE_IN_CELLS
    } else {
        Frame::V5_SIZE_IN_CELLS
    }
}

fn sprite_size(version: Version) -> u16 {
    if version < Version::V5 {
        Sprite::V0_SIZE
    } else {
        Sprite::V5_SIZE
    }
}

#[derive(BinRead, Debug)]
#[br(big, import(size: u32))]
struct ScoreHeaderV3 {
    #[br(assert(
        own_size <= size,
        "Score recorded size ({}) is larger than actual size ({})",
        own_size,
        size
    ))]
    own_size: u32,
}

#[derive(BinRead, Debug)]
#[br(big, import(size: u32))]
struct ScoreHeaderV5 {
    #[br(assert(
        own_size <= size,
        "Score recorded size ({}) is larger than actual size ({})",
        own_size,
        size
    ))]
    own_size: u32,

    #[br(assert(
        header_size == Self::SIZE,
        "Invalid V0-V7 score header size {}",
        header_size
    ))]
    header_size: u32,

    // This field is not always filled out
    frame_count: u32,

    #[br(assert(
        matches!(score_version, Version::V4 | Version::V5 | Version::V6 | Version::V7),
        "Bad score version"
    ))]
    score_version: Version,

    #[br(assert(
        frame_cell_size == sprite_size(score_version),
        "Invalid frame cell size {} for V5 score version {}",
        frame_cell_size,
        score_version,
    ))]
    frame_cell_size: u16,

    // Technically this is the number of `sizeof(Sprite)`s to make one
    // `sizeof(Frame)`; the header of the frame is exactly two
    // `sizeof(Sprite)`s, even though it does not actually contain sprite
    // data
    #[br(assert(
        frame_cell_count == frame_size_in_cells(score_version),
        "Invalid frame cell count {} for V5 score version {}",
        frame_cell_count,
        score_version,
    ))]
    frame_cell_count: u16,

    #[br(assert(field_12 == 0 || field_12 == 1, "Unexpected score field_12 {}", field_12))]
    field_12: u8,

    #[br(assert(field_13 == 0, "Unexpected score field_13 {}", field_13))]
    field_13: u8,
}

impl ScoreHeaderV5 {
    const SIZE: u32 = 20;
}

impl BinRead for Score {
    type Args = (crate::resources::config::Version, );

    // TODO: Should this receive a size option instead of relying on the input
    // being truncated?
    fn read_options<R: Read + Seek>(input: &mut R, options: &binrw::ReadOptions, (config_version, ): Self::Args) -> binrw::BinResult<Self> {
        restore_on_error(input, |input, _| {
            let mut options = *options;
            options.endian = binrw::Endian::Big;

            let size = input.bytes_left()?;
            options.count = Some(size.unwrap_into());
            let data = Vec::read_options(&mut input.take_seek(size), &options, ())?;
            options.count = None;

            let mut input = Cursor::new(data);

            let (own_size, version) = if config_version < ConfigVersion::V1113 {
                let header = ScoreHeaderV3::read_options(&mut input, &options, (size.unwrap_into(), ))?;
                (header.own_size, Version::V3)
            } else if config_version < ConfigVersion::V1222 {
                let header = ScoreHeaderV5::read_options(&mut input, &options, (size.unwrap_into(), ))?;

                // Director normally reads through all of the frame deltas here in order
                // to byte swap them into the platform’s native endianness, but since we
                // are using an endianness-aware reader, we’ll just let that happen
                // when the frames are read later

                (header.own_size, header.score_version)
            } else {
                todo!("Score config version {} parsing", config_version as i32);
            };

            let pos = input.pos()?;

            Ok(Self {
                vwsc: Stream::new(input, pos.unwrap_into(), own_size, version),
                ..Self::default()
            })
        })
    }
}

/// A frame of animation data containing computed rects for each sprite in the
/// frame.
#[derive(Clone, Debug, SmartDefault)]
struct SpriteFrame {
    frame: Frame,
    #[default([ Rect::default(); NUM_SPRITES ])]
    rects: [ Rect; NUM_SPRITES ],
}
