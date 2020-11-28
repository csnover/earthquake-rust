// TODO: You know, finish this file and then remove these overrides
#![allow(clippy::struct_excessive_bools)]
#![allow(dead_code)]

use anyhow::{bail, Context, Result as AResult};
use bitflags::bitflags;
use byteordered::{ByteOrdered, Endianness};
use crate::{ensure_sample, resources::{transition::{Kind as TransitionKind, QuarterSeconds}, cast::{MemberId, MemberKind}}};
use derive_more::{Add, AddAssign, Display};
use num_derive::FromPrimitive;
use num_traits::FromPrimitive;
use libcommon::{Reader, Resource, Unk16, Unk32, Unk8, UnkPtr, resource::Input};
use libmactoolbox::{quickdraw::Pen, Point, Rect, TEHandle};
use smart_default::SmartDefault;
use std::{io::{Cursor, Read}, iter::Rev};

macro_rules! load_enum {
    ($name: expr, $kind: expr, $input: expr) => (
        $input
        .context(concat!("Can’t read ", $name))
        .and_then(|value| {
            $kind(value)
                .with_context(|| format!(concat!("Invalid value 0x{:x} for ", $name), value))
        })
    )
}

macro_rules! load_flags {
    ($name: expr, $kind: ty, $input: expr) => (
        $input
        .context(concat!("Can’t read", $name))
        .and_then(|value| {
            <$kind>::from_bits(value)
                .with_context(|| format!(concat!("Invalid value 0x{:x} for ", $name), value))
        })
    )
}

macro_rules! load_member_num {
    ($name: expr, $input: expr) => (
        load_value!($name, $input)
            .map(|num| MemberId::new(if num == 0 { 0 } else { 1 }, num))
    );
}

macro_rules! load_resource {
    ($name: literal, $kind: ty, $input: expr, $size: expr, $context: expr) => (<$kind>::load($input, $size, &$context).context(concat!("Can’t read ", $name)));
    ($name: literal, $kind: ty, $input: expr, $context: expr) => (<$kind>::load($input, <$kind>::SIZE, &$context).context(concat!("Can’t read ", $name)));
    ($name: literal, $kind: ty, $input: expr) => (<$kind>::load($input, <$kind>::SIZE, &()).context(concat!("Can’t read", $name)));
    ($name: expr, $kind: ty, $input: expr, $size: expr, $context: expr) => (<$kind>::load($input, $size, &$context).with_context(|| format!("Can’t read {}", $name)));
    ($name: expr, $kind: ty, $input: expr, $context: expr) => (<$kind>::load($input, <$kind>::SIZE, &$context).with_context(|| format!("Can’t read {}", $name)));
    ($name: expr, $kind: ty, $input: expr) => (<$kind>::load($input, <$kind>::SIZE, &()).with_context(|| format!("Can’t read {}", $name)));
}

macro_rules! load_value {
    ($name: literal, $input: expr) => ($input.context(concat!("Can’t read ", $name)));
    ($name: expr, $input: expr) => ($input.with_context(|| format!("Can’t read {}", $name)));
}

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

#[derive(Add, AddAssign, Clone, Copy, Debug, Default, Eq, Ord, PartialEq, PartialOrd)]
pub struct FrameNum(pub i16);

#[derive(Add, AddAssign, Clone, Copy, Debug, Default, Eq, Ord, PartialEq, PartialOrd)]
pub struct ChannelNum(pub i16);

#[derive(Add, AddAssign, Clone, Copy, Debug, Default, Eq, Ord, PartialEq, PartialOrd)]
pub struct Seconds(pub i16);

#[derive(Add, AddAssign, Clone, Copy, Debug, Default, Eq, Ord, PartialEq, PartialOrd)]
pub struct FPS(pub i16);

#[derive(Clone, Copy, Debug, Eq, Ord, PartialEq, PartialOrd, SmartDefault)]
pub enum Tempo {
    #[default]
    FPS(FPS),
    WaitForVideo(ChannelNum),
    WaitForSeconds(Seconds),
    WaitForClick,
    WaitForSound1,
    WaitForSound2,
}

impl Tempo {
    pub fn new(tempo: i16) -> AResult<Self> {
        Ok(match tempo {
            0..=120 => Self::FPS(FPS(tempo)),
            -0x78..=-0x48 => Self::WaitForVideo(ChannelNum(tempo + 0x7e)),
            -60..=-1 => Self::WaitForSeconds(Seconds(-tempo)),
            -0x80 => Self::WaitForClick,
            -0x79 => Self::WaitForSound1,
            -0x7a => Self::WaitForSound2,
            value => bail!("Invalid tempo {}", value),
        })
    }

    #[must_use]
    pub fn to_primitive(self) -> i16 {
        match self {
            Tempo::FPS(fps) => fps.0,
            Tempo::WaitForVideo(channel) => channel.0 - 0x7e,
            Tempo::WaitForSeconds(seconds) => -seconds.0,
            Tempo::WaitForClick => -0x80,
            Tempo::WaitForSound1 => -0x79,
            Tempo::WaitForSound2 => -0x7a,
        }
    }
}

// TODO: Different sizes for different Director versions:
// D3: 24
// D4: 48
// D5: 48
// D6: 120
// D7: 150
const NUM_SPRITES: usize = 150;

