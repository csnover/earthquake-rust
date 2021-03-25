use anyhow::{Context, Result as AResult};
use binrw::BinRead;
use libcommon::{binrw_enum, bitflags, Reader, Resource, resource::Input};
use libmactoolbox::{quickdraw::{Pixels, RGBColor}, Rect};
use num_derive::FromPrimitive;
use num_traits::FromPrimitive;
use smart_default::SmartDefault;

#[derive(Clone, Copy, Debug, Eq, FromPrimitive, PartialEq)]
pub enum Alignment {
    Right = -1,
    Left,
    Center,
}

binrw_enum!(Alignment, i16);

#[derive(Clone, Copy, Debug, Eq, FromPrimitive, PartialEq)]
pub enum Frame {
    Fit = 0,
    Scroll,
    Fixed,
    LimitToFieldSize,
}

binrw_enum!(Frame, u8);

bitflags! {
    pub struct Flags: u8 {
        const EDITABLE     = 0x1;
        const TABBABLE     = 0x2;
        const NO_WORD_WRAP = 0x4;
    }
}

#[derive(Clone, Copy, Debug, Eq, FromPrimitive, PartialEq, SmartDefault)]
pub enum ButtonKind {
    #[default]
    None = 0,
    Button,
    CheckBox,
    Radio,
}

binrw_enum!(ButtonKind, u16);

#[derive(BinRead, Clone, Copy, Debug)]
#[br(big, import(size: u32))]
pub struct Meta {
    #[br(map = |p: u8| Pixels::from(p))]
    border_size: Pixels,
    /// Space between the field viewport and the border.
    #[br(map = |p: u8| Pixels::from(p))]
    margin_size: Pixels,
    #[br(map = |p: u8| Pixels::from(p))]
    box_shadow_size: Pixels,
    frame: Frame,
    alignment: Alignment,
    back_color: RGBColor,
    scroll_top: Pixels,
    /// The viewport of the field, excluding decorations.
    bounds: Rect,
    /// The height of the field, excluding decorations.
    #[br(assert(height == bounds.height()))]
    height: Pixels,
    #[br(map = |p: u8| Pixels::from(p))]
    text_shadow_size: Pixels,
    flags: Flags,
    /// The total height of content, which may be larger than the viewport
    /// if the field is scrollable.
    scroll_height: Pixels,
    #[br(if(size == 0x1e))]
    button_kind: ButtonKind,
}

impl Resource for Meta {
    type Context = ();

    fn load(input: &mut Input<impl Reader>, size: u32, _: &Self::Context) -> AResult<Self> where Self: Sized {
        Self::read_args(input, (size, )).context("Can’t read field meta")
    }
}
