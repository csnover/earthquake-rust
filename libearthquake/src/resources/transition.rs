use binrw::BinRead;
use libcommon::{bitflags, bitflags::BitFlags};
use num_derive::FromPrimitive;
use super::{
    config::Version as ConfigVersion,
    xtra::Properties as XtraProps,
};

#[derive(BinRead, Clone, Copy, Debug, Eq, FromPrimitive, PartialEq)]
#[br(repr(u8))]
pub(crate) enum Kind {
    Xtra = 0,
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
    pub(super) struct Flags: u8 {
        /// Transition over the entire stage instead of just the changing area.
        const ENTIRE_STAGE = 1;
        /// Not an Xtra transition.
        const STANDARD     = 2;
    }
}

#[derive(BinRead, Clone, Copy, Debug, Eq, Ord, PartialEq, PartialOrd)]
pub(crate) struct QuarterSeconds(pub(crate) u8);

#[derive(BinRead, Clone, Copy, Debug, Default, Eq, Ord, PartialEq, PartialOrd)]
pub(crate) struct Milliseconds(i16);

#[derive(BinRead, Clone, Debug)]
#[br(big, import(size: u32, version: ConfigVersion))]
// D5 checks for `size >= 4` and then later does a `version >= 1214` check to
// decide whether to read bytes 4–5, but we don’t do that because that check
// appears to be broken:
// The code path for `version < 1214` converts `legacy_duration` to `duration`
// by `* 15`, but this doesn’t make sense since that would be a conversion from
// quarter seconds to ticks, not quarter-seconds to milliseconds. For this to be
// correct, `legacy_duration` would need to be stored in units of 66.6̅ms, which
// would be pretty weird since that’s a crazy time base, it was ¼s in D4, and
// the conversion is perfect for converting ¼s… to ticks. I have no samples of
// files which would follow this code path to see what data is actually stored
// there.
#[br(pre_assert(size >= 6 && version >= ConfigVersion::V1214))]
pub(super) struct Properties {
    legacy_duration: QuarterSeconds,
    #[br(assert(chunk_size > 0 && chunk_size <= 128))]
    chunk_size: u8,
    kind: Kind,
    flags: Flags,
    duration: Milliseconds,
    #[br(args(size - 6), if(!flags.contains(Flags::STANDARD)))]
    xtra: Option<XtraProps>,
}
