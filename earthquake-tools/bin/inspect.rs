use anyhow::{bail, Result as AResult};
use libearthquake::{
    collections::{
        riff::Riff,
        riff_container::{ChunkFileKind, RiffContainer},
    },
    detection::{
        detect,
        detect_data_fork,
        FileType,
        movie::{
            DetectionInfo as MovieDetectionInfo,
            Kind as MovieKind,
        },
        projector::{
            DetectionInfo as ProjectorDetectionInfo,
            Movie as MovieInfo,
            Version as ProjectorVersion,
        },
    },
    name,
};
use libcommon::SharedStream;
use libmactoolbox::ResourceFile;
use pico_args::Arguments;
use std::{env, fs::File, io::{Seek, SeekFrom}, path::{Path, PathBuf}, process::exit};

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

fn read_embedded_movie(num_movies: u16, stream: SharedStream<File>, inspect_data: bool) -> AResult<()> {
    println!("{} embedded movies", num_movies);

    if inspect_data {
        let rom = ResourceFile::new(stream)?;
        for resource in rom.iter() {
            println!("{} {:?}", resource.id(), resource.flags());
        }
    }

    Ok(())
}

fn read_file(filename: &str, data_dir: Option<&PathBuf>, inspect_data: bool) -> AResult<()> {
    match detect(filename)? {
        FileType::Projector(p, s) => read_projector(p, s, filename, data_dir, inspect_data),
        FileType::Movie(m, s) => read_movie(m, s, inspect_data),
    }
}

fn read_movie(info: MovieDetectionInfo, mut stream: SharedStream<File>, inspect_data: bool) -> AResult<()> {
    println!("{:?}", info);
    if inspect_data {
        match info.kind() {
            MovieKind::Movie | MovieKind::Cast => inspect_riff(&mut stream)?,
            MovieKind::Accelerator | MovieKind::Embedded => read_embedded_movie(1, stream, inspect_data)?,
        }
    }
    Ok(())
}

fn inspect_riff_container(stream: &mut SharedStream<File>, inspect_data: bool) -> AResult<()> {
    let riff_container = RiffContainer::new(stream.clone())?;

    for index in 0..riff_container.len() {
        println!("\nFile {}: {}", index + 1, riff_container.filename(index).unwrap().to_string_lossy());
        if inspect_data && riff_container.kind(index).unwrap() != ChunkFileKind::Xtra {
            match riff_container.load_file(index) {
                Ok(riff) => {
                    for resource in riff.iter() {
                        println!("{}", resource);
                    }
                },
                Err(e) => println!("Could not inspect file: {}", e)
            }
        }
    }

    Ok(())
}

fn inspect_riff(stream: &mut SharedStream<File>) -> AResult<()> {
    let riff = Riff::new(stream.clone())?;
    for resource in riff.iter() {
        println!("{}", resource);
    }

    Ok(())
}

fn read_projector(info: ProjectorDetectionInfo<File>, mut stream: SharedStream<File>, filename: &str, data_dir: Option<&PathBuf>, inspect_data: bool) -> AResult<()> {
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
        MovieInfo::Internal { stream, offset, .. } => {
            println!("Internal movie at {}", offset);
            let mut stream = stream.clone();
            stream.seek(SeekFrom::Start(u64::from(*offset)))?;
            inspect_riff_container(&mut stream, inspect_data)?;
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
                match detect_data_fork(filename)? {
                    FileType::Projector(..) => bail!("Embedded movie looped back to projector"),
                    FileType::Movie(m, s) => read_movie(m, s, inspect_data)?,
                };
            }
        },
    }
    Ok(())
}