// TODO: Eventually use some crate like bit_field or bitarray
#[derive(Clone, Copy, Default)]
pub struct SpriteBitmask([ u8; SpriteBitmask::SIZE ]);

pub struct BitIter<'owner> {
    owner: &'owner SpriteBitmask,
    index: usize,
}

impl <'owner> Iterator for BitIter<'owner> {
    type Item = usize;

    fn next(&mut self) -> Option<Self::Item> {
        while self.index != SpriteBitmask::NUM_CHANNELS {
            let index = self.index;
            self.index += 1;
            if self.owner.contains(index) {
                return Some(index);
            }
        }

        None
    }

    fn size_hint(&self) -> (usize, Option<usize>) {
        (0, Some(SpriteBitmask::NUM_CHANNELS))
    }
}

impl <'owner> DoubleEndedIterator for BitIter<'owner> {
    fn next_back(&mut self) -> Option<Self::Item> {
        while self.index != 0 {
            self.index -= 1;
            if self.owner.contains(self.index) {
                return Some(self.index);
            }
        }

        None
    }
}

impl SpriteBitmask {
    const NUM_NON_SPRITE_CHANNELS: usize = 6;
    const MIN_SPRITE: usize = Self::NUM_NON_SPRITE_CHANNELS;
    const MAX_SPRITE: usize = NUM_SPRITES + Self::NUM_NON_SPRITE_CHANNELS - 1;
    const NUM_CHANNELS: usize = NUM_SPRITES + Self::NUM_NON_SPRITE_CHANNELS;
    const SIZE: usize = (Self::NUM_CHANNELS + 7) / 8;

    const SCRIPT: usize     = 0;
    const TEMPO: usize      = 1;
    const TRANSITION: usize = 2;
    const SOUND_2: usize    = 3;
    const SOUND_1: usize    = 4;
    const PALETTE: usize    = 5;

    fn all() -> Self {
        let mut bits = [ 0xFF; Self::SIZE ];
        bits[Self::SIZE - 1] &= ((1_u16 << (Self::NUM_CHANNELS % 8)) - 1) as u8;
        SpriteBitmask(bits)
    }

    fn bits(&self) -> [ u8; Self::SIZE ] {
        self.0
    }

    fn contains(&self, bit: usize) -> bool {
        assert!(bit < Self::NUM_CHANNELS);
        self.0[bit / 8] & (1 << (bit % 8)) != 0
    }

    fn empty() -> Self {
        SpriteBitmask::default()
    }

    fn iter(&self) -> BitIter<'_> {
        BitIter { owner: self, index: 0 }
    }

    fn iter_back(&self) -> Rev<BitIter<'_>> {
        BitIter { owner: self, index: Self::NUM_CHANNELS }.rev()
    }

    fn iter_sprites(&self) -> BitIter<'_> {
        BitIter { owner: self, index: Self::MIN_SPRITE }
    }

    fn is_empty(&self) -> bool {
        self.0 == [ 0; Self::SIZE ]
    }

    fn remove(&mut self, bit: usize) -> &mut Self {
        assert!(bit < Self::NUM_CHANNELS);
        self.0[bit / 8] &= !(1 << (bit % 8));
        self
    }

    fn set(&mut self, bit: usize) -> &mut Self {
        assert!(bit < Self::NUM_CHANNELS);
        self.0[bit / 8] |= 1 << (bit % 8);
        self
    }
}

impl std::ops::BitAnd for SpriteBitmask {
    type Output = Self;

    fn bitand(self, rhs: Self) -> Self::Output {
        let mut bits = self;
        std::ops::BitAndAssign::bitand_assign(&mut bits, rhs);
        bits
    }
}

impl std::ops::BitAndAssign for SpriteBitmask {
    fn bitand_assign(&mut self, rhs: Self) {
        for i in 0..Self::SIZE {
            self.0[i] &= rhs.0[i];
        }
    }
}

impl std::ops::BitOr for SpriteBitmask {
    type Output = Self;

    fn bitor(self, rhs: Self) -> Self::Output {
        let mut bits = self;
        std::ops::BitOrAssign::bitor_assign(&mut bits, rhs);
        bits
    }
}

impl std::ops::BitOrAssign for SpriteBitmask {
    fn bitor_assign(&mut self, rhs: Self) {
        for i in 0..Self::SIZE - 1 {
            self.0[i] |= rhs.0[i];
        }
        self.0[Self::SIZE - 1] |= rhs.0[Self::SIZE - 1] & ((1_u16 << (Self::NUM_CHANNELS % 8)) - 1) as u8;
    }
}

impl std::ops::Not for SpriteBitmask {
    type Output = Self;

    fn not(self) -> Self::Output {
        let mut bits = self;
        for i in 0..Self::SIZE {
            bits.0[i] = !bits.0[i];
        }
        bits
    }
}

impl std::ops::Sub for SpriteBitmask {
    type Output = Self;

    fn sub(self, rhs: Self) -> Self::Output {
        let mut bits = self;
        std::ops::SubAssign::sub_assign(&mut bits, rhs);
        bits
    }
}

impl std::ops::SubAssign for SpriteBitmask {
    fn sub_assign(&mut self, rhs: Self) {
        for i in 0..Self::SIZE {
            self.0[i] &= !rhs.0[i];
        }
    }
}

#[cfg(test)]
mod tests {
    #![allow(clippy::wildcard_imports)]
    use super::*;

