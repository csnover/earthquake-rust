use anyhow::{Context, Result as AResult};
use bitflags::bitflags;
use byteordered::{ByteOrdered, Endianness};
use crate::{assert_sample, ensure_sample};
use libcommon::{Resource, Reader};
use libmactoolbox::{Point, Rect};
use num_derive::FromPrimitive;
use num_traits::FromPrimitive;
use super::{cast::MemberId, config::Version as ConfigVersion};

#[derive(Clone, Copy, Debug)]
pub struct Meta {
    // TODO: Load function should receive the global symbol table and be
    // converted to a symbol number instead of storing the name
    symbol_name: [u8; 32],
    // TODO: The rest.
    todo_the_rest: [u8; 32],
}

impl Resource for Meta {
    type Context = ();

    fn load<T: Reader>(input: &mut ByteOrdered<T, Endianness>, size: u32, _: &Self::Context) -> AResult<Self> where Self: Sized {
        let name_size = input.read_u32().context("Can’t read Xtra name size")?;
        println!("{}", name_size);
        let symbol_name = Vec::<u8>::load(input, name_size, &()).context("Can’t read Xtra name")?;
        let todo_the_rest = Vec::<u8>::load(input, size - name_size, &()).context("Can’t read TODO Xtra the rest")?;

        let mut this = Self {
            symbol_name: [0; 32],
            todo_the_rest: [0; 32],
        };
        let symbol_name_size = std::cmp::min(symbol_name.len(), 32);
        let todo_the_rest_size = std::cmp::min(todo_the_rest.len(), 32);
        this.symbol_name[0..symbol_name_size].copy_from_slice(&symbol_name[0..symbol_name_size]);
        this.todo_the_rest[0..todo_the_rest_size].copy_from_slice(&todo_the_rest[0..todo_the_rest_size]);

        Ok(this)
    }
}

#[derive(Clone, Copy, Debug)]
pub struct TransitionMeta {
    field_20: u8,
    chunk_size: u8,
    kind: u8,
    flags: u8,
    duration: i16,
}

impl Resource for TransitionMeta {
    type Context = (ConfigVersion, );

    fn load<T: Reader>(input: &mut ByteOrdered<T, Endianness>, size: u32, context: &Self::Context) -> AResult<Self> where Self: Sized {
        ensure_sample!(size == 4 || size == 6, "Unexpected film loop meta resource size {} (should be 4 or 6)", size);
        let field_20 = input.read_u8().context("Can’t read transition field_20")?;
        let chunk_size = input.read_u8().context("Can’t read transition chunk size")?;
        let kind = input.read_u8().context("Can’t read transition kind")?;
        let flags = input.read_u8().context("Can’t read transition flags")?;
        let duration = if context.0 < ConfigVersion::V1214 {
            i16::from(field_20) * 15
        } else {
            input.read_i16().context("Can’t read transition duration")?
        };

        Ok(Self {
            field_20,
            chunk_size,
            kind,
            flags,
            duration,
        })
    }
}
