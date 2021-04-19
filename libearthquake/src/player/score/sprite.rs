use binrw::{BinRead, error::Context, io::{Read, Seek}};
use crate::resources::cast::{MemberId, MemberNum};
use libmactoolbox::quickdraw::{Pen, Point};
use derive_more::{Deref, DerefMut, From};
use libcommon::{SeekExt, Unk8, bitflags, bitflags::BitFlags, restore_on_error};
use num_traits::FromPrimitive;
use smart_default::SmartDefault;
use super::Version;

#[derive(BinRead, Copy, Clone, Debug, Eq, PartialEq, SmartDefault)]
#[br(repr(u8))]
pub(crate) enum Kind {
    #[default]
    None = 0,
    Bitmap,
    Rect,
    RoundRect,
    Oval,
    /// A line drawn from the top-left to the bottom-right.
    LineDown,
    /// A line drawn from the bottom-right to the top-left.
    LineUp,
    Field,
    Button,
    CheckBox,
    RadioButton,
    Picture,
    RectOutline,
    RoundRectOutline,
    OvalOutline,
    LineMaybe,
    Cast,
    Text,
    Script,
}

bitflags! {
    #[derive(Default)]
    pub(super) struct LineSize: u8 {
        const LINE_SIZE = 0xf;
        const BLEND     = 0x10;
        const FLAG_20   = 0x20;
        const FLAG_40   = 0x40;
        // TODO: What is this? Exists in data converted from D4 to D5
        const FLAG_80   = 0x80;
    }
}

bitflags! {
    #[derive(Default)]
    pub(super) struct Ink: u8 {
        const INK_KIND = 0x3f;
        const TRAILS   = 0x40;
        const STRETCH  = 0x80;
    }
}

bitflags! {
    #[derive(Default)]
    pub(crate) struct ScoreColor: u8 {
        const COLOR    = 0xf;
        const FLAG_10  = 0x10;
        const FLAG_20  = 0x20;
        const EDITABLE = 0x40;
        const MOVEABLE = 0x80;
    }
}

fn fix_v0_v6_sprite_kind(kind: Kind) -> Kind {
    match kind {
        Kind::Bitmap
        | Kind::Field
        | Kind::Button
        | Kind::CheckBox
        | Kind::RadioButton
        | Kind::Picture
        | Kind::Cast
        | Kind::Text => Kind::Cast,
        kind => kind
    }
}

#[derive(BinRead, Clone, Copy, Debug, Default)]
#[br(big, import(version: Version))]
pub(super) struct SpriteV3 {
    // TODO: This is an index into VWAC resource. There is no way to convert
    // this to D5, so either script field needs to be an enum (probably) or
    // extra fields should be added to Frame to store it. Also, it turns out
    // this is not the script-related field from D4.
    script: u8,
    kind: Kind,
    fore_color_index: u8,
    back_color_index: u8,
    line_size_and_flags: LineSize,
    ink_and_flags: Ink,
    id: MemberNum,
    origin: Point,
    height: i16,
    #[br(align_after(16))]
    width: i16,
}

impl From<SpriteV3> for SpriteV5 {
    fn from(old: SpriteV3) -> Self {
        SpriteV5 {
            kind: fix_v0_v6_sprite_kind(old.kind),
            ink_and_flags: old.ink_and_flags,
            id: old.id.into(),
            fore_color_index: old.fore_color_index,
            back_color_index: old.back_color_index,
            origin: old.origin,
            height: old.height,
            width: old.width,
            ..SpriteV5::default()
        }
    }
}

#[derive(BinRead, Clone, Copy, Debug, Default)]
#[br(big, import(version: Version))]
pub(super) struct SpriteV4 {
    field_0: Unk8,
    kind: Kind,
    fore_color_index: u8,
    back_color_index: u8,
    line_size_and_flags: LineSize,
    ink_and_flags: Ink,
    id: MemberNum,
    origin: Point,
    height: i16,
    width: i16,
    script: MemberNum,
    score_color_and_flags: ScoreColor,
    #[br(align_after(20))]
    blend_amount: u8,
}

impl From<SpriteV4> for SpriteV5 {
    fn from(old: SpriteV4) -> Self {
        SpriteV5 {
            kind: fix_v0_v6_sprite_kind(old.kind),
            ink_and_flags: old.ink_and_flags,
            id: old.id.into(),
            script: old.script.into(),
            fore_color_index: old.fore_color_index,
            back_color_index: old.back_color_index,
            origin: old.origin,
            height: old.height,
            width: old.width,
            score_color_and_flags: old.score_color_and_flags,
            blend_amount: old.blend_amount,
            line_size_and_flags: old.line_size_and_flags,
        }
    }
}

