use anyhow::{Context, Result as AResult};
use bitflags::bitflags;
use byteordered::{ByteOrdered, Endianness};
use crate::{assert_sample, ensure_sample};
use libcommon::{Resource, Reader};
use libmactoolbox::{Point, Rect};
use num_derive::FromPrimitive;
use num_traits::FromPrimitive;
use super::cast::MemberId;

#[derive(Clone, Copy, Debug)]
pub struct Meta {
    is_pixmap: bool,
    row_bytes: i16,
    bounds: Rect,
    origin: Point,
    field_22: u8,
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
        let bounds = Rect::load(input, Rect::SIZE, &()).context("Can’t read bounds")?;
        let origin = Point::load(input, Point::SIZE, &()).context("Can’t read origin")?;
        let (
            field_22,
            color_depth,
            palette_id
        ) = if size > 22 {(
                input.read_u8().context("Can’t read field_22")?,
                input.read_u8().context("Can’t read color depth")?,
                MemberId::load(input, MemberId::SIZE, &()).context("Can’t read palette ID")?,
        )} else {(
            0,
            1,
            MemberId::default()
        )};

        Ok(Self {
            is_pixmap,
            row_bytes,
            bounds,
            origin,
            field_22,
            color_depth,
            palette_id,
        })
    }
}
