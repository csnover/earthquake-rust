//! Type definitions for cast management.

use anyhow::{Result as AResult, anyhow};
use binrw::{BinRead, NullString, io};
use core::fmt;
use crate::{cast::GlobalLibNum, collections::riff::{ChunkIndex, Riff}, pvec, util::RawString};
use derive_more::{Deref, DerefMut, Display};
use libcommon::{io::prelude::*, prelude::*, bitflags, newtype_num};
use libmactoolbox::{resources::{ResNum, ResourceId, Source as ResourceSource}, typed_resource, types::PString};
use num_derive::FromPrimitive;
use num_traits::FromPrimitive;
use smart_default::SmartDefault;
use super::{PVecOffsets, bitmap::Properties as BitmapProps, config::{Config, Version as ConfigVersion}, field::Properties as FieldProps, film_loop::Properties as FilmLoopProps, script::Properties as ScriptProps, shape::Properties as ShapeProps, text::Properties as TextProps, transition::Properties as TransitionProps, video::Properties as VideoProps, xtra::Properties as XtraProps};

#[derive(Clone, Copy, Debug, SmartDefault)]
enum LoadId {
    Resource(ResNum),
    #[default]
    Riff(ChunkIndex),
}

/// A cast library.
#[derive(Clone, Debug, Default, Deref, DerefMut)]
pub(crate) struct Library(Vec<Member>);

impl Library {
    pub(crate) fn from_resource_source(source: &impl ResourceSource, cast_num: impl Into<ResNum>) -> AResult<Self> {
        use anyhow::Context;
        let cast_num = cast_num.into();
        let config = source.load_num::<Config>(cast_num)?;
        let map = source.load_num_args::<CastRegistry>(cast_num, (config.version(), ))?;
        let base_resource_num = cast_num + i16::from(config.min_cast_num()).into();
        let mut data = Vec::with_capacity(map.len());
        // TODO: These flags do not seem to be compatible with the D4+ flags.
        // It is unclear what they describe as there does not seem to be
        // any discernible pattern. Even empty cast members sometimes have
        // flags.
        for (i, (more_flags, properties)) in map.iter().enumerate() {
            let resource_num = base_resource_num + i16::unwrap_from(i).into();

            let metadata = if source.contains(ResourceId::new(b"VWCI", resource_num)) {
                let metadata = source.load_num::<MemberMetadata>(resource_num)
                    .with_context(|| anyhow!("error reading metadata for cast member {} (res num {})", i, resource_num))?;
                Some((*metadata).clone())
            } else {
                None
            };

            let load_id = LoadId::Resource(
                if matches!(properties, MemberProperties::None) {
                    ResNum::from(-1_i16)
                } else {
                    resource_num
                }
            );

            data.push(Member {
                load_id,
                next_free: 0,
                some_num_a: 0,
                flags: <_>::default(),
                more_flags: *more_flags,
                metadata,
                properties: properties.clone(),
            });
        }
        Ok(Self(data))
    }

    pub(crate) fn from_riff(riff: &Riff<impl Reader>, cast_num: impl Into<ResNum>) -> AResult<Self> {
        use anyhow::Context;
        let cast_num = cast_num.into();
        let config = riff.load_num::<Config>(cast_num)?;
        if config.version() < ConfigVersion::V1113 {
            Self::from_resource_source(riff, cast_num)
        } else {
            let map = riff.load_num::<CastMap>(cast_num)?;
            let mut data = Vec::with_capacity(map.len());
            let min_cast_num = config.min_cast_num();
            let version = config.version();
            for (i, &chunk_index) in map.iter().enumerate() {
                if chunk_index > ChunkIndex::new(0) {
                    let cast_member_num = min_cast_num + i16::unwrap_from(i).into();
                    let member = riff.load_chunk_args::<Member>(chunk_index, (chunk_index, version))
                        .with_context(|| format!("error reading cast member {}", cast_member_num))?;
                    data.push((*member).clone());
                } else {
                    data.push(Member::default());
                }
            }
            Ok(Self(data))
        }
    }
}

/// The Director 3 cast registry.
///
/// OsType: `'VWCR'`
#[derive(Debug, Deref, DerefMut)]
struct CastRegistry(Vec<(LegacyMemberFlags, MemberProperties)>);
typed_resource!(CastRegistry => b"VWCR");

impl BinRead for CastRegistry {
    type Args = (ConfigVersion, );

