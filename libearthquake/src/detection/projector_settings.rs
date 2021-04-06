#![allow(clippy::struct_excessive_bools)]

use anyhow::{Context, Result as AResult};
use binrw::BinRead;
use crate::{bail_sample, ensure_sample};
use libcommon::{prelude::*, io::prelude::*};
use super::{
    projector::{
        MacCPU,
        Platform,
        WinVersion,
    },
    Version,
};

#[derive(Clone, Copy, Debug)]
pub struct ProjectorSettings {
    /// Resize the stage when a new movie plays instead of keeping the
    /// stage the same size as the first movie.
    resize_stage: bool,

    /// Change the system colour depth. We do not do this, but this value is
    /// exposed by Lingo, so it has to be stored for compatibility.
    switch_color_depth: bool,

    /// The original platform of the projector. This value is exposed by Lingo,
    /// so it has to be stored for compatibility.
    platform: Platform,

    /// Run the projector in full screen mode by default.
    /// TODO: Probably we want to ignore this setting and let the user choose,
    /// since this is not exposed by Lingo.
    full_screen: bool,

    /// Loop movies instead of exiting after the last movie is finished
    /// playing.
    ///
    /// Used only by Director 3.
    loop_playback: bool,

    /// Playback of the first movie will not begin until the mouse is
    /// clicked? TODO: Verify what this actually does. It does not seem to
    /// be exposed to Lingo, so it may not be necessary to store this, since
    /// it is not a desirable behaviour except by user configuration.
    ///
    /// Used only by Director 3.
    wait_for_click: bool,

    /// Movies are external to the projector instead of being embedded.
    /// TODO: Is this actually needed? We store the movie list separately
    /// in an enum which already says if it is external or not.
    ///
    /// Used only by Director 3.
    use_external_files: bool,

    /// The number of movies in the projector.
    ///
    /// Used only by Director 3.
    num_movies: u16,

    // TODO: Seems like some pre-release version of Director 4
    // created projectors with no PJxx in the data fork. In these
    // ones the CPU flag appears to always be zero. Then before GM
    // they added the CPU flag and PJxx in the data fork. Based on
    // the corresponding structure in the Windows projectors, this
    // extra data is probably:
    //
    // 4 - "PJxx"
    // 4 - RIFF offset
    //
    // and then different by version:
    //
    // D4 (PJ93):
    // 4x9 - fixed driver offsets?
    // <PPC executable>
    //
    // D5+ (PJ95, PJ97, PJ00, etc.):
    // 4  - num drivers
    // 4  - num drivers to skip
    // .. - drivers
    // <PPC executable>
    /// Used only in Director 4 Mac?
    has_extended_data_fork: bool,

    /// Center the stage on the screen instead of putting it at the top-left
    /// corner. This value is exposed by Lingo, so it has to be stored for
    /// compatibility.
    ///
    /// New in Director 4.
    center_stage_on_screen: bool,

    /// All movies in the playlist of the projector will be played in sequence,
    /// instead of only the first movie.
    ///
    /// New in Director 4.
    play_every_movie: bool,

    /// The projector was created using optimisation which creates duplicate
    /// cast members.
    /// TODO: This is not exposed to Lingo, so it may not be necessary to
    /// store this.
    ///
    /// Used only in Director 5.
    duplicate_cast: bool,

    /// The movie in the projector has been compressed.
    ///
    /// New in Director 6.
    compressed: bool,

    /// Movie Xtras have been processed and added to the projector.
    ///
    /// New in Director 6.
    has_xtras: bool,

    /// Xtras for connecting to the internet have been added to the
    /// projector.
    ///
    /// New in Director 6.
    has_network_xtras: bool,
}

impl ProjectorSettings {
    #[must_use]
    pub fn has_extended_data_fork(&self) -> bool {
        self.has_extended_data_fork
    }

    #[must_use]
    pub fn num_movies(&self) -> u16 {
        self.num_movies
    }

