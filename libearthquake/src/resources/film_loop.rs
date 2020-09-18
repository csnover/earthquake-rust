use anyhow::{Context, Result as AResult};
use bitflags::bitflags;
use byteordered::{ByteOrdered, Endianness};
use crate::{assert_sample, ensure_sample};
use libcommon::{Resource, Reader};
use libmactoolbox::{Point, Rect};
use num_derive::FromPrimitive;
use num_traits::FromPrimitive;
use super::cast::MemberId;

bitflags! {
    pub struct Flags: u32 {
        /// Crop from the centre of the video instead of the top-left corner
        /// when cropping is enabled.
        const CROP_FROM_CENTER    = 0x1;
        /// Crop the video instead of scaling it when the bounds don’t match
        /// the source video dimensions.
        const CROP                = 0x2;
        const MAP_PALETTES        = 0x4;
        /// Enable sound during playback.
        const SOUND_ENABLED       = 0x8;
        /// Loop playback.
        const LOOP                = 0x20;
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

    fn load<T: Reader>(input: &mut ByteOrdered<T, Endianness>, size: u32, _: &Self::Context) -> AResult<Self> where Self: Sized {
        ensure_sample!(size == 14, "Unexpected film loop meta resource size {} (should be 14)", size);
        let bounds = Rect::load(input, Rect::SIZE, &()).context("Can’t read film loop bounds")?;
        let flags = {
            let value = input.read_u32().context("Can’t read film loop flags")?;
            Flags::from_bits(value).with_context(|| format!("Invalid flags 0x{:x} for film loop", value))?
        };
        let field_14 = input.read_u16().context("Can’t read film loop field_14")?;

        Ok(Self {
            bounds,
            flags,
            field_14,
        })
    }
}
