use anyhow::{Context, Result as AResult, ensure};
use byteordered::{ByteOrdered, Endianness};
use crate::{encodings::DecoderRef, Reader, string::ReadExt};
use std::{convert::TryInto, io::Read};

pub type Input<T> = ByteOrdered<T, Endianness>;

pub trait Resource : std::fmt::Debug {
    type Context;
    fn load(input: &mut Input<impl Reader>, size: u32, context: &Self::Context) -> AResult<Self> where Self: Sized;
}

impl Resource for i16 {
    type Context = ();
    fn load(input: &mut Input<impl Reader>, _: u32, _: &Self::Context) -> AResult<Self> where Self: Sized {
        Ok(input.read_i16()?)
    }
}

impl Resource for u16 {
    type Context = ();
    fn load(input: &mut Input<impl Reader>, _: u32, _: &Self::Context) -> AResult<Self> where Self: Sized {
        Ok(input.read_u16()?)
    }
}

impl Resource for i32 {
    type Context = ();
    fn load(input: &mut Input<impl Reader>, _: u32, _: &Self::Context) -> AResult<Self> where Self: Sized {
        Ok(input.read_i32()?)
    }
}

impl Resource for u32 {
    type Context = ();
    fn load(input: &mut Input<impl Reader>, _: u32, _: &Self::Context) -> AResult<Self> where Self: Sized {
        Ok(input.read_u32()?)
    }
}

impl Resource for Vec<u8> {
    type Context = ();
    fn load(input: &mut Input<impl Reader>, size: u32, _: &Self::Context) -> AResult<Self> where Self: Sized {
        let mut vec = Vec::with_capacity(size.try_into().unwrap());
        let actual = input.take(size.into()).read_to_end(&mut vec)?;
        ensure!(actual == size.try_into().unwrap(), "Expected {} bytes, read {} bytes", size, actual);
        Ok(vec)
    }
}

#[derive(Debug, Clone, Copy, Eq, PartialEq)]
pub enum StringKind {
    Sized,
    CStr,
    PascalStr,
}

pub struct StringContext(pub StringKind, pub DecoderRef);
impl Default for StringContext {
    fn default() -> Self {
        StringContext(StringKind::Sized, crate::encodings::MAC_ROMAN)
    }
}

impl Resource for String {
    type Context = StringContext;
    fn load(input: &mut Input<impl Reader>, size: u32, context: &Self::Context) -> AResult<Self> where Self: Sized {
        match context.0 {
            StringKind::Sized => Ok({
                let mut result = Vec::with_capacity(size.try_into().unwrap());
                let actual = input.take(size.into()).read_to_end(&mut result).context("Can’t read sized string")?;
                ensure!(actual == size.try_into().unwrap(), "Expected {} bytes, read {} bytes", size, actual);
                context.1.decode(&result)
            }),
            StringKind::CStr => input.read_c_str(context.1).context("Invalid C string"),
            StringKind::PascalStr => input.read_pascal_str(context.1).context("Invalid Pascal string")
        }
    }
}
