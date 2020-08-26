use anyhow::{Context, Result as AResult};
use crate::{
    script_manager::CountryCode,
};
use libcommon::{
    Reader,
    Resource,
    string::ReadExt,
};
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
pub struct Number {
    major: u8,
    minor: u8,
    stage: Stage,
    revision: u8,
}

#[derive(Debug)]
pub struct Version {
    version: Number,
    country_code: CountryCode,
    short_version: String,
    long_version: String,
}

impl Version {
    #[must_use]
    pub fn country_code(&self) -> CountryCode {
        self.country_code
    }
}

impl Resource for Version {
    type Context = ();
    fn load<T: Reader>(input: &mut byteordered::ByteOrdered<T, byteordered::Endianness>, _: u32, _: &Self::Context) -> AResult<Self> where Self: Sized {
        let version = Number {
            major: input.read_u8()?,
            minor: input.read_u8()?,
            stage: Stage::from_u8(input.read_u8()?).unwrap(),
            revision: input.read_u8()?,
        };

        let country_code = input.read_u16()?;
        let country_code = CountryCode::from_u16(country_code)
            .with_context(|| format!("Invalid country code {}", country_code))?;

        Ok(Self {
            version,
            country_code,
            short_version: input.read_pascal_str(country_code.encoding())?,
            long_version: input.read_pascal_str(country_code.encoding())?,
        })
    }
}
