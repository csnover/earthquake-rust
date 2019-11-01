use std::io::{Read, Seek};

mod chunk;
pub mod detect;
mod io;
pub(crate) mod m68k;
pub mod movie;
pub(crate) mod string;

pub trait Reader: Read + Seek {}
impl<T: Read + ?Sized + Seek> Reader for T {}