    #[test]
    fn sprite_bitmask_default() {
        let bitmask = SpriteBitmask::default();
        for i in 0..SpriteBitmask::NUM_CHANNELS {
            assert!(!bitmask.contains(i));
        }
        assert!(bitmask.is_empty());
    }

    #[test]
    fn sprite_bitmask_empty() {
        let bitmask = SpriteBitmask::empty();
        for i in 0..SpriteBitmask::NUM_CHANNELS {
            assert!(!bitmask.contains(i));
        }
    }

    #[test]
    fn sprite_bitmask_all() {
        let bitmask = SpriteBitmask::all();
        for i in 0..SpriteBitmask::NUM_CHANNELS {
            assert!(bitmask.contains(i));
        }
        assert!(!bitmask.is_empty());
    }

    #[test]
    fn sprite_bitmask_remove() {
        let mut bitmask = SpriteBitmask::all();
        bitmask.remove(0);
        assert!(!bitmask.contains(0));
        assert!(bitmask.contains(8));
    }

    #[test]
    #[should_panic]
    fn sprite_bitmask_clear_invalid() {
        let mut bitmask = SpriteBitmask::default();
        bitmask.remove(SpriteBitmask::NUM_CHANNELS);
    }

    #[test]
    #[should_panic]
    fn sprite_bitmask_contains_invalid() {
        let bitmask = SpriteBitmask::default();
        bitmask.contains(SpriteBitmask::NUM_CHANNELS);
    }

    #[test]
    #[should_panic]
    fn sprite_bitmask_set_invalid() {
        let mut bitmask = SpriteBitmask::default();
        bitmask.set(SpriteBitmask::NUM_CHANNELS);
    }
}

impl std::fmt::Debug for SpriteBitmask {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        for channel_index in self.iter() {
            match channel_index {
                Self::PALETTE => write!(f, "Pl ")?,
                Self::SCRIPT => write!(f, "Sc ")?,
                Self::SOUND_1 => write!(f, "S1 ")?,
                Self::SOUND_2 => write!(f, "S2 ")?,
                Self::TEMPO => write!(f, "Tm ")?,
                Self::TRANSITION => write!(f, "Tx ")?,
                Self::MIN_SPRITE..=Self::MAX_SPRITE => write!(f, "{:02} ", channel_index - Self::MIN_SPRITE + 1)?,
                _ => write!(f, "X{} ", channel_index)?,
            }
        }
        Ok(())
    }
}

#[derive(Clone, Copy, Debug, SmartDefault)]
pub enum Transition {
    #[default]
    None,
    Cast(MemberId),
    Custom {
        chunk_size: u8,
        which_transition: TransitionKind,
        time: QuarterSeconds,
        change_area: bool,
    },
}

impl Transition {
    const SIZE: u32 = 4;
}

impl Resource for Transition {
    type Context = (Version, );

    fn load(input: &mut Input<impl Reader>, size: u32, context: &Self::Context) -> AResult<Self> where Self: Sized {
        let mut data = [ 0; 4 ];
        input.take(u64::from(size)).read_exact(&mut data).context("Can’t read score frame transition")?;
        Ok(if context.0 < Version::V6 {
            if data[3] == 0 {
                Self::None
            } else {
                Self::Custom {
                    chunk_size: data[1],
                    which_transition: TransitionKind::from_u8(data[3]).with_context(|| format!("Invalid transition kind {}", data[3]))?,
                    time: QuarterSeconds(data[0] & !0x80),
                    change_area: data[0] & 0x80 != 0,
                }
            }
        } else if data == [ 0; 4 ] {
            Self::None
        } else {
            Self::Cast(load_resource!("score frame transition", MemberId, &mut Input::new(Cursor::new(data), Endianness::Big))?)
        })
    }
}

#[derive(Clone, Debug, SmartDefault)]
pub struct Score {
    #[default(Self::V5_HEADER_SIZE)]
    current_frame_vwsc_position: u32,
    next_frame_vwsc_position: u32,
    #[default(Input::new(<_>::default(), Endianness::Big))]
    vwsc: Input<Cursor<Vec<u8>>>,
    score_header: Vec<u8>,
    #[default(Self::V5_HEADER_SIZE)]
    vwsc_frame_data_maybe_start_pos: u32,
    #[default(Self::V5_HEADER_SIZE)]
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
    #[default([ Point { x: -0x8000, y: 0 }; NUM_SPRITES ])]
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
    vwtk: UnkPtr,
    puppet_transition: Transition,
    #[default([ Score1494::default(); NUM_SPRITES ])]
    field_1494: [ Score1494; NUM_SPRITES ],
    #[default(Unk16(0x8000))]
    field_16d4: Unk16,
    #[default(Unk16(0x8000))]
    field_16d6: Unk16,
    #[default(Unk16(0x8000))]
    field_16d8: Unk16,
    editable_sprite: TextEditor,
    last_maybe_editable_sprite_num: i16,
    maybe_current_editable_sprite_num: i16,
    field_16f2: Unk16,
    current_frame_num: FrameNum,
    #[default(Tempo::FPS(FPS(15)))]
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

    // Not normally stored by Director, but needed for multi-version
    // compatibility
    version: Version,

    // TODO: Used for debugging only
    vwsc_own_size: u32,
}

// TODO: This is just for debugging
impl Iterator for Score {
    type Item = AResult<Frame>;

