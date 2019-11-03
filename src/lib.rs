use std::io::{Read, Seek};

mod chunk;
pub(crate) mod compression;
pub mod detect;
mod io;
pub mod movie;
pub(crate) mod resources;
pub(crate) mod string;

pub trait Reader: Read + Seek {}
impl<T: Read + Seek + ?Sized> Reader for T {}
