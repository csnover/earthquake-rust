use anyhow::{Context, Result as AResult};
use binrw::BinRead;
use libcommon::{bitflags, bitflags::BitFlags, Reader, Resource, resource::Input};
use libmactoolbox::Rect;

bitflags! {
    pub struct Flags: u32 {
        /// Crop from the centre of the video instead of the top-left corner
        /// when cropping is enabled.
        const CROP_FROM_CENTER    = 0x1;
        /// Scale the video instead of cropping when the bounds don’t match
        /// the source video dimensions.
        const SCALE               = 0x2;
        /// Enable sound during playback.
        const SOUND_ENABLED       = 0x8;
        /// Loop playback.
        const LOOP                = 0x10;
        /// Render the video as an overlay on top of all other sprites instead
        /// of compositing it.
        const DIRECT_TO_STAGE     = 0x20;
        /// Show playback controls for the video.
        const SHOW_CONTROLS       = 0x40;
        /// Start the video in a paused state.
        const PAUSED_AT_START     = 0x100;
        /// Play only the audio part of the video.
        const HIDE_VIDEO          = 0x200;
        /// Preload the video from disk into memory instead of streaming.
        const PRELOAD             = 0x400;
        /// Ignore the natural frame rate of the video and play every frame
        /// without sound. Without this flag, the video will play in sync with
        /// audio and may frame skip to keep AV sync.
        const PLAY_EVERY_FRAME    = 0x800;
        /// Play back the video as quickly as possible.
        const FRAME_RATE_MAXIMUM  = 0x1000;
        /// Play back the video at a specific frame rate.
        const FRAME_RATE_FIXED    = 0x2000;
        /// The video is in a Windows video format.
        const VIDEO_KIND_AVI      = 0x4000;
        /// The video is in an invalid video format.
        const VIDEO_KIND_NULL     = 0x8000;
        /// The frame rate to use when the `PLAY_EVERY_FRAME` flag is set.
        const FRAME_RATE          = 0xFF00_0000;
    }
}

#[derive(BinRead, Clone, Copy)]
#[br(big, import(size: u32), pre_assert(size == 12))]
pub struct Meta {
    bounds: Rect,
    flags: Flags,
}

impl Meta {
    #[must_use]
    pub fn frame_rate(&self) -> u8 {
        ((self.flags.bits() & Flags::FRAME_RATE.bits()) >> 24) as u8
    }
}

impl std::fmt::Debug for Meta {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct(std::any::type_name::<Self>())
            .field("bounds", &self.bounds)
            .field("flags", &self.flags)
            .field("(frame_rate)", &self.frame_rate())
            .finish()
    }
}

impl Resource for Meta {
    type Context = ();

    fn load(input: &mut Input<impl Reader>, size: u32, _: &Self::Context) -> AResult<Self> where Self: Sized {
        Self::read_args(input, (size, )).context("Can’t read video meta")
    }
}
