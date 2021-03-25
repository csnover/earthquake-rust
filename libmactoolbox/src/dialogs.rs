// TODO: You know, finish this file and then remove these overrides
#![allow(dead_code)]
#![allow(clippy::unused_self)]

use binrw::BinRead;
use bitstream_io::{BigEndian, BitReader};
use crate::Rect;
use libcommon::SeekExt;

#[derive(Clone, Copy, Debug)]
struct Alert {
    bounds_rect: Rect,
    ditl_id: i16,
    stages: [AlertStage; 4],
    auto_position: Option<u16>,
}

#[derive(Clone, Copy, Debug, Default)]
struct AlertStage {
    bold_outline: bool,
    draw_alert: bool,
    beeps: u8,
}

impl BinRead for Alert {
    type Args = ();
    fn read_options<R: binrw::io::Read + binrw::io::Seek>(mut input: &mut R, options: &binrw::ReadOptions, _: Self::Args) -> binrw::BinResult<Self> {
        let size = input.bytes_left()?;
        let mut options = *options;
        options.endian = binrw::Endian::Big;

        let bounds_rect = Rect::read_options(input, &options, ())?;
        let ditl_id = i16::read_options(input, &options, ())?;
        let mut stages = [AlertStage::default(); 4];

        {
            let mut bits = BitReader::endian(&mut input, BigEndian);

            for mut stage in &mut stages[..].iter_mut().rev() {
                stage.bold_outline = bits.read_bit()?;
                stage.draw_alert = bits.read_bit()?;
                stage.beeps = bits.read(2)?;
            }
        }

        let auto_position = if size > 12 {
            Some(u16::read_options(input, &options, ())?)
        } else {
            None
        };

        Ok(Alert {
            bounds_rect,
            ditl_id,
            stages,
            auto_position,
        })
    }
}

struct Dialogs {
    param_text: [String; 4],
}

impl Dialogs {
    pub fn alert(&self, _alert_id: i16) -> i16 {
        todo!("alert dialog")
    }

    pub fn param_text<T: AsRef<str>>(&mut self, param0: T, param1: T, param2: T, param3: T) {
        self.param_text[0] = param0.as_ref().to_owned();
        self.param_text[1] = param1.as_ref().to_owned();
        self.param_text[2] = param2.as_ref().to_owned();
        self.param_text[3] = param3.as_ref().to_owned();
    }

    pub fn stop_alert(&self, _alert_id: i16) -> i16 {
        todo!("stop alert dialog")
    }
}
