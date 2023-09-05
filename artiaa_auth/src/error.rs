use std::{fmt, io, path::PathBuf};
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

const ARTIFACTORY_AUTH_HELP: &str = include_str!("../../resources/artiaa-format.json");

impl fmt::Display for ArtifactoryAuthError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::FileParse { source, path } => write!(
                f,
                "unable to parse Artifactory authentication file (at {}): {}\n\nAn Artifactory authentication file should match this schema:\n\n{}",
                path.display(),
                source,
                ARTIFACTORY_AUTH_HELP
            ),
            Self::Read { source, path } => write!(
                f,
                "an error happened trying to read {}: {}",
                path.display(),
                source
            ),
            Self::Write { source, path } => write!(
                f,
                "an error happened trying to write {}: {}",
                path.display(),
                source
            ),
        }
    }
}
