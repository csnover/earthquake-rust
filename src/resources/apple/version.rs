use anyhow::Result as AResult;
use byteorder::BigEndian;
use byteordered::{ByteOrdered, StaticEndianness};
use crate::{macos::script_manager::CountryCode, panic_sample, Reader, string::StringReadExt};
use num_derive::FromPrimitive;
use num_traits::FromPrimitive;

#[derive(Debug, Eq, FromPrimitive, Ord, PartialEq, PartialOrd)]
pub enum Stage {
    Dev   = 0x20,
    Alpha = 0x40,
    Beta  = 0x60,
    Final = 0x80
}

#[derive(Debug)]
pub struct Version {
    major: u8,
    minor: u8,
    stage: Stage,
    revision: u8,
}

#[derive(Debug)]
pub struct Resource {
    version: Version,
    country_code: CountryCode,
    short_version: String,
    long_version: String,
}

impl Resource {
    pub fn parse<T: Reader>(input: &mut ByteOrdered<T, StaticEndianness<BigEndian>>) -> AResult<Self> {
        let version = Version {
            major: input.read_u8()?,
            minor: input.read_u8()?,
            stage: Stage::from_u8(input.read_u8()?).unwrap(),
            revision: input.read_u8()?,
        };

        let country_code = input.read_u16()?;
        let country_code = CountryCode::from_u16(country_code)
            .unwrap_or_else(|| panic_sample!("Invalid country code {}", country_code));

        Ok(Self {
            version,
            country_code,
            short_version: input.read_pascal_str(country_code.encoding())?,
            long_version: input.read_pascal_str(country_code.encoding())?,
        })
    }

    pub fn country_code(&self) -> CountryCode {
        self.country_code
    }
}
