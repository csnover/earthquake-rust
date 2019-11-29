use anyhow::{Context, Result as AResult};
use crate::{Reader, collections::{movie::{self, DetectionInfo as MovieDetectionInfo}, projector::{self, DetectionInfo as ProjectorDetectionInfo}}};
use std::io::SeekFrom;

#[derive(Debug)]
pub enum FileType {
    Projector(ProjectorDetectionInfo),
    Movie(MovieDetectionInfo)
}

pub fn detect_type<T: Reader>(reader: &mut T) -> AResult<FileType> {
    reader.seek(SeekFrom::Start(0))?;

    projector::detect(reader)
        .map(FileType::Projector)
        .or_else(|e|
            movie::detect(reader)
            .map(FileType::Movie)
            .context(e))
}
