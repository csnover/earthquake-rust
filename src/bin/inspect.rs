use earthquake::detect::detect_type;
use std::{env, error::Error, fs::File, process::exit};

fn read_file(filename: &str) -> Result<(), Box<dyn Error>> {
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
