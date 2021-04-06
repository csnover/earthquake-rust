//! Type definitions for cast management.
//!
//! Two types of cast management exist in Director depending upon version.
//!
//! Director 3 and earlier held cast .
//!
//! D4+ use the `'CAS*'`

use anyhow::{Context, Result as AResult, anyhow, bail};
use binrw::{BinRead, NullString, io};
use core::{convert::{TryFrom, TryInto}, fmt};
use crate::{collections::riff::{ChunkIndex, Riff}, pvec};
use derive_more::{Deref, DerefMut, Display};
use libcommon::{Reader, SeekExt, TakeSeekExt, Unk32, Unk8, UnkHnd, bitflags, bitflags::BitFlags, newtype_num, restore_on_error};
use libmactoolbox::{resources::{ResNum, ResourceId, Source as ResourceSource}, typed_resource, types::PString};
use num_derive::FromPrimitive;
use num_traits::FromPrimitive;
use smart_default::SmartDefault;
use super::{bitmap::{Properties as BitmapProps, PropertiesV3 as BitmapPropsV3}, config::{Config, Version as ConfigVersion}, field::Properties as FieldProps, film_loop::Properties as FilmLoopProps, script::Properties as ScriptProps, shape::Properties as ShapeProps, text::Properties as TextProps, transition::Properties as TransitionProps, video::Properties as VideoProps, xtra::Properties as XtraProps};

/// A cast library.
#[derive(Clone, Debug, Deref, DerefMut)]
pub struct Library(Vec<Member>);

impl Library {
    pub fn from_resource_source(source: &impl ResourceSource, cast_num: impl Into<ResNum>) -> AResult<Self> {
        let cast_num = cast_num.into();
        let config = source.load_num::<Config>(cast_num)?;
        let map = source.load_num::<CastRegistry>(cast_num)?;
        let base_resource_num = cast_num + i16::from(config.min_cast_num()).into();
        let mut data = Vec::with_capacity(map.len());
        for (i, (flags, properties)) in map.iter().enumerate() {
            let resource_num = base_resource_num + i16::try_from(i).unwrap().into();
            let metadata = if source.contains(ResourceId::new(b"VWCI", resource_num)) {
                let metadata = source.load_num::<MemberMetadata>(resource_num)
                    .with_context(|| anyhow!("error reading metadata for cast member {} (res num {})", i, resource_num))?;
                Some((*metadata).clone())
            } else {
                None
            };
            data.push(Member {
                // TODO: This needs to be an Either, and this needs to get
                // the resource_num
                riff_index: 0.into(),
                next_free: 0,
                some_num_a: 0,
                // TODO: These flags are not compatible with the D3 flags.
                flags: MemberFlags::empty(),
                metadata,
                properties: properties.clone(),
            });
        }
        Ok(Self(data))
    }

    pub fn from_riff(riff: &Riff<impl Reader>, cast_num: impl Into<ResNum>) -> AResult<Self> {
        let cast_num = cast_num.into();
        let config = riff.load_num::<Config>(cast_num)?;
        let map = riff.load_num::<CastMap>(cast_num)?;
        let mut data = Vec::with_capacity(map.len());
        let min_cast_num = config.min_cast_num();
        let version = config.version();
        for (i, &chunk_index) in map.iter().enumerate() {
            if chunk_index > ChunkIndex::new(0) {
                let cast_member_num = min_cast_num + i16::try_from(i).unwrap().into();
                let member = riff.load_chunk_args::<Member>(chunk_index, (chunk_index, version))
                    .with_context(|| format!("error reading cast member {}", cast_member_num))?;
                data.push((*member).clone());
            }
        }
        Ok(Self(data))
    }
}

/// The Director 3 cast registry.
///
/// OsType: `'VWCR'`
#[derive(Debug, Deref, DerefMut)]
pub struct CastRegistry(Vec<(Unk8, MemberProperties)>);
typed_resource!(CastRegistry => b"VWCR");

impl BinRead for CastRegistry {
    type Args = ();