    fn next(&mut self) -> Option<Self::Item> {
        if self.vwsc.pos().unwrap() == u64::from(self.vwsc_own_size) {
            None
        } else {
            // TODO: If this call fails, it will not be possible to correctly
            // unpack future frames, since they will be decompressing deltas
            // against the wrong frame data.
            let next_frame = Self::unpack_frame(&mut self.vwsc, &self.current_frame.frame, self.puppet_sprites, self.version);
            if let Ok(next_frame) = next_frame.as_ref() {
                self.current_frame.frame = next_frame.clone();
                self.current_frame_num += FrameNum(1);
            }
            Some(next_frame)
        }
    }
}

impl Score {
    const V4_HEADER_SIZE: u32 = 20;
    const V5_HEADER_SIZE: u32 = 20;

    fn unpack_frame(input: &mut Input<impl Reader>, last_frame: &Frame, channels_to_keep: SpriteBitmask, version: Version) -> AResult<Frame> {
        let mut bytes_to_read = input.read_i16().context("Can’t read compressed score frame size")?;
        // In Director this check was >= 1 but obviously it needs to be at least
        // 2 bytes to read a chunk size
        ensure_sample!(bytes_to_read > 1, "Invalid compressed score frame size {}", bytes_to_read);
        bytes_to_read -= 2;

        let mut new_data = if last_frame.raw_data.is_empty() {
            vec![ 0; if version < Version::V5 { Frame::V0_SIZE } else { Frame::V5_SIZE } as usize ]
        } else {
            last_frame.raw_data.clone()
        };
        while bytes_to_read > 0 {
            let chunk_size = input.read_i16().context("Can’t read compressed score frame chunk size")?;
            if chunk_size < 0 {
                break;
            }
            ensure_sample!(chunk_size & 1 == 0, "Chunk size {} is not a multiple of two", chunk_size);
            let chunk_offset = input.read_i16().context("Can’t read compressed score frame chunk offset")? as usize;
            input.read_exact(&mut new_data[chunk_offset..chunk_offset + chunk_size as usize]).context("Can’t read frame chunk data")?;
            bytes_to_read -= chunk_size + 4;
        }

        let mut new_frame = Frame::new(new_data, version)?;
        for channel_index in channels_to_keep.iter() {
            match channel_index {
                SpriteBitmask::PALETTE => {
                    new_frame.palette = last_frame.palette;
                },
                SpriteBitmask::SOUND_1 => {
                    new_frame.sound_1 = last_frame.sound_1;
                },
                SpriteBitmask::SOUND_2 => {
                    new_frame.sound_2 = last_frame.sound_2;
                },
                SpriteBitmask::TEMPO => {
                    new_frame.tempo = last_frame.tempo;
                },
                SpriteBitmask::MIN_SPRITE..=SpriteBitmask::MAX_SPRITE => {
                    let sprite_index = channel_index - SpriteBitmask::NUM_NON_SPRITE_CHANNELS;
                    let sprite = &mut new_frame.sprites[sprite_index];
                    let script_id = sprite.script;
                    let flags = sprite.score_color_and_flags & !SpriteScoreColor::COLOR;
                    *sprite = last_frame.sprites[sprite_index];
                    sprite.script = script_id;
                    // TODO: This flag normally comes from the Movie global,
                    // by way of flag 0x100 in the corresponding VWFI
                    // field 0xC.
                    let todo_movie_legacy_flag = false;
                    if todo_movie_legacy_flag {
                        sprite.score_color_and_flags.remove(!SpriteScoreColor::COLOR);
                        sprite.score_color_and_flags |= flags;
                    }
                },
                _ => unreachable!("Invalid frame copy channel data")
            }
        }

        Ok(new_frame)
    }
}

impl Resource for Score {
    type Context = (crate::resources::config::Version, );

    fn load(input: &mut Input<impl Reader>, size: u32, _: &Self::Context) -> AResult<Self> where Self: Sized {
        let mut data = Vec::with_capacity(size as usize);
        input.take(u64::from(size))
            .read_to_end(&mut data)
            .context("Can’t read score data into memory")?;

        let mut input = ByteOrdered::new(Cursor::new(data), Endianness::Big);

        let own_size = input.read_u32().context("Can’t read score size")?;
        ensure_sample!(own_size <= size, "Score recorded size ({}) is larger than actual size ({})", own_size, size);

        let header_size = input.read_u32().context("Can’t read score header size")?;
        ensure_sample!(header_size == 20, "Invalid V0-V7 score header size {}", header_size);

        let num_frames = input.read_i32().context("Can’t read score frame count")?;

        let version = {
            let value = input.read_i16().context("Can’t read score version")?;
            Version::from_i16(value).with_context(|| format!("Unknown score version {}", value))?
        };

        dbg!(own_size, header_size, num_frames, version);

        if version > Version::V7 {
            todo!("Score version 8 parsing");
        } else {
            let (expect_sprite_size, expect_num_sprites) = if version < Version::V5 {
                (Sprite::V0_SIZE, Frame::V0_SIZE_IN_CELLS)
            } else {
                (Sprite::V5_SIZE, Frame::V5_SIZE_IN_CELLS)
            };

            let sprite_size = input.read_i16().context("Can’t read score sprite size")?;
            ensure_sample!(sprite_size == expect_sprite_size as i16, "Invalid sprite size {} for V5 score", sprite_size);
            // Technically this is the number of `sizeof(Sprite)`s to make one
            // `sizeof(Frame)`; the header of the frame is exactly two
            // `sizeof(Sprite)`s, even though it does not actually contain sprite
            // data
            let num_sprites = input.read_i16().context("Can’t read score sprite count")?;
            ensure_sample!(num_sprites == expect_num_sprites as i16, "Invalid sprite count {} for V5 score", num_sprites);
            let field_12 = input.read_u8().context("Can’t read score field_12")?;
            ensure_sample!(field_12 == 0 || field_12 == 1, "Unexpected score field_12 {}", field_12);
            let field_13 = input.read_u8().context("Can’t read score field_13")?;
            ensure_sample!(field_13 == 0, "Unexpected score field_13 {}", field_13);

            dbg!(sprite_size, num_sprites, field_12, field_13);

            // Director normally reads through all of the frame deltas here in order
            // to byte swap them into the platform’s native endianness, but since we
            // are using an endianness-aware reader, we’ll just let that happen
            // when the frames are read later
        }

        Ok(Score {
            vwsc: input,
            version,
            vwsc_own_size: own_size,
            ..Score::default()
        })
    }
}

