use anyhow::{Context, Result as AResult};
use bitflags::bitflags;
use byteordered::{Endianness, ByteOrdered};
use crate::{collections::riff::ChunkIndex, pvec};
use derive_more::{Constructor, Deref, DerefMut, Display, From, Index, IndexMut};
use libcommon::{
    encodings::DecoderRef,
    Reader,
    Resource,
    resource::{Input, StringKind},
};
use num_derive::FromPrimitive;
use num_traits::FromPrimitive;
use std::fmt;
use super::{
    bitmap::Meta as BitmapMeta,
    config::Version as ConfigVersion,
    field::Meta as FieldMeta,
    film_loop::Meta as FilmLoopMeta,
    script::Meta as ScriptMeta,
    shape::Meta as ShapeMeta,
    text::Meta as TextMeta,
    transition::Meta as TransitionMeta,
    video::Meta as VideoMeta,
    xtra::Meta as XtraMeta,
};

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

#[derive(Clone, Copy, Debug, Default, Display, Eq, From, Ord, PartialEq, PartialOrd)]
pub struct LibNum(pub i16);

#[derive(Clone, Copy, Debug, Default, Display, Eq, From, Ord, PartialEq, PartialOrd)]
pub struct MemberNum(pub i16);

#[derive(Clone, Constructor, Copy, Default, Display, Eq, PartialEq)]
#[display(fmt = "MemberId({}, {})", "_0.0", "_1.0")]
pub struct MemberId(LibNum, MemberNum);

impl fmt::Debug for MemberId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        fmt::Display::fmt(self, f)
    }
}

impl MemberId {
    pub const SIZE: u32 = 4;

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

impl Resource for MemberId {
    type Context = ();

    fn load(input: &mut Input<impl Reader>, _: u32, _: &Self::Context) -> AResult<Self> where Self: Sized {
        Ok(Self(
            input.read_i16().context("Can’t read cast library number")?.into(),
            input.read_i16().context("Can’t read cast member number")?.into()
        ))
    }
}

#[derive(Clone, Debug, Deref, DerefMut, Index, IndexMut)]
pub struct CastMap(Vec<ChunkIndex>);

impl Resource for CastMap {
    type Context = ();
    fn load(input: &mut Input<impl Reader>, size: u32, _: &Self::Context) -> AResult<Self> {
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
    fn load(input: &mut Input<impl Reader>, _: u32, _: &Self::Context) -> AResult<Self> where Self: Sized {
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
        script_handle: u32,
        #[offset(8..0xc)]
        field_8: u32,
        #[offset(0xc..0x10)]
        flags: MemberInfoFlags,
        #[offset(0x10..0x14)]
        script_context_num: u32,
        #[entry(0)]
        script_text: String,
        #[string_entry(1, StringKind::PascalStr)]
        name: String,
        #[string_entry(2, StringKind::PascalStr)]
        file_path: String,
        #[string_entry(3, StringKind::PascalStr)]
        file_name: String,
        #[entry(5)]
        entry_5: Struct14h,
        #[entry(6)]
        entry_6: STXTSub,
        #[entry(7)]
        entry_7: Struct14h,
        #[entry(8)]
        entry_8: Struct14h,
        // xtra-related
        #[entry(9)]
        entry_9: Struct9_4A2DE0,
        #[string_entry(10, StringKind::CStr)]
        xtra_name: String,
        #[entry(11)]
        entry_11: StructB_4A2E00,
        // xtra-related
        #[entry(12)]
        entry_12: StructC_4A2DC0,
        #[entry(13)]
        entry_13: StructD_439630,
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

impl Resource for Member {
    type Context = (ChunkIndex, ConfigVersion, DecoderRef);
    fn load(input: &mut Input<impl Reader>, _: u32, context: &Self::Context) -> AResult<Self> where Self: Sized {
        let mut input = ByteOrdered::new(input, Endianness::Big);
        let kind = {
            let value = input.read_u32().context("Can’t read cast member kind")?;
            MemberKind::from_u32(value)
                .with_context(|| format!("Invalid cast member kind {}", value))?
        };
        // VWCI
        let info_size = input.read_u32().context("Can’t read cast info size")?;
        // VWCR
        let meta_size = input.read_u32().context("Can’t read cast metadata size")?;

        let info = if info_size == 0 {
            None
        } else {
            Some(MemberInfo::load(&mut input, info_size, &(context.2, ))
                .with_context(|| format!("Can’t load {} cast member info", kind))?)
        };

        Ok(Self {
            riff_index: context.0,
            next_free: 0,
            some_num_a: 0,
            flags: MemberFlags::empty(),
            info,
            metadata: MemberMetadata::load(&mut input, meta_size, &(kind, context.1, context.2))
                .with_context(|| format!("Can’t load {} cast member metadata", kind))?,
        })
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

#[derive(Clone, Copy, Debug, Display, FromPrimitive)]
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

impl Default for MemberKind {
    fn default() -> Self {
        MemberKind::None
    }
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
    // Rich text
    Text(TextMeta),
    OLE(BitmapMeta),
    // Transition-specific Xtras
    Transition(TransitionMeta),
    Xtra(XtraMeta),
}

impl Resource for MemberMetadata {
    type Context = (MemberKind, ConfigVersion, DecoderRef);
    fn load(input: &mut Input<impl Reader>, size: u32, context: &Self::Context) -> AResult<Self> where Self: Sized {
        Ok(match context.0 {
            MemberKind::None => {
                input.skip(u64::from(size))?;
                MemberMetadata::None
            },
            MemberKind::Bitmap => MemberMetadata::Bitmap(BitmapMeta::load(input, size, &())?),
            MemberKind::Button => MemberMetadata::Button(FieldMeta::load(input, size, &())?),
            MemberKind::DigitalVideo => MemberMetadata::DigitalVideo(VideoMeta::load(input, size, &())?),
            MemberKind::Field => MemberMetadata::Field(FieldMeta::load(input, size, &())?),
            MemberKind::FilmLoop => MemberMetadata::FilmLoop(FilmLoopMeta::load(input, size, &())?),
            MemberKind::Movie => MemberMetadata::Movie(FilmLoopMeta::load(input, size, &())?),
            MemberKind::OLE => MemberMetadata::OLE(BitmapMeta::load(input, size, &())?),
            MemberKind::Palette => MemberMetadata::Palette,
            MemberKind::Picture => MemberMetadata::Picture,
            MemberKind::Script => MemberMetadata::Script(ScriptMeta::load(input, size, &())?),
            MemberKind::Shape => MemberMetadata::Shape(ShapeMeta::load(input, size, &())?),
            MemberKind::Sound => MemberMetadata::Sound,
            MemberKind::Text => MemberMetadata::Text(TextMeta::load(input, size, &(context.1, ))?),
            MemberKind::Transition => MemberMetadata::Transition(TransitionMeta::load(input, size, &(context.1, context.2))?),
            MemberKind::Xtra => MemberMetadata::Xtra(XtraMeta::load(input, size, &(context.1, context.2))?),
        })
    }
}
