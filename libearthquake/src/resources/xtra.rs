use binrw::BinRead;

#[derive(BinRead, Clone, Debug)]
#[br(big, import(size: u32))]
pub struct Meta {
    // TODO: Load function should receive the global symbol table and be
    // converted to a symbol number instead of storing the name
    #[br(assert(size >= name_size + 4))]
    name_size: u32,
    #[br(count = name_size)]
    symbol_name: Vec<u8>,
    // TODO: The rest.
    #[br(assert(size >= data_size + name_size + 8))]
    data_size: u32,
    #[br(count = data_size)]
    data: Vec<u8>,
}
