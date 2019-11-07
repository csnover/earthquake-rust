use crate::{Reader, collections::{projector::{self, DetectionInfo as ProjectorDetectionInfo}, riff::{self, DetectionInfo as RiffDetectionInfo}}};
use std::io::SeekFrom;

#[derive(Debug)]
pub enum FileType {
    Projector(ProjectorDetectionInfo),
    Movie(RiffDetectionInfo)
}

pub fn detect_type<T: Reader>(reader: &mut T) -> Option<FileType> {
    reader.seek(SeekFrom::Start(0)).ok()?;
    if let Some(file_type) = riff::detect(reader) {
        return Some(FileType::Movie(file_type));
    }

    if let Some(file_type) = projector::detect(reader) {
        return Some(FileType::Projector(file_type));
    }

    None
}
