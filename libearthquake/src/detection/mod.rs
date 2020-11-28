pub mod movie;
pub mod projector;
pub mod projector_settings;

use anyhow::{anyhow, Context, Result as AResult};
use crate::collections::riff;
use derive_more::Display;
use libcommon::{flatten_errors, Reader, vfs::{VirtualFile, VirtualFileSystem}};
use std::{io::SeekFrom, path::Path};

// 1. D4+Mac projector: resource fork w/ projector ostype + maybe riff in data fork
// 2. D3Mac projector: resource fork w/ projector ostype
// 3. D3Mac movie: resource fork w/o projector ostype
// 4. D3Mac accelerator: resource fork w/ EMPO ostype and data in the data fork
// 5. D3Win projector: executable w/ funky header
// 6. D4+Win projector: executable w/ standard projector header
// 7. D3Win & D4+Win movie: riff w/ specific subtype

#[derive(Clone, Copy, Debug, Display, PartialEq, PartialOrd)]
pub enum Version {
    #[display(fmt = "3")]
    D3,
    #[display(fmt = "4")]
    D4,
    #[display(fmt = "5")]
    D5,
    #[display(fmt = "6")]
    D6,
    #[display(fmt = "7")]
    D7,
}

#[derive(Clone, Debug)]
pub enum FileType {
    Projector(projector::DetectionInfo),
    Movie(movie::DetectionInfo),
}

pub struct Detection<'vfs> {
    pub info: FileType,
    pub data_fork: Option<Box<dyn VirtualFile + 'vfs>>,
    pub resource_fork: Option<Box<dyn VirtualFile + 'vfs>>,
}

pub fn detect<'vfs>(fs: &'vfs dyn VirtualFileSystem, path: impl AsRef<Path>) -> AResult<Detection<'vfs>> {
    fs.open_resource_fork(&path).and_then(|mut res_file| {
        let mut data_file = fs.open(&path).ok();
        detect_mac(&mut res_file, data_file.as_mut()).map(|ft| {
            Detection { info: ft, resource_fork: Some(res_file), data_fork: data_file }
        })
    }).or_else(|ref e| {
        let data_file = fs.open(&path).ok().context("No data file");
        flatten_errors(data_file.and_then(|mut df| {
            projector::detect_win(&mut df)
                .map_err(|e| anyhow!("Not a Director for Windows file: {}", e))
                .map(FileType::Projector)
                .or_else(|ref e| { df.reset()?; flatten_errors(detect_mac(&mut df, None::<&mut Box<dyn VirtualFile>>), e) })
                .or_else(|ref e| { df.reset()?; flatten_errors(detect_riff(&mut df), e) })
                .map(|ft| Detection { info: ft, resource_fork: None, data_fork: Some(df) })
        }), e)
    }).context("Detection failed")
}

fn detect_mac(resource_fork: &mut impl Reader, data_fork: Option<&mut impl Reader>) -> AResult<FileType> {
    let start_pos = resource_fork.pos()?;
    projector::detect_mac(resource_fork.by_ref(), data_fork)
        .map(|p| {
            resource_fork.seek(SeekFrom::Start(start_pos)).unwrap();
            FileType::Projector(p)
        })
        .or_else(|ref e| {
            resource_fork.seek(SeekFrom::Start(start_pos)).unwrap();
            flatten_errors(movie::detect_mac(resource_fork).map(|m| {
                resource_fork.seek(SeekFrom::Start(start_pos)).unwrap();
                FileType::Movie(m)
            }), e)
        })
        .map_err(|e| anyhow!("Not a Director for Mac file: {:?}", e))
}

fn detect_riff(data_fork: &mut impl Reader) -> AResult<FileType> {
    let start_pos = data_fork.pos()?;
    riff::detect(data_fork.by_ref()).and_then(|m| {
        data_fork.seek(SeekFrom::Start(start_pos))?;
        Ok(FileType::Movie(m))
    })
}
