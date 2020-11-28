use anyhow::{Context, Result as AResult};
use bitflags::bitflags;
use byteordered::{Endianness, ByteOrdered};
use crate::{ensure_sample, player::score::Tempo};
use libcommon::{Reader, Resource, resource::Input};
use libmactoolbox::Rect;
use num_derive::FromPrimitive;
use num_traits::FromPrimitive;
use super::cast::{MemberId, MemberNum};

#[derive(Clone, Copy, Debug, Default, Eq, Ord, PartialEq, PartialOrd)]
pub struct LegacyTempo(pub u8);

#[derive(Clone, Copy, Debug, Eq, FromPrimitive, PartialEq)]
pub enum Platform {
    Unknown = 0,
    Mac,
    Win,
}

impl Default for Platform {
    fn default() -> Self {
        Self::Unknown
    }
}

#[derive(Clone, Copy, Debug, Eq, FromPrimitive, Ord, PartialEq, PartialOrd)]
pub enum Version {
    Unknown,
    // D2
    V1024 = 1024,

    // D3, but only in the extended version field
    V1025,
    V1028 = 1028,
    V1029,

    // D4
    V1113 = 1113,
    V1114,
    V1115,
    V1116,
    V1117,

    // ?
    V1201 = 1201,
    V1214 = 1214,

    // D5
    V1215 = 1215,
    V1217 = 1217,
    V1222 = 1222,
    V1223,
    V1406 = 1406,
    V5692 = 5692, // protected
}

impl Version {
    #[must_use]
    pub fn d2(self) -> bool {
        matches!(self, Self::V1024)
    }

    #[must_use]
    pub fn d3(self) -> bool {
        matches!(self, Self::V1025 | Self::V1028 | Self::V1029)
    }

    #[must_use]
    pub fn d4(self) -> bool {
        matches!(self, Self::V1113 | Self::V1114 | Self::V1115 | Self::V1116 | Self::V1117)
    }

    #[must_use]
    pub fn d5(self) -> bool {
        matches!(self, Self::V1201 | Self::V1214 | Self::V1215 | Self::V1217)
    }

    #[must_use]
    pub fn d6(self) -> bool {
        matches!(self, Self::V1222 | Self::V1223)
    }

    #[must_use]
    pub fn d7(self) -> bool {
        matches!(self, Self::V1406)
    }
}

impl Default for Version {
    fn default() -> Self {
        Self::Unknown
    }
}

