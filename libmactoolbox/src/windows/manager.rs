use crate::quickdraw::{Rect, Region};
use libcommon::prelude::*;
use qt_gui::QGuiApplication;

pub struct Manager;

impl Manager {
    #[must_use]
    pub fn new() -> Self {
        Self
    }

    /// `GetGrayRgn`
    #[must_use]
    pub fn gray_region(&self) -> Region {
        unsafe {
            let mut rect = Rect {
                left: i16::MAX.into(),
                top: i16::MAX.into(),
                right: i16::MIN.into(),
                bottom: i16::MIN.into(),
            };

            for screen in QGuiApplication::screens().iter() {
                let geometry = (*screen)
                    .as_ref()
                    .expect("null QGuiApplication screen")
                    .available_virtual_geometry();
                rect.left = rect.left.min(geometry.left().unwrap_into());
                rect.top = rect.top.min(geometry.top().unwrap_into());
                rect.right = rect.right.max(geometry.right().unwrap_into());
                rect.bottom = rect.bottom.max(geometry.bottom().unwrap_into());
            }

            let mut region = Region::new();
            region.rect_region(rect);
            region
        }
    }
}