#[derive(BinRead, Clone, Copy, Default)]
#[br(big, import(version: Version))]
pub(crate) struct SpriteV5 {
    #[br(map = |kind: Kind| if version == Version::V7 { kind } else { fix_v0_v6_sprite_kind(kind) })]
    kind: Kind,
    ink_and_flags: Ink,
    id: MemberId,
    script: MemberId,
    fore_color_index: u8,
    back_color_index: u8,
    origin: Point,
    height: i16,
    width: i16,
    score_color_and_flags: ScoreColor,
    blend_amount: u8,
    #[br(align_after(24))]
    line_size_and_flags: LineSize,
}

#[derive(Clone, Copy, Default, Deref, DerefMut, From)]
#[from(forward)]
pub(crate) struct Sprite(SpriteV5);

impl Sprite {
    pub(super) const V0_SIZE: u16 = 20;
    pub(super) const V5_SIZE: u16 = 24;

    #[must_use]
    pub(crate) fn back_color_index(&self) -> u8 {
        self.back_color_index
    }

    #[must_use]
    pub(crate) fn blend(&self) -> bool {
        self.line_size_and_flags.contains(LineSize::BLEND)
    }

    #[must_use]
    pub(crate) fn blend_amount(&self) -> u8 {
        self.blend_amount
    }

    #[must_use]
    pub(crate) fn editable(&self) -> bool {
        self.score_color_and_flags.contains(ScoreColor::EDITABLE)
    }

    #[must_use]
    pub(crate) fn fore_color_index(&self) -> u8 {
        self.fore_color_index
    }

    #[must_use]
    pub(crate) fn height(&self) -> i16 {
        self.height
    }

    #[must_use]
    pub(crate) fn id(&self) -> MemberId {
        self.id
    }

    #[must_use]
    pub(crate) fn ink(&self) -> Pen {
        Pen::from_u8((self.ink_and_flags & Ink::INK_KIND).bits()).unwrap()
    }

    #[must_use]
    pub(crate) fn kind(&self) -> Kind {
        self.kind
    }

    #[must_use]
    pub(crate) fn line_size(&self) -> u8 {
        (self.line_size_and_flags & LineSize::LINE_SIZE).bits()
    }

    #[must_use]
    pub(crate) fn moveable(&self) -> bool {
        self.score_color_and_flags.contains(ScoreColor::MOVEABLE)
    }

    #[must_use]
    pub(crate) fn origin(&self) -> Point {
        self.origin
    }

    #[must_use]
    pub(crate) fn score_color(&self) -> u8 {
        (self.score_color_and_flags & ScoreColor::COLOR).bits()
    }

    #[must_use]
    pub(crate) fn score_color_flags(&self) -> ScoreColor {
        self.score_color_and_flags & !ScoreColor::COLOR
    }

    pub(crate) fn set_score_color_flags(&mut self, mut flags: ScoreColor) {
        flags &= !ScoreColor::COLOR;
        self.score_color_and_flags.remove(!ScoreColor::COLOR);
        self.score_color_and_flags |= flags;
    }

    #[must_use]
    pub(crate) fn script(&self) -> MemberId {
        self.script
    }

    pub(crate) fn script_mut(&mut self) -> &mut MemberId {
        &mut self.script
    }

    #[must_use]
    pub(crate) fn stretch(&self) -> bool {
        self.ink_and_flags.contains(Ink::STRETCH)
    }

    #[must_use]
    pub(crate) fn trails(&self) -> bool {
        self.ink_and_flags.contains(Ink::TRAILS)
    }

    #[must_use]
    pub(crate) fn width(&self) -> i16 {
        self.width
    }
}

impl BinRead for Sprite {
    type Args = (Version, );

    fn read_options<R: Read + Seek>(input: &mut R, options: &binrw::ReadOptions, args: Self::Args) -> binrw::BinResult<Self> {
        restore_on_error(input, |input, _| {
            let mut options = *options;
            options.endian = binrw::Endian::Big;

            let (version, ) = args;
            match version {
                Version::V3 => SpriteV3::read_options(input, &options, args).map(Self::from),
                Version::V4 => SpriteV4::read_options(input, &options, args).map(Self::from),
                Version::V5 | Version::V6 | Version::V7 => SpriteV5::read_options(input, &options, args).map(Self::from),
                Version::Unknown => Err(binrw::Error::AssertFail {
                    pos: input.pos()?,
                    message: format!("Unknown score version {}", version),
                })
            }.context(|| "Can’t read sprite")
        })
    }
}

impl std::fmt::Debug for Sprite {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct(std::any::type_name::<Self>())
            .field("back_color_index", &self.back_color_index())
            .field("blend", &self.blend())
            .field("blend_amount", &self.blend_amount())
            .field("editable", &self.editable())
            .field("fore_color_index", &self.fore_color_index())
            .field("height", &self.height())
            .field("id", &self.id())
            .field("ink", &self.ink())
            .field("kind", &self.kind())
            .field("line_size", &self.line_size())
            .field("moveable", &self.moveable())
            .field("origin", &self.origin())
            .field("score_color", &self.score_color())
            .field("script", &self.script())
            .field("stretch", &self.stretch())
            .field("trails", &self.trails())
            .field("width", &self.width())
            .finish()
    }
}
