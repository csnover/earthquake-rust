pub mod movie;
pub mod projector;
pub mod projector_settings;

use anyhow::{anyhow, bail, Context, Result as AResult};
use crate::{collections::riff, vfs::Native};
use libcommon::{flatten_errors, Reader, SharedStream, vfs::VirtualFileSystem};
use std::{fs::File, io::{Seek, SeekFrom}, path::Path};

// 1. D4+Mac projector: resource fork w/ projector ostype + maybe riff in data fork
// 2. D3Mac projector: resource fork w/ projector ostype
// 3. D3Mac movie: resource fork w/o projector ostype
// 4. D3Mac accelerator: resource fork w/ EMPO ostype and data in the data fork
// 5. D3Win projector: executable w/ funky header
// 6. D4+Win projector: executable w/ standard projector header
// 7. D3Win & D4+Win movie: riff w/ specific subtype

#[derive(Clone, Debug)]
pub enum FileType {
    Projector(projector::DetectionInfo<File>, SharedStream<File>),
    Movie(movie::DetectionInfo, SharedStream<File>),
}

pub fn detect<T: AsRef<Path>>(path: T) -> AResult<FileType> {
    ensure_exists(&path).and_then(|_| {
        Native::new().open(path)
    }).and_then(|file| {
        file.resource_fork().context("No resource fork")
            .and_then(|rf| detect_mac(rf.clone(), file.data_fork().map(SharedStream::clone)))
            .or_else(|ref e| {
                flatten_errors(file.data_fork().context("No data fork").and_then(|df| {
                    projector::detect_win(&mut df.clone())
                        .map_err(|e| anyhow!("Not a Director for Windows file: {}", e))
                        .map(|p| FileType::Projector(p, df.clone()))
                        .or_else(|e| flatten_errors(detect_mac(df.clone(), None::<SharedStream<File>>), &e))
                        .or_else(|e| flatten_errors(detect_riff(df.clone()), &e))
                }), e)
            })
    }).context("Detection failed")
}

fn ensure_exists<T: AsRef<Path>>(filename: T) -> AResult<()> {
    if !filename.as_ref().metadata()?.is_file() {
        bail!("{} is not a file", filename.as_ref().display())
    }

    Ok(())
}

fn detect_mac(mut resource_fork: SharedStream<File>, mut data_fork: Option<SharedStream<File>>) -> AResult<FileType> {
    let start_pos = resource_fork.pos()?;
    projector::detect_mac(&mut resource_fork, data_fork.as_mut())
        .map(|p| {
            resource_fork.seek(SeekFrom::Start(start_pos)).unwrap();
            FileType::Projector(p, resource_fork.clone())
        })
        .or_else(|ref e| {
            resource_fork.seek(SeekFrom::Start(start_pos)).unwrap();
            flatten_errors(movie::detect_mac(&mut resource_fork).map(|m| {
                resource_fork.seek(SeekFrom::Start(start_pos)).unwrap();
                FileType::Movie(m, resource_fork)
            }), e)
        })
        .map_err(|e| anyhow!("Not a Director for Mac file: {}", e))
}

fn detect_riff(mut data_fork: SharedStream<File>) -> AResult<FileType> {
    let start_pos = data_fork.pos()?;
    riff::detect(&mut data_fork).and_then(|m| {
        data_fork.seek(SeekFrom::Start(start_pos))?;
        Ok(FileType::Movie(m, data_fork))
    })
}
