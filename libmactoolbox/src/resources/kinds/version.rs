use binrw::BinRead;
use core::convert::TryFrom;
use crate::{intl::CountryCode, types::PString};

#[derive(BinRead, Debug, Eq, Ord, PartialEq, PartialOrd)]
#[br(big, repr(u8))]
pub enum Stage {
    Dev   = 0x20,
    Alpha = 0x40,
    Beta  = 0x60,
    Final = 0x80,
}

#[derive(BinRead, Debug)]
#[br(big)]
pub struct Number {
    major: u8,
    minor: u8,
    stage: Stage,
    revision: u8,
}

#[derive(BinRead, Debug)]
#[br(big)]
pub struct Version {
    version: Number,
    #[br(try_map = |input: u16| CountryCode::try_from(input))]
    country_code: CountryCode,
    short_version: PString,
    long_version: PString,
}
