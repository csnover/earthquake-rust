use anyhow::{Context, Result as AResult};
use crate::ensure_sample;
use libcommon::{Reader, Resource, resource::Input};
use libmactoolbox::{Rect, quickdraw::RGBColor};
use num_derive::FromPrimitive;
use num_traits::FromPrimitive;
use super::config::Version as ConfigVersion;

#[derive(Clone, Copy, Eq, PartialEq)]
pub struct RGB24(u32);

impl std::fmt::Debug for RGB24 {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "rgb8({}, {}, {})",
            (self.0 >> 16) & 0xff,
            (self.0 >> 8) & 0xff,
            self.0 & 0xff,
        )
    }
}

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
    fore_color: RGB24,
    back_color: RGBColor,
}

impl Resource for Meta {
    type Context = (ConfigVersion, );

    fn load(input: &mut Input<impl Reader>, _: u32, context: &Self::Context) -> AResult<Self> where Self: Sized {
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
        let fore_color = RGB24(input.read_u32().context("Can’t read text foreground color")?);
        let back_color = RGBColor::load(input, RGBColor::SIZE, &()).context("Can’t read text background color")?;
        Ok(Self {
            bounds,
            rect_2,
            anti_alias: anti_alias != 0,
            frame,
            field_12,
            anti_alias_min_font_size,
            height,
            fore_color,
            back_color,
        })
    }
}