    fn read_options<R: io::Read + io::Seek>(
        input: &mut R,
        options: &binrw::ReadOptions,
        args: Self::Args,
    ) -> binrw::BinResult<Self> {
        restore_on_error(input, |input, _| {
            use binrw::BinReaderExt;
            let mut data = Vec::new();
            while input.bytes_left()? != 0 {
                // The size of the record data, excluding the size byte
                let mut size = u32::from(input.read_be::<u8>()?);
                // TODO: .context("Can’t read cast registry member size")?;
                if size == 0 {
                    data.push((Unk8::from(0), MemberProperties::None));
                } else {
                    let kind = input.read_be::<u8>()?;
                    // TODO: .context("Can’t read cast member kind")?;
                    let kind = MemberKind::from_u8(kind)
                        .ok_or_else(|| binrw::Error::Custom {
                            pos: input.pos().unwrap() - 1,
                            err: Box::new(FromPrimitiveError("cast member kind", kind))
                        })?;

                    let flags = match kind {
                        MemberKind::Bitmap
                        | MemberKind::Button
                        | MemberKind::DigitalVideo
                        | MemberKind::Field
                        | MemberKind::FilmLoop
                        | MemberKind::Movie
                        | MemberKind::Shape => {
                            // `- 2` for the kind and the flags
                            size -= 2;
                            Unk8::read_options(input, &options, ())?
                        },
                        _ => Unk8::from(0),
                    };

                    data.push((flags, match kind {
                        MemberKind::None => unreachable!(),
                        MemberKind::Bitmap => MemberProperties::Bitmap(BitmapPropsV3::read_options(input, &options, (size, ))?.into()),
                        MemberKind::Button => MemberProperties::Button(FieldProps::read_options(input, &options, (size, ))?),
                        MemberKind::DigitalVideo => MemberProperties::DigitalVideo(VideoProps::read_options(input, &options, (size, ))?),
                        MemberKind::Field => MemberProperties::Field(FieldProps::read_options(input, &options, (size, ))?),
                        MemberKind::FilmLoop => MemberProperties::FilmLoop(FilmLoopProps::read_options(input, &options, (size, ))?),
                        MemberKind::Movie => MemberProperties::Movie(FilmLoopProps::read_options(input, &options, (size, ))?),
                        MemberKind::Palette => MemberProperties::Palette,
                        MemberKind::Picture => MemberProperties::Picture,
                        MemberKind::Shape => MemberProperties::Shape(ShapeProps::read_options(input, &options, ())?),
                        MemberKind::Sound => MemberProperties::Sound,
                        // These kinds only appear in Director 4, which uses the
                        // newer CAS* library format
                        MemberKind::Script
                        | MemberKind::Text
                        | MemberKind::Ole
                        | MemberKind::Transition
                        | MemberKind::Xtra => unreachable!()
                    }))
                }
            }

            Ok(Self(data))
        })
    }
}

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

// TODO: Put this somewhere better with a non-repetitive name
/// A map of cast member numbers to RIFF chunk indexes.
///
/// Each item in the list corresponds to a cast member starting from the
/// lowest populated cast member slot (which is recorded separately in
/// [`Config::min_cast_member`]). Empty cast member slots have a chunk index
/// of 0.
///
/// OsType: `'CAS*'`
#[allow(clippy::module_name_repetitions)]
#[derive(Clone, Debug, Deref, DerefMut)]
pub struct CastMap(Vec<ChunkIndex>);
typed_resource!(CastMap => b"CAS*");

impl BinRead for CastMap {
    type Args = ();

