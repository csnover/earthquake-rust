use binrw::{BinRead, NullString};
use libmactoolbox::types::PString;
use crate::{collections::riff::ChunkIndex, pvec};
use derive_more::{Deref, DerefMut, Display};
use libcommon::{SeekExt, TakeSeekExt, bitflags, bitflags::BitFlags, newtype_num, restore_on_error};
use num_derive::FromPrimitive;
use num_traits::FromPrimitive;
use smart_default::SmartDefault;
use std::{convert::TryFrom, fmt};
use super::{bitmap::Meta as BitmapMeta, config::Version as ConfigVersion, field::Meta as FieldMeta, film_loop::Meta as FilmLoopMeta, script::Meta as ScriptMeta, shape::Meta as ShapeMeta, text::Meta as TextMeta, transition::Meta as TransitionMeta, video::Meta as VideoMeta, xtra::Meta as XtraMeta};

// CAS* - list of ChunkIndex to CASt resources
// CASt - (flags, VWCI size, VWCR size) + VWCI resource + VWCR data
// VWCR - ()

// #[derive(Debug)]
// struct VideoWorksCastRegistry(Vec<MemberMetadata>);
// impl Resource for VideoWorksCastRegistry {
//     type Data = ();
//     fn load<T: Reader>(input: &mut ByteOrdered<T, Endianness>, _: u32, _: Self::Data) -> AResult<Self> where Self: Sized {
//         let mut data = Vec::new();
//         while !input.is_empty()? {
//             let size = input.read_u8().context("Can’t read cast registry member size")?;
//             if size == 0 {
//                 data.push(MemberMetadata::None);
//             } else {
//                 let kind = input.read_u8().context("Can’t read cast member kind")?;
//                 let kind = MemberKind::from_u8(kind)
//                     .with_context(|| format!("Invalid cast member kind {}", kind))?;
//                 data.push(match kind {
//                     MemberKind::None => unreachable!(),
//                     MemberKind::Bitmap => todo!(),
//                     MemberKind::FilmLoop => todo!(),
//                     MemberKind::Field => field_load_metadata(&mut input)?,
//                     MemberKind::Palette => todo!(),
//                     MemberKind::Picture => todo!(),
//                     MemberKind::Sound => todo!(),
//                     MemberKind::Button => todo!(),
//                     MemberKind::Shape => todo!(),
//                     MemberKind::Movie => todo!(),
//                     MemberKind::DigitalVideo => todo!(),
//                     // These kinds only appear in Director 4, which uses the
//                     // newer CAS* library format
//                     MemberKind::Script
//                     | MemberKind::Text
//                     | MemberKind::OLE
//                     | MemberKind::Transition
//                     | MemberKind::Xtra => unreachable!()
//                 })
//             }
//         }

//         Ok(VideoWorksCastRegistry(data))
//     }
// }

// impl VideoWorksCastRegistry {
//     pub fn into_inner(self) -> Vec<MemberMetadata> {
//         self.0
//     }
// }

newtype_num! {
    #[derive(BinRead, Debug)]
    pub struct LibNum(pub i16);
}

newtype_num! {
    #[derive(BinRead, Debug)]
    pub struct MemberNum(pub i16);
}

#[derive(BinRead, Clone, Copy, Default, Display, Eq, Ord, PartialEq, PartialOrd)]
#[display(fmt = "MemberId({}, {})", "_0.0", "_1.0")]
pub struct MemberId(LibNum, MemberNum);

impl fmt::Debug for MemberId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        fmt::Display::fmt(self, f)
    }
}

impl MemberId {
    pub const SIZE: u32 = 4;

    pub fn new(lib_num: impl Into<LibNum>, member_num: impl Into<MemberNum>) -> Self {
        Self(lib_num.into(), member_num.into())
    }

    #[must_use]
    pub fn lib(self) -> LibNum {
        self.0
    }

    pub fn lib_mut(&mut self) -> &mut LibNum {
        &mut self.0
    }

    #[must_use]
    pub fn num(self) -> MemberNum {
        self.1
    }

    pub fn num_mut(&mut self) -> &mut MemberNum {
        &mut self.1
    }
}

impl From<MemberNum> for MemberId {
    fn from(num: MemberNum) -> Self {
        Self(if num.0 == 0 { 0_i16 } else { 1_i16 }.into(), num)
    }
}

// TODO: Rewrite this to use binrw and put it somewhere better with a
// non-repetitive name
#[allow(clippy::module_name_repetitions)]
#[derive(Clone, Debug, Deref, DerefMut)]
pub struct CastMap(Vec<ChunkIndex>);

impl BinRead for CastMap {
    type Args = ();

    fn read_options<R: std::io::Read + std::io::Seek>(reader: &mut R, options: &binrw::ReadOptions, _: Self::Args) -> binrw::BinResult<Self> {
        restore_on_error(reader, |reader, _| {
            let mut options = *options;
            options.endian = binrw::Endian::Big;

            let count = reader.bytes_left()? / 4;
            let mut data = Vec::with_capacity(usize::try_from(count).unwrap());
            for _ in 0..count {
                let value = ChunkIndex::read_options(reader, &options, ())?;
                data.push(value);
            }
            Ok(Self(data))
        })
    }
}

