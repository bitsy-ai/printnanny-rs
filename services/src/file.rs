use std::fs::File;
use std::io;
use std::path::Path;

pub fn open<P: AsRef<Path>>(path: P) -> io::Result<File> {
    File::open(&path).map_err(|err| {
        io::Error::new(
            io::ErrorKind::Other,
            format!("Failed to open file at {:?}: {}", path.as_ref(), err),
        )
    })
}
