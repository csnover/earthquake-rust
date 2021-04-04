use crate::resources::{Rc, cast::{LibNum, MemberId}};

#[derive(Debug)]
pub struct CastLib {
    __: Rc,
    num: LibNum,
}

#[derive(Debug)]
pub struct CastMember {
    __: Rc,
    id: MemberId,
}
