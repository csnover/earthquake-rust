use anyhow::{Context, Result as AResult};
use bitflags::bitflags;
use byteordered::{ByteOrdered, Endianness};
use crate::ensure_sample;
use libcommon::{Resource, Reader};
use libmactoolbox::Rect;
use num_derive::FromPrimitive;
use num_traits::FromPrimitive;

#[derive(Clone, Copy, Debug, Eq, FromPrimitive, PartialEq)]
pub enum Alignment {
    Right = -1,
    Left,
    Center,
}

#[derive(Clone, Copy, Debug, Eq, FromPrimitive, PartialEq)]
pub enum Frame {
    Fit,
    Scroll,
    Fixed,
    LimitToFieldSize,
}

bitflags! {
    pub struct Flags: u8 {
        const EDITABLE     = 0x1;
        const TABBABLE     = 0x2;
        const NO_WORD_WRAP = 0x4;
    }
}

#[derive(Clone, Copy, Debug, Eq, FromPrimitive, PartialEq)]
pub enum ButtonKind {
    None,
    Button,
    Radio,
    CheckBox,
}

#[derive(Clone, Copy, Debug)]
pub struct Meta {
    border_size: u8,
    /// Space between the field viewport and the border.
    margin_size: u8,
    box_shadow_size: u8,
    frame: Frame,
    alignment: Alignment,
    field_e: i16,
    field_10: i16,
    field_12: i16,
    scroll_top: u16,
    /// The viewport of the field, excluding decorations.
    bounds: Rect,
    /// The height of the field, excluding decorations.
    height: u16,
    text_shadow_size: u8,
    flags: Flags,
    /// The total height of content, which may be larger than the viewport
    /// if the field is scrollable.
    scroll_height: u16,
    button_kind: ButtonKind,
}

impl Resource for Meta {
    type Context = ();

    fn load<T: Reader>(input: &mut ByteOrdered<T, Endianness>, size: u32, _: &Self::Context) -> AResult<Self> where Self: Sized {
        let border_size = input.read_u8().context("Can’t read border size")?;
        let margin_size = input.read_u8().context("Can’t read margin size")?;
        let box_shadow_size = input.read_u8().context("Can’t read box shadow size")?;
        let frame = {
            let value = input.read_u8().context("Can’t read field frame")?;
            Frame::from_u8(value).with_context(|| format!("Invalid value {} for field frame", value))?
        };
        let alignment = {
            let value = input.read_i16().context("Can’t read alignment")?;
            Alignment::from_i16(value).with_context(|| format!("Invalid value {} for field alignment", value))?
        };
        let field_e = input.read_i16().context("Can’t read field_e")?;
        ensure_sample!(field_e == -1, "Field_e is {}, not -1", field_e);
        let field_10 = input.read_i16().context("Can’t read field_10")?;
        ensure_sample!(field_10 == -1, "Field_10 is {}, not -1", field_10);
        let field_12 = input.read_i16().context("Can’t read field_12")?;
        ensure_sample!(field_12 == -1, "Field_12 is {}, not -1", field_12);
        let scroll_top = input.read_u16().context("Can’t read scroll top")?;
        let bounds = Rect::load(input, Rect::SIZE, &()).context("Can’t read bounds")?;
        let height = input.read_u16().context("Can’t read height")?;
        ensure_sample!(height == bounds.height() as u16, "Height {} does not match bounds height {}", height, bounds.height());
        let text_shadow_size = input.read_u8().context("Can’t read text shadow size")?;
        let flags = {
            let value = input.read_u8().context("Can’t read field flags")?;
            Flags::from_bits(value).with_context(|| format!("Invalid flags 0x{:x} for field", value))?
        };
        let scroll_height = input.read_u16().context("Can’t read scroll height")?;
        let button_kind = if size == 0x1e {
            let value = input.read_u16().context("Can’t read button kind")?;
            ButtonKind::from_u16(value).with_context(|| format!("Invalid button kind {}", value))?
        } else {
            ButtonKind::None
        };

        Ok(Self {
            border_size,
            margin_size,
            box_shadow_size,
            frame,
            alignment,
            field_e,
            field_10,
            field_12,
            scroll_top,
            bounds,
            height,
            text_shadow_size,
            flags,
            scroll_height,
            button_kind,
        })
    }
}
