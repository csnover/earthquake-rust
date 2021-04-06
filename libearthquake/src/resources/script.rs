use binrw::BinRead;
use smart_default::SmartDefault;

#[derive(BinRead, Clone, Copy, Debug, Eq, PartialEq, SmartDefault)]
#[br(big, repr(u16))]
enum Kind {
    Score = 1,
    #[default]
    Movie = 3,
    Parent = 7,
}

#[derive(BinRead, Clone, Copy, Debug)]
#[br(big, import(size: u32), pre_assert(size == 0 || size == 2))]
pub struct Properties {
    #[br(if(size == 2))]
    kind: Kind,
}