bitflags! {
    #[derive(Default)]
    pub struct PaletteFlags: u8 {
        /// The palette transition will occur during the playback of the score,
        /// instead of in between frames of the score.
        const SPAN_FRAMES = 4;

        const FLAG_8 = 8;

        /// Palette cycling will ping-pong instead of looping.
        const CYCLE_AUTO_REVERSE = 0x10;

        /// Fade to black.
        const FADE_REVERSE = 0x20;

        /// Fade (to white).
        const FADE = 0x40;

        /// Do palette cycling instead of palette transitioning.
        const ACTION_CYCLE = 0x80;
    }
}

#[derive(Clone, Copy, Debug, Default)]
pub struct Palette {
    id: MemberId,
    rate: FPS,
    flags: PaletteFlags,
    cycle_start_color: i8,
    cycle_end_color: i8,
    num_frames: i16,
    num_cycles: i16,
    field_c: Unk8,
    field_d: Unk8,
    field_e: Unk8,
    field_f: Unk8,
}

impl Palette {
    const V0_SIZE: u32 = 20;
    const SIZE: u32 = 24;

    fn load_v0(input: &mut Input<impl Reader>, size: u32) -> AResult<Self> {
        ensure_sample!(size == Self::V0_SIZE, "Unexpected V0 palette size {}", size);
        let id = load_member_num!("palette transition cast member number", input.read_i16())?;
        let cycle_start_color = load_value!("palette cycle start color", input.read_i8())?;
        let cycle_end_color = load_value!("palette cycle end color", input.read_i8())?;
        let flags = load_flags!("palette flags", PaletteFlags, input.read_u8())?;
        let rate = FPS(i16::from(load_value!("palette rate", input.read_i8())?));
        let num_frames = load_value!("palette num frames", input.read_i16())?;
        let num_cycles = load_value!("palette cycles", input.read_i16())?;
        let field_c = Unk8(load_value!("palette field_c", input.read_u8())?);
        let field_d = Unk8(load_value!("palette field_d", input.read_u8())?);
        let field_e = Unk8(load_value!("palette field_e", input.read_u8())?);
        input.skip(5).context("Can’t skip unused palette fields")?;
        let field_f = Unk8(load_value!("palette field_f", input.read_u8())?);
        if size > 19 {
            input.skip(u64::from(size - 19)).context("Can’t skip end of frame palette")?;
        }
        Ok(Self {
            id,
            rate,
            flags,
            cycle_start_color,
            cycle_end_color,
            num_frames,
            num_cycles,
            field_c,
            field_d,
            field_e,
            field_f,
        })
    }

    fn load_v5(input: &mut Input<impl Reader>, size: u32) -> AResult<Self> {
        let id = load_resource!("palette transition ID", MemberId, input)?;
        let rate = FPS(i16::from(load_value!("palette rate", input.read_i8())?));
        let flags = load_flags!("palette flags", PaletteFlags, input.read_u8())?;
        let cycle_start_color = load_value!("palette cycle start color", input.read_i8())?;
        let cycle_end_color = load_value!("palette cycle end color", input.read_i8())?;
        let num_frames = load_value!("palette num frames", input.read_i16())?;
        let num_cycles = load_value!("palette cycles", input.read_i16())?;
        let field_c = Unk8(load_value!("palette field_c", input.read_u8())?);
        let field_d = Unk8(load_value!("palette field_d", input.read_u8())?);
        let field_e = Unk8(load_value!("palette field_e", input.read_u8())?);
        let field_f = Unk8(load_value!("palette field_f", input.read_u8())?);
        if size > 16 {
            input.skip(u64::from(size - 16)).context("Can’t skip end of frame palette")?;
        }
        Ok(Self {
            id,
            rate,
            flags,
            cycle_start_color,
            cycle_end_color,
            num_frames,
            num_cycles,
            field_c,
            field_d,
            field_e,
            field_f,
        })
    }
}

impl Resource for Palette {
    type Context = (Version, );

    fn load(input: &mut Input<impl Reader>, size: u32, context: &Self::Context) -> AResult<Self> where Self: Sized {
        if context.0 > Version::V7 {
            todo!("Score palette version 8 parsing")
        } else if context.0 >= Version::V5 {
            Self::load_v5(input, size)
        } else {
            Self::load_v0(input, size)
        }
    }
}