    fn read_options<R: io::Read + io::Seek>(reader: &mut R, options: &binrw::ReadOptions, _: Self::Args) -> binrw::BinResult<Self> {
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
    struct MemberInfoFlags: u32 {
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
    /// Cast member metadata.
    ///
    /// OsType: `'Cinf'` `'VWCI'`
    #[derive(Clone, Debug)]
    pub struct MemberMetadata {
        header_size = header_size;

        header {
            script_handle: UnkHnd,
            field_8: Unk32,
            flags: MemberInfoFlags,
            #[br(if(header_size >= 20))]
            script_context_num: i32,
        }

        offsets = offsets;

        entries {
            /// Cast member script text.
            ///
            /// Used only by D3. D4 and later store the cast member script in
            /// `'Lctx'`/`'Lscr'` chunks.
            #[br(count = offsets.entry_size(0).unwrap_or(0))]
            0 => script_text: Vec<u8>,

            /// The name of the cast member.
            1 => name: PString,

            /// For an external cast member, the original file path.
            2 => file_path: PString,

            /// For an external cast member, the original filename.
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
typed_resource!(MemberMetadata => b"Cinf" b"VWCI");

/// A cast member.
///
/// OsType: `'CASt'`
#[derive(Clone, Debug)]
pub struct Member {
    // TODO: This needs to be an Either; for Director 3 it is an i16 resource
    // number, for Director 4+ it is also sometimes a RIFF chunk index maybe?
    riff_index: ChunkIndex,
    next_free: i16,
    some_num_a: i16,
    flags: MemberFlags,
    metadata: Option<MemberMetadata>,
    properties: MemberProperties,
}
typed_resource!(Member => b"CASt");

impl BinRead for Member {
    type Args = (ChunkIndex, ConfigVersion);

    fn read_options<R: io::Read + io::Seek>(input: &mut R, options: &binrw::ReadOptions, (index, version): Self::Args) -> binrw::BinResult<Self> {
        restore_on_error(input, |input, _| {
            let mut options = *options;
            options.endian = binrw::Endian::Big;

            let header = if version < ConfigVersion::V1201 {
                MemberHeaderV4::read_options(input, &options, ())?.into()
            } else {
                MemberHeaderV5::read_options(input, &options, ())?
            };

            let info = if header.metadata_size == 0 {
                None
            } else {
                Some(MemberMetadata::read_options(&mut input.take_seek(header.metadata_size.into()), &options, ())?)
                    // TODO: Figure out how to get this context back
                    // .with_context(|| format!("Can’t load {} cast member info", kind))?;
            };

            let metadata = MemberProperties::read_options(&mut input.take_seek(header.properties_size.into()), &options, (header, version))?;
                // TODO: Figure out how to get this context back
                // .with_context(|| format!("Can’t load {} cast member metadata", meta.kind))?;

            Ok(Self {
                riff_index: index,
                next_free: 0,
                some_num_a: 0,
                flags: MemberFlags::empty(),
                metadata: info,
                properties: metadata,
            })
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
struct MemberHeaderV4 {
    _unknown: u16,
    properties_size: u16,
    metadata_size: u16,
    #[br(try_map = |kind: u8| MemberKind::from_u8(kind).ok_or(FromPrimitiveError("cast member kind", kind)))]
    kind: MemberKind,
}

#[derive(BinRead, Copy, Clone, Debug)]
#[br(big)]
pub struct MemberHeaderV5 {
    #[br(try_map = |kind: u32| MemberKind::from_u32(kind).ok_or(FromPrimitiveError("cast member kind", kind)))]
    kind: MemberKind,
    metadata_size: u32,
    properties_size: u32,
}

impl From<MemberHeaderV4> for MemberHeaderV5 {
    fn from(other: MemberHeaderV4) -> Self {
        Self {
            kind: other.kind,
            properties_size: other.properties_size.into(),
            metadata_size: other.metadata_size.into(),
        }
    }
}

bitflags! {
    struct MemberFlags: u16 {
        const DIRTY_MAYBE          = 1;
        const DATA_MODIFIED        = 4;
        const PROPERTIES_MODIFIED  = 8;
        const LOCKED               = 0x10;
        const FILE_NOT_FOUND_MAYBE = 0x40;
        const FLAG_80              = 0x80;
        const NOT_PURGEABLE        = 0x100;
        const LINKED_FILE_MAYBE    = 0x200;
        const DATA_IN_MEMORY       = 0x800;
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
pub enum MemberProperties {
    None,
    Bitmap(BitmapProps),
    FilmLoop(FilmLoopProps),
    Field(FieldProps),
    Palette,
    Picture,
    Sound,
    Button(FieldProps),
    Shape(ShapeProps),
    Movie(FilmLoopProps),
    DigitalVideo(VideoProps),
    Script(ScriptProps),
    // This uses Microsoft RTF, whereas the Field type uses Mac Styled Text
    Text(TextProps),
    Ole(BitmapProps),
    Transition(TransitionProps),
    Xtra(XtraProps),
}

impl BinRead for MemberProperties {
    type Args = (MemberHeaderV5, ConfigVersion);

    fn read_options<R: io::Read + io::Seek>(input: &mut R, options: &binrw::ReadOptions, args: Self::Args) -> binrw::BinResult<Self> {
        restore_on_error(input, |input, _| {
            let mut options = *options;
            options.endian = binrw::Endian::Big;

            let (meta, version) = args;
            let size = meta.properties_size;

            Ok(match meta.kind {
                MemberKind::None => {
                    input.skip(size.into())?;
                    MemberProperties::None
                },
                MemberKind::Bitmap => MemberProperties::Bitmap(BitmapProps::read_options(input, &options, (size, ))?),
                MemberKind::Button => MemberProperties::Button(FieldProps::read_options(input, &options, (size, ))?),
                MemberKind::DigitalVideo => MemberProperties::DigitalVideo(VideoProps::read_options(input, &options, (size, ))?),
                MemberKind::Field => MemberProperties::Field(FieldProps::read_options(input, &options, (size, ))?),
                MemberKind::FilmLoop => MemberProperties::FilmLoop(FilmLoopProps::read_options(input, &options, (size, ))?),
                MemberKind::Movie => MemberProperties::Movie(FilmLoopProps::read_options(input, &options, (size, ))?),
                MemberKind::Ole => MemberProperties::Ole(BitmapProps::read_options(input, &options, (size, ))?),
                MemberKind::Palette => MemberProperties::Palette,
                MemberKind::Picture => MemberProperties::Picture,
                MemberKind::Script => MemberProperties::Script(ScriptProps::read_options(input, &options, (size, ))?),
                MemberKind::Shape => MemberProperties::Shape(ShapeProps::read_options(input, &options, ())?),
                MemberKind::Sound => MemberProperties::Sound,
                MemberKind::Text => MemberProperties::Text(TextProps::read_options(input, &options, (version, ))?),
                MemberKind::Transition => MemberProperties::Transition(TransitionProps::read_options(input, &options, (size, version))?),
                MemberKind::Xtra => MemberProperties::Xtra(XtraProps::read_options(input, &options, (size, ))?),
            })
        })
    }
}
