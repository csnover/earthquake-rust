use anyhow::{Context, Result as AResult};
use byteordered::{ByteOrdered, Endianness};
use crate::ensure_sample;
use libcommon::{Reader, Resource};
use libmactoolbox::Rect;
use num_derive::FromPrimitive;
use num_traits::FromPrimitive;
use super::config::Version as ConfigVersion;

#[derive(Clone, Copy, Debug, Eq, FromPrimitive, PartialEq)]
pub enum Frame {
    Fit,
    Scroll,
    Crop,
}

#[derive(Clone, Copy, Debug)]
pub struct Meta {
    bounds: Rect,
    rect_2: Rect,
    anti_alias: bool,
    frame: Frame,
    field_12: u16,
    anti_alias_min_font_size: i16,
    height: u16,
    field_18: u32,
    field_1c: i16,
    field_1e: i16,
    field_20: i16,
}

impl Resource for Meta {
    type Context = (ConfigVersion, );

    fn load<T: Reader>(input: &mut ByteOrdered<T, Endianness>, _: u32, context: &Self::Context) -> AResult<Self> where Self: Sized {
        if context.0 < ConfigVersion::V1217 {
            todo!("text member kind for < V1217");
        }

        let bounds = Rect::load(input, Rect::SIZE, &()).context("Can’t read text bounds")?;
        let rect_2 = Rect::load(input, Rect::SIZE, &()).context("Can’t read text rect 2")?;
        let anti_alias = input.read_u8().context("Can’t read text anti-aliasing flag")?;
        ensure_sample!(anti_alias == 0 || anti_alias == 1, "Unexpected anti-aliasing value {}", anti_alias);
        let frame = {
            let value = input.read_u8().context("Can’t read text frame")?;
            Frame::from_u8(value).with_context(|| format!("Invalid text frame {}", value))?
        };
        let field_12 = input.read_u16().context("Can’t read text field_12")?;
        let anti_alias_min_font_size = input.read_i16().context("Can’t read anti-aliasing minimum font size")?;
        let height = input.read_u16().context("Can’t read text height")?;
        let field_18 = input.read_u32().context("Can’t read text field_18")?;
        ensure_sample!(field_18 == 0, "Unexpected field_18 value 0x{:x}", field_18);
        let field_1c = input.read_i16().context("Can’t read text field_1c")?;
        ensure_sample!(field_1c == -1, "Unexpected field_1c value {}", field_1c);
        let field_1e = input.read_i16().context("Can’t read text field_1e")?;
        ensure_sample!(field_1e == -1, "Unexpected field_1e value {}", field_1e);
        let field_20 = input.read_i16().context("Can’t read text field_20")?;
        ensure_sample!(field_20 == -1, "Unexpected field_20 value {}", field_20);
        Ok(Self {
            bounds,
            rect_2,
            anti_alias: anti_alias != 0,
            frame,
            field_12,
            anti_alias_min_font_size,
            height,
            field_18,
            field_1c,
            field_1e,
            field_20,
        })
    }
}
