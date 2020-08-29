use anyhow::{Context, Result as AResult};
use bitflags::bitflags;
use byteordered::{Endianness, ByteOrdered};
use crate::{collections::riff::ChunkIndex, pvec};
use derive_more::{Deref, DerefMut, Index, IndexMut};
use libcommon::{encodings::MAC_ROMAN, Reader, Resource, resource::{StringContext, StringKind}};
use num_derive::FromPrimitive;
use num_traits::FromPrimitive;
use super::{
    field::Meta as FieldMeta,
    shape::Meta as ShapeMeta,
    text::Meta as TextMeta,
};

#[derive(Clone, Copy, Debug, Eq, Ord, PartialEq, PartialOrd)]
pub enum ConfigVersion {
    V1215 = 1215,
    V1217 = 1217,
}

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

pub struct LibNum(pub i16);
pub struct MemberNum(pub i16);
pub struct MemberId(pub LibNum, pub MemberNum);

impl From<(i16, i16)> for MemberId {
    fn from(tuple: (i16, i16)) -> Self {
        Self(LibNum(tuple.0), MemberNum(tuple.1))
    }
}

impl From<LibNum> for MemberId {
    fn from(lib_num: LibNum) -> Self {
        Self(lib_num, MemberNum(0))
    }
}

#[derive(Clone, Debug, Deref, DerefMut, Index, IndexMut)]
pub struct CastMap(Vec<ChunkIndex>);

impl Resource for CastMap {
    type Context = ();
    fn load<T: Reader>(input: &mut ByteOrdered<T, Endianness>, size: u32, _: &Self::Context) -> AResult<Self> {
        let mut input = ByteOrdered::new(input, Endianness::Big);
        let capacity = size / 4;
        let mut chunk_indexes = Vec::with_capacity(capacity as usize);
        for _ in 0..capacity {
            chunk_indexes.push(ChunkIndex::new(input.read_i32()?));
        }
        Ok(Self(chunk_indexes))
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
        const NONE = 0;
    }
}

impl Resource for MemberInfoFlags {
    type Context = ();
    fn load<T: Reader>(input: &mut ByteOrdered<T, Endianness>, _: u32, _: &Self::Context) -> AResult<Self> where Self: Sized {
        let flags = input.read_u32()?;
        Self::from_bits(flags).with_context(|| format!("Invalid MemberInfoFlags (0x{:x})", flags))
    }
}

type Struct14h = Vec<u8>;
type STXTSub = Vec<u8>;
type Struct9_4A2DE0 = Vec<u8>;
type StructB_4A2E00 = Vec<u8>;
type StructC_4A2DC0 = Vec<u8>;
type StructD_439630 = Vec<u8>;

pvec! {
    pub struct MemberInfo {
        #[offset(4..8)]
        field_4: u32,
        #[offset(8..0xc)]
        field_8: u32,
        #[offset(0xc..0x10)]
        flags: MemberInfoFlags,
        #[offset(0x10..0x14)]
        field_10: u32,
        #[entry(0)]
        entry_0: String,
        #[entry(1, StringContext(StringKind::PascalStr, MAC_ROMAN))]
        name: String,
        #[entry(2, StringContext(StringKind::PascalStr, MAC_ROMAN))]
        entry_2: String,
        #[entry(3, StringContext(StringKind::PascalStr, MAC_ROMAN))]
        entry_3: String,
        #[entry(5)]
        entry_5: Struct14h,
        #[entry(6)]
        entry_6: STXTSub,
        #[entry(7)]
        entry_7: Struct14h,
        #[entry(8)]
        entry_8: Struct14h,
        #[entry(9)]
        entry_9: Struct9_4A2DE0,
        #[entry(10)]
        entry_10: String,
        #[entry(11)]
        entry_11: StructB_4A2E00,
        #[entry(12)]
        entry_12: StructC_4A2DC0,
        #[entry(13)]
        entry_13: StructD_439630,
    }
}

#[derive(Debug)]
pub struct Member {
    // TODO: This needs to be an Either; for Director 3 it is an i16 resource
    // number, for Director 4+ it is a RIFF chunk index
    riff_index: ChunkIndex,
    next_free: i16,
    some_num_a: i16,
    flags: MemberFlags,
    info: Option<MemberInfo>,
    metadata: MemberMetadata,
}

impl Resource for Member {
    type Context = (ChunkIndex, ConfigVersion);
    fn load<T: Reader>(input: &mut ByteOrdered<T, Endianness>, _: u32, context: &Self::Context) -> AResult<Self> where Self: Sized {
        let mut input = ByteOrdered::new(input, Endianness::Big);
        let kind = input.read_u32()
            .with_context(|| format!("Can’t read cast member kind in chunk {}", context.0))?;
        let kind = MemberKind::from_u32(kind)
            .with_context(|| format!("Invalid cast member kind {} in chunk {}", kind, context.0))?;
        // VWCI
        let info_size = input.read_u32()?;
        // VWCR
        let meta_size = input.read_u32()?;

        let info = if info_size == 0 {
            None
        } else {
            Some(MemberInfo::load(&mut input, info_size, &Default::default())
                .with_context(|| format!("Invalid member info for chunk {}", context.0))?)
        };

        Ok(Self {
            riff_index: context.0,
            next_free: 0,
            some_num_a: 0,
            flags: MemberFlags::empty(),
            info,
            metadata: MemberMetadata::load(&mut input, meta_size, &(kind, context.1))?,
        })
    }
}

bitflags! {
    struct MemberFlags: u16 {
        const FLAG_4   = 4;
        const FLAG_10  = 0x10;
        const FLAG_40  = 0x40;
        const FLAG_80  = 0x80;
        const FLAG_100 = 0x100;
        const FLAG_200 = 0x200;
    }
}

#[derive(Clone, Copy, Debug, FromPrimitive)]
pub enum MemberKind {
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
    OLE,
    Transition,
    Xtra,
}

#[derive(Clone, Copy, Debug)]
pub enum MemberMetadata {
    None,
    Bitmap {},
    FilmLoop {},
    Field(FieldMeta),
    Palette,
    Picture,
    Sound {},
    Button(FieldMeta),
    Shape(ShapeMeta),
    Movie {},
    DigitalVideo {},
    Script {},
    // Rich text
    Text(TextMeta),
    OLE {},
    // Transition-specific Xtras
    Transition {},
    Xtra {},
}

impl Resource for MemberMetadata {
    type Context = (MemberKind, ConfigVersion);
    fn load<T: Reader>(input: &mut ByteOrdered<T, Endianness>, size: u32, context: &Self::Context) -> AResult<Self> where Self: Sized {
        Ok(match context.0 {
            MemberKind::None => {
                input.skip(u64::from(size))?;
                MemberMetadata::None
            },
            MemberKind::Button => MemberMetadata::Button(FieldMeta::load(input, size, &())?),
            MemberKind::Field => MemberMetadata::Field(FieldMeta::load(input, size, &())?),
            MemberKind::Shape => MemberMetadata::Shape(ShapeMeta::load(input, size, &())?),
            MemberKind::Text => MemberMetadata::Text(TextMeta::load(input, size, &(context.1, ))?),
            MemberKind::Bitmap
            | MemberKind::FilmLoop
            | MemberKind::Palette
            | MemberKind::Picture
            | MemberKind::Sound
            | MemberKind::Movie
            | MemberKind::DigitalVideo
            | MemberKind::Script
            | MemberKind::OLE
            | MemberKind::Transition
            | MemberKind::Xtra => todo!()
        })
    }
}
