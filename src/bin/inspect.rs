use anyhow::{anyhow, Result as AResult};
use byteordered::ByteOrdered;
use earthquake::{
    collections::riff::Riff,
    detection::{
        detect,
        detect_data_fork,
        FileType,
        movie::{
            DetectionInfo as MovieDetectionInfo,
            MovieType,
        },
        projector::{
            DetectionInfo as ProjectorDetectionInfo,
            Movie as MovieInfo,
            Platform,
            ProjectorVersion,
        },
    },
    macos::MacResourceFile,
    resources::parse_resource,
    SharedStream,
};
use encoding::all::MAC_ROMAN;
use pico_args::Arguments;
use std::{env, fs::File, io::{Seek, SeekFrom}, path::{Path, PathBuf}, process::exit};

fn main() -> AResult<()> {
    const VERSION: Option<&'static str> = option_env!("CARGO_PKG_VERSION");

    println!("Earthquake {} file inspector", VERSION.unwrap_or(""));

    let mut args = Arguments::from_env();
    let data_dir = args.opt_value_from_str::<_, PathBuf>("--data")?;
    let files = args.free()?;

    if files.is_empty() {
        println!("Usage: {} [--data <dir>] <exe/cxr/dxr ...>", env::args().nth(0).unwrap_or_else(|| "inspect".to_string()));
        println!("\nOptional arguments:\n    --data: The path to movies referenced by a Projector");
        exit(1);
    }

    for filename in files {
        read_file(&filename, data_dir.as_ref())?;
    }

    Ok(())
}

fn read_embedded_movie(num_movies: u16, stream: SharedStream<File>) -> AResult<()> {
    println!("{} embedded movies", num_movies);

    let rom = MacResourceFile::new(stream)?;
    for resource in rom.iter() {
        println!("{} {:?}", resource.id(), resource.flags());
        if resource.id().0.as_bytes() == b"VWCR" {
            let data = std::io::Cursor::new(resource.data()?);
            let reader = ByteOrdered::be(data);
            println!("{:?}", parse_resource(resource.id().0, reader, Some(MAC_ROMAN)));
        }
    }

    Ok(())
}

fn read_file(filename: &str, data_dir: Option<&PathBuf>) -> AResult<()> {
    match detect(filename)? {
        FileType::Projector(p, s) => read_projector(p, s, filename, data_dir),
        FileType::Movie(m, s) => read_movie(m, s),
    }
}

fn read_movie(info: MovieDetectionInfo, stream: SharedStream<File>) -> AResult<()> {
    println!("{:?}", info);
    match info.kind() {
        MovieType::Movie | MovieType::Cast => {
            let riff = Riff::new(stream)?;
            for resource in riff.iter() {
                println!("{}", resource.id());
            }
        },
        MovieType::Accelerator | MovieType::Embedded => {
            read_embedded_movie(1, stream)?;
        },
    }
    Ok(())
}

fn read_projector(info: ProjectorDetectionInfo, stream: SharedStream<File>, filename: &str, data_dir: Option<&PathBuf>) -> AResult<()> {
    println!("{:?}", info);
    match info.movie() {
        MovieInfo::Internal { offset, .. } => {
            println!("Internal movie at {}", offset);
            let mut stream = if info.platform() == Platform::Mac {
                // TODO: This does not work correctly for AppleSingle and
                // MacBinary files
                SharedStream::new(File::open(filename)?)
            } else {
                stream
            };
            stream.seek(SeekFrom::Start(u64::from(*offset)))?;
            let riff = Riff::new(&mut stream)?;
            for resource in riff.iter() {
                println!("{}", resource.id());
            }
        },
        MovieInfo::External(filenames) => {
            for filename in filenames {
                println!("External movie at {}", filename);

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
                        read_file(file_path.to_str().unwrap(), data_dir)?;
                        break;
                    }
                }
            }
        },
        MovieInfo::Embedded(num_movies) => {
            if info.version() == ProjectorVersion::D3 {
                read_embedded_movie(*num_movies, stream)?;
            } else {
                match detect_data_fork(filename)? {
                    FileType::Projector(..) => return Err(anyhow!("Embedded movie looped back to projector")),
                    FileType::Movie(m, s) => read_movie(m, s)?,
                };
            }
        },
    }
    Ok(())
}
