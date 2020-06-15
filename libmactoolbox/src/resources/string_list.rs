use anyhow::{Context, Result as AResult};
use byteordered::{ByteOrdered, Endianness};
use crate::{
    System,
    string::ReadExt,
};
use derive_more::{Deref, DerefMut, Index, IndexMut, IntoIterator};
use libcommon::{
    Reader,
    Resource,
};

#[derive(Clone, Debug, Deref, DerefMut, Index, IndexMut, IntoIterator)]
#[into_iterator(owned, ref, ref_mut)]
pub struct StringList(Vec<String>);

impl StringList {
    #[must_use]
    pub fn into_vec(mut self) -> Vec<String> {
        std::mem::take(&mut self.0)
    }
}

impl Resource for StringList {
    fn load<T: Reader>(input: &mut ByteOrdered<T, Endianness>, _size: u32) -> AResult<Self> where Self: Sized {
        let count = input.read_u16()
            .context("Failed to read StringList count")?;
        let mut strings = Vec::with_capacity(count as usize);
        for index in 0..count {
            strings.push(
                input.read_pascal_str(System::instance().decoder())
                .with_context(|| format!("Failed to read StringList item {}", index))?
            );
        }
        Ok(Self(strings))
    }
}
