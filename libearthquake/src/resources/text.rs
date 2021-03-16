use anyhow::{Context, Result as AResult};
use binread::BinRead;
use libcommon::{Reader, Resource, Unk16, binread_enum, resource::Input};
use libmactoolbox::{Rect, quickdraw::{Pixels, RGBColor}};
use num_derive::FromPrimitive;
use num_traits::FromPrimitive;
use super::config::Version as ConfigVersion;

#[derive(BinRead, Clone, Copy, Eq, PartialEq)]
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

binread_enum!(Frame, u8);

#[derive(BinRead, Clone, Copy, Debug)]
#[br(big)]
pub struct Meta {
    bounds: Rect,
    rect_2: Rect,
    // TODO: Fallibly assert
    #[br(map = |v: u8| v != 0)]
    anti_alias: bool,
    frame: Frame,
    field_12: Unk16,
    anti_alias_min_font_size: Pixels,
    height: Pixels,
    fore_color: RGB24,
    back_color: RGBColor,
}

impl Resource for Meta {
    type Context = (ConfigVersion, );

    fn load(input: &mut Input<impl Reader>, _: u32, context: &Self::Context) -> AResult<Self> where Self: Sized {
        if context.0 < ConfigVersion::V1217 {
            todo!("text member kind for < V1217");
        }

        Self::read(input).context("Can’t read text meta")
    }
}
