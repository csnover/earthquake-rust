use anyhow::Result as AResult;
use byteordered::{ByteOrdered, Endianness};
use crate::Reader;
use std::io::Read;

pub trait Resource : std::fmt::Debug {
    fn load<T: Reader>(input: &mut ByteOrdered<T, Endianness>, size: u32) -> AResult<Self> where Self: Sized;
}

impl Resource for Vec<u8> {
    fn load<T: Reader>(input: &mut ByteOrdered<T, Endianness>, size: u32) -> AResult<Self> where Self: Sized {
        let mut vec = Vec::with_capacity(size as usize);
        input.take(u64::from(size)).read_to_end(&mut vec)?;
        Ok(vec)
    }
}
