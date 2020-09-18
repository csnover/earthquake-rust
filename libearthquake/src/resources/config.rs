use anyhow::{Context, Result as AResult};
use byteordered::{Endianness, ByteOrdered};
use crate::ensure_sample;
use either::Either;
use libcommon::{Reader, Resource};
use libmactoolbox::{Point, Rect};
use num_derive::FromPrimitive;
use num_traits::FromPrimitive;
use super::cast::{MemberId, MemberNum};

#[derive(Clone, Copy, Debug, Eq, Ord, PartialEq, PartialOrd)]
pub struct Tempo(pub u8);

#[derive(Clone, Copy, Debug, Eq, FromPrimitive, Ord, PartialEq, PartialOrd)]
pub enum Version {
    Unknown,
    V1025 = 1025,
    V1113 = 1113,
    V1114,
    V1115,
    V1116,
    V1201 = 1201,
    V1214 = 1214,
    V1215 = 1215,
    V1217 = 1217,
    V6_5  = 1223,
    V5692 = 5692, // protected
}

impl Default for Version {
    fn default() -> Self {
        Self::Unknown
    }
}

#[derive(Clone, Copy, Debug)]
pub struct Config {
    own_size: u16,
    version: Version,
    rect: Rect,
    min_cast_num: MemberNum,
    max_cast_num: MemberNum,
    tempo: Tempo,
    legacy_back_color_is_black: bool,
    field_12: u16,
    field_14: u16,
    field_16: u16,
    field_18: u8,
    field_19: u8,
    stage_color_index: u16,
    field_1c: u16,
    field_1e: u8,
    field_1f: u8,
    field_20: i32,
    maybe_original_version: Version,
    field_26: u16,
    field_28: i32,
    field_2c: i32,
    field_30: i32,
    field_34: u16,
    field_36: u16,
    field_38: u16,
    field_3a: u16,
    field_3c: u32,
    checksum: u32,
    field_44: u16,
    field_46: u16,
    field_48: u32,
    field_4c: Either<MemberId, u32>,
}

impl Config {
    #[must_use]
    pub fn checksum(&self) -> u32 {
        (0_i32
        .wrapping_add(i32::from(self.own_size) + 1)
        .wrapping_mul(self.version as i32 + 2)
        .wrapping_div(i32::from(self.rect.top) + 3)
        .wrapping_mul(i32::from(self.rect.left) + 4)
        .wrapping_div(i32::from(self.rect.bottom) + 5)
        .wrapping_mul(i32::from(self.rect.right) + 6)
        .wrapping_sub(i32::from(self.min_cast_num.0) + 7)
        .wrapping_mul(i32::from(self.max_cast_num.0) + 8)
        .wrapping_sub(i32::from(self.tempo.0))
        .wrapping_sub(i32::from(self.legacy_back_color_is_black))
        .wrapping_add(i32::from(self.field_12))
        .wrapping_sub(8)
        .wrapping_mul(i32::from(self.field_14) + 12)
        .wrapping_add(i32::from(self.field_16) + 13)
        .wrapping_mul(i32::from(self.field_18) + 14)
        .wrapping_add(i32::from(self.stage_color_index))
        .wrapping_add(i32::from(self.field_1c))
        .wrapping_add(i32::from(self.field_1e))
        .wrapping_add(48)
        .wrapping_mul(i32::from(self.field_1f) + 18)
        .wrapping_add(self.field_20 + 19)
        .wrapping_mul(self.maybe_original_version as i32 + 20)
        .wrapping_add(i32::from(self.field_26))
        .wrapping_add(self.field_28)
        .wrapping_add(self.field_2c)
        .wrapping_add(self.field_30)
        .wrapping_add(90)
        .wrapping_mul(i32::from(self.field_34) + 25)
        .wrapping_add(i32::from(self.field_36) + 26)
        .wrapping_mul(
            i32::from(self.field_3a)
            .wrapping_mul(3590)
            .wrapping_sub(0xbb_0000)
        )
        .wrapping_mul(i32::from(self.field_38) + 27)
        ^ 0x7261_6C66) as u32
    }

    #[must_use]
    pub fn valid(&self) -> bool {
        self.checksum() == self.checksum
    }
}

impl Resource for Config {
    type Context = ();

