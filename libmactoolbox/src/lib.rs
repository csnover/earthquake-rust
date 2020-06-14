mod apple_double;
mod application_vise;
mod mac_binary;
mod os_type;
pub mod resources;
mod resource_file;
mod resource_id;
pub mod script_manager;
pub mod string;
mod system;

pub use apple_double::*;
pub use application_vise::*;
pub use mac_binary::*;
pub use os_type::*;
pub use resource_file::*;
pub use resource_id::*;
pub use system::System;

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
