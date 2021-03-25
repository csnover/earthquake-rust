use anyhow::{Context, Result as AResult};
use binrw::BinRead;
use libcommon::{binrw_enum, Reader, Resource, resource::Input};
use libmactoolbox::{Rect, quickdraw::{PaletteIndex, Pixels}};
use num_derive::FromPrimitive;
use num_traits::FromPrimitive;

#[derive(Clone, Copy, Debug, Eq, FromPrimitive, PartialEq)]
pub enum Kind {
    Rect = 1,
    RoundRect,
    Oval,
    Line,
}

binrw_enum!(Kind, u16);

#[derive(Clone, Copy, Debug, Eq, FromPrimitive, PartialEq)]
pub enum LineDirection {
    TopToBottom = 5,
    BottomToTop,
}

binrw_enum!(LineDirection, u8);

#[derive(BinRead, Clone, Copy, Debug)]
#[br(big)]
pub struct Meta {
    kind: Kind,
    bounds: Rect,
    pattern: i16,
    fore_color: PaletteIndex,
    back_color: PaletteIndex,
    // TODO: Fallibly assert 0 or 1
    #[br(map = |filled: u8| filled != 0)]
    filled: bool,
    // Director does not normalise file data, nor data to/from Lingo,
    // so this value can be anything 0-255. Only in the paint function
    // does it get clamped by (effectively) `max(0, (line_size & 0xf) - 1)`.
    #[br(map = |p: u8| Pixels::from(p))]
    line_size: Pixels,
    line_direction: LineDirection,
}

impl Resource for Meta {
    type Context = ();

    fn load(input: &mut Input<impl Reader>, _: u32, _: &Self::Context) -> AResult<Self> where Self: Sized {
        Self::read(input).context("Can’t read shape meta")
    }
}
