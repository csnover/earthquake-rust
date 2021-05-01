use binrw::{BinRead, error::Context, io::{Read, Seek}};
use crate::resources::cast::{LibNum, MemberId, MemberNum};
use derive_more::{Deref, DerefMut, From};
use libcommon::{Unk8, bitflags, newtype_num, restore_on_error};
use libmactoolbox::quickdraw::PaletteIndex;
use super::{Fps, Version};

bitflags! {
    #[derive(Default)]
    pub(super) struct PaletteFlags: u8 {
        /// The palette transition will occur during the playback of the score,
        /// instead of in between frames of the score.
        const SPAN_FRAMES = 4;

        const FLAG_8 = 8;

        /// Palette cycling will ping-pong instead of looping.
        const CYCLE_AUTO_REVERSE = 0x10;

        /// Fade to black.
        const FADE_REVERSE = 0x20;

        /// Fade (to white).
        const FADE = 0x40;

        /// Do palette cycling instead of palette transitioning.
        const ACTION_CYCLE = 0x80;
    }
}

newtype_num! {
    #[derive(BinRead, Debug)]
    pub(super) struct SignedPaletteIndex(i8);
}

impl From<SignedPaletteIndex> for PaletteIndex {
    fn from(value: SignedPaletteIndex) -> Self {
        ((i16::from(value.0) + 128) as u8).into()
    }
}

#[derive(Clone, Copy, Debug, Default, Deref, DerefMut, From)]
#[from(forward)]
pub(crate) struct Palette(PaletteV5);

impl Palette {
    pub(crate) const SYSTEM_LIB: LibNum = LibNum::from_raw(-1);
    pub(crate) const SYSTEM_MAC: MemberId = MemberId::from_raw(-1, -1);
    pub(crate) const RAINBOW: MemberId = MemberId::from_raw(-1, -2);
    pub(crate) const GRAYSCALE: MemberId = MemberId::from_raw(-1, -3);
    pub(crate) const PASTELS: MemberId = MemberId::from_raw(-1, -4);
    pub(crate) const VIVID: MemberId = MemberId::from_raw(-1, -5);
    pub(crate) const NTSC: MemberId = MemberId::from_raw(-1, -6);
    pub(crate) const METALLIC: MemberId = MemberId::from_raw(-1, -7);
    pub(crate) const VGA: MemberId = MemberId::from_raw(-1, -8);
    pub(crate) const SYSTEM_WIN_DIR_4: MemberId = MemberId::from_raw(-1, -101);
    pub(crate) const SYSTEM_WIN: MemberId = MemberId::from_raw(-1, -102);
    pub(crate) const NOTHING: MemberId = MemberId::from_raw(-1, -200);

    pub(crate) const fn system_default() -> MemberId {
        // TODO: This should be SYSTEM_MAC for Macintosh
        Self::SYSTEM_WIN_DIR_4
    }
}

impl BinRead for Palette {
    type Args = (Version, );

    fn read_options<R: Read + Seek>(input: &mut R, options: &binrw::ReadOptions, (version, ): Self::Args) -> binrw::BinResult<Self> {
        restore_on_error(input, |input, _| {
            let mut options = *options;
            options.endian = binrw::Endian::Big;

            if version > Version::V7 {
                todo!("Score palette version 8 parsing")
            } else if version >= Version::V5 {
                PaletteV5::read_options(input, &options, ()).map(Self::from)
            } else {
                PaletteV4::read_options(input, &options, (version, )).map(Self::from)
            }.context(|| "Can’t read score palette")
        })
    }
}

#[derive(BinRead, Clone, Copy, Debug, Default)]
#[br(big)]
pub(crate) struct PaletteV5 {
    id: MemberId,
    #[br(map = |num: i8| num.into())]
    rate: Fps,
    flags: PaletteFlags,
    cycle_start_color: SignedPaletteIndex,
    cycle_end_color: SignedPaletteIndex,
    num_frames: i16,
    num_cycles: i16,
    field_c: Unk8,
    field_d: Unk8,
    field_e: Unk8,
    field_f: Unk8,
}

#[derive(BinRead, Clone, Copy, Debug, Default)]
#[br(big, import(version: Version))]
pub(super) struct PaletteV4 {
    id: MemberNum,
    cycle_start_color: SignedPaletteIndex,
    cycle_end_color: SignedPaletteIndex,
    flags: PaletteFlags,
    #[br(map = |num: i8| num.into())]
    rate: Fps,
    num_frames: i16,
    num_cycles: i16,
    field_c: Unk8,
    field_d: Unk8,
    field_e: Unk8,
    #[br(pad_before(if version == Version::V4 { 5 } else { 2 }))]
    field_f: Unk8,
}

impl From<PaletteV4> for PaletteV5 {
    fn from(old: PaletteV4) -> Self {
        Self {
            id: old.id.into(),
            cycle_start_color: old.cycle_start_color,
            cycle_end_color: old.cycle_end_color,
            flags: old.flags,
            rate: old.rate,
            num_frames: old.num_frames,
            num_cycles: old.num_cycles,
            field_c: old.field_c,
            field_d: old.field_d,
            field_e: old.field_e,
            field_f: old.field_f,
        }
    }
}
