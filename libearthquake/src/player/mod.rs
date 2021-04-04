// TODO: You know, finish this file and then remove these overrides
#![allow(dead_code)]

pub mod movie;
pub mod score;
mod window;

use anyhow::Result as AResult;
use crate::detection::{detect, FileType};
use libmactoolbox::quickdraw::{Point, Rect};
use std::{path::{Path, PathBuf}, time::Instant};
use libcommon::vfs::VirtualFileSystem;

pub struct Player {
    gray_rgn: Rect,
    last_mouse_down: Instant,
    last_key_down: Instant,
    last_mouse_move: Instant,
    last_mouse_pos: Point,
    file_type: FileType,
    path: PathBuf,
}

impl Player {
    pub fn open(fs: &impl VirtualFileSystem, path: impl AsRef<Path>) -> AResult<Self> {
        let now = Instant::now();

        let file_type = detect(fs, &path)?.info;

        Ok(Self {
            gray_rgn: Rect::default(),
            file_type,
            path: path.as_ref().to_path_buf(),
            last_mouse_down: now,
            last_key_down: now,
            last_mouse_move: now,
            last_mouse_pos: Point::default(),
        })
    }
}
