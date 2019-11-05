use std::{fmt, io};

pub trait Reader: io::Read + io::Seek + fmt::Debug {}
impl<T: io::Read + io::Seek + ?Sized + fmt::Debug> Reader for T {}
