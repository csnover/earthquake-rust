use anyhow::{Context, Result as AResult};
use byteordered::{Endianness, ByteOrdered};
use either::Either;
use libcommon::{Reader, Resource};
use libmactoolbox::{Point, Rect};
use num_derive::FromPrimitive;
use num_traits::FromPrimitive;
use super::cast::MemberNum;
use crate::ensure_sample;

#[derive(Clone, Copy, Debug, Eq, Ord, PartialEq, PartialOrd)]
pub struct Tempo(pub u8);

#[derive(Clone, Copy, Debug, Eq, FromPrimitive, Ord, PartialEq, PartialOrd)]
pub enum Version {
    V1025 = 1025,
    V1113 = 1113,
    V1114,
    V1115,
    V1201 = 1201,
    V1215 = 1215,
    V1217 = 1217,
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
    stage_color_index: u16,
    field_1c: u16,
    field_1e: u16,
    field_20: u32,
    field_24: u16,
    field_26: u16,
    field_28: u32,
    field_2c: u32,
    field_38: u16,
    field_3a: u16,
    field_3c: u32,
    field_40: u32,
    field_44: u16,
    field_46: u16,
    field_48: u32,
    field_4c: Either<Point, u32>,
}

impl Resource for Config {
    type Context = ();

    fn load<T: Reader>(input: &mut ByteOrdered<T, Endianness>, size: u32, context: &Self::Context) -> AResult<Self> where Self: Sized {
        let mut input = ByteOrdered::new(input, Endianness::Big);
        let own_size = input.read_u16().context("Can’t read movie config size")?;
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
        let (
            stage_color_index,
            field_1c,
            field_1e,
            field_20,
            field_24,
            field_26,
            field_28,
            field_2c,
            field_30,
            field_36,
            field_38,
        ) = if version >= Version::V1025 {(
            input.read_u16().context("Can’t read stage color")?,
            input.read_u16().context("Can’t read field_1c")?,
            input.read_u16().context("Can’t read field_1e")?,
            input.read_u32().context("Can’t read field_20")?,
            input.read_u16().context("Can’t read field_24")?,
            input.read_u16().context("Can’t read field_26")?,
            input.read_u32().context("Can’t read field_28")?,
            input.read_u32().context("Can’t read field_2c")?,
            input.read_u32().context("Can’t read field_30")?,
            {
                input.skip(2).context("Can’t skip field_34")?;
                input.read_u16().context("Can’t read field_36")?
            },
            input.read_u16().context("Can’t read field_38")?
        )} else {
            Default::default()
        };

        let (
            field_3a,
            field_3c,
            field_40,
        ) = if version >= Version::V1113 {(
            input.read_u16().context("Can’t read field_3a")?,
            input.read_u32().context("Can’t read field_3c")?,
            input.read_u32().context("Can’t read field_40")?,
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
            Either::Left(Point::load(&mut input, Point::SIZE, &()).context("Can’t read field_4c")?)
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
            stage_color_index,
            field_1c,
            field_1e,
            field_20,
            field_24,
            field_26,
            field_28,
            field_2c,
            field_38,
            field_3a,
            field_3c,
            field_40,
            field_44,
            field_46,
            field_48,
            field_4c,
        })
    }
}
