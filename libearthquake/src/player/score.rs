// TODO: You know, finish this file and then remove these overrides
#![allow(clippy::struct_excessive_bools)]
#![allow(dead_code)]

use anyhow::{bail, Context, Result as AResult};
use bitflags::bitflags;
use byteorder::{BigEndian, ByteOrder};
use byteordered::{ByteOrdered, Endianness};
use crate::{ensure_sample, resources::cast::{MemberId, MemberKind}};
use derive_more::{Add, AddAssign};
use num_derive::FromPrimitive;
use num_traits::FromPrimitive;
use libcommon::{Reader, Resource, Unk16, Unk32, Unk8, UnkPtr, resource::Input};
use libmactoolbox::{Point, Rect, TEHandle};
use smart_default::SmartDefault;
use std::{io::{Cursor, Read}, iter::Rev};

macro_rules! load_enum {
    ($name: expr, $kind: expr, $input: expr) => (
        $input
        .context(stringify!("Can’t read", $name))
        .and_then(|value| {
            $kind(value)
                .with_context(|| format!(stringify!("Invalid value 0x{:x} for ", $name), value))
        })
    )
}

macro_rules! load_flags {
    ($name: expr, $kind: ty, $input: expr) => (
        $input
        .context(stringify!("Can’t read", $name))
        .and_then(|value| {
            <$kind>::from_bits(value)
                .with_context(|| format!(stringify!("Invalid value 0x{:x} for ", $name), value))
        })
    )
}

macro_rules! load_resource {
    ($name: literal, $kind: ty, $input: expr, $size: expr, $context: expr) => (<$kind>::load($input, $size, &$context).context(stringify!("Can’t read ", $name)));
    ($name: literal, $kind: ty, $input: expr, $context: expr) => (<$kind>::load($input, <$kind>::SIZE, &$context).context(stringify!("Can’t read ", $name)));
    ($name: literal, $kind: ty, $input: expr) => (<$kind>::load($input, <$kind>::SIZE, &()).context(stringify!("Can’t read", $name)));
    ($name: expr, $kind: ty, $input: expr, $size: expr, $context: expr) => (<$kind>::load($input, $size, &$context).with_context(|| format!("Can’t read {}", $name)));
    ($name: expr, $kind: ty, $input: expr, $context: expr) => (<$kind>::load($input, <$kind>::SIZE, &$context).with_context(|| format!("Can’t read {}", $name)));
    ($name: expr, $kind: ty, $input: expr) => (<$kind>::load($input, <$kind>::SIZE, &()).with_context(|| format!("Can’t read {}", $name)));
}

macro_rules! load_value {
    ($name: literal, $input: expr) => ($input.context(stringify!("Can’t read ", $name)));
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
    Cast(MemberId),
    Custom { chunk_size: u8, which_transition: u8, time: u8, change_area: bool },
    Todo([ u8; 4 ]),
}

impl Transition {
    const SIZE: u32 = 4;
}

impl Resource for Transition {
    type Context = (Version, );

