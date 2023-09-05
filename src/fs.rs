//! Wrapper around std::fs and std::io that attaches file paths to errors.
//!
//! We won't use all these wrappers all the time, so it's okay if some of them
//! are unused.

use std::{
    fs,
    io::{self, BufWriter, Read},
    path::Path,
};

/// A wrapper around std::fs::read that returns None if the file does not exist.
pub fn try_read<P: AsRef<Path>>(path: P) -> ForemanResult<Option<Vec<u8>>> {
    let path = path.as_ref();

    match fs::read(&path).map(Some) {
        Ok(contents) => Ok(contents),
        Err(err) => {
            if err.kind() == io::ErrorKind::NotFound {
                Ok(None)
            } else {
                Err(ForemanError::read_error(err, path))
            }
        }
    }
}

/// A wrapper around std::fs::read_to_string that returns None if the file does not exist.
pub fn try_read_to_string<P: AsRef<Path>>(path: P) -> ForemanResult<Option<String>> {
    let path = path.as_ref();

    match fs::read_to_string(&path).map(Some) {
        Ok(contents) => Ok(contents),
        Err(err) => {
            if err.kind() == io::ErrorKind::NotFound {
                Ok(None)
            } else {
                Err(ForemanError::read_error(err, path))
            }
        }
    }
}

/// A wrapper around std::fs::write that only writes if the file does not exist.
pub fn write_if_not_found<P: AsRef<Path>, C: AsRef<[u8]>>(
    path: P,
    contents: C,
) -> ForemanResult<()> {
    let path = path.as_ref();

    if let Err(err) = std::fs::metadata(&path) {
        if err.kind() == io::ErrorKind::NotFound {
            write(&path, contents)
        } else {
            Err(ForemanError::write_error(err, path))
        }
    } else {
        Ok(())
    }
}

/// A wrapper around std::fs::write.
pub fn write<P: AsRef<Path>, C: AsRef<[u8]>>(path: P, contents: C) -> ForemanResult<()> {
    let path = path.as_ref();

    fs::write(path, contents).map_err(|source| ForemanError::write_error(source, path))
}

/// A wrapper around std::fs::copy.
pub fn copy<P: AsRef<Path>, Q: AsRef<Path>>(source_path: P, dest_path: Q) -> ForemanResult<u64> {
    let source_path = source_path.as_ref();
    let dest_path = dest_path.as_ref();

    fs::copy(source_path, dest_path)
        .map_err(|source| ForemanError::copy_error(source, source_path, dest_path))
}

/// A wrapper around std::io::copy.
pub fn copy_from_reader<R: Read + ?Sized, P: AsRef<Path>>(
    reader: &mut R,
    dest_path: P,
) -> ForemanResult<u64> {
    let dest_path = dest_path.as_ref();
    let output_file = std::fs::File::create(&dest_path)
        .map_err(|err| ForemanError::create_file_error(err, &dest_path))?;
    let mut output = BufWriter::new(output_file);

    io::copy(reader, &mut output).map_err(|err| ForemanError::write_error(err, &dest_path))
}

/// A wrapper around std::fs::create_dir_all.
///
/// Currently reports all errors as happening from the given path.
pub fn create_dir_all<P: AsRef<Path>>(path: P) -> ForemanResult<()> {
    let path = path.as_ref();

    fs::create_dir_all(path).map_err(|source| ForemanError::write_error(source, path))
}

pub use fs::Permissions;

use crate::error::{ForemanError, ForemanResult};

#[cfg(unix)]
/// A wrapper around std::fs::set_permissions
pub fn set_permissions<P: AsRef<Path>>(path: P, permissions: Permissions) -> ForemanResult<()> {
    let path = path.as_ref();

    fs::set_permissions(path, permissions)
        .map_err(|source| ForemanError::set_permission_error(source, path))
}
