use anyhow::{Context, Result as AResult};
use byteordered::ByteOrdered;
use earthquake::{collections::{projector::{DetectionInfo as Projector, Movie as MovieInfo}, riff::Riff, rsrc::MacResourceFile}, detect::{self, FileType}, io, resources::parse_resource};
use encoding::all::MAC_ROMAN;
use std::{env, fs::File, io::{Seek, SeekFrom}, process::exit};

fn read_file(filename: &str) -> AResult<()> {
    // Files from Macs have both data and resource forks. In the case of
    // projectors, the resource fork should be checked first, since the data
    // fork in D4+ projectors also contains valid movie data, even though the
    // user intends to inspect the projector. In the case of movie files in D4
    // and later, the resource fork exists and contains data, but not Director
    // data.
    // So, if a resource fork exists but detection fails, we still need to check
    // the data fork to see if it’s actually a movie file.
    if let Ok(mut file) = io::open_resource_fork(&filename) {
        match detect::detect_type(&mut file) {
            Ok(FileType::Projector(projector)) => return read_projector(&mut file, &projector),
            Ok(FileType::Movie(_)) => return read_embedded_movie(&mut file),
            Err(_) => {},
        }
    }

    let mut file = File::open(filename)?;

    if let Ok(movie) = Riff::new(&mut file) {
        println!("{}: Version {} {}", filename, movie.version(), movie.kind());

        for resource in movie.iter() {
            println!("{}", resource.id());
        }
    } else {
        match detect::detect_type(&mut file)
            .with_context(|| format!("{} is not a Director file", filename))? {
            FileType::Projector(projector) => read_projector(&mut file, &projector)?,
            FileType::Movie(_) => panic!("Got a movie instead of a projector after trying to detect a movie"),
        }
    }

    Ok(())
}

fn read_projector(mut file: &mut File, projector: &Projector) -> AResult<()> {
    println!("{:?}", projector);

    for movie in &projector.movies {
        match movie {
            MovieInfo::Internal { offset, .. } => {
                println!("Internal movie at {}", offset);
                file.seek(SeekFrom::Start(u64::from(*offset)))?;
                let riff = Riff::new(&mut file)?;
                for resource in riff.iter() {
                    println!("{}", resource.id());
                }
            },
            MovieInfo::External(filename) => {
                println!("External movie at {}", filename);
            },
            MovieInfo::Embedded => {
                println!("Embedded movie");
                read_embedded_movie(file)?;
            },
        }
    }

    Ok(())
}

fn read_embedded_movie(mut file: &mut File) -> AResult<()> {
    file.seek(SeekFrom::Start(0))?;
    let rom = MacResourceFile::new(&mut file)?;
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

fn main() -> AResult<()> {
    const VERSION: Option<&'static str> = option_env!("CARGO_PKG_VERSION");

    println!("Earthquake {} file inspector", VERSION.unwrap_or(""));

    let args: Vec<_> = env::args().collect();
    if args.len() < 2 {
        println!("Usage: {} <exe/cxr/dxr>", args[0]);
        exit(1);
    }

    for arg in &args[1..] {
        read_file(&arg)?;
    }

    Ok(())
}
