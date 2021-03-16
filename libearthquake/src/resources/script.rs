use anyhow::{Context, Result as AResult};
use binread::BinRead;
use libcommon::{binread_enum, Reader, Resource, resource::Input};
use num_derive::FromPrimitive;
use num_traits::FromPrimitive;
use smart_default::SmartDefault;

#[derive(Clone, Copy, Debug, Eq, FromPrimitive, PartialEq, SmartDefault)]
enum Kind {
    Score = 1,
    #[default]
    Movie = 3,
    Parent = 7,
}

binread_enum!(Kind, u16);

#[derive(BinRead, Clone, Copy, Debug)]
#[br(big, import(size: u32), pre_assert(size == 0 || size == 2))]
pub struct Meta {
    #[br(if(size == 2))]
    kind: Kind,
}

impl Resource for Meta {
    type Context = ();

    fn load(input: &mut Input<impl Reader>, size: u32, _: &Self::Context) -> AResult<Self> where Self: Sized {
        Self::read_args(input, (size, )).context("Can’t read script meta")
    }
}
