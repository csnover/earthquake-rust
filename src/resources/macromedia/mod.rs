use anyhow::Result as AResult;
use byteordered::{Endianness, ByteOrdered};
use crate::{ensure_sample, Reader};
use derive_more::{Deref, DerefMut, Index, IndexMut};
use std::io::Read;
use super::Resource;

#[derive(Copy, Clone, Debug)]
pub struct Rc;
impl Resource for Rc {
    fn load<T: Reader>(input: &mut ByteOrdered<T, Endianness>, size: u32) -> AResult<Self> {
        assert_eq!(size, 8);
        input.skip(8)?;
        Ok(Self)
    }
}

#[derive(Clone, Debug, Deref, DerefMut, Index, IndexMut)]
pub struct List<T: Resource>(Vec<T>);
impl <T: Resource> Resource for List<T> {
    fn load<U: Reader>(input: &mut ByteOrdered<U, Endianness>, size: u32) -> AResult<Self> {
        Rc::load(input, 8)?;
        let used = input.read_u32()?;
        let capacity = input.read_u32()?;
        let header_size = input.read_u16()?;
        let item_size = input.read_u16()?;
        ensure_sample!(u32::from(header_size) + u32::from(item_size) * used <= size, "Bad List size at {}", input.pos()? - 0x14);
        input.skip(u64::from(header_size) - 0x14)?;
        let mut data = Vec::with_capacity(capacity as usize);
        for _ in 0..used {
            data.push(T::load(input, u32::from(item_size))?);
        }

        Ok(Self(data))
    }
}

#[derive(Clone, Debug, Deref, DerefMut, Index, IndexMut)]
pub struct ByteVec(Vec<u8>);
impl ByteVec {
    pub const HEADER_SIZE: usize = 0x12;
}
impl Resource for ByteVec {
    fn load<T: Reader>(input: &mut ByteOrdered<T, Endianness>, size: u32) -> AResult<Self> {
        Rc::load(input, 8)?;
        let used = input.read_u32()?;
        let capacity = input.read_u32()?;
        let header_size = input.read_u16()?;
        let mut data = Vec::with_capacity(capacity as usize);
        ensure_sample!(used <= size, "Bad ByteVec size at {} ({} > {})", input.pos()? - 0x12, used, size);
        ensure_sample!(header_size == 0x12, "Generic ByteVec loader called on specialised ByteVec with header size {} at {}", header_size, input.pos()? - 0x12);
        input.inner_mut().take(u64::from(used) - u64::from(header_size)).read_to_end(&mut data)?;

        Ok(Self(data))
    }
}