    #[must_use]
    pub fn platform(&self) -> Platform {
        self.platform
    }

    #[must_use]
    pub fn use_external_files(&self) -> bool {
        self.use_external_files
    }
}

impl BinRead for ProjectorSettings {
    type Args = (Version, Platform);

    fn read_options<R: binrw::io::Read + binrw::io::Seek>(
        reader: &mut R,
        _: &binrw::ReadOptions,
        (version, platform): Self::Args,
    ) -> binrw::BinResult<Self> {
        restore_on_error(reader, |reader, pos| {
            let mut bits = Vec::with_capacity(reader.bytes_left()?.unwrap_into());
            reader.read_to_end(&mut bits)?;

            match version {
                Version::D3 => if matches!(platform, Platform::Mac(..)) {
                    D3Settings::from_bits_mac(&bits)
                } else {
                    D3Settings::from_bits_win(&bits)
                }.map(Self::from),
                Version::D4 | Version::D5 | Version::D6 => match platform {
                    platform @ Platform::Win(..) => D6Settings::from_bits_win(&bits, version, platform),
                    Platform::Mac(..) => D6Settings::from_bits_mac(&bits, version)
                }.map(Self::from),
                Version::D7 => todo!("D7 projector settings parser"),
            }
            .map_err(|err| binrw::Error::Custom {
                pos,
                err: Box::new(err),
            })
        })
    }
}

#[derive(Clone, Copy, Debug)]
/// The strategy used when there is not enough memory to load an accelerator
/// into memory.
/// TODO: The existence of this configuration option makes absolutely no sense
/// to me. Why would you ever not want to load in chunks?
pub enum AccelMode {
    /// Play only the part of the accelerator which fits in memory.
    FillMemory,

    /// Load into memory frame by frame.
    Frame,

    /// Load into memory in chunks.
    Chunk,
}

#[derive(Clone, Copy, Debug)]
pub struct D3Settings {
    /// Resize the stage when a new movie plays instead of keeping the
    /// stage the same size as the first movie.
    resize_stage: bool,

    /// Change the system colour depth. We do not do this, but this value is
    /// exposed by Lingo, so it has to be stored for compatibility.
    switch_color_depth: bool,

    /// The original platform of the projector. This value is exposed by Lingo,
    /// so it has to be stored for compatibility.
    platform: Platform,

    /// Run the projector in full screen mode by default.
    /// TODO: Probably we want to ignore this setting and let the user choose,
    /// since this is not exposed by Lingo.
    full_screen: bool,

    /// Loop movies instead of exiting after the last movie is finished
    /// playing.
    loop_playback: bool,

    /// Movies are external to the projector instead of being embedded.
    /// TODO: Is this actually needed? We store the movie list separately
    /// in an enum which already says if it is external or not.
    use_external_files: bool,

    /// The number of movies embedded in the projector.
    num_movies: u16,

    /// Playback of the first movie will not begin until the mouse is
    /// clicked?
    ///
    /// Mac only.
    /// TODO: Verify what this actually does. It does not seem to
    /// be exposed to Lingo, so it may not be necessary to store this, since
    /// it is not a desirable behaviour except by user configuration.
    wait_for_click: bool,

    /// The loading strategy used for Accelerator files.
    ///
    /// Mac only.
    /// TODO: Is this actually needed? We can pick our own playback
    /// strategy.
    accel_mode: AccelMode,

    /// Hide the desktop in windowed mode.
    ///
    /// Windows only.
    hide_desktop: bool,
}

impl From<D3Settings> for ProjectorSettings {
    fn from(other: D3Settings) -> Self {
        Self {
            resize_stage: other.resize_stage,
            switch_color_depth: other.switch_color_depth,
            platform: other.platform,
            full_screen: other.full_screen,
            loop_playback: other.loop_playback,
            use_external_files: other.use_external_files,
            num_movies: other.num_movies,
            wait_for_click: other.wait_for_click,
            has_extended_data_fork: false,
            center_stage_on_screen: false,
            play_every_movie: true,
            duplicate_cast: false,
            compressed: false,
            has_xtras: false,
            has_network_xtras: false,
        }
    }
}

