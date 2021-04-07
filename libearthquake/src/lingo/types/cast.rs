use crate::resources::{Rc, cast::{LibNum, MemberId}};

#[derive(Debug)]
pub struct Lib {
    __: Rc,
    num: LibNum,
}

#[derive(Debug)]
pub struct Member {
    __: Rc,
    id: MemberId,
}
