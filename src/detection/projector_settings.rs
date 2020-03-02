#![allow(clippy::struct_excessive_bools)]

use anyhow::{anyhow, Result as AResult};
use crate::{bail_sample, ensure_sample};
use super::projector::{
    MacCPU,
    Platform,
    Version as ProjectorVersion,
    WinVersion,
};

#[derive(Debug)]
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

#[derive(Debug)]
pub enum D3PlatformSettings {
    Mac {
        /// Playback of the first movie will not begin until the mouse is
        /// clicked? TODO: Verify what this actually does. It does not seem to
        /// be exposed to Lingo, so it may not be necessary to store this, since
        /// it is not a desirable behaviour except by user configuration.
        wait_for_click: bool,

        /// The loading strategy used for Accelerator files.
        /// TODO: Is this actually needed? We can pick our own playback
        /// strategy.
        accel_mode: AccelMode,
    },

    Win {
        /// Hide the desktop in windowed mode.
        hide_desktop: bool,
    },
}

#[derive(Debug)]
pub struct D3Settings {
    /// Loop movies instead of exiting after the last movie is finished
    /// playing.
    loop_playback: bool,

    /// Movies are external to the projector instead of being embedded.
    /// TODO: Is this actually needed? We store the movie list separately
    /// in an enum which already says if it is external or not.
    use_external_files: bool,

    /// Platform-specific Director 3 settings.
    per_platform: D3PlatformSettings,
}

impl D3Settings {
    #[must_use]
    pub fn use_external_files(&self) -> bool {
        self.use_external_files
    }
}

#[derive(Debug)]
pub struct D4Settings {
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
    /// TODO: This may not be exposed to Lingo; check the behaviour of the
    /// titleVisible property to see if it exposes this configuration or not. If
    /// it is not exposed, this should not be stored, since title bars should
    /// always exist for windowed projectors.
    show_title_bar: bool,
}

#[derive(Debug)]
pub struct D5Settings {
    /// The projector was created using optimisation which creates duplicate
    /// cast members.
    /// TODO: This is not exposed to Lingo, so it may not be necessary to
    /// store this.
    duplicate_cast: bool,
}

#[derive(Debug)]
pub struct D6Settings {
    /// The movie in the projector has been compressed.
    compressed: bool,

    /// Movie Xtras have been processed and added to the projector.
    has_xtras: bool,

    /// Xtras for connecting to the internet have been added to the
    /// projector.
    has_network_xtras: bool,
}

#[derive(Debug)]
pub enum PerVersionSettings {
    D3(D3Settings),
    D4(D4Settings),
    D5(D4Settings, D5Settings),
    D6(D4Settings, D6Settings),
}

#[derive(Debug)]
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

    /// Director-version—specific settings.
    per_version: PerVersionSettings,
}

impl ProjectorSettings {
    #[must_use]
    pub fn d3(&self) -> Option<&D3Settings> {
        if let PerVersionSettings::D3(settings) = &self.per_version {
            Some(settings)
        } else {
            None
        }
    }

