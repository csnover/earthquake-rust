use crate::resources::{Rc, cast::{LibNum, MemberId}};

#[derive(Debug)]
pub(super) struct Lib {
    __: Rc,
    num: LibNum,
}

#[derive(Debug)]
pub(super) struct Member {
    __: Rc,
    id: MemberId,
}
