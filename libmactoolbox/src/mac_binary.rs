use anyhow::{bail, Context, Result as AResult};
use byteorder::{ByteOrder, BigEndian};
use crate::{OSType, os};
use crc::crc16::checksum_x25;
use libcommon::{Reader, SharedStream};
use std::io::{Cursor, SeekFrom};
use super::script_manager::decode_text;

#[derive(Debug)]
pub struct MacBinary<T: Reader> {
    input: SharedStream<T>,
    name: String,
    data_fork_start: u64,
    data_fork_end: u64,
    resource_fork_start: u64,
    resource_fork_end: u64,
}

impl<T: Reader> MacBinary<T> {
    fn build(data: T, header: &[u8], version: Version) -> Self {
        const BLOCK_SIZE: u32 = 128;
        const HEADER_SIZE: u32 = 128;
        const SCRIPT_FLAG: u8 = 0x80;

        let aligned_header_size = HEADER_SIZE + if version == Version::V1 {
            0
        } else {
            align_power_of_two(u32::from(BigEndian::read_u16(&header[120..])), BLOCK_SIZE)
        };

        let data_fork_size = BigEndian::read_u32(&header[83..]);

        let script_code = if version == Version::V3 && header[106] & SCRIPT_FLAG != 0 {
            header[106] & !SCRIPT_FLAG
        } else {
            // TODO: Chardet
            0
        };

        let name = {
            let raw_name = &header[2..2 + usize::from(header[1])];
            decode_text(&mut Cursor::new(raw_name), script_code)
        };

        let data_fork_start = u64::from(aligned_header_size);
        let data_fork_end = data_fork_start + u64::from(data_fork_size);
        let resource_fork_start = u64::from(aligned_header_size + align_power_of_two(data_fork_size, BLOCK_SIZE));
        let resource_fork_end = resource_fork_start + u64::from(BigEndian::read_u32(&header[87..]));

        Self {
            input: SharedStream::from(data),
            name,
            data_fork_start,
            data_fork_end,
            resource_fork_start,
            resource_fork_end,
        }
    }

    pub fn new(mut data: T) -> AResult<Self> {
        let start_pos = data.pos()?;
        let header = {
            let mut header = [ 0; 128 ];
            data.read_exact(&mut header).context("File too small")?;
            data.seek(SeekFrom::Start(start_pos))?;
            header
        };

        if header[0] != 0 {
            bail!("Bad magic byte 0");
        }

        if header[74] != 0 {
            bail!("Bad magic byte 1");
        }

        if OSType::from(BigEndian::read_u32(&header[102..])) == os!(b"mBIN") {
            return Ok(Self::build(data, &header, Version::V3));
        }

        // According to https://entropymine.wordpress.com/2019/02/13/detecting-macbinary-format/,
        // some MacBinary II encoders would just leave the checksum empty,
        // so while a matching non-zero checksum is a true positive,
        // anything else may be a false negative
        let v2_checksum = BigEndian::read_u16(&header[124..]);
        if (v2_checksum != 0 && checksum_x25(&header[0..124]) == v2_checksum) ||
           (v2_checksum == 0 && header[122] == 129 && header[123] == 129) {
            return Ok(Self::build(data, &header, Version::V2));
        }

        if header[82] != 0 {
            bail!("Bad magic byte 2");
        }

        for byte in &header[101..=125] {
            if *byte != 0 {
                bail!("Bad header padding");
            }
        }

        if header[1] < 1 || header[1] > 63 {
            bail!("Bad filename length");
        }

        let resource_size = BigEndian::read_u32(&header[83..]);
        let data_size = BigEndian::read_u32(&header[87..]);

        if resource_size > 0x7f_ffff || data_size > 0x7f_ffff || (resource_size == 0 && data_size == 0) {
            bail!("Bad fork length");
        }

        Ok(Self::build(data, &header, Version::V1))
    }

    #[must_use]
    pub fn data_fork(&self) -> Option<SharedStream<T>> {
        if self.data_fork_start == self.data_fork_end {
            None
        } else {
            Some(self.input.substream(self.data_fork_start, self.data_fork_end))
        }
    }

    #[must_use]
    pub fn name(&self) -> &String {
        &self.name
    }

    #[must_use]
    pub fn resource_fork(&self) -> Option<SharedStream<T>> {
        if self.resource_fork_start == self.resource_fork_end {
            None
        } else {
            Some(self.input.substream(self.resource_fork_start, self.resource_fork_end))
        }
    }
}

#[derive(Copy, Clone, Debug, Eq, PartialEq)]
enum Version {
    V1,
    V2,
    V3,
}

#[inline]
fn align_power_of_two(n: u32, mut align: u32) -> u32 {
    align -= 1;
    (n + align) & !(align)
}

#[cfg(test)]
mod tests {
    #![allow(clippy::wildcard_imports)]
    use super::*;

    #[test]
    fn validate() {
        use std::io::Cursor;
        const DATA: &'_ [u8] = include_bytes!("./tests/data/mac_binary/test.bin");
        let data = Cursor::new(DATA);
        let header = MacBinary::new(data).unwrap();
        assert_eq!(header.name, "File I/O TextFile");
        assert!(header.data_fork_end <= header.resource_fork_start);
        assert!(header.resource_fork_end as usize <= DATA.len());
    }
}
