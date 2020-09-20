use anyhow::{Context, Result as AResult};
use bitflags::bitflags;
use byteordered::{ByteOrdered, Endianness};
use crate::{assert_sample, ensure_sample};
use libcommon::{Resource, Reader};
use libmactoolbox::{Point, Rect};
use num_derive::FromPrimitive;
use num_traits::FromPrimitive;
use super::cast::MemberId;

bitflags! {
    // TODO: These are tested when painting colour bitmaps, but there does not
    // seem to be a way to actually set them and they are currently 0 in all
    // available corpus data. They seem to be related to setting a global
    // palette and using a dithering pen to paint.
    struct Flags: u8 {
        const FLAG_1 = 1;
        const FLAG_2 = 2;
        // This flag is found on resources in the wild but there does not seem
        // to be anything that it corresponds to, nor does it seem to be ever
        // used in a projector
        const FLAG_8 = 8;
    }
}

#[derive(Clone, Copy, Debug)]
pub struct Meta {
    is_pixmap: bool,
    row_bytes: i16,
    bounds: Rect,
    origin: Point,
    flags: Flags,
    color_depth: u8,
    palette_id: MemberId,
}

impl Resource for Meta {
    type Context = ();

    fn load<T: Reader>(input: &mut ByteOrdered<T, Endianness>, size: u32, _: &Self::Context) -> AResult<Self> where Self: Sized {
        ensure_sample!(size == 22 || size == 28, "Unexpected bitmap meta resource size {} (should be 22 or 28)", size);
        let (is_pixmap, row_bytes) = {
            let data = input.read_i16().context("Can’t read row bytes")?;
            (data < 0, data & 0x7fff)
        };
        ensure_sample!(row_bytes <= 0x3fff, "Unexpected row bytes size {}", row_bytes);
        let bounds = Rect::load(input, Rect::SIZE, &()).context("Can’t read bounds")?;
        input.skip(8).context("Can’t skip unused data at 0x16")?;
        let origin = Point::load(input, Point::SIZE, &()).context("Can’t read origin")?;
        let (
            flags,
            color_depth,
            palette_id
        ) = if size > 22 {(
            {
                let value = input.read_u8().context("Can’t read bitmap flags")?;
                Flags::from_bits(value).with_context(|| format!("Invalid bitmap flags (0x{:x})", value))?
            },
            input.read_u8().context("Can’t read color depth")?,
            MemberId::load(input, MemberId::SIZE, &()).context("Can’t read palette ID")?,
        )} else {(
            Flags::empty(),
            1,
            MemberId::default()
        )};

        Ok(Self {
            is_pixmap,
            row_bytes,
            bounds,
            origin,
            flags,
            color_depth,
            palette_id,
        })
    }
}
