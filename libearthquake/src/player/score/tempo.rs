use anyhow::{bail, Result as AResult};
use binrw::BinRead;
use smart_default::SmartDefault;
use super::{ChannelNum, Fps, Seconds};

#[derive(BinRead, Clone, Copy, Debug, Eq, Ord, PartialEq, PartialOrd, SmartDefault)]
#[br(try_map = Self::new)]
pub enum Tempo {
    #[default]
    Inherit,
    Fps(Fps),
    WaitForVideo(ChannelNum),
    WaitForSeconds(Seconds),
    WaitForClick,
    WaitForSound1,
    WaitForSound2,
}

impl Tempo {
    pub fn new(tempo: i16) -> AResult<Self> {
        Ok(match tempo {
            0 => Self::Inherit,
            1..=120 => Self::Fps(Fps(tempo)),
            -0x78..=-0x48 => Self::WaitForVideo(ChannelNum(tempo + 0x7e)),
            -60..=-1 => Self::WaitForSeconds(Seconds(-tempo)),
            -0x80 => Self::WaitForClick,
            -0x79 => Self::WaitForSound1,
            -0x7a => Self::WaitForSound2,
            value => bail!("Invalid tempo {}", value),
        })
    }

    #[must_use]
    pub fn to_primitive(self) -> i16 {
        match self {
            Tempo::Inherit => 0,
            Tempo::Fps(fps) => fps.0,
            Tempo::WaitForVideo(channel) => channel.0 - 0x7e,
            Tempo::WaitForSeconds(seconds) => -seconds.0,
            Tempo::WaitForClick => -0x80,
            Tempo::WaitForSound1 => -0x79,
            Tempo::WaitForSound2 => -0x7a,
        }
    }
}
