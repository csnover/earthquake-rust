pub mod movie;
pub mod projector;
pub mod projector_settings;

use anyhow::{anyhow, bail, Context, Result as AResult};
use crate::{collections::riff, io::open_resource_fork};
use libcommon::{Reader, SharedStream};
use libmactoolbox::{AppleDouble, MacBinary};
use std::{fs::File, io::{Seek, SeekFrom}, path::Path};

// 1. D4+Mac projector: resource fork w/ projector ostype + maybe riff in data fork
// 2. D3Mac projector: resource fork w/ projector ostype
// 3. D3Mac movie: resource fork w/o projector ostype
// 4. D3Mac accelerator: resource fork w/ EMPO ostype and data in the data fork
// 5. D3Win projector: executable w/ funky header
// 6. D4+Win projector: executable w/ standard projector header
// 7. D3Win & D4+Win movie: riff w/ specific subtype

#[derive(Debug)]
pub enum FileType {
    Projector(projector::DetectionInfo<File>, SharedStream<File>),
    Movie(movie::DetectionInfo, SharedStream<File>),
}

pub fn detect<T: AsRef<Path>>(filename: T) -> AResult<FileType> {
    ensure_exists(&filename).and_then(|_|
        detect_resource_fork(&filename)
            .or_else(|e| flatten_errors(detect_apple_single_or_apple_double(&filename, false), &e))
            .or_else(|e| flatten_errors(detect_mac_binary(&filename, false), &e))
            .or_else(|e| flatten_errors(detect_file(&filename), &e))
    ).context("Detection failed")
}

fn ensure_exists<T: AsRef<Path>>(filename: T) -> AResult<()> {
    if !filename.as_ref().metadata()?.is_file() {
        return Err(anyhow!("{} is not a file", filename.as_ref().display()))
    }

    Ok(())
}

pub fn detect_data_fork<T: AsRef<Path>>(filename: T) -> AResult<FileType> {
    detect_apple_single_or_apple_double(&filename, true)
        .or_else(|e| flatten_errors(detect_mac_binary(&filename, true), &e))
        .or_else(|e| {
            let file = SharedStream::new(File::open(&filename).map_err(|e| anyhow!("Could not open {}: {}", filename.as_ref().display(), e))?);
            flatten_errors(detect_riff(file), &e)
        })
}

fn detect_apple_single_or_apple_double<T: AsRef<Path>>(filename: T, only_data_fork: bool) -> AResult<FileType> {
    let apple_file = AppleDouble::open(filename)
        .map_err(|e| anyhow!("Not an AppleSingle/AppleDouble file: {}", e))?;

    let resource_fork = if only_data_fork {
        None
    } else {
        apple_file.resource_fork()
    };

    if let Some(resource_fork) = resource_fork {
        detect_mac(resource_fork, apple_file.data_fork())
            .or_else(|e| {
                if let Some(data_fork) = apple_file.data_fork() {
                    flatten_errors(detect_riff(data_fork), &e)
                } else {
                    Err(e)
                }
            })
    } else if let Some(data_fork) = apple_file.data_fork() {
        detect_riff(data_fork)
    } else {
        bail!("No data in AppleSingle/AppleDouble file")
    }
}

fn detect_file<T: AsRef<Path>>(filename: T) -> AResult<FileType> {
    let file = SharedStream::new(File::open(&filename)
        .map_err(|e| anyhow!("Could not open {}: {}", filename.as_ref().display(), e))?);

        projector::detect_win(&mut file.clone())
        .map(|p| FileType::Projector(p, file.clone()))
        .or_else(|e| flatten_errors(detect_mac(file.clone(), None::<SharedStream<File>>), &e))
        .or_else(|e| flatten_errors(detect_riff(file), &e))
}

fn detect_mac(mut stream: SharedStream<File>, mut data_fork: Option<SharedStream<File>>) -> AResult<FileType> {
    let start_pos = stream.pos()?;
    projector::detect_mac(&mut stream, data_fork.as_mut())
        .map(|p| {
            stream.seek(SeekFrom::Start(start_pos)).unwrap();
            FileType::Projector(p, stream.clone())
        })
        .or_else(|e| {
            stream.seek(SeekFrom::Start(start_pos)).unwrap();
            flatten_errors(movie::detect_mac(&mut stream).map(|m| {
                stream.seek(SeekFrom::Start(start_pos)).unwrap();
                FileType::Movie(m, stream)
            }), &e)
        })
        .map_err(|e| anyhow!("Not a Director for Mac file: {}", e))
}

fn detect_mac_binary<T: AsRef<Path>>(filename: T, only_data_fork: bool) -> AResult<FileType> {
    let mac_binary = MacBinary::new(File::open(&filename)?)
        .map_err(|e| anyhow!("Not a MacBinary file: {}", e))?;

    let resource_fork = if only_data_fork {
        None
    } else {
        mac_binary.resource_fork()
    };

    if let Some(resource_fork) = resource_fork {
        detect_mac(resource_fork, mac_binary.data_fork()).or_else(|e| {
            if let Some(data_fork) = mac_binary.data_fork() {
                flatten_errors(detect_riff(data_fork), &e)
            } else {
                Err(e)
            }
        })
    } else if let Some(data_fork) = mac_binary.data_fork() {
        detect_riff(data_fork)
    } else {
        bail!("No data in MacBinary file")
    }
}

fn detect_resource_fork<T: AsRef<Path>>(filename: T) -> AResult<FileType> {
    detect_mac(
        SharedStream::new(open_resource_fork(&filename).map_err(|e| anyhow!("Could not open resource fork: {}", e))?),
        Some(SharedStream::new(File::open(&filename).map_err(|e| anyhow!("Could not open data fork: {}", e))?))
    )
}

fn detect_riff(mut stream: SharedStream<File>) -> AResult<FileType> {
    let start_pos = stream.pos()?;
    riff::detect(&mut stream).and_then(|m| {
        stream.seek(SeekFrom::Start(start_pos))?;
        Ok(FileType::Movie(m, stream))
    })
}

fn flatten_errors<T>(mut result: AResult<T>, chained_error: &anyhow::Error) -> AResult<T> {
    for error in chained_error.chain() {
        result = result.context(anyhow!("{}", error));
    }
    result
}
