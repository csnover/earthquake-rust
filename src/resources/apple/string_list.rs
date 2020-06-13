use anyhow::{Context, Result as AResult};
use byteordered::{ByteOrdered, Endianness};
use crate::{
    macos::System,
    Reader,
    resources::Resource,
    string::StringReadExt,
};
use derive_more::{Deref, DerefMut, Index, IndexMut, IntoIterator};

#[derive(Clone, Debug, Deref, DerefMut, Index, IndexMut, IntoIterator)]
pub struct StringList(Vec<String>);

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