    pub(super) fn parse_mac(version: ProjectorVersion, bits: &[u8]) -> AResult<Self> {
        if version == ProjectorVersion::D3 {
            return Self::parse_d3_mac(bits);
        }

        // Sanity check: these bits cannot normally be changed by an author
        ensure_sample!(bits[0..=1] == [ 0; 2 ], "Unexpected D4+Mac PJst bytes 0-1");
        ensure_sample!(bits[4..=5] == [ 0; 2 ], "Unexpected D4+Mac PJst bytes 4-5");
        ensure_sample!(bits[8] == 0, "Unexpected D4+Mac PJst byte 8");
        match version {
            ProjectorVersion::D3 => unreachable!(),
            ProjectorVersion::D4 => {
                // TODO: This is 0x14 for the post-release D4 and 0x04 in the
                // pre-release D4.
                ensure_sample!(bits[11] & 4 != 0, "Unexpected D4Mac PJst byte 11");
            },
            ProjectorVersion::D5 => {
                // TODO: This flag is 0 in Safecracker
                // ensure_sample!(bits[6] & 8 != 0, "Unexpected D5Mac PJst byte 6");
            },
            ProjectorVersion::D6 => {
                ensure_sample!(bits[6] & 0x24 == 0x24, "Unexpected D6Mac PJst byte 6");
            },
            ProjectorVersion::D7 => todo!(),
        }

        let cpu = if bits[7] == 0 {
            // Pre-release Director 4 with no CPU type seems to be always M68k
            // according to the available corpus
            MacCPU::M68K
        } else {
            MacCPU::from_bits(bits[7])
                .ok_or_else(|| anyhow!("D4+Mac PJst unknown CPU {}", bits[7]))?
        };
        let resize_stage           = bits[11] & 1 != 0;
        let switch_color_depth     = bits[10] & 0x40 != 0;
        let center_stage_on_screen = bits[9] & 1 != 0;
        let play_every_movie       = bits[3] & 1 != 0;
        let play_in_background     = bits[2] & 1 != 0;
        let show_title_bar         = false;
        let platform               = Platform::Mac(cpu);

        Ok(match version {
            ProjectorVersion::D3 => unreachable!(),
            ProjectorVersion::D4 => {
                Self {
                    resize_stage,
                    switch_color_depth,
                    platform,
                    full_screen: false,
                    per_version: PerVersionSettings::D4(D4Settings {
                        center_stage_on_screen,
                        play_every_movie,
                        play_in_background,
                        show_title_bar,
                    }),
                }
            },
            ProjectorVersion::D5 => {
                Self {
                    resize_stage,
                    switch_color_depth,
                    platform,
                    full_screen: bits[6] & 2 != 0,
                    per_version: PerVersionSettings::D5(D4Settings {
                        center_stage_on_screen,
                        play_every_movie,
                        play_in_background,
                        show_title_bar,
                    }, D5Settings {
                        duplicate_cast: bits[6] & 1 != 0,
                    }),
                }
            },
            ProjectorVersion::D6 => {
                Self {
                    resize_stage,
                    switch_color_depth,
                    platform,
                    full_screen: bits[6] & 2 != 0,
                    per_version: PerVersionSettings::D6(D4Settings {
                        center_stage_on_screen,
                        play_every_movie,
                        play_in_background,
                        show_title_bar,
                    }, D6Settings {
                        compressed: bits[6] & 1 != 0,
                        has_xtras: bits[6] & 0x80 != 0,
                        has_network_xtras: bits[6] & 0x40 != 0,
                    }),
                }
            },
            ProjectorVersion::D7 => todo!(),
        })
    }

