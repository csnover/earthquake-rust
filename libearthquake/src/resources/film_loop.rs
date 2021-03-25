use binrw::BinRead;
use libcommon::{Unk16, bitflags};
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

#[derive(BinRead, Clone, Copy, Debug)]
#[br(big, import(size: u32), pre_assert(size == 14))]
pub struct Meta {
    bounds: Rect,
    flags: Flags,
    field_14: Unk16,
}
