// TODO: You know, finish this file and then remove these overrides
#![allow(dead_code)]

use crate::{Point, Rect};
use anyhow::{Context, Result as AResult};
use num_derive::FromPrimitive;
use num_traits::FromPrimitive;
use libcommon::{
    Reader,
    Resource,
    resource::Input,
    UnkHnd,
    UnkPtr,
};
use std::{
    cell::{Ref, RefCell},
    rc::Rc,
};

type PixPatHandle = UnkHnd;
type PixMapHandle = UnkHnd;
type RgnHandle = UnkHnd;
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

#[derive(Clone, Copy, Eq, PartialEq)]
pub struct RGBColor {
    pub r: u16,
    pub g: u16,
    pub b: u16,
}

impl RGBColor {
    pub const SIZE: u32 = 6;
}

impl Resource for RGBColor {
    type Context = ();

    fn load(input: &mut Input<impl Reader>, size: u32, _: &Self::Context) -> AResult<Self> where Self: Sized {
        assert_eq!(size, Self::SIZE);
        Ok(Self {
            r: input.read_u16().context("Can’t read red channel")?,
            g: input.read_u16().context("Can’t read green channel")?,
            b: input.read_u16().context("Can’t read blue channel")?,
        })
    }
}

impl std::fmt::Debug for RGBColor {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "rgb16({}, {}, {})", self.r, self.g, self.b)
    }
}

#[derive(Clone, Copy)]
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
    rgb_fg_color: RGBColor,
    rgb_bk_color: RGBColor,
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

pub struct QuickDraw {
    graf_port: Rc<RefCell<CGrafPort>>,
}

impl QuickDraw {
    pub fn use_port(&self, f: impl Fn(Ref<'_, CGrafPort>)) {
        let old_port = *self.graf_port.borrow();
        f(self.graf_port.borrow());
        self.graf_port.replace(old_port);
    }

    #[must_use]
    pub fn port(&self) -> &Rc<RefCell<CGrafPort>> {
        &self.graf_port
    }

    pub fn port_mut(&mut self) -> &mut Rc<RefCell<CGrafPort>> {
        &mut self.graf_port
    }
}
