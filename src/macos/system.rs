use crate::encodings::Decoder;

pub struct System {
    decoder: &'static dyn Decoder,
}

impl System {
    pub fn init() {
        // 1. Is this Windows? If yes:
        //    Load asifont.map and projectr.rsr from the projector exe
        // otherwise it is Mac and shit gotta get loaded from the resource fork
        // instead
    }

    pub fn instance() -> &'static System {
        &INSTANCE
    }

    pub fn decoder(&self) -> &'static dyn Decoder {
        self.decoder
    }

    pub fn decoder_mut(&mut self) -> &mut &'static dyn Decoder {
        &mut self.decoder
    }
}

static INSTANCE: System = System {
    decoder: crate::encodings::MAC_ROMAN,
};
