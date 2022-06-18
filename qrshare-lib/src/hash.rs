use std::{
    fs::File,
    io::{self, Read},
    path::PathBuf,
};

use blake2::Digest;

/// Take the hash of file at `path` using the specified digest method `d`, and
/// return a byte buffer.
///
/// Return an error when the file cannot be opened.
pub fn path_hash(path: PathBuf) -> io::Result<impl Iterator<Item = u8>> {
    file_hash(File::open(path)?)
}

/// Take the hash of a file handle `file` (which may not currently point at the
/// beginning of the underlying file) using the specified digest method `d`, and
/// return a byte buffer.
///
/// Return an error when reading the file yields an error.
pub fn file_hash(mut file: File) -> io::Result<impl Iterator<Item = u8>> {
    let mut buf = vec![];
    file.read_to_end(&mut buf)?;
    Ok(bytes_hash(buf))
}

/// Take the hash of a byte array `data` using the specified digest method `d`,
/// and return a byte buffer.
pub fn bytes_hash(data: impl AsRef<[u8]>) -> impl Iterator<Item = u8> {
    digest().chain(data).finalize().into_iter()
}

/// Return a digest object.
pub fn digest() -> blake2::Blake2b {
    blake2::Blake2b::new()
}
