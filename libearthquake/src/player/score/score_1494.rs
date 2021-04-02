use crate::resources::cast::{MemberId, MemberKind};
use libcommon::{Unk8, UnkPtr};

#[derive(Clone, Copy, Debug, Default)]
pub struct Score1494 {
    data: UnkPtr,
    id: MemberId,
    field_8: Unk8,
    field_9: Unk8,
    flags: u8,
    cast_member_kind: MemberKind,
}