    pub(crate) fn parse_win(version: ProjectorVersion, platform: Platform, bits: &[u8]) -> AResult<Self> {
        // Sanity check: these bits cannot normally be changed by an author
        match version {
            ProjectorVersion::D3 => {
                // There are no known existing copies of Gaffer so it is
                // impossible to test which bits *might* be set by inspecting
                // the authoring environment
            },
            ProjectorVersion::D4 => {
                ensure_sample!(bits[1..=3] == [ 0; 3 ], "Unexpected D4Win PJ93 bytes 1-3");
                ensure_sample!(bits[6..=11] == [ 0, 0, 0x80, 2, 0xe0, 1 ], "Unexpected D4Win PJ93 bytes 6-11");
            },
            ProjectorVersion::D5 => {
                ensure_sample!(bits[0] & 0x10 != 0, "Unexpected D5Win PJ95 byte 0");
                ensure_sample!(bits[1..=3] == [ 0; 3 ], "Unexpected D5Win PJ95 bytes 1-3");

                // bytes 8-11 are (x, y) for the initial stage window but always
                // seem to be (0, 0) in every sample
                ensure_sample!(bits[5..=11] == [ 0; 7 ], "Unexpected D5Win PJ95 bytes 5-11");
            },
            ProjectorVersion::D6 => {
                ensure_sample!(bits[0] & 0x20 != 0, "Unexpected D6Win PJ95 byte 0");
                ensure_sample!(bits[5..=11] == [ 0; 7 ], "Unexpected D6Win PJ95 bytes 5-11");
            },
            ProjectorVersion::D7 => todo!(),
        }

        Ok(match version {
            ProjectorVersion::D3 => Self {
                resize_stage: false,
                switch_color_depth: false,
                full_screen: bits[2] & 1 == 0,
                platform: Platform::Win(WinVersion::Win3),

                per_version: PerVersionSettings::D3(D3Settings {
                    loop_playback: bits[3] & 1 != 0,
                    use_external_files: bits[5] & 1 != 0,

                    per_platform: D3PlatformSettings::Win {
                        hide_desktop: bits[5] & 4 != 0,
                    },
                }),
            },
            ProjectorVersion::D4 => Self {
                resize_stage:           bits[0] & 4 != 0,
                switch_color_depth:     false,
                full_screen:            bits[0] & 8 != 0,
                platform:               Platform::Win(WinVersion::Win3),
                per_version: PerVersionSettings::D4(D4Settings {
                    center_stage_on_screen: true,
                    play_every_movie:       bits[0] & 1 != 0,
                    play_in_background:     bits[0] & 2 != 0,
                    show_title_bar:         bits[0] & 0x10 != 0,
                }),
            },
            ProjectorVersion::D5 => Self {
                resize_stage:           bits[4] & 4 != 0,
                switch_color_depth:     false,
                full_screen:            bits[0] & 2 != 0,
                platform,
                per_version: PerVersionSettings::D5(D4Settings {
                    center_stage_on_screen: true,
                    play_every_movie:       bits[4] & 1 != 0,
                    play_in_background:     bits[4] & 2 != 0,
                    show_title_bar:         bits[4] & 8 != 0,
                }, D5Settings {
                    duplicate_cast:         bits[0] & 1 != 0,
                }),
            },
            ProjectorVersion::D6 => Self {
                resize_stage:           bits[4] & 4 != 0,
                switch_color_depth:     false,
                full_screen:            bits[0] & 2 != 0,
                platform,
                per_version: PerVersionSettings::D6(D4Settings {
                    center_stage_on_screen: true,
                    play_every_movie:       bits[4] & 1 != 0,
                    play_in_background:     bits[4] & 2 != 0,
                    // different from D5 starting here
                    show_title_bar:         bits[4] & 8 != 0,
                }, D6Settings {
                    compressed:             bits[0] & 1 != 0,
                    // TODO: Other bytes are 0xff when this is enabled; not sure
                    // if this is garbage or actually has significance
                    has_xtras:              bits[0] & 0x80 != 0,
                    has_network_xtras:      bits[0] & 0x40 != 0,
                }),
            },
            ProjectorVersion::D7 => todo!(),
        })
    }

    #[must_use]
    pub fn platform(&self) -> Platform {
        self.platform
    }

    fn parse_d3_mac(bits: &[u8]) -> AResult<Self> {
        // Sanity check: these bits cannot normally be changed by an author
        // This is 1 in GADGET. ensure_sample!(bits[0] == 0, "D3Mac PJst byte 0");
        ensure_sample!(bits[11] == 0, "Unexpected D3Mac PJst byte 11");

        Ok(Self {
            resize_stage:       bits[2] & 1 != 0,
            switch_color_depth: bits[3] & 1 != 0,
            platform:           Platform::Mac(MacCPU::M68K),
            full_screen:        false,
            per_version: PerVersionSettings::D3(D3Settings {
                loop_playback:      bits[1] & 1 != 0,
                use_external_files: bits[4] & 1 != 0,
                per_platform: D3PlatformSettings::Mac {
                    wait_for_click:     bits[5] & 1 == 0,
                    accel_mode:         match bits[10] {
                        1 => AccelMode::FillMemory,
                        2 => AccelMode::Frame,
                        3 => AccelMode::Chunk,
                        _ => bail_sample!("Unknown accel mode {}", bits[10]),
                    },
                },
            }),
        })
    }
}
