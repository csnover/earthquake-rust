use anyhow::{Context, Result as AResult};
use bitflags::bitflags;
use byteordered::{ByteOrdered, Endianness};
use crate::ensure_sample;
use libcommon::{Resource, Reader};
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
    }
}

#[derive(Clone, Copy, Debug)]
pub struct Meta {
    bounds: Rect,
    flags: Flags,
    frame_rate: u8,
}

impl Resource for Meta {
    type Context = ();

    fn load<T: Reader>(input: &mut ByteOrdered<T, Endianness>, size: u32, _: &Self::Context) -> AResult<Self> where Self: Sized {
        ensure_sample!(size == 12, "Unexpected video meta resource size {} (should be 12)", size);
        let bounds = Rect::load(input, Rect::SIZE, &()).context("Can’t read video bounds")?;
        let (flags, frame_rate) = {
            let value = input.read_u32().context("Can’t read video flags")?;
            let flags_value = value & 0xFF_FFFF;
            let flags = Flags::from_bits(flags_value).with_context(|| format!("Invalid video flags (0x{:x})", flags_value))?;
            (flags, (value >> 24) as u8)
        };

        Ok(Self {
            bounds,
            flags,
            frame_rate,
        })
    }
}
