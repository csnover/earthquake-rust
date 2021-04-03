use binrw::{BinRead, io::{Read, Seek, SeekFrom}};
use crate::resources::cast::{MemberId, MemberNum};
use derive_more::{Deref, DerefMut, From};
use libcommon::{Unk16, Unk8, restore_on_error};
use smart_default::SmartDefault;
use super::{NUM_SPRITES, Palette, Sprite, Tempo, Transition, Version, palette::PaletteV4, sprite::{SpriteV3, SpriteV4}};

#[derive(Clone, Debug, Default, Deref, DerefMut, From)]
#[from(forward)]
pub struct Frame(FrameV5);

impl Frame {
    pub(super) const V4_CELL_COUNT: u16 = 48;
    pub(super) const V0_SIZE_IN_CELLS: u16 = 50;
    pub(super) const V5_CELL_COUNT: u16 = 48;
    pub(super) const V5_SIZE_IN_CELLS: u16 = 50;
    pub(super) const V5_SIZE: u16 = Sprite::V5_SIZE * Self::V5_SIZE_IN_CELLS;
}

impl BinRead for Frame {
    type Args = (Version, );

    fn read_options<R: Read + Seek>(
        reader: &mut R,
        options: &binrw::ReadOptions,
        args: Self::Args,
    ) -> binrw::BinResult<Self> {
        let (version, ) = args;
        restore_on_error(reader, |reader, pos| {
            let mut options = *options;
            options.endian = binrw::Endian::Big;

            match version {
                Version::V3 => FrameV3::read_options(reader, &options, args).map(Frame::from),
                Version::V4 => FrameV4::read_options(reader, &options, args).map(Frame::from),
                Version::V5 | Version::V6 | Version::V7 => FrameV5::read_options(reader, &options, args).map(Frame::from),
                Version::Unknown => Err(binrw::Error::AssertFail {
                    pos,
                    message: String::from("bad score version")
                })
            }
        })
    }
}

fn parse_sprites<T, R>(reader: &mut R, options: &binrw::ReadOptions, (version, ): (Version, )) -> binrw::BinResult<[T; NUM_SPRITES]>
where
    T: BinRead<Args = (Version, )> + Copy + Default,
    R: Read + Seek
{
    let mut sprites = [ T::default(); NUM_SPRITES ];
    for sprite in sprites.iter_mut().take(if version >= Version::V5 { Frame::V5_CELL_COUNT } else { Frame::V4_CELL_COUNT }.into()) {
        *sprite = T::read_options(reader, options, (version, ))?;
    }
    Ok(sprites)
}

#[derive(BinRead, Clone, Copy, Debug)]
#[br(big, import(version: Version))]
struct FrameV3 {
    // TODO: This is an index into VWAC resource. There is no way to convert
    // this to D5, so either script field needs to be an enum (probably) or
    // extra fields should be added to Frame to store it. Also, it turns out
    // this is not the script-related field from D4.
    script: u8,
    sound_1_kind_maybe: Unk8,
    #[br(args(version))]
    transition: Transition,
    sound_1: MemberNum,
    sound_2: MemberNum,
    sound_2_kind_maybe: Unk8,
    #[br(args(version), align_after(16), align_before(16))]
    palette: PaletteV4,
    #[br(args(version), parse_with = parse_sprites::<SpriteV3, _>)]
    sprites: [ SpriteV3; NUM_SPRITES ],
}

impl From<FrameV3> for FrameV5 {
    fn from(old: FrameV3) -> Self {
        Self {
            sound_1: old.sound_1.into(),
            sound_2: old.sound_2.into(),
            transition: old.transition,
            tempo: old.transition.tempo(),
            palette: old.palette.into(),
            sprites: {
                // TODO: (1) more efficient, (2) needs const generics.
                let mut sprites = [ Sprite::default(); NUM_SPRITES ];
                for (i, sprite) in old.sprites.iter().enumerate() {
                    sprites[i] = Sprite::from(*sprite)
                }
                sprites
            },
            ..Self::default()
        }
    }
}

#[derive(BinRead, Clone, Copy, Debug)]
#[br(big, import(version: Version))]
struct FrameV4 {
    field_0: Unk16,
    #[br(args(version))]
    transition: Transition,
    sound_1: MemberNum,
    sound_2: MemberNum,
    field_a: Unk8,
    field_b: Unk8,
    field_c: Unk8,
    tempo_related: Unk8,
    sound_1_related: Unk8,
    sound_2_related: Unk8,
    script: MemberNum,
    script_related: Unk8,
    transition_related: Unk8,
    #[br(args(version), align_after(20), align_before(20))]
    palette: PaletteV4,
    #[br(args(version), parse_with = parse_sprites::<SpriteV4, _>)]
    sprites: [ SpriteV4; NUM_SPRITES ],
}

impl From<FrameV4> for FrameV5 {
    fn from(old: FrameV4) -> Self {
        Self {
            script: old.script.into(),
            sound_1: old.sound_1.into(),
            sound_2: old.sound_2.into(),
            transition: old.transition,
            tempo_related: old.tempo_related,
            sound_1_related: old.sound_1_related,
            sound_2_related: old.sound_2_related,
            script_related: old.script_related,
            transition_related: old.transition_related,
            tempo: old.transition.tempo(),
            palette: old.palette.into(),
            sprites: {
                // TODO: (1) more efficient, (2) needs const generics.
                let mut sprites = [ Sprite::default(); NUM_SPRITES ];
                for (i, sprite) in old.sprites.iter().enumerate() {
                    sprites[i] = Sprite::from(*sprite)
                }
                sprites
            },
        }
    }
}

#[derive(BinRead, Clone, Debug, SmartDefault)]
#[br(big, import(version: Version))]
pub struct FrameV5 {
    pub script: MemberId,
    pub sound_1: MemberId,
    pub sound_2: MemberId,
    #[br(args(version))]
    pub transition: Transition,
    pub tempo_related: Unk8,
    pub sound_1_related: Unk8,
    pub sound_2_related: Unk8,
    pub script_related: Unk8,
    pub transition_related: Unk8,
    #[br(args(version, transition), parse_with = parse_tempo)]
    pub tempo: Tempo,
    #[br(align_before(24))]
    pub palette: Palette,
    #[default([ Sprite::default(); NUM_SPRITES ])]
    #[br(args(version), parse_with = parse_sprites::<Sprite, _>)]
    pub sprites: [ Sprite; NUM_SPRITES ],
}

fn parse_tempo<R>(reader: &mut R, options: &binrw::ReadOptions, args: (Version, Transition)) -> binrw::BinResult<Tempo>
where R: Read + binrw::io::Seek {
    let (version, transition) = args;
    if version < Version::V6 {
        Ok(transition.tempo())
    } else {
        let last_pos = reader.seek(SeekFrom::Current(0))?;
        let value = i8::read_options(reader, options, ())?;
        Tempo::new(value.into()).map_err(|e| binrw::Error::AssertFail {
            pos: last_pos,
            message: format!("{}", e)
        })
    }
}