impl D3Settings {
    fn from_bits_mac(bits: &[u8]) -> AResult<Self> {
        // Sanity check: these bits cannot normally be changed by an author
        // This is 1 in GADGET. ensure_sample!(bits[0] == 0, "D3Mac PJst byte 0");
        ensure_sample!(bits[11] == 0, "Unexpected D3Mac PJst byte 11");

        Ok(Self {
            resize_stage:       bits[2] & 1 != 0,
            switch_color_depth: bits[3] & 1 != 0,
            platform:           Platform::Mac(MacCPU::M68K),
            full_screen:        false,
            loop_playback:      bits[1] & 1 != 0,
            use_external_files: bits[4] & 1 != 0,
            num_movies:         u16::from_be_bytes((&bits[6..8]).unwrap_into()),
            wait_for_click:     bits[5] & 1 == 0,
            accel_mode:         match bits[10] {
                1 => AccelMode::FillMemory,
                2 => AccelMode::Frame,
                3 => AccelMode::Chunk,
                mode => bail_sample!("Unknown accel mode {}", mode),
            },
            hide_desktop: false,
        })
    }

    fn from_bits_win(bits: &[u8]) -> AResult<Self> {
        Ok(Self {
            resize_stage:       false,
            switch_color_depth: false,
            full_screen:        bits[2] & 1 == 0,
            platform:           Platform::Win(WinVersion::Win3),
            loop_playback:      bits[3] & 1 != 0,
            use_external_files: bits[5] & 1 != 0,
            num_movies:         u16::from_le_bytes((&bits[0..2]).unwrap_into()),
            hide_desktop:       bits[5] & 4 != 0,
            wait_for_click:     false,
            accel_mode:         AccelMode::Chunk,
        })
    }
}

#[derive(Clone, Copy, Debug)]
pub struct D6Settings {
    /// Resize the stage when a new movie plays instead of keeping the
    /// stage the same size as the first movie.
    resize_stage: bool,

    /// Change the system colour depth. We do not do this, but this value is
    /// exposed by Lingo, so it has to be stored for compatibility.
    switch_color_depth: bool,

    /// The original platform of the projector. This value is exposed by Lingo,
    /// so it has to be stored for compatibility.
    platform: Platform,

    /// Run the projector in full screen mode by default.
    /// TODO: Probably we want to ignore this setting and let the user choose,
    /// since this is not exposed by Lingo.
    full_screen: bool,

    /// Center the stage on the screen instead of putting it at the top-left
    /// corner. This value is exposed by Lingo, so it has to be stored for
    /// compatibility.
    center_stage_on_screen: bool,

    /// All movies in the playlist of the projector will be played in sequence,
    /// instead of only the first movie.
    play_every_movie: bool,

    /// Continue playing the movie when the window is not focused.
    /// TODO: This is not exposed to Lingo and is something that, at most, users
    /// should be specifying instead of the projector.
    play_in_background: bool,

    /// Show the title bar of the movie in windowed mode.
    ///
    /// Windows only.
    /// TODO: This may not be exposed to Lingo; check the behaviour of the
    /// titleVisible property to see if it exposes this configuration or not. If
    /// it is not exposed, this should not be stored, since title bars should
    /// always exist for windowed projectors.
    show_title_bar: bool,

    /// Added in Mac Director 4.
    has_extended_data_fork: bool,

    /// The projector was created using optimisation which creates duplicate
    /// cast members.
    ///
    /// Used only by Director 5.
    /// TODO: This is not exposed to Lingo, so it may not be necessary to
    /// store this.
    duplicate_cast: bool,

    /// The movie in the projector has been compressed.
    ///
    /// Added in Director 6.
    compressed: bool,

