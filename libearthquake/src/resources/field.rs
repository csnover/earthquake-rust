use binrw::BinRead;
use libcommon::bitflags;
use libmactoolbox::quickdraw::{Pixels, Rect, RgbColor};
use smart_default::SmartDefault;

#[derive(BinRead, Clone, Copy, Debug, Eq, PartialEq)]
#[br(big, repr(i16))]
pub(super) enum Alignment {
    Right = -1,
    Left,
    Center,
}

#[derive(BinRead, Clone, Copy, Debug, Eq, PartialEq, SmartDefault)]
#[br(repr(u8))]
pub(super) enum Frame {
    #[default]
    Fit = 0,
    Scroll,
    Fixed,
    LimitToFieldSize,
}

bitflags! {
    pub(super) struct Flags: u8 {
        const EDITABLE     = 0x1;
        const TABBABLE     = 0x2;
        const NO_WORD_WRAP = 0x4;
    }
}

#[derive(BinRead, Clone, Copy, Debug, Eq, PartialEq, SmartDefault)]
#[br(big, repr(u16))]
pub(super) enum ButtonKind {
    #[default]
    None = 0,
    Button,
    CheckBox,
    Radio,
}

// TODO: D3Mac does extra stuff on loading for versions < 1026:
// frame -> 0, flags -> 0, scroll_top -> 0, height -> scroll_height
#[derive(BinRead, Clone, Copy, Debug)]
#[br(big, import(size: u32), pre_assert(size == 24 || size == 28 || size == 30, "unexpected field properties size {}", size))]
pub(super) struct Properties {
    #[br(if(size >= 28), map = |p: u8| p.into())]
    border_size: Pixels,
    /// Space between the field viewport and the border.
    #[br(if(size >= 28), map = |p: u8| p.into())]
    margin_size: Pixels,
    #[br(if(size >= 28), map = |p: u8| p.into())]
    box_shadow_size: Pixels,
    #[br(if(size >= 28))]
    frame: Frame,
    alignment: Alignment,
    back_color: RgbColor,
    scroll_top: Pixels,
    /// The viewport of the field, excluding decorations.
    bounds: Rect,
    /// The height of the field, excluding decorations.
    height: Pixels,
    #[br(map = |p: u8| p.into())]
    text_shadow_size: Pixels,
    flags: Flags,
    /// The total height of content, which may be larger than the viewport
    /// if the field is scrollable.
    scroll_height: Pixels,
    #[br(if(size == 30))]
    button_kind: ButtonKind,
}
