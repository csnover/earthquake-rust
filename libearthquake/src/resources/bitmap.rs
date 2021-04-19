use binrw::BinRead;
use libcommon::{bitflags, io::prelude::*, prelude::*};
use libmactoolbox::quickdraw::{Point, Rect};
use super::cast::MemberId;

bitflags! {
    // TODO: These are tested when painting colour bitmaps, but there does not
    // seem to be a way to actually set them and they are currently 0 in all
    // available corpus data. They seem to be related to setting a global
    // palette and using a dithering pen to paint.
    #[derive(Default)]
    struct Flags: u8 {
        const FLAG_1 = 1;
        const FLAG_2 = 2;
        // This flag is found on resources in the wild but there does not seem
        // to be anything that it corresponds to, nor does it seem to be ever
        // used in a projector
        const FLAG_8 = 8;
    }
}

#[derive(BinRead, Clone, Copy)]
#[br(big, import(size: u32), pre_assert(size == 22 || size == 26 || size == 28, "bad bitmap properties size ({})", size))]
pub(super) struct Properties {
    #[br(assert(row_bytes & 0x7fff < 0x4000))]
    row_bytes: i16,
    bounds: Rect,
    // There was a rect here, but it is unused
    #[br(pad_before(8))]
    origin: Point,
    #[br(if(size >= 28))]
    flags: Flags,
    #[br(if(size >= 26), parse_with = parse_color_depth, args(size))]
    #[br(assert(matches!(color_depth, 0 | 1 | 2 | 4 | 8 | 16 | 24 | 32), "bad bitmap color depth {}", color_depth))]
    color_depth: u8,
    #[br(if(size >= 26), parse_with = MemberId::parse_num, args(size >= 28))]
    palette_id: MemberId,
}

fn parse_color_depth<R: Read + Seek>(input: &mut R, options: &binrw::ReadOptions, (size, ): (u32, )) -> binrw::BinResult<u8> {
    if size == 26 {
        Ok(i16::read_options(input, options, ())?.unwrap_into())
    } else {
        u8::read_options(input, options, ())
    }
}

impl std::fmt::Debug for Properties {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct(std::any::type_name::<Self>())
            .field("row_bytes", &self.row_bytes())
            .field("bounds", &self.bounds)
            .field("origin", &self.origin)
            .field("flags", &self.flags)
            .field("color_depth", &self.color_depth)
            .field("palette_id", &self.palette_id)
            .field("(is_pixmap)", &self.is_pixmap())
            .finish()
    }
}

impl Properties {
    #[must_use]
    pub(super) fn is_pixmap(&self) -> bool {
        self.row_bytes < 0
    }

    #[must_use]
    pub(super) fn row_bytes(&self) -> i16 {
        self.row_bytes & 0x7fff
    }
}