    fn read_options<R: io::Read + io::Seek>(
        input: &mut R,
        options: &binrw::ReadOptions,
        (version, ): Self::Args,
    ) -> binrw::BinResult<Self> {
        restore_on_error(input, |input, _| {
            use binrw::{BinReaderExt, error::Context};
            let mut data = Vec::new();
            while input.bytes_left()? != 0 {
                // The size of the record data, excluding the size byte
                let mut size = u32::from(input.read_be::<u8>()
                    .context(|| "Can’t read cast registry member size")?);
                if size == 0 {
                    data.push((<_>::default(), MemberProperties::None));
                } else {
                    let kind = input.read_be::<u8>()
                        .context(|| "Can’t read cast member kind")?;
                    let kind = MemberKind::from_u8(kind)
                        .ok_or_else(|| binrw::Error::Custom {
                            pos: input.pos().unwrap() - 1,
                            err: Box::new(FromPrimitiveError("cast member kind", kind))
                        })?;

                    let flags = if kind.has_extra_flags() {
                        // `- 2` for the kind and the flags
                        size -= 2;
                        LegacyMemberFlags::read_options(input, &options, ())?
                    } else {
                        <_>::default()
                    };

                    data.push((flags, MemberProperties::read_options(&mut input.take_seek(size.into()), &options, (kind, size, version))?))
                }
            }

            Ok(Self(data))
        })
    }
}

newtype_num! {
    #[derive(BinRead, Debug)]
    pub struct LibNum(i16);
}

impl LibNum {
    /// Makes a new `LibNum` from an i16.
    ///
    /// This is a workaround to allow a generic constructor whilst also allowing
    /// `LibNum` to be statically constructed.
    #[must_use]
    pub(crate) const fn from_raw(lib_num: i16) -> Self {
        Self(lib_num)
    }
}

