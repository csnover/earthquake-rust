// TODO: You know, finish this file and then remove these overrides
#![allow(clippy::struct_excessive_bools)]
#![allow(dead_code)]

use anyhow::{anyhow, bail, Context, Result as AResult};
use binread::{BinRead, ReadOptions};
use bitflags::bitflags;
use byteordered::{ByteOrdered, Endianness};
use crate::{ensure_sample, resources::{transition::{Kind as TransitionKind, QuarterSeconds}, cast::{MemberId, MemberKind}}};
use derive_more::{Add, AddAssign, Display};
use num_derive::FromPrimitive;
use num_traits::FromPrimitive;
use libcommon::{Reader, Resource, Unk16, Unk32, Unk8, UnkPtr, binread_enum, binread_flags, resource::Input};
use libmactoolbox::{quickdraw::Pen, Point, Rect, TEHandle};
use smart_default::SmartDefault;
use std::{convert::{TryFrom, TryInto}, io::{Cursor, Read}, io::SeekFrom, iter::Rev, io::Seek};

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

binread_flags!(Flags, u16);

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
        bits[Self::SIZE - 1] &= u8::try_from((1_u16 << (Self::NUM_CHANNELS % 8)) - 1).unwrap();
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
        self.0[Self::SIZE - 1] |= rhs.0[Self::SIZE - 1] & u8::try_from((1_u16 << (Self::NUM_CHANNELS % 8)) - 1).unwrap();
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
    LegacyTempo(Tempo),
    Legacy {
        chunk_size: u8,
        which_transition: TransitionKind,
        time: QuarterSeconds,
        change_area: bool,
        tempo: Tempo,
    },
}

impl Transition {
    fn tempo(&self) -> Tempo {
        match self {
            Self::Legacy { tempo, .. } | Self::LegacyTempo(tempo) => *tempo,
            Self::None | Self::Cast(..) => Tempo::default(),
        }
    }
}

impl BinRead for Transition {
    type Args = (Version, );

    fn read_options<R: binread::io::Read + binread::io::Seek>(reader: &mut R, _: &ReadOptions, args: Self::Args) -> binread::BinResult<Self> {
        let last_pos = reader.seek(SeekFrom::Current(0))?;

        let make_tempo = |tempo: u8| {
            Tempo::new((tempo as i8).into()).map_err(|e| binread::Error::AssertFail {
                pos: last_pos.try_into().unwrap(),
                message: format!("{}", e),
            })
        };

        let mut data = [ 0; 4 ];
        reader.read_exact(&mut data)?;
        Ok(if args.0 < Version::V6 {
            if data[3] == 0 {
                if data[2] == 0 {
                    Self::None
                } else {
                    Self::LegacyTempo(make_tempo(data[2])?)
                }
            } else {
                Self::Legacy {
                    chunk_size: data[1],
                    which_transition: TransitionKind::from_u8(data[3])
                        .ok_or_else(|| binread::Error::AssertFail {
                            pos: last_pos.try_into().unwrap(),
                            message: format!("Invalid transition kind {}", data[3]),
                        })?,
                    time: QuarterSeconds(data[0] & !0x80),
                    change_area: data[0] & 0x80 != 0,
                    tempo: make_tempo(data[2])?
                }
            }
        } else if data == [ 0; 4 ] {
            Self::None
        } else {
            let mut options = ReadOptions::default();
            options.endian = binread::Endian::Big;
            Self::Cast(MemberId::read_options(&mut Cursor::new(data), &options, ())?)
        })
    }
}

#[derive(Clone, Debug, SmartDefault)]
struct ScoreStream {
    #[default(Input::new(<_>::default(), Endianness::Big))]
    input: Input<Cursor<Vec<u8>>>,
    data_start_pos: u32,
    data_end_pos: u32,
    version: Version,
    last_frame: Frame,
    #[default([ 0; Frame::V5_SIZE as usize ])]
    raw_last_frame: [ u8; Frame::V5_SIZE as usize ],
}

impl ScoreStream {
    fn new(mut input: Input<Cursor<Vec<u8>>>, data_start_pos: u32, data_end_pos: u32, version: Version) -> Self {
        input.seek(SeekFrom::Start(data_start_pos.into())).unwrap();
        Self {
            input,
            data_start_pos,
            data_end_pos,
            version,
            last_frame: Frame::default(),
            raw_last_frame: [ 0; Frame::V5_SIZE as usize ],
        }
    }

