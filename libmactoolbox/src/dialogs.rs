use anyhow::Result as AResult;
use bitstream_io::{BigEndian, BitReader};
use byteordered::{ByteOrdered, Endianness};
use crate::Rect;
use libcommon::{Reader, Resource};
use qt_widgets::QApplication;
use std::io::Read;

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

impl Resource for Alert {
    fn load<T: Reader>(mut input: &mut ByteOrdered<T, Endianness>, size: u32) -> AResult<Self> where Self: Sized {
        assert!(size >= 12);
        let bounds_rect = Rect::load(input, 8)?;
        let ditl_id = input.read_i16()?;
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
            Some(input.read_u16()?)
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
    pub fn alert(&self, alert_id: i16) -> i16 {
        todo!()
    }

    pub fn param_text<T: AsRef<str>>(&mut self, param0: T, param1: T, param2: T, param3: T) {
        self.param_text[0] = param0.as_ref().to_owned();
        self.param_text[1] = param1.as_ref().to_owned();
        self.param_text[2] = param2.as_ref().to_owned();
        self.param_text[3] = param3.as_ref().to_owned();
    }

    pub fn stop_alert(&self, alert_id: i16) -> i16 {
        todo!()
    }
}