#[derive(Clone, Copy, Debug, Default)]
struct Score1494 {
    data: UnkPtr,
    id: MemberId,
    field_8: Unk8,
    field_9: Unk8,
    flags: u8,
    cast_member_kind: MemberKind,
}

#[derive(Clone, Copy, Debug, Default)]
struct TextEditor {
    te: TEHandle,
    rect: Rect,
    sprite_num: ChannelNum,
    id: MemberId,
    is_editing: bool,
}

#[derive(Clone, Debug, SmartDefault)]
struct SpriteFrame {
    frame: Frame,
    #[default([ Rect::default(); NUM_SPRITES ])]
    rects: [ Rect; NUM_SPRITES ],
}

#[derive(Clone, Debug, SmartDefault)]
pub struct Frame {
    pub script: MemberId,
    pub sound_1: MemberId,
    pub sound_2: MemberId,
    pub transition: Transition,
    pub tempo_related: Unk8,
    pub sound_1_related: Unk8,
    pub sound_2_related: Unk8,
    pub script_related: Unk8,
    pub transition_related: Unk8,
    pub tempo: Tempo,
    pub palette: Palette,
    #[default([ Sprite::default(); NUM_SPRITES ])]
    pub sprites: [ Sprite; NUM_SPRITES ],

    // TODO: There is probably a better way to handle this. Director’s
    // serialised data formats were just dumps of memory to disk. This is
    // generally not a problem, but the frame data is delta-compressed in
    // unaligned 16-bit chunks, so in order to reconstruct the next frame, it is
    // necessary to have an original raw data representation of the previous
    // frame that the binary diff can be applied to.
    raw_data: Vec<u8>,
}

impl Frame {
    const V4_CELL_COUNT: u32 = 48;
    const V0_SIZE_IN_CELLS: u32 = 50;
    const V0_SIZE: u32 = Sprite::V0_SIZE * Self::V0_SIZE_IN_CELLS;
    const V5_CELL_COUNT: u32 = 48;
    const V5_SIZE_IN_CELLS: u32 = 50;
    const V5_SIZE: u32 = Sprite::V5_SIZE * Self::V5_SIZE_IN_CELLS;

    fn new(data: Vec<u8>, version: Version) -> AResult<Self> {
        let input = Input::new(Cursor::new(data), Endianness::Big);
        if version > Version::V7 {
            todo!("Score frame version 8 parsing")
        } else if version >= Version::V5 {
            Self::new_v5(input, version)
        } else {
            Self::new_v0(input, version)
        }
    }

    fn new_v0(mut input: Input<Cursor<Vec<u8>>>, version: Version) -> AResult<Self> {
        let unknown = load_value!("frame v0 unknown", input.read_i16())?;
        let (transition, tempo) = Self::load_transition(&mut input, version)?;
        let sound_1 = load_member_num!("frame sound 1 cast member number", input.read_i16())?;
        let sound_2 = load_member_num!("frame sound 2 cast member number", input.read_i16())?;
        let field_a = load_value!("frame field_a", input.read_u8())?;
        let field_b = load_value!("frame field_b", input.read_u8())?;
        let field_c = load_value!("frame field_c", input.read_u8())?;
        let tempo_related = Unk8(load_value!("frame tempo_related", input.read_u8())?);
        let sound_1_related = Unk8(load_value!("frame sound_1_related", input.read_u8())?);
        let sound_2_related = Unk8(load_value!("frame sound_2_related", input.read_u8())?);
        let script = load_member_num!("frame script cast member number", input.read_i16())?;
        let script_related = Unk8(load_value!("frame script_related", input.read_u8())?);
        let transition_related = Unk8(load_value!("frame transition_related", input.read_u8())?);
        let palette = load_resource!("frame palette", Palette, &mut input, Palette::V0_SIZE, (version, ))?;

        let mut sprites = [ Sprite::default(); NUM_SPRITES ];
        for (i, sprite) in sprites.iter_mut().enumerate().take(Self::V4_CELL_COUNT as usize) {
            *sprite = load_resource!(format!("frame sprite {}", i + 1), Sprite, &mut input, Sprite::V0_SIZE, (version, ))?;
        }

        Ok(Frame {
            script,
            sound_1,
            sound_2,
            transition,
            tempo_related,
            sound_1_related,
            sound_2_related,
            script_related,
            transition_related,
            tempo,
            palette,
            sprites,
            raw_data: input.into_inner().into_inner(),
        })
    }