    /// Movie Xtras have been processed and added to the projector.
    ///
    /// Added in Director 6.
    has_xtras: bool,

    /// Xtras for connecting to the internet have been added to the
    /// projector.
    ///
    /// Added in Director 6.
    has_network_xtras: bool,
}

impl From<D6Settings> for ProjectorSettings {
    fn from(other: D6Settings) -> Self {
        Self {
            resize_stage: other.resize_stage,
            switch_color_depth: other.switch_color_depth,
            platform: other.platform,
            full_screen: other.full_screen,
            loop_playback: false,
            use_external_files: false,
            num_movies: 0,
            wait_for_click: false,
            has_extended_data_fork: other.has_extended_data_fork,
            center_stage_on_screen: other.center_stage_on_screen,
            play_every_movie: other.play_every_movie,
            duplicate_cast: other.duplicate_cast,
            compressed: other.compressed,
            has_xtras: other.has_xtras,
            has_network_xtras: other.has_network_xtras,
        }
    }
}

impl D6Settings {
    fn from_bits_mac(bits: &[u8], version: Version) -> AResult<Self> {
        // Sanity check: these bits cannot normally be changed by an author
        // (but may be different for Education editions)
        ensure_sample!(bits[0..=1] == [ 0; 2 ], "Unexpected D4+Mac PJst bytes 0-1");
        ensure_sample!(bits[4..=5] == [ 0; 2 ], "Unexpected D4+Mac PJst bytes 4-5");
        ensure_sample!(bits[8] == 0, "Unexpected D4+Mac PJst byte 8");
        match version {
            Version::D4 => {
                // TODO: This is 0x14 for the post-release D4 and 0x04 in the
                // pre-release D4.
                ensure_sample!(bits[11] & 4 != 0, "Unexpected D4Mac PJst byte 11");
            },
            Version::D5 => {
                // TODO: This flag is 0 in Safecracker
                // ensure_sample!(bits[6] & 8 != 0, "Unexpected D5Mac PJst byte 6");
            },
            Version::D6 => {
                ensure_sample!(bits[6] & 0x24 == 0x24, "Unexpected D6Mac PJst byte 6");
            },
            Version::D3 | Version::D7 => unreachable!(),
        }

        let cpu = if bits[7] == 0 {
            // Pre-release Director 4 with no CPU type seems to be always M68k
            // according to the available corpus
            MacCPU::M68K
        } else {
            MacCPU::from_bits(bits[7])
                .with_context(|| format!("D4+Mac PJst unknown CPU {}", bits[7]))?
        };
        let resize_stage           = bits[11] & 1 != 0;
        let switch_color_depth     = bits[10] & 0x40 != 0;
        let center_stage_on_screen = bits[9] & 1 != 0;
        let play_every_movie       = bits[3] & 1 != 0;
        let play_in_background     = bits[2] & 1 != 0;
        let show_title_bar         = false;
        let platform               = Platform::Mac(cpu);
        let has_extended_data_fork = bits[7] != 0;
        let full_screen;
        let duplicate_cast;
        let compressed;
        let has_xtras;
        let has_network_xtras;

        match version {
            Version::D4 => {
                full_screen = false;
                duplicate_cast = false;
                compressed = false;
                has_xtras = false;
                has_network_xtras = false;
            },
            Version::D5 => {
                full_screen = bits[6] & 2 != 0;
                duplicate_cast = bits[6] & 1 != 0;
                compressed = false;
                has_xtras = false;
                has_network_xtras = false;
            },
            Version::D6 => {
                full_screen = bits[6] & 2 != 0;
                duplicate_cast = false;
                compressed = bits[6] & 1 != 0;
                has_xtras = bits[6] & 0x80 != 0;
                has_network_xtras = bits[6] & 0x40 != 0;
            },
            Version::D3 | Version::D7 => unreachable!(),
        }

        Ok(Self {
            resize_stage,
            switch_color_depth,
            platform,
            full_screen,
            center_stage_on_screen,
            play_every_movie,
            play_in_background,
            has_extended_data_fork,
            show_title_bar,
            duplicate_cast,
            compressed,
            has_xtras,
            has_network_xtras,
        })
    }

