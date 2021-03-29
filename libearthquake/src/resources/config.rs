use binrw::BinRead;
use crate::player::score::Tempo;
use libcommon::{Unk16, Unk32, Unk8, bitflags, bitflags::BitFlags, newtype_num};
use libmactoolbox::quickdraw::{PaletteIndex, Rect};
use std::convert::TryInto;
use smart_default::SmartDefault;
use super::cast::{MemberId, MemberNum};

newtype_num! {
    #[derive(BinRead, Debug)]
    pub struct LegacyTempo(u8);
}

#[derive(BinRead, Clone, Copy, Debug, Eq, PartialEq, SmartDefault)]
#[br(big, repr(i16))]
pub enum Platform {
    #[default]
    Unknown = 0,
    Mac,
    Win,
}

#[derive(BinRead, Clone, Copy, Debug, Eq, Ord, PartialEq, PartialOrd, SmartDefault)]
#[br(big, repr(i16))]
pub enum Version {
    #[default]
    Unknown,

    // D1
    V1023 = 1023,

    // D1 and D2
    V1024 = 1024,

    // D3, but only in the extended version field
    V1025,
    V1028 = 1028,
    V1029,

    // D4
    V1113 = 1113,
    V1114,
    V1115,
    V1116,
    V1117,

    // ?
    V1201 = 1201,
    V1214 = 1214,

    // D5
    V1215 = 1215,
    V1217 = 1217,
    V1222 = 1222,
    V1223,
    V1406 = 1406,
    V5692 = 5692, // protected
}

impl Version {
    #[must_use]
    pub fn d1(self) -> bool {
        matches!(self, Self::V1023 | Self::V1024)
    }

    #[must_use]
    pub fn d2(self) -> bool {
        matches!(self, Self::V1024)
    }

    #[must_use]
    pub fn d3(self) -> bool {
        matches!(self, Self::V1024 | Self::V1025 | Self::V1028 | Self::V1029)
    }

    #[must_use]
    pub fn d4(self) -> bool {
        matches!(self, Self::V1113 | Self::V1114 | Self::V1115 | Self::V1116 | Self::V1117)
    }

    #[must_use]
    pub fn d5(self) -> bool {
        matches!(self, Self::V1201 | Self::V1214 | Self::V1215 | Self::V1217)
    }

    #[must_use]
    pub fn d6(self) -> bool {
        matches!(self, Self::V1222 | Self::V1223)
    }

    #[must_use]
    pub fn d7(self) -> bool {
        matches!(self, Self::V1406)
    }
}

bitflags! {
    #[derive(Default)]
    pub struct Flags: u32 {
        const MOVIE_FIELD_46       = 0x20;
        const PALETTE_MAPPING      = 0x40;
        const LEGACY_FLAG_1        = 0x80;
        const LEGACY_FLAG_2        = 0x100;
        const UPDATE_MOVIE_ENABLED = 0x200;
        const PRELOAD_EVENT_ABORT  = 0x400;
    }
}

#[derive(BinRead, Clone, Copy, Debug, Eq, PartialEq, SmartDefault)]
#[br(big, import(version: Version))]
pub enum PaletteId {
    #[br(pre_assert(version >= Version::V1201))]
    Cast(MemberId),
    #[default]
    Number(i32),
}

#[derive(BinRead, Clone, Copy, Debug, Default)]
#[br(big)]
pub struct Config {
    own_size: i16,
    version: Version,
    rect: Rect,
    min_cast_num: MemberNum,
    max_cast_num: MemberNum,
    legacy_tempo: LegacyTempo,
    // TODO: Fallibly assert 0 or 1
    #[br(map = |b: u8| b != 0)]
    legacy_back_color_is_black: bool,
    field_12: Unk16,
    field_14: Unk16,
    field_16: Unk16,
    field_18: Unk8,
    field_19: Unk8,
    #[br(map = |c: i16| c.try_into().unwrap())]
    #[br(if(version >= Version::V1025))]
    stage_color: PaletteIndex,
    #[br(if(version >= Version::V1025))]
    default_color_depth: i16,
    #[br(if(version >= Version::V1025))]
    field_1e: Unk8,
    #[br(if(version >= Version::V1025))]
    field_1f: Unk8,
    #[br(if(version >= Version::V1025))]
    field_20: Unk32,
    #[br(if(version >= Version::V1025))]
    original_version: Version,
    #[br(if(version >= Version::V1025))]
    max_cast_color_depth: i16,
    #[br(if(version >= Version::V1025))]
    flags: Flags,
    #[br(if(version >= Version::V1025))]
    field_2c: Unk32,
    #[br(if(version >= Version::V1025))]
    field_30: Unk32,
    #[br(if(version >= Version::V1025))]
    field_34: Unk8,
    #[br(if(version >= Version::V1025))]
    field_35: Unk8,
    #[br(if(version >= Version::V1025))]
    current_tempo: Tempo,
    #[br(if(version >= Version::V1025))]
    platform: Platform,
    #[br(if(version >= Version::V1113))]
    field_3a: Unk16,
    #[br(if(version >= Version::V1113))]
    field_3c: Unk32,
    #[br(if(version >= Version::V1113))]
    checksum: u32,
    #[br(if(version >= Version::V1114))]
    field_44: Unk16,
    #[br(if(version >= Version::V1115))]
    field_46: Unk16,
    #[br(if(version >= Version::V1115))]
    max_cast_resource_num: u32,
    #[br(if(version >= Version::V1115), args(version))]
    default_palette: PaletteId,
}

