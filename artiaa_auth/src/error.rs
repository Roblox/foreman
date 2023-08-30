use std::{io, path::PathBuf};
pub type ArtifactoryAuthResult<T> = Result<T, ArtifactoryAuthError>;

#[derive(Debug)]
pub enum ArtifactoryAuthError {
    FileParse { source: String, path: PathBuf },
    Read { source: io::Error, path: PathBuf },
    Write { source: io::Error, path: PathBuf },
}

impl ArtifactoryAuthError {
    pub fn auth_parsing<P: Into<PathBuf>, S: Into<String>>(auth_path: P, source: S) -> Self {
        Self::FileParse {
            source: source.into(),
            path: auth_path.into(),
        }
    }
    pub fn read_error<P: Into<PathBuf>>(source: io::Error, path: P) -> Self {
        Self::Read {
            source,
            path: path.into(),
        }
    }
    pub fn write_error<P: Into<PathBuf>>(source: io::Error, path: P) -> Self {
        Self::Write {
            source,
            path: path.into(),
        }
    }
}
