use anyhow::{Context, Result as AResult};
use byteordered::{Endianness, ByteOrdered};
use crate::ensure_sample;
use libcommon::{Reader, Resource};
use libmactoolbox::Rect;
use num_derive::FromPrimitive;
use num_traits::FromPrimitive;
use super::cast::{ConfigVersion, MemberKind};

#[derive(Clone, Copy, Debug, Eq, FromPrimitive, PartialEq)]
pub enum Kind {
    Rect      = 1,
    RoundRect = 2,
    Oval      = 3,
    Line      = 4,
}

#[derive(Clone, Copy, Debug, Eq, FromPrimitive, PartialEq)]
pub enum LineDirection {
    TopToBottom = 5,
    BottomToTop = 6,
}

#[derive(Clone, Copy, Debug)]
pub struct Meta {
    kind: Kind,
    bounds: Rect,
    pattern: u16,
    fore_color: u8,
    back_color: u8,
    filled: bool,
    line_size: u8,
    line_direction: LineDirection,
}

impl Resource for Meta {
    type Context = ();

    fn load<T: Reader>(input: &mut ByteOrdered<T, Endianness>, _: u32, _: &Self::Context) -> AResult<Self> where Self: Sized {
        let kind = {
            let value = input.read_u16().context("Can’t read shape kind")?;
            Kind::from_u16(value).with_context(|| format!("Invalid shape kind {}", value))?
        };
        let bounds = Rect::load(input, Rect::SIZE, &()).context("Can’t read shape bounds")?;
        let pattern = input.read_u16().context("Can’t read shape pattern")?;
        let fore_color = input.read_u8().context("Can’t read shape foreground color")?;
        let back_color = input.read_u8().context("Can’t read shape background color")?;
        let filled = input.read_u8().context("Can’t read shape filled flag")?;
        ensure_sample!(filled == 0 || filled == 1, "Unexpected filled value {}", filled);
        let line_size = input.read_u8().context("Can’t read shape line size")?;
        ensure_sample!(line_size != 0, "Unexpected zero shape line size");
        let line_direction = {
            let value = input.read_u8().context("Can’t read shape line kind")?;
            LineDirection::from_u8(value)
                .with_context(|| format!("Invalid line direction {}", value))?
        };

        Ok(Self {
            kind,
            bounds,
            pattern,
            fore_color,
            back_color,
            filled: filled != 0,
            line_size: line_size - 1,
            line_direction,
        })
    }
}