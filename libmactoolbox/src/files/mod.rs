mod apple_double;
mod mac_binary;

pub use apple_double::AppleDouble;
pub use mac_binary::MacBinary;
use std::{ffi::OsString, fs::{File, self}, io, path::Path};

fn open_named_fork<T: AsRef<Path>>(filename: T) -> io::Result<File> {
    let mut path = filename.as_ref().to_path_buf();
    path.push("..namedfork/rsrc");
    let metadata = fs::metadata(&path)?;
    if metadata.len() > 0 {
        File::open(&path)
    } else {
        Err(io::Error::from(io::ErrorKind::NotFound))
    }
}

pub fn open_resource_fork<T: AsRef<Path>>(filename: T) -> io::Result<File> {
    open_named_fork(filename.as_ref())
        .or_else(|_| File::open({
            let mut path = filename.as_ref().to_path_buf();
            path.set_extension({
                path.extension().map_or_else(|| OsString::from("rsrc"), |ext| {
                    let mut ext = ext.to_os_string();
                    ext.push(".rsrc");
                    ext
                })
            });
            path
        }))
}
