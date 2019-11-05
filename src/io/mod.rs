use std::{fs::{self, File}, io, path::PathBuf};

fn open_named_fork<T: AsRef<str>>(filename: T) -> io::Result<File> {
    let path = format!("{}/..namedfork/rsrc", filename.as_ref());
    let metadata = fs::metadata(&path)?;
    if metadata.len() > 0 {
        File::open(&path)
    } else {
        Err(io::Error::from(io::ErrorKind::NotFound))
    }
}

fn open_apple_double<T: AsRef<str>>(filename: T) -> io::Result<File> {
    let mut path = PathBuf::from(filename.as_ref());
    let filename = format!("._{}", path.file_name().unwrap().to_str().unwrap());
    path.set_file_name(filename);
    File::open(path)
}

pub fn open_resource_fork<T: AsRef<str>>(filename: T) -> io::Result<File> {
    open_named_fork(filename.as_ref())
        .or_else(|_| File::open(format!("{}.rsrc", filename.as_ref())))
        .or_else(|_| open_apple_double(&filename))
}
