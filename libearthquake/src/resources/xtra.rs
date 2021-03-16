use anyhow::{Context, Result as AResult};
use binread::BinRead;
use libcommon::{
    encodings::DecoderRef,
    Reader,
    Resource,
    resource::Input,
};
use super::config::Version as ConfigVersion;

#[derive(BinRead, Clone, Debug)]
#[br(big, import(size: u32, decoder: DecoderRef))]
pub struct Meta {
    // TODO: Load function should receive the global symbol table and be
    // converted to a symbol number instead of storing the name
    #[br(assert(size >= name_size + 4))]
    name_size: u32,
    #[br(count = name_size, map = |v: Vec<u8>| decoder.decode(&v))]
    symbol_name: String,
    // TODO: The rest.
    #[br(assert(size >= data_size + name_size + 8))]
    data_size: u32,
    #[br(count = data_size)]
    data: Vec<u8>,
}

impl Resource for Meta {
    type Context = (ConfigVersion, DecoderRef);

    fn load(input: &mut Input<impl Reader>, size: u32, context: &Self::Context) -> AResult<Self> where Self: Sized {
        Self::read_args(input, (size, context.1)).context("Can’t read Xtra meta")
    }
}