    fn load(input: &mut Input<impl Reader>, size: u32, context: &Self::Context) -> AResult<Self> where Self: Sized {
        let mut data = [ 0; 4 ];
        input.take(u64::from(size)).read_exact(&mut data).context("Can’t read frame transition")?;
        Ok(Self::Todo(data))
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
    const V5_HEADER_SIZE: u32 = 20;

    fn parse_v5(input: &mut Input<impl Reader>) -> AResult<()> {
        let sprite_size = input.read_i16().context("Can’t read score sprite size")?;
        ensure_sample!(sprite_size == FrameCell::V5_SIZE as i16, "Invalid sprite size {} for V5 score", sprite_size);
        // Technically this is the number of `sizeof(Sprite)`s to make one
        // `sizeof(Frame)`; the header of the frame is exactly two
        // `sizeof(Sprite)`s, even though it does not actually contain sprite
        // data
        let num_sprites = input.read_i16().context("Can’t read score sprite count")?;
        ensure_sample!(num_sprites == Frame::V5_SIZE_IN_CELLS as i16, "Invalid sprite count {} for V5 score", num_sprites);
        let field_12 = input.read_u8().context("Can’t read score field_12")?;
        ensure_sample!(field_12 == 0, "Unexpected score field_12 {}", field_12);
        let field_13 = input.read_u8().context("Can’t read score field_13")?;
        ensure_sample!(field_13 == 0, "Unexpected score field_13 {}", field_13);

        // Director normally reads through all of the frame deltas here in order
        // to byte swap them into the platform’s native endianness, but since we
        // are using an endianness-aware reader, we’ll just let that happen
        // when the frames are read later

        Ok(())
    }

    fn unpack_frame(input: &mut Input<impl Reader>, last_frame: &Frame, channels_to_keep: SpriteBitmask, version: Version) -> AResult<Frame> {
        let mut bytes_to_read = input.read_i16().context("Can’t read compressed score frame size")?;
        // In Director this check was >= 1 but obviously it needs to be at least
        // 2 bytes to read a chunk size
        ensure_sample!(bytes_to_read > 1, "Invalid compressed score frame size {}", bytes_to_read);
        bytes_to_read -= 2;

        let mut new_data = if last_frame.raw_data.is_empty() {
            vec![ 0; Frame::V5_SIZE as usize ]
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
    type Context = ();

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
        let field_8 = input.read_i32().context("Can’t read score field_8")?;
        ensure_sample!(field_8 == 0, "Unexpected score field_8 value (0x{:x})", field_8);
        let version = {
            let value = input.read_i16().context("Can’t read score version")?;
            Version::from_i16(value).with_context(|| format!("Unknown score version {}", value))?
        };
        dbg!(own_size, header_size, field_8, version);
        if version < Version::V5 {
            todo!("Score version 4 parsing");
        } else if version > Version::V7 {
            todo!("Score version 8 parsing");
        } else {
            Score::parse_v5(&mut input)?;
        }

        Ok(Score {
            vwsc: input,
            version,
            vwsc_own_size: own_size,
            ..Score::default()
        })
    }
}

#[derive(Clone, Copy, Debug, Default)]
pub struct Palette {
    id: MemberId,
    rate: FPS,
    flags: Unk8,
    cycle_start_color: i8,
    cycle_end_color: i8,
    num_frames: i16,
    num_cycles: i16,
    field_c: Unk8,
    field_d: Unk8,
    field_e: Unk8,
}

impl Palette {
    const SIZE: u32 = 24;
}

impl Resource for Palette {
    type Context = (Version, );

    fn load(input: &mut Input<impl Reader>, size: u32, context: &Self::Context) -> AResult<Self> where Self: Sized {
        let id = load_resource!("transition ID", MemberId, input)?;
        let rate = FPS(i16::from(load_value!("rate", input.read_i8())?));
        let flags = Unk8(load_value!("flags", input.read_u8())?);
        let cycle_start_color = load_value!("cycle start color", input.read_i8())?;
        let cycle_end_color = load_value!("cycle end color", input.read_i8())?;
        let num_frames = load_value!("num_frames", input.read_i16())?;
        let num_cycles = load_value!("cycles", input.read_i16())?;
        let field_c = Unk8(load_value!("field_c", input.read_u8())?);
        let field_d = Unk8(load_value!("field_d", input.read_u8())?);
        let field_e = Unk8(load_value!("field_e", input.read_u8())?);
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
        })
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
    #[default([ FrameCell::default(); NUM_SPRITES ])]
    pub sprites: [ FrameCell; NUM_SPRITES ],

    // TODO: There is probably a better way to handle this. Director’s
    // serialised data formats were just dumps of memory to disk. This is
    // generally not a problem, but the frame data is delta-compressed in
    // unaligned 16-bit chunks, so in order to reconstruct the next frame, it is
    // necessary to have an original raw data representation of the previous
    // frame that the binary diff can be applied to.
    raw_data: Vec<u8>,
}

impl Frame {
    const V5_CELL_COUNT: u32 = 48;
    const V5_SIZE_IN_CELLS: u32 = 50;
    const V5_SIZE: u32 = FrameCell::V5_SIZE * Self::V5_SIZE_IN_CELLS;

    fn new(data: Vec<u8>, version: Version) -> AResult<Self> {
        if version < Version::V5 {
            todo!()
        } else {
            Self::new_v5(data, version)
        }
    }