impl Config {
    #[must_use]
    pub fn calculate_checksum(&self) -> u32 {
        ((i32::from(self.own_size) + 1)
        .wrapping_mul(self.version as i32 + 2)
        .wrapping_div(i32::from(self.rect.top) + 3)
        .wrapping_mul(i32::from(self.rect.left) + 4)
        .wrapping_div(i32::from(self.rect.bottom) + 5)
        .wrapping_mul(i32::from(self.rect.right) + 6)
        .wrapping_sub(i32::from(self.min_cast_num.0) + 7)
        .wrapping_mul(i32::from(self.max_cast_num.0) + 8)
        .wrapping_sub(i32::from(self.legacy_tempo.0) + 9)
        .wrapping_sub(i32::from(self.legacy_back_color_is_black) + 10)
        .wrapping_add(i32::from(self.field_12) + 11)
        .wrapping_mul(i32::from(self.field_14) + 12)
        .wrapping_add(i32::from(self.field_16) + 13)
        .wrapping_mul(i32::from(self.field_18) + 14)
        .wrapping_add(i32::from(self.stage_color) + 15)
        .wrapping_add(i32::from(self.default_color_depth) + 16)
        .wrapping_add(i32::from(self.field_1e) + 17)
        .wrapping_mul(i32::from(self.field_1f) + 18)
        .wrapping_add(i32::from(self.field_20) + 19)
        .wrapping_mul(self.original_version as i32 + 20)
        .wrapping_add(i32::from(self.max_cast_color_depth) + 21)
        .wrapping_add(self.flags.bits() as i32 + 22)
        .wrapping_add(i32::from(self.field_2c) + 23)
        .wrapping_add(i32::from(self.field_30) + 24)
        .wrapping_mul(i32::from(self.field_34) + 25)
        .wrapping_add(i32::from(self.current_tempo.to_primitive()) + 26)
        .wrapping_mul(self.platform as i32 + 27)
        .wrapping_mul(
            i32::from(self.field_3a)
            .wrapping_mul(3590)
            .wrapping_sub(0xbb_0000)
        )
        ^ 0x7261_6C66) as u32
    }

    #[must_use]
    pub fn generate_field_3a(flag: bool) -> i16 {
        let (state, a) = Self::field_3a_1(0x123_4567);
        let (_, b) = Self::field_3a_1(state);
        a % 1423 * 23 + if flag {
            0
        } else {
            b % 19 + 1
        }
    }

    #[must_use]
    pub fn min_cast_num(&self) -> MemberNum {
        self.min_cast_num
    }

    #[must_use]
    pub fn original_version(&self) -> Version {
        self.original_version
    }

    #[must_use]
    pub fn valid(&self) -> bool {
        if self.version < Version::V1113 {
            true
        } else {
            self.calculate_checksum() == self.checksum
        }
    }

    #[must_use]
    pub fn version(&self) -> Version {
        if self.version == Version::V5692 {
            self.original_version
        } else {
            self.version
        }
    }

    fn field_3a_1(old_state: i32) -> (i32, i16) {
        let mut state = (old_state % 127_773 * 16807).wrapping_sub(old_state / 127_773 * 2836);
        if state < 0 {
            state += 0x7fff_ffff;
        }
        (state, ((state >> 14) as i16).abs())
    }
}
