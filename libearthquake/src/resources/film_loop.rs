use anyhow::{Context, Result as AResult};
use bitflags::bitflags;
use crate::ensure_sample;
use libcommon::{Reader, Resource, resource::Input};
use libmactoolbox::Rect;

bitflags! {
    pub struct Flags: u32 {
        /// Crop from the centre of the film loop instead of the top-left corner
        /// when cropping is enabled.
        const CROP_FROM_CENTER    = 0x1;
        /// Scale the film loop instead of cropping when the bounds don’t match
        /// the source stage dimensions.
        const SCALE               = 0x2;
        const MAP_PALETTES        = 0x4;
        /// Enable sound during playback.
        const SOUND_ENABLED       = 0x8;
        /// Enable movie scripts. Only applies to movies, not film loops.
        const ENABLE_SCRIPTS      = 0x10;
        /// Do not loop playback.
        const NO_LOOP             = 0x20;
    }
}

#[derive(Clone, Copy, Debug)]
pub struct Meta {
    bounds: Rect,
    flags: Flags,
    field_14: u16,
}

impl Resource for Meta {
    type Context = ();

    fn load(input: &mut Input<impl Reader>, size: u32, _: &Self::Context) -> AResult<Self> where Self: Sized {
        ensure_sample!(size == 14, "Unexpected film loop meta resource size {} (should be 14)", size);
        let bounds = Rect::load(input, Rect::SIZE, &()).context("Can’t read film loop bounds")?;
        let flags = {
            let value = input.read_u32().context("Can’t read film loop flags")?;
            Flags::from_bits(value).with_context(|| format!("Invalid film loop flags (0x{:x})", value))?
        };
        let field_14 = input.read_u16().context("Can’t read film loop field_14")?;

        Ok(Self {
            bounds,
            flags,
            field_14,
        })
    }
}