    fn load<T: Reader>(input: &mut ByteOrdered<T, Endianness>, size: u32, _: &Self::Context) -> AResult<Self> where Self: Sized {
        let mut input = ByteOrdered::new(input, Endianness::Big);
        let own_size = input.read_u16().context("Can’t read movie config size")?;
        ensure_sample!(u32::from(own_size) == size, "Recorded size is not true size ({} != {})", own_size, size);
        let version = {
            let value = input.read_u16().context("Can’t read movie config version")?;
            Version::from_u16(value).with_context(|| format!("Unknown config version {}", value))?
        };
        let rect = Rect::load(&mut input, Rect::SIZE, &()).context("Can’t read stage rect")?;
        let min_cast_num = MemberNum(input.read_i16().context("Can’t read minimum cast number")?);
        let max_cast_num = MemberNum(input.read_i16().context("Can’t read maximum cast number")?);
        let tempo = Tempo(input.read_u8().context("Can’t read tempo")?);
        let legacy_back_color_is_black = input.read_u8().context("Can’t read legacy background is black flag")?;
        ensure_sample!(legacy_back_color_is_black < 2, "Unexpected legacy background is black flag {}", legacy_back_color_is_black);
        let field_12 = input.read_u16().context("Can’t read field_12")?;
        let field_14 = input.read_u16().context("Can’t read field_14")?;
        let field_16 = input.read_u16().context("Can’t read field_16")?;
        let field_18 = input.read_u8().context("Can’t read field_18")?;
        let field_19 = input.read_u8().context("Can’t read field_19")?;

        let (
            stage_color_index,
            field_1c,
            field_1e,
            field_1f,
            field_20,
            maybe_original_version,
            field_26,
            field_28,
            field_2c,
            field_30,
            field_34,
            field_36,
            field_38,
        ) = if version >= Version::V1025 {(
            input.read_u16().context("Can’t read stage color")?,
            input.read_u16().context("Can’t read field_1c")?,
            input.read_u8().context("Can’t read field_1e")?,
            input.read_u8().context("Can’t read field_1f")?,
            input.read_i32().context("Can’t read field_20")?,
            {
                let value = input.read_u16().context("Can’t read original? movie config version")?;
                Version::from_u16(value).with_context(|| format!("Unknown original? config version {}", value))?
            },
            input.read_u16().context("Can’t read field_26")?,
            input.read_i32().context("Can’t read field_28")?,
            input.read_i32().context("Can’t read field_2c")?,
            input.read_i32().context("Can’t read field_30")?,
            input.read_u16().context("Can’t read field_34")?,
            input.read_u16().context("Can’t read field_36")?,
            input.read_u16().context("Can’t read field_38")?
        )} else {
            (0, 0, 0, 0, 0, Version::default(), 0, 0, 0, 0, 0, 0, 0)
        };

        let (
            field_3a,
            field_3c,
            checksum,
        ) = if version >= Version::V1113 {(
            input.read_u16().context("Can’t read field_3a")?,
            input.read_u32().context("Can’t read field_3c")?,
            input.read_u32().context("Can’t read checksum")?,
        )} else {
            Default::default()
        };

        let field_44 = if version >= Version::V1114 {
            input.read_u16().context("Can’t read field_44")?
        } else {
            Default::default()
        };

        let (
            field_46,
            field_48,
        ) = if version >= Version::V1115 {(
            input.read_u16().context("Can’t read field_46")?,
            input.read_u32().context("Can’t read field_48")?,
        )} else {
            Default::default()
        };

        let field_4c = if version >= Version::V1201 {
            Either::Left(MemberId::load(&mut input, MemberId::SIZE, &()).context("Can’t read field_4c")?)
        } else if version >= Version::V1115 {
            Either::Right(input.read_u32().context("Can’t read field_4c")?)
        } else {
            Either::Right(Default::default())
        };

        Ok(Self {
            own_size,
            version,
            rect,
            min_cast_num,
            max_cast_num,
            tempo,
            legacy_back_color_is_black: legacy_back_color_is_black != 0,
            field_12,
            field_14,
            field_16,
            field_18,
            field_19,
            stage_color_index,
            field_1c,
            field_1e,
            field_1f,
            field_20,
            maybe_original_version,
            field_26,
            field_28,
            field_2c,
            field_30,
            field_34,
            field_36,
            field_38,
            field_3a,
            field_3c,
            checksum,
            field_44,
            field_46,
            field_48,
            field_4c,
        })
    }
}
