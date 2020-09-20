use anyhow::{Context, Result as AResult};
use bitflags::bitflags;
use byteordered::{ByteOrdered, Endianness};
use crate::ensure_sample;
use libcommon::{encodings::DecoderRef, Reader, Resource};
use num_derive::FromPrimitive;
use num_traits::FromPrimitive;
use super::{
    config::Version as ConfigVersion,
    xtra::Meta as XtraMeta,
};

#[derive(Clone, Copy, Debug, Eq, FromPrimitive, PartialEq)]
pub enum Kind {
    Xtra,
    WipeRight,
    WipeLeft,
    WipeDown,
    WipeUp,
    CenterOutHoriz,
    EdgesInHoriz,
    CenterOutVert,
    EdgesInVert,
    CenterOutSquare,
    EdgesInSquare,
    PushLeft,
    PushRight,
    PushDown,
    PushUp,
    RevealUp,
    RevealUpRight,
    RevealRight,
    RevealDownRight,
    RevealDown,
    RevealDownLeft,
    RevealLeft,
    RevealUpLeft,
    DissolvePixelsFast,
    DissolveBoxyRects,
    DissolveBoxySquares,
    DissolvePatterns,
    RandomRows,
    RandomColumns,
    CoverDown,
    CoverDownLeft,
    CoverDownRight,
    CoverLeft,
    CoverRight,
    CoverUp,
    CoverUpLeft,
    CoverUpRight,
    VenetianBlinds,
    Checkerboard,
    StripsBottomBuildLeft,
    StripsBottomBuildRight,
    StripsLeftBuildDown,
    StripsLeftBuildUp,
    StripsRightBuildDown,
    StripsRightBuildUp,
    StripsTopBuildLeft,
    StripsTopBuildRight,
    ZoomOpen,
    ZoomClose,
    VerticalBlinds,
    DissolveBitsFast,
    DissolvePixels,
    DissolveBits,
}

bitflags! {
    pub struct Flags: u8 {
        /// Transition over the entire stage instead of just the changing area.
        const ENTIRE_STAGE = 1;
        /// Not an Xtra transition.
        const STANDARD     = 2;
    }
}

#[derive(Clone, Copy, Debug, Eq, Ord, PartialEq, PartialOrd)]
pub struct QuarterSeconds(pub u8);

#[derive(Clone, Copy, Debug, Eq, Ord, PartialEq, PartialOrd)]
pub struct Milliseconds(pub i16);

#[derive(Clone, Copy, Debug)]
pub struct StandardMeta {
    legacy_duration: QuarterSeconds,
    chunk_size: u8,
    kind: Kind,
    flags: Flags,
    duration: Milliseconds,
}

#[derive(Clone, Debug)]
pub enum Meta {
    Standard(StandardMeta),
    Xtra(StandardMeta, XtraMeta),
}

impl Resource for Meta {
    type Context = (ConfigVersion, DecoderRef);

    fn load<T: Reader>(input: &mut ByteOrdered<T, Endianness>, size: u32, context: &Self::Context) -> AResult<Self> where Self: Sized {
        ensure_sample!(size >= 4, "Unexpected transition meta resource size {} (should be at least 4)", size);
        let legacy_duration = QuarterSeconds(input.read_u8().context("Can’t read transition maybe legacy duration")?);
        let chunk_size = input.read_u8().context("Can’t read transition chunk size")?;
        ensure_sample!(chunk_size > 0 && chunk_size <= 128, "Unexpected transition chunk size {}", chunk_size);
        let kind = {
            let value = input.read_u8().context("Can’t read transition kind")?;
            Kind::from_u8(value).with_context(|| format!("Invalid transition kind {}", value))?
        };
        let flags = {
            let value = input.read_u8().context("Can’t read transition flags")?;
            Flags::from_bits(value).with_context(|| format!("Invalid transition flags (0x{:x})", value))?
        };
        let duration = Milliseconds(if context.0 < ConfigVersion::V1214 {
            i16::from(legacy_duration.0) * 15
        } else {
            input.read_i16().context("Can’t read transition duration")?
        });

        let standard_meta = StandardMeta {
            legacy_duration,
            chunk_size,
            kind,
            flags,
            duration,
        };

        Ok(if flags.contains(Flags::STANDARD) {
            Self::Standard(standard_meta)
        } else {
            Self::Xtra(
                standard_meta,
                XtraMeta::load(input, size - 6, context)
                    .context("Can’t load transition Xtra metadata")?
            )
        })
    }
}
