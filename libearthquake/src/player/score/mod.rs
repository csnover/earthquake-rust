mod frame;
mod palette;
mod score_1494;
#[allow(clippy::module_inception)]
mod score;
mod sprite_bitmask;
mod sprite;
mod stream;
mod tempo;
mod text_editor;
mod transition;

pub use frame::Frame;
pub use palette::Palette;
pub(super) use score_1494::Score1494;
pub use score::Score;
pub(super) use sprite::Sprite;
pub(super) use sprite_bitmask::SpriteBitmask;
pub(super) use stream::Stream;
pub use tempo::Tempo;
pub(super) use text_editor::TextEditor;
pub(super) use transition::Transition;

use binrw::BinRead;
use derive_more::Display;
use libcommon::newtype_num;
use smart_default::SmartDefault;

newtype_num! {
    #[derive(Debug)]
    pub struct ChannelNum(pub i16);
}

newtype_num! {
    #[derive(BinRead, Debug)]
    pub struct FrameNum(pub i16);
}

newtype_num! {
    #[derive(Debug)]
    pub struct Fps(pub i16);
}

newtype_num! {
    #[derive(Debug)]
    pub struct Seconds(pub i16);
}

// TODO: Different sizes for different Director versions:
// D1–D3: 24
// D4: 48
// D5: 48
// D6: 120
// D7: 150
pub(super) const NUM_SPRITES: usize = 150;

#[derive(BinRead, Clone, Copy, Debug, Display, Eq, Ord, PartialEq, PartialOrd, SmartDefault)]
#[br(repr(i16))]
pub enum Version {
    #[default]
    Unknown,
    V3 = 3,
    V4,
    V5,
    V6,
    V7,
}