    fn new_v5(mut input: Input<Cursor<Vec<u8>>>, version: Version) -> AResult<Self> {
        let script = load_resource!("frame script cast member ID", MemberId, &mut input)?;
        let sound_1 = load_resource!("frame sound 1 cast member ID", MemberId, &mut input)?;
        let sound_2 = load_resource!("frame sound 2 cast member ID", MemberId, &mut input)?;
        let (transition, v0_tempo) = Self::load_transition(&mut input, version)?;
        let tempo_related = Unk8(load_value!("frame tempo related", input.read_u8())?);
        let sound_1_related = Unk8(load_value!("frame sound 1 related", input.read_u8())?);
        let sound_2_related = Unk8(load_value!("frame sound 2 related", input.read_u8())?);
        let script_related = Unk8(load_value!("frame script related", input.read_u8())?);
        let transition_related = Unk8(load_value!("frame transition related", input.read_u8())?);
        let tempo = if version < Version::V6 {
            input.skip(1).context("Can’t skip frame tempo")?;
            v0_tempo
        } else {
            Tempo::new(i16::from(load_value!("frame tempo", input.read_i8())?))?
        };
        input.skip(2).context("Can’t skip after frame tempo")?;
        let palette = load_resource!("frame palette", Palette, &mut input, (version, ))?;

        let mut sprites = [ Sprite::default(); NUM_SPRITES ];
        for (i, sprite) in sprites.iter_mut().enumerate().take(Self::V5_CELL_COUNT as usize) {
            *sprite = load_resource!(format!("frame sprite {}", i + 1), Sprite, &mut input, Sprite::V5_SIZE, (version, ))?;
        }

        Ok(Frame {
            script,
            sound_1,
            sound_2,
            transition,
            tempo_related,
            sound_1_related,
            sound_2_related,
            script_related,
            transition_related,
            tempo,
            palette,
            sprites,
            raw_data: input.into_inner().into_inner(),
        })
    }

    fn load_transition(input: &mut Input<impl Reader>, version: Version) -> AResult<(Transition, Tempo)> {
        let mut data = [ 0; 4 ];
        input.read_exact(&mut data).context("Can’t read frame transition data")?;
        let transition = load_resource!("frame transition", Transition, &mut Input::new(Cursor::new(&data), Endianness::Big), (version, ))?;
        Ok((transition, Tempo::new(i16::from(if version < Version::V7 { data[3] } else { 0 }))?))
    }
}

#[derive(Clone, Copy, Debug, Display, Eq, FromPrimitive, Ord, PartialEq, PartialOrd, SmartDefault)]
pub enum Version {
    #[default]
    Unknown,
    V3 = 3,
    V4,
    V5,
    V6,
    V7,
}

#[derive(Copy, Clone, Debug, Eq, FromPrimitive, PartialEq, SmartDefault)]
pub enum SpriteKind {
    #[default]
    None = 0,
    Bitmap,
    Rect,
    RoundRect,
    Oval,
    LineTLToBR,
    LineBLToTR,
    Field,
    Button,
    CheckBox,
    RadioButton,
    Picture,
    RectOutline,
    RoundRectOutline,
    OvalOutline,
    LineMaybe,
    Cast,
    Text,
    Script,
}

bitflags! {
    #[derive(Default)]
    pub struct SpriteLineSize: u8 {
        const LINE_SIZE = 0xf;
        const BLEND     = 0x10;
        const FLAG_20   = 0x20;
        const FLAG_40   = 0x40;
        // TODO: What is this? Exists in data converted from D4 to D5
        const FLAG_80   = 0x80;
    }
}

bitflags! {
    #[derive(Default)]
    pub struct SpriteInk: u8 {
        const INK_KIND = 0x3f;
        const TRAILS   = 0x40;
        const STRETCH  = 0x80;
    }
}

bitflags! {
    #[derive(Default)]
    pub struct SpriteScoreColor: u8 {
        const COLOR    = 0xf;
        const FLAG_10  = 0x10;
        const FLAG_20  = 0x20;
        const EDITABLE = 0x40;
        const MOVEABLE = 0x80;
    }
}

#[derive(Clone, Copy, Default)]
pub struct Sprite {
    kind: SpriteKind,
    ink_and_flags: SpriteInk,
    id: MemberId,
    script: MemberId,
    fore_color_index: u8,
    back_color_index: u8,
    origin: Point,
    height: i16,
    width: i16,
    score_color_and_flags: SpriteScoreColor,
    blend_amount: u8,
    line_size_and_flags: SpriteLineSize,
}

impl Sprite {
    const V0_SIZE: u32 = 20;
    const V5_SIZE: u32 = 24;

    #[must_use]
    pub fn back_color_index(&self) -> u8 {
        self.back_color_index
    }

    #[must_use]
    pub fn blend(&self) -> bool {
        self.line_size_and_flags.contains(SpriteLineSize::BLEND)
    }

    #[must_use]
    pub fn blend_amount(&self) -> u8 {
        self.blend_amount
    }

    #[must_use]
    pub fn editable(&self) -> bool {
        self.score_color_and_flags.contains(SpriteScoreColor::EDITABLE)
    }

    #[must_use]
    pub fn fore_color_index(&self) -> u8 {
        self.fore_color_index
    }

    #[must_use]
    pub fn height(&self) -> i16 {
        self.height
    }

    #[must_use]
    pub fn id(&self) -> MemberId {
        self.id
    }

    #[must_use]
    pub fn ink(&self) -> Pen {
        Pen::from_u8((self.ink_and_flags & SpriteInk::INK_KIND).bits()).unwrap()
    }

    #[must_use]
    pub fn kind(&self) -> SpriteKind {
        self.kind
    }

    #[must_use]
    pub fn line_size(&self) -> u8 {
        (self.line_size_and_flags & SpriteLineSize::LINE_SIZE).bits()
    }

    #[must_use]
    pub fn moveable(&self) -> bool {
        self.score_color_and_flags.contains(SpriteScoreColor::MOVEABLE)
    }

    #[must_use]
    pub fn origin(&self) -> Point {
        self.origin
    }

    #[must_use]
    pub fn score_color(&self) -> u8 {
        (self.score_color_and_flags & SpriteScoreColor::COLOR).bits()
    }

    #[must_use]
    pub fn script(&self) -> MemberId {
        self.script
    }

