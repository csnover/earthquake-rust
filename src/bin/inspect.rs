use earthquake::{detect::detect_type, io::open_resource_fork, riff::Riff};
use std::{env, error::Error, fs::File, process::exit};

fn read_file(filename: &str) -> Result<(), Box<dyn Error>> {
    // Files from Macs have both data and resource forks; in the case of
    // projectors, we want to prefer the resource fork (to detect the projector
    // instead of movie data in the data fork), but in the case of standalone
    // dir/dxr files, we get a mostly empty resource fork which fails detection,
    // and then need to go read the data fork to detect the movie

    // TODO: Create a projector instead of just the detected type info
    if let Ok(mut file) = open_resource_fork(&filename) {
        if let Some(projector) = detect_type(&mut file) {
            println!("{:?}", projector);
            return Ok(());
        }
    }

    // TODO: Try for a projector first, and then a movie
    let mut file = File::open(filename)?;

    if let Ok(movie) = Riff::new(&mut file) {
        println!("{}: Version {} {}", filename, movie.version(), movie.kind());

        for resource in movie.iter() {
            println!("{:?}", resource.id);
        }
    } else if let Some(file_info) = detect_type(&mut file) {
        println!("{:?}", file_info);
    } else {
        println!("{}: Invalid or unknown Director projector or movie.", filename);
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
