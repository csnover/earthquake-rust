use std::{fs::{File, self}, io};

fn open_named_fork<T: AsRef<str>>(filename: T) -> io::Result<File> {
    let path = format!("{}/..namedfork/rsrc", filename.as_ref());
    let metadata = fs::metadata(&path)?;
    if metadata.len() > 0 {
        File::open(&path)
    } else {
        Err(io::Error::from(io::ErrorKind::NotFound))
    }
}

pub fn open_resource_fork<T: AsRef<str>>(filename: T) -> io::Result<File> {
    open_named_fork(filename.as_ref())
        .or_else(|_| File::open(format!("{}.rsrc", filename.as_ref())))
}