    fn next(&mut self, channels_to_keep: SpriteBitmask) -> AResult<Option<Frame>> {
        if self.input.pos()? >= self.data_end_pos.into() {
            return Ok(None);
        }

        let mut bytes_to_read = self.input.read_i16().context("Can’t read compressed score frame size")?;

        if self.version < Version::V4 {
            bytes_to_read = std::cmp::max(0, bytes_to_read - 2);
        } else {
            // In D5 this check was >= 1 but obviously it needs to be at least 2
            // bytes to read a chunk size
            ensure_sample!(bytes_to_read > 1, "Invalid compressed score frame size {}", bytes_to_read);
            bytes_to_read -= 2;
        }

        let mut new_data = self.raw_last_frame;

        while bytes_to_read > 0 {
            let (chunk_size, chunk_offset) = if self.version < Version::V4 {
                let chunk_size = i16::from(self.input.read_u8().context("Can’t read compressed score frame chunk size")?) * 2;
                let chunk_offset = usize::from(self.input.read_u8().context("Can’t read compressed score frame chunk offset")?) * 2;
                bytes_to_read -= chunk_size + 2;
                (chunk_size, chunk_offset)
            } else {
                let chunk_size = self.input.read_i16().context("Can’t read compressed score frame chunk size")?;
                if chunk_size < 0 {
                    break;
                }
                ensure_sample!(chunk_size & 1 == 0, "Chunk size {} is not a multiple of two", chunk_size);
                let chunk_offset = usize::try_from(self.input.read_i16().context("Can’t read compressed score frame chunk offset")?).unwrap();
                bytes_to_read -= chunk_size + 4;
                (chunk_size, chunk_offset)
            };

            self.input.read_exact(&mut new_data[chunk_offset..chunk_offset + usize::try_from(chunk_size).unwrap()]).context("Can’t read frame chunk data")?;
        }

        let cursor = &mut Cursor::new(&new_data);
        let args = (self.version, );
        let mut new_frame = match self.version {
            Version::V3 => FrameV3::read_args(cursor, args).map(Frame::from),
            Version::V4 => FrameV4::read_args(cursor, args).map(Frame::from),
            Version::V5 | Version::V6 | Version::V7 => Frame::read_args(cursor, args),
            Version::Unknown => bail!("Unknown score version"),
        }.context("Can’t read frame")?;

        for channel_index in channels_to_keep.iter() {
            match channel_index {
                SpriteBitmask::PALETTE => {
                    new_frame.palette = self.last_frame.palette;
                },
                SpriteBitmask::SOUND_1 => {
                    new_frame.sound_1 = self.last_frame.sound_1;
                },
                SpriteBitmask::SOUND_2 => {
                    new_frame.sound_2 = self.last_frame.sound_2;
                },
                SpriteBitmask::TEMPO => {
                    new_frame.tempo = self.last_frame.tempo;
                },
                SpriteBitmask::MIN_SPRITE..=SpriteBitmask::MAX_SPRITE => {
                    let sprite_index = channel_index - SpriteBitmask::NUM_NON_SPRITE_CHANNELS;
                    let sprite = &mut new_frame.sprites[sprite_index];
                    let script_id = sprite.script;
                    let flags = sprite.score_color_and_flags & !SpriteScoreColor::COLOR;
                    *sprite = self.last_frame.sprites[sprite_index];
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

        self.raw_last_frame = new_data;

        Ok(Some(new_frame))
    }

    fn reset(&mut self) -> AResult<()> {
        self.raw_last_frame = [ 0; Frame::V5_SIZE as usize ];
        self.input.seek(SeekFrom::Start(self.data_start_pos.into())).context("Can’t reset score stream")?;
        Ok(())
    }
}

#[derive(Clone, Debug, SmartDefault)]
pub struct Score {
    #[default(Self::V5_HEADER_SIZE.into())]
    current_frame_vwsc_position: u32,
    next_frame_vwsc_position: u32,
    vwsc: ScoreStream,
    score_header: Vec<u8>,
    #[default(Self::V5_HEADER_SIZE.into())]
    vwsc_frame_data_maybe_start_pos: u32,
    #[default(Self::V5_HEADER_SIZE.into())]
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

impl Score {
    const V4_HEADER_SIZE: u8 = 20;
    const V5_HEADER_SIZE: u8 = 20;
}

impl Resource for Score {
    type Context = (crate::resources::config::Version, );

    fn load(input: &mut Input<impl Reader>, size: u32, context: &Self::Context) -> AResult<Self> where Self: Sized {
        let mut data = Vec::with_capacity(size.try_into().unwrap());
        input.take(size.into())
            .read_to_end(&mut data)
            .context("Can’t read score data into memory")?;

        let mut input = ByteOrdered::new(Cursor::new(data), Endianness::Big);

        let own_size = input.read_u32().context("Can’t read score size")?;
        ensure_sample!(own_size <= size, "Score recorded size ({}) is larger than actual size ({})", own_size, size);

        let version = if context.0.d4() || context.0.d5() {
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
            } else if version >= Version::V4 {
                let (expect_sprite_size, expect_num_sprites) = if version < Version::V5 {
                    (Sprite::V0_SIZE, Frame::V0_SIZE_IN_CELLS)
                } else {
                    (Sprite::V5_SIZE, Frame::V5_SIZE_IN_CELLS)
                };

                let sprite_size = input.read_i16().context("Can’t read score sprite size")?;
                ensure_sample!(expect_sprite_size == sprite_size.try_into().unwrap(), "Invalid sprite size {} for V5 score", sprite_size);
                // Technically this is the number of `sizeof(Sprite)`s to make one
                // `sizeof(Frame)`; the header of the frame is exactly two
                // `sizeof(Sprite)`s, even though it does not actually contain sprite
                // data
                let num_sprites = input.read_i16().context("Can’t read score sprite count")?;
                ensure_sample!(expect_num_sprites == num_sprites.try_into().unwrap(), "Invalid sprite count {} for V5 score", num_sprites);
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

            version
        } else if context.0.d3() {
            Version::V3
        } else {
            todo!("Score config version {} parsing", context.0 as i32);
        };

        let pos = input.pos()?;

        dbg!(own_size, pos);

        Ok(Self {
            vwsc: ScoreStream::new(input, pos.try_into().unwrap(), own_size, version),
            ..Self::default()
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

binread_flags!(PaletteFlags, u8);

#[derive(BinRead, Clone, Copy, Debug, Default)]
#[br(big)]
pub struct Palette {
    id: MemberId,
    #[br(map = |num: i8| FPS(num.into()))]
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

#[derive(BinRead, Clone, Copy, Debug, Default)]
#[br(big, import(version: Version))]
struct PaletteV4 {
    id: i16,
    cycle_start_color: i8,
    cycle_end_color: i8,
    flags: PaletteFlags,
    #[br(map = |num: i8| FPS(num.into()))]
    rate: FPS,
    num_frames: i16,
    num_cycles: i16,
    field_c: Unk8,
    field_d: Unk8,
    field_e: Unk8,
    #[br(pad_before(if version == Version::V4 { 5 } else { 2 }))]
    field_f: Unk8,
}

impl From<PaletteV4> for Palette {
    fn from(old: PaletteV4) -> Self {
        Self {
            id: old.id.into(),
            cycle_start_color: old.cycle_start_color,
            cycle_end_color: old.cycle_end_color,
            flags: old.flags,
            rate: old.rate,
            num_frames: old.num_frames,
            num_cycles: old.num_cycles,
            field_c: old.field_c,
            field_d: old.field_d,
            field_e: old.field_e,
            field_f: old.field_f,
        }
    }
}

impl Resource for Palette {
    type Context = (Version, );

    fn load(input: &mut Input<impl Reader>, _: u32, context: &Self::Context) -> AResult<Self> where Self: Sized {
        if context.0 > Version::V7 {
            todo!("Score palette version 8 parsing")
        } else if context.0 >= Version::V5 {
            Self::read(input)
        } else {
            PaletteV4::read_args(input, *context).map(Self::from)
        }.context("Can’t read score palette")
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

#[derive(BinRead, Clone, Debug, SmartDefault)]
#[br(big, import(version: Version))]
pub struct Frame {
    pub script: MemberId,
    pub sound_1: MemberId,
    pub sound_2: MemberId,
    #[br(args(version))]
    pub transition: Transition,
    pub tempo_related: Unk8,
    pub sound_1_related: Unk8,
    pub sound_2_related: Unk8,
    pub script_related: Unk8,
    pub transition_related: Unk8,
    #[br(args(version, transition), parse_with = Self::parse_tempo)]
    pub tempo: Tempo,
    #[br(align_before(24))]
    pub palette: Palette,
    #[default([ Sprite::default(); NUM_SPRITES ])]
    #[br(args(version), parse_with = parse_sprites::<Sprite, _>)]
    pub sprites: [ Sprite; NUM_SPRITES ],
}

impl Frame {
    fn parse_tempo<R>(reader: &mut R, options: &ReadOptions, args: (Version, Transition)) -> binread::BinResult<Tempo>
    where R: binread::io::Read + binread::io::Seek {
        let (version, transition) = args;
        if version < Version::V6 {
            Ok(match transition {
                Transition::Legacy { tempo, .. } | Transition::LegacyTempo(tempo) => tempo,
                Transition::None | Transition::Cast(..) => Tempo::default()
            })
        } else {
            let last_pos = reader.seek(SeekFrom::Current(0))?;
            let value = i8::read_options(reader, options, ())?;
            Tempo::new(value.into()).map_err(|e| binread::Error::AssertFail {
                pos: last_pos.try_into().unwrap(),
                message: format!("{}", e)
            })
        }
    }
}

fn parse_sprites<T, R>(reader: &mut R, options: &ReadOptions, args: (Version, )) -> binread::BinResult<[T; NUM_SPRITES]>
where
    T: BinRead<Args = (Version, )> + Copy + Default,
    R: binread::io::Read + binread::io::Seek
{
    let mut sprites = [ T::default(); NUM_SPRITES ];
    for sprite in sprites.iter_mut().take(if args.0 >= Version::V5 { Frame::V5_CELL_COUNT } else { Frame::V4_CELL_COUNT }.into()) {
        *sprite = T::read_options(reader, options, args)?;
    }
    Ok(sprites)
}

#[derive(BinRead, Clone, Copy, Debug)]
#[br(big, import(version: Version))]
struct FrameV3 {
    // TODO: This is an index into VWAC resource. There is no way to convert
    // this to D5, so either script field needs to be an enum (probably) or
    // extra fields should be added to Frame to store it. Also, it turns out
    // this is not the script-related field from D4.
    script: u8,
    sound_1_kind_maybe: Unk8,
    #[br(args(version))]
    transition: Transition,
    sound_1: i16,
    sound_2: i16,
    sound_2_kind_maybe: Unk8,
    #[br(args(version), align_after(16), align_before(16))]
    palette: PaletteV4,
    #[br(args(version), parse_with = parse_sprites::<SpriteV3, _>)]
    sprites: [ SpriteV3; NUM_SPRITES ],
}

impl From<FrameV3> for Frame {
    fn from(old: FrameV3) -> Self {
        Self {
            sound_1: old.sound_1.into(),
            sound_2: old.sound_2.into(),
            transition: old.transition,
            tempo: old.transition.tempo(),
            palette: old.palette.into(),
            sprites: {
                // TODO: (1) more efficient, (2) needs const generics.
                let mut sprites = [ Sprite::default(); NUM_SPRITES ];
                for (i, sprite) in old.sprites.iter().enumerate() {
                    sprites[i] = Sprite::from(*sprite)
                }
                sprites
            },
            ..Self::default()
        }
    }
}

#[derive(BinRead, Clone, Copy, Debug)]
#[br(big, import(version: Version))]
struct FrameV4 {
    field_0: Unk16,
    #[br(args(version))]
    transition: Transition,
    sound_1: i16,
    sound_2: i16,
    field_a: Unk8,
    field_b: Unk8,
    field_c: Unk8,
    tempo_related: Unk8,
    sound_1_related: Unk8,
    sound_2_related: Unk8,
    script: i16,
    script_related: Unk8,
    transition_related: Unk8,
    #[br(args(version), align_after(20), align_before(20))]
    palette: PaletteV4,
    #[br(args(version), parse_with = parse_sprites::<SpriteV4, _>)]
    sprites: [ SpriteV4; NUM_SPRITES ],
}

impl From<FrameV4> for Frame {
    fn from(old: FrameV4) -> Self {
        Self {
            script: old.script.into(),
            sound_1: old.sound_1.into(),
            sound_2: old.sound_2.into(),
            transition: old.transition,
            tempo_related: old.tempo_related,
            sound_1_related: old.sound_1_related,
            sound_2_related: old.sound_2_related,
            script_related: old.script_related,
            transition_related: old.transition_related,
            tempo: old.transition.tempo(),
            palette: old.palette.into(),
            sprites: {
                // TODO: (1) more efficient, (2) needs const generics.
                let mut sprites = [ Sprite::default(); NUM_SPRITES ];
                for (i, sprite) in old.sprites.iter().enumerate() {
                    sprites[i] = Sprite::from(*sprite)
                }
                sprites
            },
        }
    }
}

impl Frame {
    const V4_CELL_COUNT: u16 = 48;
    const V0_SIZE_IN_CELLS: u16 = 50;
    const V0_SIZE: u16 = Sprite::V0_SIZE * Self::V0_SIZE_IN_CELLS;
    const V5_CELL_COUNT: u16 = 48;
    const V5_SIZE_IN_CELLS: u16 = 50;
    const V5_SIZE: u16 = Sprite::V5_SIZE * Self::V5_SIZE_IN_CELLS;

    fn new(data: Vec<u8>, version: Version) -> AResult<Self> {
        let mut input = Input::new(Cursor::new(data), Endianness::Big);
        if version == Version::V3 {
            FrameV3::read_args(&mut input, (version, )).map(Self::from)
        } else if version == Version::V4 {
            FrameV4::read_args(&mut input, (version, )).map(Self::from)
        } else if version >= Version::V5 && version <= Version::V7 {
            Self::read_args(&mut input, (version, ))
        } else {
            todo!("Bad score frame version {}", version)
        }.context("Can’t read frame")
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

binread_enum!(SpriteKind, u8);

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

binread_flags!(SpriteLineSize, u8);

bitflags! {
    #[derive(Default)]
    pub struct SpriteInk: u8 {
        const INK_KIND = 0x3f;
        const TRAILS   = 0x40;
        const STRETCH  = 0x80;
    }
}

// TODO: Reintroduce validation:
// let ink = (ink_and_flags & SpriteInk::INK_KIND).bits();
// Pen::from_u8(ink).with_context(|| format!("Invalid sprite ink {}", ink))?;
binread_flags!(SpriteInk, u8);

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

binread_flags!(SpriteScoreColor, u8);

fn fix_v0_v6_sprite_kind(kind: SpriteKind) -> SpriteKind {
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

#[derive(BinRead, Clone, Copy, Default)]
#[br(big, import(version: Version))]
pub struct Sprite {
    #[br(map = |kind: SpriteKind| if version == Version::V7 { kind } else { fix_v0_v6_sprite_kind(kind) })]
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
    #[br(align_after(24))]
    line_size_and_flags: SpriteLineSize,
}

#[derive(BinRead, Clone, Copy, Debug, Default)]
#[br(big, import(version: Version))]
struct SpriteV3 {
    // TODO: This is an index into VWAC resource. There is no way to convert
    // this to D5, so either script field needs to be an enum (probably) or
    // extra fields should be added to Frame to store it. Also, it turns out
    // this is not the script-related field from D4.
    script: u8,
    kind: SpriteKind,
    fore_color_index: u8,
    back_color_index: u8,
    line_size_and_flags: SpriteLineSize,
    ink_and_flags: SpriteInk,
    id: i16,
    origin: Point,
    height: i16,
    #[br(align_after(16))]
    width: i16,
}

impl From<SpriteV3> for Sprite {
    fn from(old: SpriteV3) -> Self {
        Sprite {
            kind: fix_v0_v6_sprite_kind(old.kind),
            ink_and_flags: old.ink_and_flags,
            id: old.id.into(),
            fore_color_index: old.fore_color_index,
            back_color_index: old.back_color_index,
            origin: old.origin,
            height: old.height,
            width: old.width,
            ..Sprite::default()
        }
    }
}

#[derive(BinRead, Clone, Copy, Debug, Default)]
#[br(big, import(version: Version))]
struct SpriteV4 {
    field_0: Unk8,
    kind: SpriteKind,
    fore_color_index: u8,
    back_color_index: u8,
    line_size_and_flags: SpriteLineSize,
    ink_and_flags: SpriteInk,
    id: i16,
    origin: Point,
    height: i16,
    width: i16,
    script: i16,
    score_color_and_flags: SpriteScoreColor,
    #[br(align_after(20))]
    blend_amount: u8,
}

impl From<SpriteV4> for Sprite {
    fn from(old: SpriteV4) -> Self {
        Sprite {
            kind: fix_v0_v6_sprite_kind(old.kind),
            ink_and_flags: old.ink_and_flags,
            id: old.id.into(),
            script: old.script.into(),
            fore_color_index: old.fore_color_index,
            back_color_index: old.back_color_index,
            origin: old.origin,
            height: old.height,
            width: old.width,
            score_color_and_flags: old.score_color_and_flags,
            blend_amount: old.blend_amount,
            line_size_and_flags: old.line_size_and_flags,
        }
    }
}

impl Sprite {
    const V0_SIZE: u16 = 20;
    const V5_SIZE: u16 = 24;

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
}

impl Resource for Sprite {
    type Context = (Version, );

    fn load(input: &mut Input<impl Reader>, _: u32, context: &Self::Context) -> AResult<Self> where Self: Sized {
        if context.0 == Version::V3 {
            SpriteV3::read_args(input, *context).map(Self::from)
        } else if context.0 < Version::V5 {
            SpriteV4::read_args(input, *context).map(Self::from)
        } else if context.0 <= Version::V7 {
            Self::read_args(input, *context)
        } else {
            bail!("Invalid frame cell version {}", context.0)
        }.context("Can’t read sprite")
    }
}

impl std::fmt::Debug for Sprite {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct(std::any::type_name::<Self>())
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
