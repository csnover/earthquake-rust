mod score;

use anyhow::Result as AResult;
use crate::{
    detection::{detect, FileType},
};
use libmactoolbox::{Point, Rect};
use std::time::Instant;

pub struct Player {
    gray_rgn: Rect,
    last_mouse_down: Instant,
    last_key_down: Instant,
    last_mouse_move: Instant,
    last_mouse_pos: Point,
    file_type: FileType,
    path: String,
}

impl Player {
    pub fn open(path: &str) -> AResult<Self> {
        let now = Instant::now();

        let file_type = detect(path)?;

        Ok(Self {
            gray_rgn: Rect::default(),
            file_type,
            path: path.to_string(),
            last_mouse_down: now,
            last_key_down: now,
            last_mouse_move: now,
            last_mouse_pos: Point::default(),
        })
    }
}
