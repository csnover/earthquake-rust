use libcommon::Reader;
use crate::resource_manager::ResourceManager;
use libcommon::encodings::Decoder;

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

    #[must_use]
    pub fn instance() -> &'static System {
        &INSTANCE
    }

    #[must_use]
    pub fn decoder(&self) -> &'static dyn Decoder {
        self.decoder
    }

    pub fn decoder_mut(&mut self) -> &mut &'static dyn Decoder {
        &mut self.decoder
    }
}

static INSTANCE: System = System {
    decoder: libcommon::encodings::MAC_ROMAN,
};
