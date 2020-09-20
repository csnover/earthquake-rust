use anyhow::{Context, Result as AResult};
use byteordered::{ByteOrdered, Endianness};
use crate::ensure_sample;
use libcommon::{Resource, Reader};
use num_derive::FromPrimitive;
use num_traits::FromPrimitive;

#[derive(Clone, Copy, Debug, Eq, FromPrimitive, PartialEq)]
enum Kind {
    Score = 1,
    Movie = 3,
    Parent = 7,
}

#[derive(Clone, Copy, Debug)]
pub struct Meta {
    kind: Kind,
}

impl Resource for Meta {
    type Context = ();

    fn load<T: Reader>(input: &mut ByteOrdered<T, Endianness>, size: u32, _: &Self::Context) -> AResult<Self> where Self: Sized {
        ensure_sample!(size == 0 || size == 2, "Unexpected script meta resource size {} (should be 0 or 2)", size);
        let kind = if size == 2 {
            let value = input.read_u16().context("Can’t read script kind")?;
            Kind::from_u16(value).with_context(|| format!("Invalid script kind {}", value))?
        } else {
            Kind::Movie
        };

        Ok(Self {
            kind,
        })
    }
}
