use anyhow::Result as AResult;
use byteordered::{ByteOrdered, Endianness};
use crate::Reader;

pub(crate) mod apple;
pub(crate) mod macromedia;

pub trait Resource : std::fmt::Debug {
    fn load<T: Reader>(input: &mut ByteOrdered<T, Endianness>, size: u32) -> AResult<Self> where Self: Sized;
}