bitflags! {
    struct FileInfoFlags: u32 {
        const REMAP_PALETTES       = 0x40;
        const MOVIE_FIELD_47       = 0x100;
        const UPDATE_MOVIE_ENABLED = 0x200;
        const PRELOAD_EVENT_ABORT  = 0x400;
        const MOVIE_FIELD_4D       = 0x1000;
        const MOVIE_FIELD_4E       = 0x2000;
    }
}

bitflags! {
    struct MemberInfoFlags: u32 {
        const NONE          = 0;
        const EXTERNAL_FILE = 1;
        const PURGE_NEVER   = 4;
        const PURGE_LAST    = 8;
        const PURGE_NEXT    = Self::PURGE_NEVER.bits | Self::PURGE_LAST.bits;
        const SOUND_ON      = 0x10;
    }
}

type Struct14h = Vec<u8>;
type STXTSub = Vec<u8>;
#[allow(non_camel_case_types)]
type Struct9_4A2DE0 = Vec<u8>;
#[allow(non_camel_case_types)]
type StructB_4A2E00 = Vec<u8>;
#[allow(non_camel_case_types)]
type StructC_4A2DC0 = Vec<u8>;
#[allow(non_camel_case_types)]
type StructD_439630 = Vec<u8>;

pvec! {
    #[derive(Debug)]
    pub struct MemberInfo {
        header {
            script_handle: u32,
            field_8: u32,
            flags: MemberInfoFlags,
            script_context_num: u32,
        }

        offsets = offsets;

        entries {
            #[br(count = offsets.entry_size(0).unwrap_or(0))]
            0 => script_text: Vec<u8>,
            1 => name: PString,
            2 => file_path: PString,
            3 => file_name: PString,
            4 => _,
            #[br(count = offsets.entry_size(5).unwrap_or(0))]
            5 => entry_5: Struct14h,
            #[br(count = offsets.entry_size(6).unwrap_or(0))]
            6 => entry_6: STXTSub,
            #[br(count = offsets.entry_size(7).unwrap_or(0))]
            7 => entry_7: Struct14h,
            #[br(count = offsets.entry_size(8).unwrap_or(0))]
            8 => entry_8: Struct14h,
            // xtra-related
            #[br(count = offsets.entry_size(9).unwrap_or(0))]
            9 => entry_9: Struct9_4A2DE0,
            10 => xtra_name: NullString,
            // script related
            #[br(count = offsets.entry_size(11).unwrap_or(0))]
            11 => entry_11: StructB_4A2E00,
            // xtra-related
            #[br(count = offsets.entry_size(12).unwrap_or(0))]
            12 => entry_12: StructC_4A2DC0,
            #[br(count = offsets.entry_size(13).unwrap_or(0))]
            13 => entry_13: StructD_439630,
            // for some reason there is a video-related entry in slot 14, but
            // it seems to not ever be referenced in projector code.
            14..
        }
    }
}

#[derive(Debug)]
pub struct Member {
    // TODO: This needs to be an Either; for Director 3 it is an i16 resource
    // number, for Director 4+ it is also sometimes a RIFF chunk index maybe?
    riff_index: ChunkIndex,
    next_free: i16,
    some_num_a: i16,
    flags: MemberFlags,
    info: Option<MemberInfo>,
    metadata: MemberMetadata,
}

impl BinRead for Member {
    type Args = (ChunkIndex, ConfigVersion);

    fn read_options<R: std::io::Read + std::io::Seek>(input: &mut R, options: &binrw::ReadOptions, (index, version): Self::Args) -> binrw::BinResult<Self> {
        let mut options = *options;
        options.endian = binrw::Endian::Big;

        let meta = if version < ConfigVersion::V1201 {
            MemberMetaV4::read_options(input, &options, ())?.into()
        } else {
            MemberMetaV5::read_options(input, &options, ())?
        };

        let info = if meta.info_size == 0 {
            None
        } else {
            Some(MemberInfo::read_options(&mut input.take_seek(meta.info_size.into()), &options, ())?)
                // TODO: Figure out how to get this context back
                // .with_context(|| format!("Can’t load {} cast member info", kind))?;
        };

        let metadata = MemberMetadata::read_options(&mut input.take_seek(meta.meta_size.into()), &options, (meta, version))?;
            // TODO: Figure out how to get this context back
            // .with_context(|| format!("Can’t load {} cast member metadata", meta.kind))?;

        Ok(Self {
            riff_index: index,
            next_free: 0,
            some_num_a: 0,
            flags: MemberFlags::empty(),
            info,
            metadata,
        })
    }
}

#[derive(thiserror::Error)]
#[error("invalid {0} 0x{1:x}")]
struct FromPrimitiveError<T: core::fmt::Display, U: core::fmt::LowerHex>(T, U);

