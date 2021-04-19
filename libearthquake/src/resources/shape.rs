use binrw::BinRead;
use libmactoolbox::quickdraw::{PaletteIndex, Pixels, Rect};

#[derive(BinRead, Clone, Copy, Debug, Eq, PartialEq)]
#[br(big, repr(u16))]
pub(super) enum Kind {
    Rect = 1,
    RoundRect,
    Oval,
    Line,
}

#[derive(BinRead, Clone, Copy, Debug, Eq, PartialEq)]
#[br(repr(u8))]
pub(super) enum LineDirection {
    TopToBottom = 5,
    BottomToTop,
}

#[derive(BinRead, Clone, Copy, Debug)]
#[br(big)]
pub(super) struct Properties {
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
