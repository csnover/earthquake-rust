use crate::{
    Reader,
    resources::{
        projector::{detect as detect_projector, DetectionInfo as ProjectorDetectionInfo},
        riff::{detect as detect_riff, DetectionInfo as RiffDetectionInfo}
    }
};

#[derive(Debug)]
pub enum FileType {
    Projector(ProjectorDetectionInfo),
    Movie(RiffDetectionInfo)
}

pub fn detect_type<T: Reader>(reader: &mut T) -> Option<FileType> {
    if let Some(file_type) = detect_riff(reader) {
        return Some(FileType::Movie(file_type));
    }

    if let Some(file_type) = detect_projector(reader) {
        return Some(FileType::Projector(file_type));
    }

    None
}
