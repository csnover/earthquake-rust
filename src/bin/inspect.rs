use earthquake::detect::detect_type;
use std::{env, error::Error, fs::File, io::{Error as IoError, ErrorKind, Result as IoResult}, path::PathBuf, process::exit};

fn open_named_fork(filename: &str) -> IoResult<File> {
    let path = format!("{}/..namedfork/rsrc", filename);
    let metadata = std::fs::metadata(&path)?;
    if metadata.len() > 0 {
        File::open(&path)
    } else {
        Err(IoError::from(ErrorKind::NotFound))
    }
}

fn open_apple_double(filename: &str) -> IoResult<File> {
    let mut path = PathBuf::from(filename);
    let filename = format!("._{}", path.file_name().unwrap().to_str().unwrap());
    path.set_file_name(filename);
    File::open(path)
}

fn open_resource_fork(filename: &str) -> IoResult<File> {
    open_named_fork(&filename)
        .or_else(|_| File::open(format!("{}.rsrc", filename)))
        .or_else(|_| open_apple_double(&filename))
}

fn read_file(filename: &str) -> Result<(), Box<dyn Error>> {
    // Files from Macs have both data and resource forks; in the case of
    // projectors, we want to prefer the resource fork (to detect the projector
    // instead of movie data in the data fork), but in the case of standalone
    // dir/dxr files, we get a mostly empty resource fork which fails detection,
    // and then need to go read the data fork to detect the movie
    if let Ok(mut file) = open_resource_fork(&filename) {
        if let Some(file_type) = detect_type(&mut file) {
            println!("{:?}", file_type);
            return Ok(());
        }
    }

    let mut file = File::open(filename)?;
    match detect_type(&mut file) {
        Some(file_type) => { println!("{:?}", file_type); },
        None => { println!("{}: Invalid or unknown Director projector or movie.", filename); }
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
