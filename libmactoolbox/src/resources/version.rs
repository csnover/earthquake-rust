use binread::BinRead;
use crate::{errors::ScriptError, script_manager::CountryCode, types::PString};
use num_derive::FromPrimitive;
use num_traits::FromPrimitive;

#[derive(BinRead, Debug, Eq, FromPrimitive, Ord, PartialEq, PartialOrd)]
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
    #[br(try_map = |input: u16|
        CountryCode::from_u16(input)
            .ok_or(ScriptError::BadCountryCode(input))
    )]
    country_code: CountryCode,
    short_version: PString,
    long_version: PString,
}

impl Version {
    #[must_use]
    pub fn country_code(&self) -> CountryCode {
        self.country_code
    }
}