newtype_num! {
    #[derive(BinRead, Debug)]
    pub struct MemberNum(i16);
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
    pub fn new(lib_num: impl Into<LibNum>, member_num: impl Into<MemberNum>) -> Self {
        Self(lib_num.into(), member_num.into())
    }

    /// Makes a new `MemberId` from two i16s.
    ///
    /// This is a workaround to allow a generic constructor whilst also allowing
    /// `OsType` to be statically constructed.
    #[must_use]
    pub(crate) const fn from_raw(lib_num: i16, member_num: i16) -> Self {
        Self(LibNum(lib_num), MemberNum(member_num))
    }

    /// A parser which will parse either a `MemberNum` or `MemberId` according
    /// to the given argument.
    pub(super) fn parse_num<R: Read + Seek>(input: &mut R, options: &binrw::ReadOptions, (is_id, ): (bool, )) -> binrw::BinResult<MemberId> {
        if is_id {
            Self::read_options(input, options, ())
        } else {
            Ok(MemberNum::read_options(input, options, ())?.into())
        }
    }

    #[must_use]
    pub(super) fn lib(self) -> LibNum {
        self.0
    }

    pub(super) fn lib_mut(&mut self) -> &mut LibNum {
        &mut self.0
    }

    #[must_use]
    pub(super) fn num(self) -> MemberNum {
        self.1
    }

    pub(super) fn num_mut(&mut self) -> &mut MemberNum {
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
pub(super) struct CastMap(Vec<ChunkIndex>);
typed_resource!(CastMap => b"CAS*");

impl BinRead for CastMap {
    type Args = ();

    fn read_options<R: io::Read + io::Seek>(reader: &mut R, options: &binrw::ReadOptions, _: Self::Args) -> binrw::BinResult<Self> {
        restore_on_error(reader, |reader, _| {
            let mut options = *options;
            options.endian = binrw::Endian::Big;

            let count = reader.bytes_left()? / 4;
            let mut data = Vec::with_capacity(usize::unwrap_from(count));
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
        const AUTO_HILITE   = 2;
        const PURGE_NEVER   = 4;
        const PURGE_LAST    = 8;
        const PURGE_NEXT    = Self::PURGE_NEVER.bits | Self::PURGE_LAST.bits;
        const SOUND_ON      = 0x10;
    }
}

type Struct14h = Vec<u8>;
type StxtSub = Vec<u8>;
#[allow(non_camel_case_types)]
type Struct9_4a2de0 = Vec<u8>;
#[allow(non_camel_case_types)]
type StructB_4a2e00 = Vec<u8>;
#[allow(non_camel_case_types)]
type StructC_4a2dc0 = Vec<u8>;
#[allow(non_camel_case_types)]
type StructD_439630 = Vec<u8>;

pvec! {
    /// Cast metadata.
    ///
    /// OsType: `'Cinf'`
    #[derive(Clone, Debug)]
    pub(crate) struct CastMetadata {
        #[br(assert(header_size == 4, "unexpected Cinf header size {}", header_size))]
        header_size = header_size;

        header {}

        offsets = offsets;

        entries {
            // entry 1 appears to contain a struct of
            // { i16, i16, i16, i16, i32 }, and entry 2 appears to contain a
            // struct of { i16, i16, i16, i16 }, but neither are used for
            // playback
            0..=2 => _,
            /// For an external cast, the original file path.
            3 => file_path: PString,
            4..
        }
    }
}
typed_resource!(CastMetadata => b"Cinf");

pvec! {
    /// Cross-cast links.
    ///
    /// If a cast member in one cast references a cast member in another
    /// cast (e.g. a bitmap palette, a Lingo `member of castLib`), the global
    /// cast library numbers and paths to those other casts will be given in
    /// this resource.
    ///
    /// OsType: `'ccl '`
    #[derive(Clone, Debug)]
    pub(crate) struct Ccl {
        #[br(assert(header_size == 4, "unexpected ccl header size {}", header_size))]
        header_size = header_size;

        header {}

        offsets = offsets;

        entries {
            #[br(args(offsets.clone()))]
            _ => links: CclEntries,
        }
    }
}
typed_resource!(Ccl => b"ccl ");

#[derive(Clone, Debug, Deref, DerefMut)]
pub(crate) struct CclEntries(Vec<CclEntry>);

impl BinRead for CclEntries {
    type Args = (PVecOffsets, );

    fn read_options<R: Read + Seek>(reader: &mut R, options: &binrw::ReadOptions, (offsets, ): Self::Args) -> binrw::BinResult<Self> {
        restore_on_error(reader, |reader, _| {
            use binrw::error::Context;
            let mut options = *options;
            options.endian = binrw::Endian::Big;

            let count = offsets.len();
            let mut data = Vec::with_capacity(count);
            for index in 0..count {
                // In OD, it is actually expected that there will never be holes
                // in the resource data.
                if let Some(pad_size_to) = offsets.entry_size(index) {
                    let pad_size_to = u64::from(pad_size_to);
                    let pos = reader.pos()?;
                    data.push(CclEntry::read_options(reader, &options, ())
                        .context(|| format!("bad ccl entry {}", index))?);
                    let bytes_read = reader.pos()? - pos;
                    if bytes_read < pad_size_to {
                        reader.skip(pad_size_to - bytes_read)?;
                    }
                } else {
                    return Err(binrw::Error::AssertFail {
                        pos: reader.pos()?,
                        message: format!("unexpected sparse CclEntries; index {} is empty", index)
                    });
                }
            }
            Ok(Self(data))
        })
    }
}

#[derive(BinRead, Clone, Debug)]
pub(crate) struct CclEntry {
    /// The library number of a linked cast. If the value is zero, the cast
    /// is an external cast.
    // RE: This value is conditionally negated at runtime, which is very
    // confusing. When this happens it seems to be that the CCL list changes
    // and then it is trying to rediscover an internal cast by an external file
    // name?
    global_cast_lib_num: GlobalLibNum,
    /// A path or name of an externally linked cast. Populated for internal
    /// casts, but not used in that case.
    path: PString,
}

pvec! {
    /// Cast member metadata.
    ///
    /// OsType: `'VWCI'`
    #[derive(Clone, Debug)]
    pub(super) struct MemberMetadata {
        #[br(assert(header_size == 16 || header_size == 20, "unexpected VWCI header size {}", header_size))]
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
            0 => script_text: RawString,

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
            6 => entry_6: StxtSub,

            #[br(count = offsets.entry_size(7).unwrap_or(0))]
            7 => entry_7: Struct14h,

            #[br(count = offsets.entry_size(8).unwrap_or(0))]
            8 => entry_8: Struct14h,

            // xtra-related
            #[br(count = offsets.entry_size(9).unwrap_or(0))]

            9 => entry_9: Struct9_4a2de0,

            10 => xtra_name: NullString,

            // script related
            #[br(count = offsets.entry_size(11).unwrap_or(0))]
            11 => entry_11: StructB_4a2e00,

            // xtra-related
            #[br(count = offsets.entry_size(12).unwrap_or(0))]
            12 => entry_12: StructC_4a2dc0,

            #[br(count = offsets.entry_size(13).unwrap_or(0))]
            13 => entry_13: StructD_439630,

            // for some reason there is a video-related entry in slot 14, but
            // it seems to not ever be referenced in projector code.
            14..
        }
    }
}
typed_resource!(MemberMetadata => b"VWCI");

bitflags! {
    // TODO: These are probably the `moreFlags` that are stuffed into the cast
    // member after the `kind`. That would also fit with the way that the data
    // was originally stored in VWCR with the kind byte and then flags byte.
    #[derive(Default)]
    struct LegacyMemberFlags: u8 {
        const FLAG_1  = 1;
        const FLAG_2  = 2;
        const FLAG_4  = 4;
        const FLAG_8  = 8;
        const FLAG_10 = 0x10;
        const FLAG_20 = 0x20;
        const FLAG_40 = 0x40;
        const FLAG_80 = 0x80;
    }
}

/// A cast member.
///
/// OsType: `'CASt'`
#[derive(Clone, Debug, Default)]
pub(crate) struct Member {
    load_id: LoadId,
    next_free: i16,
    some_num_a: i16,
    flags: MemberFlags,
    more_flags: LegacyMemberFlags,
    metadata: Option<MemberMetadata>,
    properties: MemberProperties,
}
typed_resource!(Member => b"CASt");

impl BinRead for Member {
    type Args = (ChunkIndex, ConfigVersion);

    fn read_options<R: io::Read + io::Seek>(input: &mut R, options: &binrw::ReadOptions, (chunk_index, version): Self::Args) -> binrw::BinResult<Self> {
        restore_on_error(input, |input, _| {
            use binrw::error::Context;
            let mut options = *options;
            options.endian = binrw::Endian::Big;

            let properties;
            let metadata;
            let more_flags;
            if version < ConfigVersion::V1201 {
                let header = MemberHeaderV4::read_options(input, &options, ())?;
                let mut size = header.registry_size - 1;
                more_flags = if header.kind.has_extra_flags() {
                    size -= 1;
                    LegacyMemberFlags::read_options(input, &options, ())?
                } else {
                    <_>::default()
                };
                properties = MemberProperties::read_options(&mut input.take_seek(size.into()), &options, (header.kind, size.into(), version))
                    .context(|| format!("error reading {} cast member properties", header.kind))?;
                metadata = if header.metadata_size == 0 {
                    None
                } else {
                    Some(MemberMetadata::read_options(&mut input.take_seek(header.metadata_size.into()), &options, ())
                        .context(|| format!("error reading {} cast member metadata", header.kind))?)
                };
            } else {
                let header = MemberHeaderV5::read_options(input, &options, ())?;
                metadata = if header.metadata_size == 0 {
                    None
                } else {
                    Some(MemberMetadata::read_options(&mut input.take_seek(header.metadata_size.into()), &options, ())
                        .context(|| format!("error reading {} cast member metadata", header.kind))?)
                };
                properties = MemberProperties::read_options(&mut input.take_seek(header.properties_size.into()), &options, (header.kind, header.properties_size, version))
                    .context(|| format!("error reading {} cast member properties", header.kind))?;
                more_flags = <_>::default();
            };

            Ok(Self {
                load_id: LoadId::Riff(chunk_index),
                next_free: 0,
                some_num_a: 0,
                flags: <_>::default(),
                more_flags,
                metadata,
                properties,
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

// TODO: This is guesswork, but probably correct.
#[derive(BinRead, Copy, Clone, Debug)]
#[br(big)]
struct MemberHeaderV4 {
    registry_size: u16,
    metadata_size: u32,
    #[br(try_map = |kind: u8| MemberKind::from_u8(kind).ok_or(FromPrimitiveError("cast member kind", kind)))]
    kind: MemberKind,
}

#[derive(BinRead, Copy, Clone, Debug)]
#[br(big)]
pub(super) struct MemberHeaderV5 {
    #[br(try_map = |kind: u32| MemberKind::from_u32(kind).ok_or(FromPrimitiveError("cast member kind", kind)))]
    kind: MemberKind,
    metadata_size: u32,
    properties_size: u32,
}

bitflags! {
    #[derive(Default)]
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
pub(crate) enum MemberKind {
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

impl MemberKind {
    /// In D4-, there is an extra flags-like byte for certain kinds of members.
    fn has_extra_flags(self) -> bool {
        matches!(self,
            MemberKind::Bitmap
            | MemberKind::Button
            | MemberKind::DigitalVideo
            | MemberKind::Field
            | MemberKind::FilmLoop
            | MemberKind::Movie
            | MemberKind::Shape
            | MemberKind::Script
        )
    }
}

#[derive(Clone, Debug, SmartDefault)]
pub(super) enum MemberProperties {
    #[default]
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
    type Args = (MemberKind, u32, ConfigVersion);

    fn read_options<R: io::Read + io::Seek>(input: &mut R, options: &binrw::ReadOptions, (kind, size, version): Self::Args) -> binrw::BinResult<Self> {
        restore_on_error(input, |input, _| {
            let mut options = *options;
            options.endian = binrw::Endian::Big;

            Ok(match kind {
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