    fn new_v5(data: Vec<u8>, version: Version) -> AResult<Self> {
        let mut input = Input::new(Cursor::new(data), Endianness::Big);
        let script = load_resource!("frame script cast member ID", MemberId, &mut input)?;
        let sound_1 = load_resource!("frame sound 1 cast member ID", MemberId, &mut input)?;
        let sound_2 = load_resource!("frame sound 2 cast member ID", MemberId, &mut input)?;
        let transition = load_resource!("frame transition", Transition, &mut input, (version, ))?;
        let tempo_related = Unk8(load_value!("frame tempo related", input.read_u8())?);
        let sound_1_related = Unk8(load_value!("frame sound 1 related", input.read_u8())?);
        let sound_2_related = Unk8(load_value!("frame sound 2 related", input.read_u8())?);
        let script_related = Unk8(load_value!("frame script related", input.read_u8())?);
        let transition_related = Unk8(load_value!("frame transition related", input.read_u8())?);
        let tempo = Tempo::new(i16::from(load_value!("frame tempo", input.read_i8())?))?;
        input.skip(2).context("Can’t skip after frame tempo")?;
        let palette = load_resource!("frame palette", Palette, &mut input, (version, ))?;

        let mut sprites = [ FrameCell::default(); NUM_SPRITES ];
        for (i, sprite) in sprites.iter_mut().enumerate().take(Self::V5_CELL_COUNT as usize) {
            *sprite = load_resource!(format!("frame sprite {}", i + 1), FrameCell, &mut input, FrameCell::V5_SIZE, (version, ))?;
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
}

#[derive(Clone, Copy, Debug, Eq, FromPrimitive, Ord, PartialEq, PartialOrd, SmartDefault)]
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
    SoundMaybe = 23,
}

bitflags! {
    #[derive(Default)]
    pub struct SpriteLineSize: u8 {
        const LINE_SIZE = 0xf;
        const BLEND     = 0x10;
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
        const EDITABLE = 0x40;
        const MOVEABLE = 0x80;
    }
}

#[derive(Clone, Copy, Default)]
pub struct FrameCell {
    pub kind: SpriteKind,
    pub ink_and_flags: SpriteInk,
    pub id: MemberId,
    pub script: MemberId,
    pub fore_color_index: u8,
    pub back_color_index: u8,
    pub origin: Point,
    pub height: i16,
    pub width: i16,
    pub score_color_and_flags: SpriteScoreColor,
    pub blend_amount: u8,
    pub line_size_and_flags: SpriteLineSize,
}

impl FrameCell {
    const V5_SIZE: u32 = 24;
}

impl Resource for FrameCell {
    type Context = (Version, );

    fn load(input: &mut Input<impl Reader>, _: u32, context: &Self::Context) -> AResult<Self> where Self: Sized {
        if context.0 < Version::V5 {
            todo!("frame cell V0–V4");
        }

        let kind = load_enum!("sprite kind", SpriteKind::from_u8, input.read_u8())?;
        let ink_and_flags = load_flags!("sprite ink", SpriteInk, input.read_u8())?;
        let id = load_resource!("sprite cast ID", MemberId, input)?;
        let script = load_resource!("sprite script ID", MemberId, input)?;
        let fore_color_index = load_value!("sprite fore color", input.read_u8())?;
        let back_color_index = load_value!("sprite back color", input.read_u8())?;
        let origin = load_resource!("sprite registration point", Point, input)?;
        let height = load_value!("sprite height", input.read_i16())?;
        let width = load_value!("sprite width", input.read_i16())?;
        let score_color_and_flags = load_flags!("sprite score color", SpriteScoreColor, input.read_u8())?;
        let blend_amount = load_value!("sprite blend amount", input.read_u8())?;
        let line_size_and_flags = load_flags!("sprite line size", SpriteLineSize, input.read_u8())?;

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

impl std::fmt::Debug for FrameCell {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("FrameCell")
            .field("kind", &self.kind)
            .field("ink", &(self.ink_and_flags.bits() & SpriteInk::INK_KIND.bits()))
            .field("ink_flags", &self.ink_and_flags.clone().remove(SpriteInk::INK_KIND))
            .field("id", &self.id)
            .field("script", &self.script)
            .field("fore_color_index", &self.fore_color_index)
            .field("back_color_index", &self.back_color_index)
            .field("origin", &self.origin)
            .field("height", &self.height)
            .field("width", &self.width)
            .field("score_color", &(self.score_color_and_flags.bits() & SpriteScoreColor::COLOR.bits()))
            .field("score_color_flags", &self.score_color_and_flags.clone().remove(SpriteScoreColor::COLOR))
            .field("blend_amount", &self.blend_amount)
            .field("line_size", &(self.line_size_and_flags.bits() & SpriteLineSize::LINE_SIZE.bits()))
            .field("line_size_flags", &self.line_size_and_flags.clone().remove(SpriteLineSize::LINE_SIZE))
            .finish()
    }
}