    fn from_bits_win(bits: &[u8], version: Version, platform: Platform) -> AResult<Self> {
        // Sanity check: these bits cannot normally be changed by an author
        match version {
            Version::D3 => {
                // There are no known existing copies of Gaffer so it is
                // impossible to test which bits *might* be set by inspecting
                // the authoring environment
            },
            Version::D4 => {
                ensure_sample!(bits[1..=3] == [ 0; 3 ], "Unexpected D4Win PJ93 bytes 1-3");
                ensure_sample!(bits[6..=11] == [ 0, 0, 0x80, 2, 0xe0, 1 ], "Unexpected D4Win PJ93 bytes 6-11");
            },
            Version::D5 => {
                ensure_sample!(bits[0] & 0x10 != 0, "Unexpected D5Win PJ95 byte 0");
                ensure_sample!(bits[1..=3] == [ 0; 3 ], "Unexpected D5Win PJ95 bytes 1-3");

                // bytes 8-11 are (x, y) for the initial stage window but always
                // seem to be (0, 0) in every sample
                ensure_sample!(bits[5..=11] == [ 0; 7 ], "Unexpected D5Win PJ95 bytes 5-11");
            },
            Version::D6 => {
                ensure_sample!(bits[0] & 0x20 != 0, "Unexpected D6Win PJ95 byte 0");
                ensure_sample!(bits[5..=11] == [ 0; 7 ], "Unexpected D6Win PJ95 bytes 5-11");
            },
            Version::D7 => unreachable!(),
        }

        Ok(match version {
            Version::D4 => Self {
                resize_stage:           bits[0] & 4 != 0,
                switch_color_depth:     false,
                full_screen:            bits[0] & 8 != 0,
                platform:               Platform::Win(WinVersion::Win3),
                center_stage_on_screen: true,
                play_every_movie:       bits[0] & 1 != 0,
                play_in_background:     bits[0] & 2 != 0,
                has_extended_data_fork: false,
                show_title_bar:         bits[0] & 0x10 != 0,
                duplicate_cast:         false,
                compressed:             false,
                has_xtras:              false,
                has_network_xtras:      false,
            },
            Version::D5 => Self {
                resize_stage:           bits[4] & 4 != 0,
                switch_color_depth:     false,
                full_screen:            bits[0] & 2 != 0,
                platform,
                center_stage_on_screen: true,
                play_every_movie:       bits[4] & 1 != 0,
                play_in_background:     bits[4] & 2 != 0,
                has_extended_data_fork: false,
                show_title_bar:         bits[4] & 8 != 0,
                duplicate_cast:         bits[0] & 1 != 0,
                compressed:             false,
                has_xtras:              false,
                has_network_xtras:      false,
            },
            Version::D6 => Self {
                resize_stage:           bits[4] & 4 != 0,
                switch_color_depth:     false,
                full_screen:            bits[0] & 2 != 0,
                platform,
                center_stage_on_screen: true,
                play_every_movie:       bits[4] & 1 != 0,
                play_in_background:     bits[4] & 2 != 0,
                has_extended_data_fork: false,
                // different from D5 starting here
                duplicate_cast:         false,
                show_title_bar:         bits[4] & 8 != 0,
                compressed:             bits[0] & 1 != 0,
                // TODO: Other bytes are 0xff when this is enabled; not sure
                // if this is garbage or actually has significance
                has_xtras:              bits[0] & 0x80 != 0,
                has_network_xtras:      bits[0] & 0x40 != 0,
            },
            Version::D3 | Version::D7 => todo!("D7Win projector settings parser"),
        })
    }
}
