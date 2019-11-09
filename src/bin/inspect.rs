use earthquake::{collections::{projector::{DetectionInfo as Projector, Movie as MovieInfo}, riff::Riff, rsrc::MacResourceFile}, detect::{self, FileType}, io};
use std::{env, error::Error, fs::File, io::{Seek, SeekFrom}, process::exit};

fn read_file(filename: &str) -> Result<(), Box<dyn Error>> {
    // Files from Macs have both data and resource forks; in the case of
    // projectors, we want to prefer the resource fork (to detect the projector
    // instead of movie data in the data fork), but in the case of standalone
    // dir/dxr files, we get a mostly empty resource fork which fails detection,
    // and then need to go read the data fork to detect the movie

    // TODO: Create a projector instead of just the detected type info
    if let Ok(mut file) = io::open_resource_fork(&filename) {
        match detect::detect_type(&mut file) {
            Some(FileType::Projector(projector)) => return read_projector(&mut file, &projector),
            Some(FileType::Movie(_)) => panic!("Got a movie instead of a projector from the resource fork"),
            None => return Ok(())
        }
    }

    // TODO: Try for a projector first, and then a movie
    let mut file = File::open(filename)?;

    if let Ok(movie) = Riff::new(&mut file) {
        println!("{}: Version {} {}", filename, movie.version(), movie.kind());

        for resource in movie.iter() {
            println!("{:?}", resource.id());
        }
    } else {
        match detect::detect_type(&mut file) {
            Some(FileType::Projector(projector)) => read_projector(&mut file, &projector)?,
            Some(FileType::Movie(_)) => panic!("Got a movie instead of a projector after trying to detect a movie"),
            None => println!("{}: Invalid or unknown Director projector or movie.", filename)
        }
    }

    Ok(())
}

fn read_projector(mut file: &mut File, projector: &Projector) -> Result<(), Box<dyn Error>> {
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
                file.seek(SeekFrom::Start(0))?;
                let rom = MacResourceFile::new(&mut file)?;
                for resource in rom.iter() {
                    println!("{}", resource.id());
                }
            },
        }
    }

    Ok(())
}

fn main() {
    const VERSION: Option<&'static str> = option_env!("CARGO_PKG_VERSION");

    println!("Earthquake {} file inspector", VERSION.unwrap_or(""));

    let args: Vec<_> = env::args().collect();
    if args.len() < 2 {
        println!("Usage: {} <exe/cxr/dxr>", args[0]);
        exit(1);
    }

    for arg in &args[1..] {
        match read_file(&arg) {
            Ok(_) => {},
            Err(error) => {
                println!("{}", error);
                exit(1);
            },
        };
    }
}
