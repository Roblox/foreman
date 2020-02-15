//! Wrapper around std::fs and std::io that attaches file paths to errors.
//!
//! We won't use all these wrappers all the time, so it's okay if some of them
//! are unused.

#![allow(unused)]

use std::{
    error::Error as StdError,
    fmt, fs,
    io::{self, Read, Write},
    path::{Path, PathBuf},
};

pub type Result<T> = io::Result<T>;

/// A wrapper around std::fs::read.
pub fn read<P: AsRef<Path>>(path: P) -> Result<Vec<u8>> {
    let path = path.as_ref();

    fs::read(path).map_err(|source| Error::new(source, path))
}

/// A wrapper around std::fs::write.
pub fn write<P: AsRef<Path>, C: AsRef<[u8]>>(path: P, contents: C) -> Result<()> {
    let path = path.as_ref();

    fs::write(path, contents).map_err(|source| Error::new(source, path))
}

/// A wrapper around std::fs::read.
pub fn copy<P: AsRef<Path>, Q: AsRef<Path>>(source_path: P, dest_path: Q) -> Result<u64> {
    let source_path = source_path.as_ref();
    let dest_path = dest_path.as_ref();

    fs::copy(source_path, dest_path)
        .map_err(|source| CopyError::new(source, source_path, dest_path))
}

/// A wrapper around std::fs::create_dir_all.
///
/// Currently reports all errors as happening from the given path.
pub fn create_dir_all<P: AsRef<Path>>(path: P) -> Result<()> {
    let path = path.as_ref();

    fs::create_dir_all(path).map_err(|source| Error::new(source, path))
}

/// A wrapper around std::fs::metadata.
pub fn metadata<P: AsRef<Path>>(path: P) -> Result<fs::Metadata> {
    let path = path.as_ref();

    fs::metadata(path).map_err(|source| Error::new(source, path))
}

pub use fs::Permissions;

/// A wrapper around std::fs::set_permissions
pub fn set_permissions<P: AsRef<Path>>(path: P, permissions: Permissions) -> Result<()> {
    let path = path.as_ref();

    fs::set_permissions(path, permissions).map_err(|source| Error::new(source, path))
}

/// A wrapper around std::fs::File that contains file path information in error
/// cases.
#[derive(Debug)]
pub struct File {
    source: fs::File,
    path: PathBuf,
}

impl File {
    pub fn create<P: AsRef<Path>>(path: P) -> Result<Self> {
        let path = path.as_ref();
        let source = fs::File::create(path).map_err(|source| Error::new(source, path))?;

        Ok(Self {
            source,
            path: path.to_owned(),
        })
    }

    pub fn open<P: AsRef<Path>>(path: P) -> Result<Self> {
        let path = path.as_ref();
        let source = fs::File::open(path).map_err(|source| Error::new(source, path))?;

        Ok(Self {
            source,
            path: path.to_owned(),
        })
    }
}

impl Read for File {
    fn read(&mut self, buf: &mut [u8]) -> Result<usize> {
        self.source
            .read(buf)
            .map_err(|source| Error::new(source, &self.path))
    }
}

impl Write for File {
    fn write(&mut self, buf: &[u8]) -> Result<usize> {
        self.source
            .write(buf)
            .map_err(|source| Error::new(source, &self.path))
    }

    fn flush(&mut self) -> Result<()> {
        self.source
            .flush()
            .map_err(|source| Error::new(source, &self.path))
    }
}

/// Wrapper around std::fs::read_dir.
pub fn read_dir<P: AsRef<Path>>(path: P) -> Result<ReadDir> {
    let path = path.as_ref();

    fs::read_dir(path)
        .map(|source| ReadDir {
            source,
            path: path.to_owned(),
        })
        .map_err(|source| Error::new(source, path))
}

/// Wrapper around std::fs::ReadDir.
pub struct ReadDir {
    source: fs::ReadDir,
    path: PathBuf,
}

impl Iterator for ReadDir {
    type Item = Result<fs::DirEntry>;

    fn next(&mut self) -> Option<Self::Item> {
        Some(
            self.source
                .next()?
                .map_err(|source| Error::new(source, &self.path)),
        )
    }
}

/// Contains an IO error that has a file path attached.
///
/// This type is never returned directly, but is instead wrapped inside yet
/// another IO error.
#[derive(Debug)]
struct Error {
    source: io::Error,
    path: PathBuf,
}

impl Error {
    fn new<P: Into<PathBuf>>(source: io::Error, path: P) -> io::Error {
        io::Error::new(
            source.kind(),
            Self {
                source,
                path: path.into(),
            },
        )
    }
}

impl fmt::Display for Error {
    fn fmt(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
        write!(formatter, "{} in path {}", self.source, self.path.display())
    }
}

impl StdError for Error {
    fn source(&self) -> Option<&(dyn StdError + 'static)> {
        Some(&self.source)
    }
}

/// Contains an IO error from a copy operation, containing both paths.
#[derive(Debug)]
struct CopyError {
    source: io::Error,
    source_path: PathBuf,
    dest_path: PathBuf,
}

impl CopyError {
    fn new<P: Into<PathBuf>, P2: Into<PathBuf>>(
        source: io::Error,
        source_path: P,
        dest_path: P2,
    ) -> io::Error {
        io::Error::new(
            source.kind(),
            Self {
                source,
                source_path: source_path.into(),
                dest_path: dest_path.into(),
            },
        )
    }
}

impl fmt::Display for CopyError {
    fn fmt(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
        write!(
            formatter,
            "{} copying path {} to {}",
            self.source,
            self.source_path.display(),
            self.dest_path.display()
        )
    }
}

impl StdError for CopyError {
    fn source(&self) -> Option<&(dyn StdError + 'static)> {
        Some(&self.source)
    }
}
