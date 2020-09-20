// https://github.com/rust-lang/cargo/issues/5034
#![warn(clippy::pedantic)]
#![allow(
    clippy::cast_possible_truncation,
    clippy::cast_possible_wrap,
    clippy::cast_sign_loss,
    clippy::missing_errors_doc,
    clippy::non_ascii_literal,
    clippy::verbose_bit_mask,
)]
#![warn(rust_2018_idioms)]

use anyhow::{bail, Result as AResult};
use libearthquake::{
    collections::{
        riff::{ChunkIndex, Riff},
        riff_container::{ChunkFileKind, RiffContainer},
    },
    detection::{
        detect,
        FileType,
        movie::{
            DetectionInfo as MovieDetectionInfo,
            Kind as MovieKind,
        },
        projector::{
            DetectionInfo as ProjectorDetectionInfo,
            Movie as MovieInfo,
            Version as ProjectorVersion,
        }, Detection,
    },
    name, resources::{cast::{CastMap, Member}, config::{Config, Version as ConfigVersion}},
};
use libcommon::{Reader, vfs::VirtualFileSystem};
use libmactoolbox::{ResourceFile, vfs::HostFileSystem, rsid};
use pico_args::Arguments;
use std::{env, io::SeekFrom, path::{Path, PathBuf}, process::exit};

fn main() -> AResult<()> {
    println!("{} file inspector", name(true));

    let mut args = Arguments::from_env();
    let data_dir = args.opt_value_from_str::<_, PathBuf>("--data")?;
    let inspect_data = args.contains("--inspect-data");
    let files = args.free()?;

    if files.is_empty() {
        println!(include_str!("inspect.usage"), env::args().next().unwrap_or_else(|| "inspect".to_string()));
        exit(1);
    }

    for filename in files {
        read_file(&filename, data_dir.as_ref(), inspect_data)?;
    }

    Ok(())
}

fn inspect_riff(stream: &mut impl Reader) -> AResult<()> {
    let riff = Riff::new(stream)?;
    inspect_riff_contents(&riff);
    Ok(())
}

fn inspect_riff_contents(riff: &Riff<impl Reader>) {
    let config_id = if riff.has_id(rsid!(b"VWCF", 1024)) {
        Some(rsid!(b"VWCF", 1024))
    } else if riff.has_id(rsid!(b"DRCF", 1024)) {
        Some(rsid!(b"DRCF", 1024))
    } else {
        None
    };

    let min_cast_num = if let Some(config_id) = config_id {
        let config = riff.load_id::<Config>(config_id, &()).unwrap();
        if !config.valid() {
            println!("Configuration checksum failure!");
        }
        println!("{:#?}", config);
        config.min_cast_num().0
    } else {
        println!("No config chunk!");
        0
    };

    for resource in riff.iter() {
        println!("{}", resource);
    }

    if let Ok(cast) = riff.load_id::<CastMap>(rsid!(b"CAS*", 1024), &()) {
        for (i, &chunk_index) in cast.iter().enumerate() {
            if chunk_index > ChunkIndex::new(0) {
                let cast_member_num = min_cast_num + (i as i16);
                match riff.load::<Member>(chunk_index, &(chunk_index, ConfigVersion::V1217)) {
                    Ok(member) => println!("{}: {:#?}", cast_member_num, member),
                    Err(err) => println!("Failed to inspect cast member {}: {:#}", cast_member_num, err),
                }
            }
        }
    } else {
        println!("No cast!")
    }
}

fn inspect_riff_container(stream: impl Reader, inspect_data: bool) -> AResult<()> {
    let riff_container = RiffContainer::new(stream)?;
    for index in 0..riff_container.len() {
        if inspect_data {
            println!();
        }
        println!("File {}: {}", index + 1, riff_container.filename(index).unwrap().to_string_lossy());
        if inspect_data && riff_container.kind(index).unwrap() != ChunkFileKind::Xtra {
            match riff_container.load_file(index) {
                Ok(riff) => {
                    inspect_riff_contents(&riff);
                },
                Err(e) => println!("Could not inspect file: {}", e)
            }
        }
    }

    Ok(())
}

fn read_embedded_movie(num_movies: u16, stream: impl Reader, inspect_data: bool) -> AResult<()> {
    println!("{} embedded movies", num_movies);

    if inspect_data {
        let rom = ResourceFile::new(stream)?;
        for resource_id in rom.iter() {
            println!("{}", resource_id);
        }
    }

    Ok(())
}

fn read_file(filename: &str, data_dir: Option<&PathBuf>, inspect_data: bool) -> AResult<()> {
    let fs = HostFileSystem::new();
    let Detection { info, resource_fork, data_fork } = detect(&fs, filename)?;
    match info {
        FileType::Projector(p) => read_projector(
            &fs,
            &p,
            if p.version() == ProjectorVersion::D3 {
                resource_fork.or(data_fork)
            } else {
                data_fork
            }.unwrap(),
            filename,
            data_dir,
            inspect_data
        )?,
        FileType::Movie(m) => read_movie(&m, resource_fork.or(data_fork).unwrap(), inspect_data)?,
    }
    Ok(())
}

fn read_movie(info: &MovieDetectionInfo, mut stream: impl Reader, inspect_data: bool) -> AResult<()> {
    println!("{:?}", info);
    if inspect_data {
        match info.kind() {
            MovieKind::Movie | MovieKind::Cast => inspect_riff(&mut stream)?,
            MovieKind::Accelerator | MovieKind::Embedded => read_embedded_movie(1, stream, inspect_data)?,
        }
    }
    Ok(())
}

fn read_projector(
    fs: &impl VirtualFileSystem,
    info: &ProjectorDetectionInfo,
    mut stream: impl Reader,
    filename: &str,
    data_dir: Option<&PathBuf>,
    inspect_data: bool
) -> AResult<()> {
    println!("{:?}", info);
    match info.movie() {
        MovieInfo::D3Win(movies) => {
            for movie in movies {
                println!("Internal movie at {}", movie.offset);
                if inspect_data {
                    stream.seek(SeekFrom::Start(u64::from(movie.offset)))?;
                    inspect_riff(&mut stream)?;
                }
            }
        },
        MovieInfo::Internal(offset) => {
            println!("Internal movie at {}", offset);
            stream.seek(SeekFrom::Start(u64::from(*offset)))?;
            inspect_riff_container(stream, inspect_data)?;
        },
        MovieInfo::External(filenames) => {
            for filename in filenames {
                println!("External movie at {}", filename);

                if inspect_data {
                    let mut components = Path::new(filename).components();
                    loop {
                        components.next();
                        let components_path = components.as_path();
                        if components_path.file_name().is_none() {
                            println!("File not found");
                            break;
                        }

                        let file_path = if let Some(data_dir) = data_dir {
                            let mut file_path = data_dir.clone();
                            file_path.push(components_path);
                            file_path
                        } else {
                            PathBuf::from(components_path)
                        };

                        if file_path.exists() {
                            read_file(file_path.to_str().unwrap(), data_dir, inspect_data)?;
                            break;
                        }
                    }
                }
            }
        },
        MovieInfo::Embedded(num_movies) => {
            if info.version() == ProjectorVersion::D3 {
                read_embedded_movie(*num_movies, stream, inspect_data)?;
            } else {
                match detect(fs, filename)? {
                    Detection { info: FileType::Projector(..), .. } => bail!("Embedded movie looped back to projector"),
                    Detection { info: FileType::Movie(m), resource_fork, data_fork } => read_movie(&m, resource_fork.or(data_fork).unwrap(), inspect_data)?,
                };
            }
        },
    }
    Ok(())
}
