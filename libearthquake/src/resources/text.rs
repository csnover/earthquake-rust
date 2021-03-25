use binrw::BinRead;
use libcommon::Unk16;
use libmactoolbox::{Rect, quickdraw::{Pixels, RGBColor}};
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

#[derive(BinRead, Clone, Copy, Debug, Eq, PartialEq)]
#[br(repr(u8))]
pub enum Frame {
    Fit = 0,
    Scroll,
    Crop,
}

#[derive(BinRead, Clone, Copy, Debug)]
#[br(big, import(version: ConfigVersion))]
#[br(pre_assert(version >= ConfigVersion::V1217, "TODO: text member kind for < V1217"))]
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
