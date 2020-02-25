pub mod movie;
pub mod projector;

use anyhow::{anyhow, Context, Result as AResult};
use crate::{collections::riff, io::open_resource_fork, macos::{AppleDouble, MacBinary}, Reader, SharedStream};
use std::{fs::File, io::{Seek, SeekFrom}};

// 1. D4+Mac projector: resource fork w/ projector ostype + maybe riff in data fork
// 2. D3Mac projector: resource fork w/ projector ostype
// 3. D3Mac movie: resource fork w/o projector ostype
// 4. D3Mac accelerator: resource fork w/ EMPO ostype and data in the data fork
// 5. D3Win projector: executable w/ funky header
// 6. D4+Win projector: executable w/ standard projector header
// 7. D3Win & D4+Win movie: riff w/ specific subtype

#[derive(Debug)]
pub enum FileType {
    Projector(projector::DetectionInfo, SharedStream<File>),
    Movie(movie::DetectionInfo, SharedStream<File>),
}

pub fn detect(filename: &str) -> AResult<FileType> {
    detect_resource_fork(filename)
        .or_else(|e| detect_apple_single_or_apple_double(filename).context(e))
        .or_else(|e| detect_mac_binary(filename).context(e))
        .or_else(|e| detect_file(filename).context(e))
}

fn detect_apple_single_or_apple_double(filename: &str) -> AResult<FileType> {
    let apple_file = AppleDouble::open(filename)?;
    if let Some(resource_fork) = apple_file.resource_fork() {
        detect_mac(resource_fork)
            .or_else(|e| {
                if let Some(data_fork) = apple_file.data_fork() {
                    detect_riff(data_fork).context(e)
                } else {
                    Err(e)
                }
            })
    } else if let Some(data_fork) = apple_file.data_fork() {
        detect_riff(data_fork)
    } else {
        Err(anyhow!("No data in AppleSingle/AppleDouble file"))
    }
}

fn detect_file(filename: &str) -> AResult<FileType> {
    let file = SharedStream::new(File::open(filename)?);
    projector::detect_win(&mut file.clone())
        .map(|p| FileType::Projector(p, file.clone()))
        .or_else(|e| detect_mac(file.clone()).context(e))
        .or_else(|e| detect_riff(file).context(e))
}

fn detect_mac(mut stream: SharedStream<File>) -> AResult<FileType> {
    let start_pos = stream.seek(SeekFrom::Current(0))?;
    projector::detect_mac(&mut stream)
        .map(|p| {
            stream.seek(SeekFrom::Start(start_pos)).unwrap();
            FileType::Projector(p, stream.clone())
        })
        .or_else(|e| {
            stream.seek(SeekFrom::Start(start_pos)).unwrap();
            movie::detect_mac(&mut stream).map(|m| {
                stream.seek(SeekFrom::Start(start_pos)).unwrap();
                FileType::Movie(m, stream)
            }).context(e)
        })
}

fn detect_mac_binary(filename: &str) -> AResult<FileType> {
    let mac_binary = MacBinary::new(File::open(filename)?)?;
    if let Some(resource_fork) = mac_binary.resource_fork() {
        detect_mac(resource_fork).or_else(|e| {
            if let Some(data_fork) = mac_binary.data_fork() {
                detect_riff(data_fork).context(e)
            } else {
                Err(e)
            }
        })
    } else if let Some(data_fork) = mac_binary.data_fork() {
        detect_riff(data_fork)
    } else {
        Err(anyhow!("No data in MacBinary file"))
    }
}

fn detect_resource_fork(filename: &str) -> AResult<FileType> {
    detect_mac(SharedStream::new(open_resource_fork(filename)?))
}

fn detect_riff(mut stream: SharedStream<File>) -> AResult<FileType> {
    let start_pos = stream.seek(SeekFrom::Current(0))?;
    riff::detect(&mut stream).and_then(|m| {
        stream.seek(SeekFrom::Start(start_pos))?;
        Ok(FileType::Movie(m, stream))
    })
}
