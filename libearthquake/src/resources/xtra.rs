use anyhow::{Context, Result as AResult};
use byteordered::{ByteOrdered, Endianness};
use crate::ensure_sample;
use libcommon::{Resource, Reader};
use super::config::Version as ConfigVersion;

#[derive(Clone, Debug)]
pub struct Meta {
    // TODO: Load function should receive the global symbol table and be
    // converted to a symbol number instead of storing the name
    symbol_name: String,
    // TODO: The rest.
    data: Vec<u8>,
}

impl Resource for Meta {
    type Context = (ConfigVersion, );

    fn load<T: Reader>(input: &mut ByteOrdered<T, Endianness>, size: u32, _: &Self::Context) -> AResult<Self> where Self: Sized {
        let name_size = input.read_u32().context("Can’t read Xtra name size")?;
        ensure_sample!(name_size <= size - 4, "Invalid Xtra name size ({} > {})", name_size, size - 4);
        let symbol_name = String::load(input, name_size, &Default::default()).context("Can’t read Xtra name")?;
        let data_size = input.read_u32().context("Can’t read Xtra data size")?;
        ensure_sample!(data_size <= size - name_size - 8, "Invalid Xtra data size ({} > {})", data_size, size - name_size - 8);
        let data = Vec::<u8>::load(input, data_size, &()).context("Can’t read Xtra data")?;
        Ok(Self {
            symbol_name,
            data,
        })
    }
}
