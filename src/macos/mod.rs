mod apple_double;
mod application_vise;
mod mac_binary;
mod resource_file;
mod resource_id;
pub mod script_manager;
mod system;

pub(crate) use apple_double::*;
pub(crate) use application_vise::*;
pub(crate) use mac_binary::*;
pub use resource_file::*;
pub(crate) use resource_id::*;
pub(crate) use system::System;

#[derive(Default)]
pub struct Point {
    pub x: i16,
    pub y: i16,
}

#[derive(Default)]
pub struct Rect {
    pub top: i16,
    pub left: i16,
    pub bottom: i16,
    pub right: i16,
}

// TODO
pub struct TEHandle(u32);
