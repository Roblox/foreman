//! Wrapper around std::fs and std::io that attaches file paths to errors.
//!
//! We won't use all these wrappers all the time, so it's okay if some of them
//! are unused.

use std::{
    fs,
    io::{self},
    path::Path,
};

/// A wrapper around std::fs::read that returns None if the file does not exist.
pub fn try_read<P: AsRef<Path>>(path: P) -> ArtifactoryAuthResult<Option<Vec<u8>>> {
    let path = path.as_ref();

    match fs::read(&path).map(Some) {
        Ok(contents) => Ok(contents),
        Err(err) => {
            if err.kind() == io::ErrorKind::NotFound {
                Ok(None)
            } else {
                Err(ArtifactoryAuthError::read_error(err, path))
            }
        }
    }
}

/// A wrapper around std::fs::write.
#[cfg(test)]
pub fn write<P: AsRef<Path>, C: AsRef<[u8]>>(path: P, contents: C) -> ArtifactoryAuthResult<()> {
    let path = path.as_ref();

    fs::write(path, contents).map_err(|source| ArtifactoryAuthError::write_error(source, path))
}

use crate::error::{ArtifactoryAuthError, ArtifactoryAuthResult};
