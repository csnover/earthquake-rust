// TODO: You know, finish this file and then remove these overrides
#![allow(dead_code)]

use binrw::BinRead;
use num_derive::FromPrimitive;
use libcommon::{
    newtype_num,
    UnkHnd,
    UnkPtr,
};
use smart_default::SmartDefault;

pub type PixPatHandle = UnkHnd;
pub type PixMapHandle = UnkHnd;
pub type RgnHandle = UnkHnd;
type StyleField = u16;
type Fixed = u32;

#[derive(Clone, Copy, Debug, Eq, FromPrimitive, PartialEq)]
pub enum Pen {
    SrcCopy = 0,
    SrcOr,
    SrcXor,
    SrcBic,
    NotSrcCopy,
    NotSrcOr,
    NotSrcXor,
    NotSrcBic,
    PatCopy,
    PatOr,
    PatXor,
    PatBic,
    NotPatCopy,
    NotPatOr,
    NotPatXor,
    NotPatBic,
    Blend         = 32,
    AddPin,
    AddOver,
    SubPin,
    Transparent,
    AdMax,
    SubOver,
    AdMin,
    GrayishTextOr = 49,
    Hilite,
    DitherCopy    = 64,
}

newtype_num! {
    #[derive(BinRead)]
    pub struct Pixels(i16);
}

impl std::fmt::Debug for Pixels {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}px", self.0)
    }
}

#[derive(BinRead, Clone, Copy, Debug, Default)]
pub struct Point {
    pub x: Pixels,
    pub y: Pixels,
}

#[derive(BinRead, Clone, Copy, Default)]
pub struct Rect {
    pub top: Pixels,
    pub left: Pixels,
    pub bottom: Pixels,
    pub right: Pixels,
}

impl Rect {
    #[inline]
    #[must_use]
    pub fn height(self) -> Pixels {
        self.bottom - self.top
    }

    /// `InsetRect`
    pub fn inset(&mut self, dh: Pixels, dv: Pixels) {
        self.top -= dv;
        self.left -= dh;
        self.bottom -= dv;
        self.right -= dh;
    }

    #[inline]
    #[must_use]
    pub fn width(self) -> Pixels {
        self.right - self.left
    }
}

impl std::fmt::Debug for Rect {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct(std::any::type_name::<Self>())
            .field("top", &self.top)
            .field("left", &self.left)
            .field("bottom", &self.bottom)
            .field("right", &self.right)
            .field("(width)", &self.width())
            .field("(height)", &self.height())
            .finish()
    }
}

#[derive(Clone, Copy, Debug, SmartDefault)]
pub struct Region {
    #[default(10)]
    pub size: i16,
    pub bounding_box: Rect,
}

impl Region {
    /// `NewRgn`
    #[must_use]
    pub fn new() -> Self {
        <_>::default()
    }

    /// `SetRectRgn`
    pub fn set_rect_region(&mut self, left: impl Into<Pixels>, top: impl Into<Pixels>, right: impl Into<Pixels>, bottom: impl Into<Pixels>) {
        self.size = 10;
        self.bounding_box.top = top.into();
        self.bounding_box.left = left.into();
        self.bounding_box.right = right.into();
        self.bounding_box.bottom = bottom.into();
    }

    /// `RectRgn`
    pub fn rect_region(&mut self, rect: Rect) {
        self.size = 10;
        self.bounding_box = rect;
    }
}

newtype_num! {
    #[derive(BinRead)]
    pub struct PaletteIndex(u8);
}

impl std::fmt::Debug for PaletteIndex {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

#[derive(BinRead, Clone, Copy, Default, Eq, PartialEq)]
pub struct RgbColor {
    pub r: u16,
    pub g: u16,
    pub b: u16,
}

impl RgbColor {
    pub const SIZE: u32 = 6;
}

impl std::fmt::Debug for RgbColor {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "rgb16({}, {}, {})", self.r, self.g, self.b)
    }
}

#[derive(Clone, Copy, Debug, Default)]
pub struct CGrafPort {
    device: u16,
    port_pix_map: PixMapHandle,
    port_version: u16,
    graf_vars: UnkHnd,
    ch_extra: u16,
    pn_loc_h_frac: u16,
    port_rect: Rect,
    vis_rgn: RgnHandle,
    clip_rgn: RgnHandle,
    bk_pix_pat: PixPatHandle,
    rgb_fg_color: RgbColor,
    rgb_bk_color: RgbColor,
    pn_loc: Point,
    pn_size: Point,
    pn_mode: u16,
    pn_pix_pat: PixPatHandle,
    fill_pix_pat: PixPatHandle,
    pn_vis: u16,
    tx_font: u16,
    tx_face: StyleField,
    tx_mode: u16,
    tx_size: u16,
    sp_extra: Fixed,
    fg_color: u32,
    bk_color: u32,
    colr_bit: u16,
    pat_stretch: u16,
    pic_save: UnkHnd,
    rgn_save: UnkHnd,
    poly_save: UnkHnd,
    graf_procs: UnkPtr,
}

impl CGrafPort {
    pub fn port_size(&mut self, width: Pixels, height: Pixels) {
        self.port_rect.right = self.port_rect.left + width;
        self.port_rect.bottom = self.port_rect.top + height;
    }
}
