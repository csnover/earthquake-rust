use anyhow::{Context, Result as AResult};
use bitflags::bitflags;
use byteordered::{Endianness, ByteOrdered};
use crate::{collections::riff::{Riff, ChunkIndex}, ensure_sample, pvec};
use derive_more::{Deref, DerefMut, Index, IndexMut};
use libcommon::{Reader, Resource, resource::{StringContext, StringKind}, vfs::VirtualFile, encodings::MAC_ROMAN};
use num_derive::FromPrimitive;
use num_traits::FromPrimitive;
use std::{cell::RefCell, rc::{Weak, Rc}, io::{Cursor, Read, Seek, SeekFrom}};
use super::field::load_metadata as field_load_metadata;
use libmactoolbox::Rect;

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

// #[derive(Debug)]
// struct VideoWorksCastInfo {
//     data: Vec<u8>,
//     indexes: Vec<u16>,
// }

// impl Resource for VideoWorksCastInfo {
//     type Data = ();
//     fn load<T: Reader>(input: &mut ByteOrdered<T, Endianness>, size: u32, _data: Self::Data) -> AResult<Self> where Self: Sized {
//         todo!()
//     }
// }

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
    type Context = (ChunkIndex, );
    fn load<T: Reader>(input: &mut ByteOrdered<T, Endianness>, _: u32, context: &Self::Context) -> AResult<Self> where Self: Sized {
        let mut input = ByteOrdered::new(input, Endianness::Big);
        let kind = input.read_u32()
            .with_context(|| format!("Can’t read cast member kind in chunk {}", context.0))?;
        let kind = MemberKind::from_u32(kind)
            .with_context(|| format!("Invalid cast member kind {} in chunk {}", kind, context.0))?;
        let vwci_size = input.read_u32()?;
        let vwcr_size = input.read_u32()?;

        let info = if vwci_size == 0 {
            None
        } else {
            Some(MemberInfo::load(&mut input, vwci_size, &Default::default())
                .with_context(|| format!("Invalid member info for chunk {}", context.0))?)
        };

        Ok(Self {
            riff_index: context.0,
            next_free: 0,
            some_num_a: 0,
            flags: MemberFlags::empty(),
            info,
            metadata: MemberMetadata::load(&mut input, vwcr_size, &(kind, ))?,
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

#[derive(Clone, Copy, Debug, Eq, FromPrimitive, PartialEq)]
pub enum FieldFrame {
    Fit              = 0,
    Scroll           = 1,
    Fixed            = 2,
    LimitToFieldSize = 3,
}

bitflags! {
    pub struct FieldFlags: u8 {
        const EDITABLE     = 0x1;
        const TABBABLE     = 0x2;
        const NO_WORD_WRAP = 0x4;
    }
}

#[derive(Clone, Copy, Debug, Eq, FromPrimitive, PartialEq)]
pub enum ShapeKind {
    Rect      = 1,
    RoundRect = 2,
    Oval      = 3,
    Line      = 4,
}

#[derive(Clone, Copy, Debug, Eq, FromPrimitive, PartialEq)]
pub enum ShapeLineDirection {
    TopToBottom = 5,
    BottomToTop = 6,
}

#[derive(Clone, Copy, Debug)]
pub enum MemberMetadata {
    None,
    Bitmap {

    },
    FilmLoop {

    },
    Field {
        border_size: u8,
        /// Space between the field viewport and the border.
        margin_size: u8,
        box_shadow_size: u8,
        field_frame: FieldFrame,
        field_c: i16,
        field_e: i16,
        field_10: i16,
        field_12: i16,
        scroll_top: u16,
        /// The viewport of the field, excluding decorations.
        bounds: Rect,
        /// The height of the field, excluding decorations.
        height: u16,
        text_shadow_size: u8,
        flags: FieldFlags,
        /// The total height of content, which may be larger than the viewport
        /// if the field is scrollable.
        scroll_height: u16,
    },
    Palette,
    Picture,
    Sound {

    },
    Button {

    },
    Shape {
        kind: ShapeKind,
        bounds: Rect,
        pattern: u16,
        fore_color: u8,
        back_color: u8,
        filled: bool,
        line_size: u8,
        line_direction: ShapeLineDirection,
    },
    Movie {

    },
    DigitalVideo {

    },
    Script {

    },
    // Rich text
    Text {

    },
    OLE {

    },
    // Transition-specific Xtras
    Transition {

    },
    Xtra {

    },
}

impl Resource for MemberMetadata {
    type Context = (MemberKind, );
    fn load<T: Reader>(input: &mut ByteOrdered<T, Endianness>, size: u32, context: &Self::Context) -> AResult<Self> where Self: Sized {
        Ok(match context.0 {
            MemberKind::None => {
                input.skip(u64::from(size))?;
                MemberMetadata::None
            },
            MemberKind::Field => {
                let border_size = input.read_u8().context("Can’t read border size")?;
                let margin_size = input.read_u8().context("Can’t read margin size")?;
                let box_shadow_size = input.read_u8().context("Can’t read box shadow size")?;
                let field_frame = {
                    let value = input.read_u8().context("Can’t read field frame")?;
                    FieldFrame::from_u8(value).with_context(|| format!("Invalid value {} for field frame", value))?
                };
                let field_c = input.read_i16().context("Can’t read field_c")?;
                ensure_sample!(field_c == 0, "Field_c is not 0");
                let field_e = input.read_i16().context("Can’t read field_e")?;
                ensure_sample!(field_e == -1, "Field_e is not -1");
                let field_10 = input.read_i16().context("Can’t read field_10")?;
                ensure_sample!(field_10 == -1, "Field_10 is not -1");
                let field_12 = input.read_i16().context("Can’t read field_12")?;
                ensure_sample!(field_12 == -1, "Field_12 is not -1");
                let scroll_top = input.read_u16().context("Can’t read scroll top")?;
                let bounds = Rect::load(input, 8, &()).context("Can’t read bounds")?;
                let height = input.read_u16().context("Can’t read height")?;
                ensure_sample!(height == bounds.height() as u16, "Height does not match bounds height");
                let text_shadow_size = input.read_u8().context("Can’t read text shadow size")?;
                let flags = {
                    let value = input.read_u8().context("Can’t read field flags")?;
                    FieldFlags::from_bits(value).with_context(|| format!("Invalid flags 0x{:x} for field", value))?
                };
                let scroll_height = input.read_u16().context("Can’t read scroll height")?;

                MemberMetadata::Field {
                    border_size,
                    margin_size,
                    box_shadow_size,
                    field_frame,
                    field_c,
                    field_e,
                    field_10,
                    field_12,
                    scroll_top,
                    bounds,
                    height,
                    text_shadow_size,
                    flags,
                    scroll_height,
                }
            },
            MemberKind::Shape => {
                let kind = {
                    let value = input.read_u16().context("Can’t read shape kind")?;
                    ShapeKind::from_u16(value).with_context(|| format!("Invalid shape kind {}", value))?
                };
                let bounds = Rect::load(input, 8, &()).context("Can’t read bounds")?;
                let pattern = input.read_u16().context("Can’t read shape pattern")?;
                let fore_color = input.read_u8().context("Can’t read shape foreground color")?;
                let back_color = input.read_u8().context("Can’t read shape background color")?;
                let filled = input.read_u8().context("Can’t read shape filled flag")?;
                ensure_sample!(filled == 0 || filled == 1, "Unexpected filled value {}", filled);
                let line_size = input.read_u8().context("Can’t read shape line size")?;
                ensure_sample!(line_size != 0, "Unexpected zero shape line size");
                let line_direction = {
                    let value = input.read_u8().context("Can’t read shape line kind")?;
                    ShapeLineDirection::from_u8(value)
                        .with_context(|| format!("Invalid line direction {}", value))?
                };

                MemberMetadata::Shape {
                    kind,
                    bounds,
                    pattern,
                    fore_color,
                    back_color,
                    filled: filled != 0,
                    line_size: line_size - 1,
                    line_direction,
                }
            },
            MemberKind::Button => {
                // println!("{:x?}", Vec::<u8>::load(input, size, &())?);
                MemberMetadata::None
            },
            MemberKind::Text => {
                // [0, 0, 0, 0, 0, 10, 0, 78, 0, 0, 0, 0, 0, 10, 0, 78, 1, 0, 0, 0,  0,  c, 0, 10, 0, 0, 0, 0, ff, ff, ff, ff, ff, ff] aa default 12
                // [0, 0, 0, 0, 0, 10, 0, 77, 0, 0, 0, 0, 0, 10, 0, 77, 0, 0, 0, 0, ff, f4, 0, 10, 0, 0, 0, 0, ff, ff, ff, ff, ff, ff] aa none
                // [0, 0, 0, 0, 0, 10, 0, 77, 0, 0, 0, 0, 0, 10, 0, 77, 1, 0, 0, 0, ff, f4, 0, 10, 0, 0, 0, 0, ff, ff, ff, ff, ff, ff] aa all
                // [0, 0, 0, 0, 0, 10, 0, 77, 0, 0, 0, 0, 0, 10, 0, 77, 1, 2, 0, 0,  0,  c, 0, 10, 0, 0, 0, 0, ff, ff, ff, ff, ff, ff] text crop
                // [0, 0, 0, 0, 0, 10, 0, 78, 0, 0, 0, 0, 0, 1f, 0, 78, 1, 1, 0, 0,  0,  c, 0, 1f, 0, 0, 0, 0, ff, ff, ff, ff, ff, ff] text scrollable
                // [0, 0, 0, 0, 0, 10, 0, 78, 0, 0, 0, 0, 0, 10, 0, 78, 1, 0, 0, 0,  0,  7, 0, 10, 0, 0, 0, 0, ff, ff, ff, ff, ff, ff] aa over 7
                println!("{:x?}", Vec::<u8>::load(input, size, &())?);
                MemberMetadata::None
            },
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