bitflags! {
    #[derive(Default)]
    pub struct Flags: u32 {
        const MOVIE_FIELD_46       = 0x20;
        const PALETTE_MAPPING      = 0x40;
        const LEGACY_FLAG_1        = 0x80;
        const LEGACY_FLAG_2        = 0x100;
        const UPDATE_MOVIE_ENABLED = 0x200;
        const PRELOAD_EVENT_ABORT  = 0x400;
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum PaletteId {
    Cast(MemberId),
    Number(i32),
}

impl Default for PaletteId {
    fn default() -> Self {
        Self::Number(0)
    }
}

#[derive(Clone, Copy, Debug, Default)]
pub struct Config {
    own_size: i16,
    version: Version,
    rect: Rect,
    min_cast_num: MemberNum,
    max_cast_num: MemberNum,
    legacy_tempo: LegacyTempo,
    legacy_back_color_is_black: bool,
    field_12: i16,
    field_14: i16,
    field_16: i16,
    field_18: u8,
    field_19: u8,
    stage_color_index: i16,
    default_color_depth: i16,
    field_1e: u8,
    field_1f: u8,
    field_20: i32,
    original_version: Version,
    max_cast_color_depth: i16,
    flags: Flags,
    field_2c: i32,
    field_30: i32,
    field_34: i8,
    field_35: i8,
    current_tempo: Tempo,
    platform: Platform,
    field_3a: i16,
    field_3c: u32,
    checksum: u32,
    field_44: u16,
    field_46: u16,
    max_cast_resource_num: u32,
    default_palette: PaletteId,
}

impl Config {
    #[must_use]
    pub fn calculate_checksum(&self) -> u32 {
        ((i32::from(self.own_size) + 1)
        .wrapping_mul(self.version as i32 + 2)
        .wrapping_div(i32::from(self.rect.top) + 3)
        .wrapping_mul(i32::from(self.rect.left) + 4)
        .wrapping_div(i32::from(self.rect.bottom) + 5)
        .wrapping_mul(i32::from(self.rect.right) + 6)
        .wrapping_sub(i32::from(self.min_cast_num.0) + 7)
        .wrapping_mul(i32::from(self.max_cast_num.0) + 8)
        .wrapping_sub(i32::from(self.legacy_tempo.0) + 9)
        .wrapping_sub(i32::from(self.legacy_back_color_is_black) + 10)
        .wrapping_add(i32::from(self.field_12) + 11)
        .wrapping_mul(i32::from(self.field_14) + 12)
        .wrapping_add(i32::from(self.field_16) + 13)
        .wrapping_mul(i32::from(self.field_18) + 14)
        .wrapping_add(i32::from(self.stage_color_index) + 15)
        .wrapping_add(i32::from(self.default_color_depth) + 16)
        .wrapping_add(i32::from(self.field_1e) + 17)
        .wrapping_mul(i32::from(self.field_1f) + 18)
        .wrapping_add(self.field_20 + 19)
        .wrapping_mul(self.original_version as i32 + 20)
        .wrapping_add(i32::from(self.max_cast_color_depth) + 21)
        .wrapping_add(self.flags.bits as i32 + 22)
        .wrapping_add(self.field_2c + 23)
        .wrapping_add(self.field_30 + 24)
        .wrapping_mul(i32::from(self.field_34) + 25)
        .wrapping_add(i32::from(self.current_tempo.to_primitive()) + 26)
        .wrapping_mul(self.platform as i32 + 27)
        .wrapping_mul(
            i32::from(self.field_3a)
            .wrapping_mul(3590)
            .wrapping_sub(0xbb_0000)
        )
        ^ 0x7261_6C66) as u32
    }

    #[must_use]
    pub fn generate_field_3a(flag: bool) -> i16 {
        let (state, a) = Self::field_3a_1(0x123_4567);
        let (_, b) = Self::field_3a_1(state);
        a % 1423 * 23 + if flag {
            0
        } else {
            b % 19 + 1
        }
    }

    #[must_use]
    pub fn min_cast_num(&self) -> MemberNum {
        self.min_cast_num
    }

    #[must_use]
    pub fn original_version(&self) -> Version {
        self.original_version
    }

    #[must_use]
    pub fn valid(&self) -> bool {
        if self.version < Version::V1113 {
            true
        } else {
            self.calculate_checksum() == self.checksum
        }
    }

    #[must_use]
    pub fn version(&self) -> Version {
        self.version
    }

    fn field_3a_1(old_state: i32) -> (i32, i16) {
        let mut state = (old_state % 127_773 * 16807).wrapping_sub(old_state / 127_773 * 2836);
        if state < 0 {
            state += 0x7fff_ffff;
        }
        (state, ((state >> 14) as i16).abs())
    }

    fn load_1025(this: &mut Self, input: &mut ByteOrdered<impl Reader, Endianness>) -> AResult<()> {
        this.stage_color_index = input.read_i16().context("Can’t read stage color")?;
        this.default_color_depth = input.read_i16().context("Can’t read default color depth")?;
        this.field_1e = input.read_u8().context("Can’t read field_1e")?;
        this.field_1f = input.read_u8().context("Can’t read field_1f")?;
        this.field_20 = input.read_i32().context("Can’t read field_20")?;
        this.original_version = {
            let value = input.read_u16().context("Can’t read original movie config version")?;
            Version::from_u16(value).with_context(|| format!("Unknown original config version {}", value))?
        };
        this.max_cast_color_depth = input.read_i16().context("Can’t read cast maximum color depth")?;
        this.flags = {
            let value = input.read_u32().context("Can’t read flags")?;
            Flags::from_bits(value).with_context(|| format!("Invalid config flags (0x{:x})", value))?
        };
        this.field_2c = input.read_i32().context("Can’t read field_2c")?;
        this.field_30 = input.read_i32().context("Can’t read field_30")?;
        this.field_34 = input.read_i8().context("Can’t read field_34")?;
        this.field_35 = input.read_i8().context("Can’t read field_35")?;
        this.current_tempo = Tempo::new(input.read_i16().context("Can’t read current tempo")?)?;
        this.platform = {
            let value = input.read_i16().context("Can’t read platform")?;
            Platform::from_i16(value).with_context(|| format!("Unknown config platform {}", value))?
        };
        Ok(())
    }

    fn load_1113(this: &mut Self, input: &mut ByteOrdered<impl Reader, Endianness>) -> AResult<()> {
        this.field_3a = input.read_i16().context("Can’t read field_3a")?;
        this.field_3c = input.read_u32().context("Can’t read field_3c")?;
        this.checksum = input.read_u32().context("Can’t read checksum")?;
        Ok(())
    }

    fn load_1114(this: &mut Self, input: &mut ByteOrdered<impl Reader, Endianness>) -> AResult<()> {
        this.field_44 = input.read_u16().context("Can’t read field_44")?;
        Ok(())
    }

    fn load_1115(this: &mut Self, input: &mut ByteOrdered<impl Reader, Endianness>) -> AResult<()> {
        this.field_46 = input.read_u16().context("Can’t read field_46")?;
        this.max_cast_resource_num = input.read_u32().context("Can’t read maximum cast resource number")?;
        Ok(())
    }

    fn load_1201(this: &mut Self, version: Version, input: &mut ByteOrdered<impl Reader, Endianness>) -> AResult<()> {
        if version >= Version::V1201 {
            this.default_palette = PaletteId::Cast(MemberId::load(input, MemberId::SIZE, &()).context("Can’t read default palette")?);
        } else if version >= Version::V1115 {
            this.default_palette = PaletteId::Number(input.read_i32().context("Can’t read default palette")?);
        }
        Ok(())
    }
}

impl Resource for Config {
    type Context = ();

    fn load(input: &mut Input<impl Reader>, size: u32, _: &Self::Context) -> AResult<Self> where Self: Sized {
        let mut input = input.as_mut().into_endianness(Endianness::Big);
        let own_size = input.read_i16().context("Can’t read movie config size")?;
        ensure_sample!(own_size as u32 == size, "Recorded size is not true size ({} != {})", own_size, size);
        let version = {
            let value = input.read_i16().context("Can’t read movie config version")?;
            Version::from_i16(value).with_context(|| format!("Unknown config version {}", value))?
        };
        let rect = Rect::load(&mut input, Rect::SIZE, &()).context("Can’t read stage rect")?;
        let min_cast_num = MemberNum(input.read_i16().context("Can’t read minimum cast number")?);
        let max_cast_num = MemberNum(input.read_i16().context("Can’t read maximum cast number")?);
        let legacy_tempo = LegacyTempo(input.read_u8().context("Can’t read legacy tempo")?);
        let legacy_back_color_is_black = input.read_u8().context("Can’t read legacy background is black flag")?;
        ensure_sample!(legacy_back_color_is_black < 2, "Unexpected legacy background is black flag {}", legacy_back_color_is_black);
        let field_12 = input.read_i16().context("Can’t read field_12")?;
        let field_14 = input.read_i16().context("Can’t read field_14")?;
        let field_16 = input.read_i16().context("Can’t read field_16")?;
        let field_18 = input.read_u8().context("Can’t read field_18")?;
        let field_19 = input.read_u8().context("Can’t read field_19")?;

        let mut this = Self {
            own_size,
            version,
            rect,
            min_cast_num,
            max_cast_num,
            legacy_tempo,
            legacy_back_color_is_black: legacy_back_color_is_black != 0,
            field_12,
            field_14,
            field_16,
            field_18,
            field_19,
            ..Self::default()
        };

        if version >= Version::V1025 {
            Self::load_1025(&mut this, &mut input)?;
        }
        if version >= Version::V1113 {
            Self::load_1113(&mut this, &mut input)?;
        }
        if version >= Version::V1114 {
            Self::load_1114(&mut this, &mut input)?;
        }
        if version >= Version::V1115 {
            Self::load_1115(&mut this, &mut input)?;
        }
        Self::load_1201(&mut this, version, &mut input)?;

        Ok(this)
    }
}
