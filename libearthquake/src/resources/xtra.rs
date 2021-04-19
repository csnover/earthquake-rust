use binrw::BinRead;
use crate::util::RawString;

#[derive(BinRead, Clone, Debug)]
#[br(big, import(size: u32))]
pub(super) struct Properties {
    // TODO: Load function should receive the global symbol table and be
    // converted to a symbol number instead of storing the name
    #[br(assert(size >= name_size + 4, "Xtra properties symbol name too big ({} < {})", size - 4, name_size))]
    name_size: u32,
    #[br(count = name_size)]
    symbol_name: RawString,
    // TODO: The rest.
    #[br(assert(size >= data_size + name_size + 8, "Xtra properties data too big ({} < {})", size - name_size - 8, data_size))]
    data_size: u32,
    #[br(count = data_size)]
    data: Vec<u8>,
}