    #[must_use]
    pub fn stretch(&self) -> bool {
        self.ink_and_flags.contains(SpriteInk::STRETCH)
    }

    #[must_use]
    pub fn trails(&self) -> bool {
        self.ink_and_flags.contains(SpriteInk::TRAILS)
    }

    #[must_use]
    pub fn width(&self) -> i16 {
        self.width
    }

    fn load_ink_and_flags(input: &mut Input<impl Reader>) -> AResult<SpriteInk> {
        load_flags!("sprite ink & flags", SpriteInk, input.read_u8()).and_then(|ink_and_flags| {
            let ink = (ink_and_flags & SpriteInk::INK_KIND).bits();
            Pen::from_u8(ink).with_context(|| format!("Invalid sprite ink {}", ink))?;
            Ok(ink_and_flags)
        })
    }

    fn load_kind(input: &mut Input<impl Reader>, version: Version) -> AResult<SpriteKind> {
        load_enum!("sprite kind", SpriteKind::from_u8, input.read_u8()).map(|kind| {
            if version == Version::V7 {
                kind
            } else {
                match kind {
                    SpriteKind::Bitmap
                    | SpriteKind::Field
                    | SpriteKind::Button
                    | SpriteKind::CheckBox
                    | SpriteKind::RadioButton
                    | SpriteKind::Picture
                    | SpriteKind::Cast
                    | SpriteKind::Text => SpriteKind::Cast,
                    kind => kind
                }
            }
        })
    }

    fn load_v0(input: &mut Input<impl Reader>, version: Version) -> AResult<Self> {
        let unknown = load_value!("sprite field 0", input.read_u8())?;
        let kind = Self::load_kind(input, version)?;
        let fore_color_index = load_value!("sprite fore color", input.read_u8())?;
        let back_color_index = load_value!("sprite back color", input.read_u8())?;
        let line_size_and_flags = load_flags!("sprite line size & flags", SpriteLineSize, input.read_u8())?;
        let ink_and_flags = Self::load_ink_and_flags(input)?;
        let id = load_member_num!("sprite cast member number", input.read_i16())?;
        let origin = load_resource!("sprite registration point", Point, input)?;
        let height = load_value!("sprite height", input.read_i16())?;
        let width = load_value!("sprite width", input.read_i16())?;

        // TODO: The rest of this may be wrong for D3.
        let script = load_member_num!("sprite script cast member number", input.read_i16())?;
        let score_color_and_flags = load_flags!("sprite score color & flags", SpriteScoreColor, input.read_u8())?;
        let blend_amount = load_value!("sprite blend amount", input.read_u8())?;

        Ok(Self {
            kind,
            ink_and_flags,
            id,
            script,
            fore_color_index,
            back_color_index,
            origin,
            height,
            width,
            score_color_and_flags,
            blend_amount,
            line_size_and_flags,
        })
    }

    fn load_v5(input: &mut Input<impl Reader>, version: Version) -> AResult<Self> {
        let kind = Self::load_kind(input, version)?;
        let ink_and_flags = Self::load_ink_and_flags(input)?;
        let id = load_resource!("sprite cast ID", MemberId, input)?;
        let script = load_resource!("sprite script ID", MemberId, input)?;
        let fore_color_index = load_value!("sprite fore color", input.read_u8())?;
        let back_color_index = load_value!("sprite back color", input.read_u8())?;
        let origin = load_resource!("sprite registration point", Point, input)?;
        let height = load_value!("sprite height", input.read_i16())?;
        let width = load_value!("sprite width", input.read_i16())?;
        let score_color_and_flags = load_flags!("sprite score color & flags", SpriteScoreColor, input.read_u8())?;
        let blend_amount = load_value!("sprite blend amount", input.read_u8())?;
        let line_size_and_flags = load_flags!("sprite line size & flags", SpriteLineSize, input.read_u8())?;
        input.skip(1).context("Can’t skip sprite frame padding")?;

        Ok(Self {
            kind,
            ink_and_flags,
            id,
            script,
            fore_color_index,
            back_color_index,
            origin,
            height,
            width,
            score_color_and_flags,
            blend_amount,
            line_size_and_flags,
        })
    }
}

impl Resource for Sprite {
    type Context = (Version, );

    fn load(input: &mut Input<impl Reader>, _: u32, context: &Self::Context) -> AResult<Self> where Self: Sized {
        if context.0 < Version::V5 {
            Self::load_v0(input, context.0)
        } else if context.0 <= Version::V7 {
            Self::load_v5(input, context.0)
        } else {
            bail!("Invalid frame cell version {}", context.0)
        }
    }
}

impl std::fmt::Debug for Sprite {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("FrameCell")
            .field("back_color_index", &self.back_color_index())
            .field("blend", &self.blend())
            .field("blend_amount", &self.blend_amount())
            .field("editable", &self.editable())
            .field("fore_color_index", &self.fore_color_index())
            .field("height", &self.height())
            .field("id", &self.id())
            .field("ink", &self.ink())
            .field("kind", &self.kind())
            .field("line_size", &self.line_size())
            .field("moveable", &self.moveable())
            .field("origin", &self.origin())
            .field("score_color", &self.score_color())
            .field("script", &self.script())
            .field("stretch", &self.stretch())
            .field("trails", &self.trails())
            .field("width", &self.width())
            .finish()
    }
}
