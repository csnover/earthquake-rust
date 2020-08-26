use anyhow::{Context, Result as AResult};
use byteordered::{ByteOrdered, Endianness};
use libcommon::Reader;
use super::cast::MemberMetadata;

pub fn load_metadata(input: &mut ByteOrdered<impl Reader, Endianness>) -> AResult<MemberMetadata> {
    todo!()
}
