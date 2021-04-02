use binrw::{BinRead, BinReaderExt, io::{Cursor, Read, Seek}};
use libcommon::restore_on_error;
use crate::resources::{cast::MemberId, transition::{Kind as TransitionKind, QuarterSeconds}};
use num_traits::FromPrimitive;
use smart_default::SmartDefault;
use super::{Tempo, Version};

#[derive(Clone, Copy, Debug, SmartDefault)]
pub enum Transition {
    #[default]
    None,
    Cast(MemberId),
    LegacyTempo(Tempo),
    Legacy {
        chunk_size: u8,
        which_transition: TransitionKind,
        time: QuarterSeconds,
        change_area: bool,
        tempo: Tempo,
    },
}

impl Transition {
    pub(super) fn tempo(&self) -> Tempo {
        match self {
            Self::Legacy { tempo, .. } | Self::LegacyTempo(tempo) => *tempo,
            Self::None | Self::Cast(..) => Tempo::default(),
        }
    }
}

impl BinRead for Transition {
    type Args = (Version, );

    fn read_options<R: Read + Seek>(reader: &mut R, _: &binrw::ReadOptions, (version, ): Self::Args) -> binrw::BinResult<Self> {
        restore_on_error(reader, |reader, pos| {
            let make_tempo = |tempo: u8| {
                Tempo::new((tempo as i8).into()).map_err(|e| binrw::Error::AssertFail {
                    pos,
                    message: format!("{}", e),
                })
            };

            let mut data = [ 0; 4 ];
            reader.read_exact(&mut data)?;
            Ok(if version < Version::V6 {
                if data[3] == 0 {
                    if data[2] == 0 {
                        Self::None
                    } else {
                        Self::LegacyTempo(make_tempo(data[2])?)
                    }
                } else {
                    Self::Legacy {
                        chunk_size: data[1],
                        which_transition: TransitionKind::from_u8(data[3])
                            .ok_or_else(|| binrw::Error::AssertFail {
                                pos,
                                message: format!("Invalid transition kind {}", data[3]),
                            })?,
                        time: QuarterSeconds(data[0] & !0x80),
                        change_area: data[0] & 0x80 != 0,
                        tempo: make_tempo(data[2])?
                    }
                }
            } else if data == [ 0; 4 ] {
                Self::None
            } else {
                let reader = &mut Cursor::new(data);
                Self::Cast(reader.read_be::<MemberId>()?)
            })
        })
    }
}
