use crate::resources::cast::MemberId;
use libmactoolbox::{quickdraw::Rect, text_edit::Handle as TEHandle};
use super::ChannelNum;

#[derive(Clone, Copy, Debug, Default)]
pub(crate) struct TextEditor {
    te: TEHandle,
    rect: Rect,
    sprite_num: ChannelNum,
    id: MemberId,
    is_editing: bool,
}
