use anyhow::{bail, Result as AResult};
use byteordered::ByteOrdered;
use earthquake::{
    collections::riff::Riff,
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
    encodings::MAC_ROMAN,
    macos::ResourceFile,
    resources::parse_resource,
    SharedStream,
};
use pico_args::Arguments;
use std::{env, fs::File, io::{Seek, SeekFrom}, path::{Path, PathBuf}, process::exit};

fn main() -> AResult<()> {
    const VERSION: Option<&'static str> = option_env!("CARGO_PKG_VERSION");

    println!("Earthquake {} file inspector", VERSION.unwrap_or(""));

    let mut args = Arguments::from_env();
    let data_dir = args.opt_value_from_str::<_, PathBuf>("--data")?;
    let inspect_data = args.contains("--inspect-data");
    let files = args.free()?;

    if files.is_empty() {
        println!("Usage: {} [--inspect-data] [--data <dir>] <exe/cxr/dxr ...>", env::args().nth(0).unwrap_or_else(|| "inspect".to_string()));
        println!("\nOptional arguments:\n    --data: The path to movies referenced by a Projector\n    --inspect-data: Print movie contents");
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
            if resource.id().0.as_bytes() == b"VWCR" {
                let data = std::io::Cursor::new(resource.data()?);
                let reader = ByteOrdered::be(data);
                println!("{:?}", parse_resource(resource.id().0, reader, Some(MAC_ROMAN)));
            }
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

fn read_movie(info: MovieDetectionInfo, stream: SharedStream<File>, inspect_data: bool) -> AResult<()> {
    println!("{:?}", info);
    if inspect_data {
        match info.kind() {
            MovieKind::Movie | MovieKind::Cast => {
                let riff = Riff::new(stream)?;
                for resource in riff.iter() {
                    println!("{}", resource.id());
                }
            },
            MovieKind::Accelerator | MovieKind::Embedded => {
                read_embedded_movie(1, stream, inspect_data)?;
            },
        }
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
                    let riff = Riff::new(&mut stream)?;
                    for resource in riff.iter() {
                        println!("{}", resource.id());
                    }
                }
            }
        },
        MovieInfo::Internal { stream, offset, .. } => {
            println!("Internal movie at {}", offset);
            if inspect_data {
                let mut stream = stream.clone();
                stream.seek(SeekFrom::Start(u64::from(*offset)))?;
                let riff = Riff::new(stream)?;
                for resource in riff.iter() {
                    println!("{}", resource.id());
                }
            }
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