impl <T: core::fmt::Display, U: core::fmt::LowerHex> core::fmt::Debug for FromPrimitiveError<T, U> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_tuple("FromPrimitiveError")
            .field(&format!("{}", self.0))
            .field(&format!("{:x}", self.1))
            .finish()
    }
}

// TODO: This is incorrect guesswork.
#[derive(BinRead, Copy, Clone, Debug)]
#[br(big)]
struct MemberMetaV4 {
    _unknown: u16,
    // VWCR
    meta_size: u16,
    // VWCI
    info_size: u16,
    #[br(try_map = |kind: u8| MemberKind::from_u8(kind).ok_or_else(|| anyhow::anyhow!("wow")))]
    kind: MemberKind,
}

#[derive(BinRead, Copy, Clone, Debug)]
#[br(big)]
pub struct MemberMetaV5 {
    #[br(try_map = |kind: u32| MemberKind::from_u32(kind).ok_or(FromPrimitiveError("cast member kind", kind)))]
    kind: MemberKind,
    // VWCI
    info_size: u32,
    // VWCR
    meta_size: u32,
}

impl From<MemberMetaV4> for MemberMetaV5 {
    fn from(other: MemberMetaV4) -> Self {
        Self {
            kind: other.kind,
            meta_size: other.meta_size.into(),
            info_size: other.info_size.into(),
        }
    }
}

bitflags! {
    struct MemberFlags: u16 {
        const FLAG_4   = 4;
        const FLAG_8   = 8;
        const FLAG_10  = 0x10;
        const FLAG_40  = 0x40;
        const FLAG_80  = 0x80;
        const FLAG_100 = 0x100;
        const FLAG_200 = 0x200;
        const FLAG_800 = 0x800;
    }
}

#[derive(Clone, Copy, Debug, Display, FromPrimitive, SmartDefault)]
pub enum MemberKind {
    #[default]
    None = 0,
    Bitmap,
    FilmLoop,
    Field,
    Palette,
    Picture,
    Sound,
    Button,
    Shape,
    Movie,
    DigitalVideo,
    Script,
    Text,
    Ole,
    Transition,
    Xtra,
}

#[derive(Clone, Debug)]
pub enum MemberMetadata {
    None,
    Bitmap(BitmapMeta),
    FilmLoop(FilmLoopMeta),
    Field(FieldMeta),
    Palette,
    Picture,
    Sound,
    Button(FieldMeta),
    Shape(ShapeMeta),
    Movie(FilmLoopMeta),
    DigitalVideo(VideoMeta),
    Script(ScriptMeta),
    // This uses Microsoft RTF, whereas the Field type uses Mac Styled Text
    Text(TextMeta),
    Ole(BitmapMeta),
    Transition(TransitionMeta),
    Xtra(XtraMeta),
}

impl BinRead for MemberMetadata {
    type Args = (MemberMetaV5, ConfigVersion);

    fn read_options<R: binrw::io::Read + binrw::io::Seek>(input: &mut R, options: &binrw::ReadOptions, args: Self::Args) -> binrw::BinResult<Self> {
        let mut options = *options;
        options.endian = binrw::Endian::Big;

        let (meta, version) = args;
        let size = meta.meta_size;

        Ok(match meta.kind {
            MemberKind::None => {
                input.skip(size.into())?;
                MemberMetadata::None
            },
            MemberKind::Bitmap => MemberMetadata::Bitmap(BitmapMeta::read_options(input, &options, (size, ))?),
            MemberKind::Button => MemberMetadata::Button(FieldMeta::read_options(input, &options, (size, ))?),
            MemberKind::DigitalVideo => MemberMetadata::DigitalVideo(VideoMeta::read_options(input, &options, (size, ))?),
            MemberKind::Field => MemberMetadata::Field(FieldMeta::read_options(input, &options, (size, ))?),
            MemberKind::FilmLoop => MemberMetadata::FilmLoop(FilmLoopMeta::read_options(input, &options, (size, ))?),
            MemberKind::Movie => MemberMetadata::Movie(FilmLoopMeta::read_options(input, &options, (size, ))?),
            MemberKind::Ole => MemberMetadata::Ole(BitmapMeta::read_options(input, &options, (size, ))?),
            MemberKind::Palette => MemberMetadata::Palette,
            MemberKind::Picture => MemberMetadata::Picture,
            MemberKind::Script => MemberMetadata::Script(ScriptMeta::read_options(input, &options, (size, ))?),
            MemberKind::Shape => MemberMetadata::Shape(ShapeMeta::read_options(input, &options, ())?),
            MemberKind::Sound => MemberMetadata::Sound,
            MemberKind::Text => MemberMetadata::Text(TextMeta::read_options(input, &options, (version, ))?),
            MemberKind::Transition => MemberMetadata::Transition(TransitionMeta::read_options(input, &options, (size, version))?),
            MemberKind::Xtra => MemberMetadata::Xtra(XtraMeta::read_options(input, &options, (size, ))?),
        })
    }
}
