use std::{fmt, io, path::PathBuf};

use semver::Version;

use crate::config::{ConfigFile, ToolSpec};

pub type ForemanResult<T> = Result<T, ForemanError>;

#[derive(Debug)]
pub enum ForemanError {
    IO {
        source: io::Error,
        message: Option<String>,
    },
    Read {
        source: io::Error,
        path: PathBuf,
    },
    CreateFile {
        source: io::Error,
        path: PathBuf,
    },
    Write {
        source: io::Error,
        path: PathBuf,
    },
    Copy {
        source: io::Error,
        source_path: PathBuf,
        destination_path: PathBuf,
    },
    #[cfg(unix)]
    SetPermissions {
        source: io::Error,
        path: PathBuf,
    },
    ConfigFileParse {
        source: String,
        path: PathBuf,
    },
    AuthFileParse {
        source: String,
        path: PathBuf,
    },
    ToolCacheParse {
        source: String,
        path: PathBuf,
    },
    RequestFailed {
        source: reqwest::Error,
    },
    UnexpectedResponseBody {
        source: String,
        response_body: String,
        url: String,
    },
    NoCompatibleVersionFound {
        tool: ToolSpec,
        available_versions: Vec<Version>,
    },
    InvalidReleaseAsset {
        tool: ToolSpec,
        version: Version,
        message: String,
    },
    ToolNotInstalled {
        name: String,
        current_path: PathBuf,
        config_file: ConfigFile,
    },
}

impl ForemanError {
    pub fn io_error_with_context<S: Into<String>>(source: io::Error, message: S) -> Self {
        Self::IO {
            source,
            message: Some(message.into()),
        }
    }

    pub fn read_error<P: Into<PathBuf>>(source: io::Error, path: P) -> Self {
        Self::Read {
            source,
            path: path.into(),
        }
    }

    pub fn create_file_error<P: Into<PathBuf>>(source: io::Error, path: P) -> Self {
        Self::CreateFile {
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

    pub fn copy_error<P: Into<PathBuf>, P2: Into<PathBuf>>(
        source: io::Error,
        source_path: P,
        destination_path: P2,
    ) -> Self {
        Self::Copy {
            source,
            source_path: source_path.into(),
            destination_path: destination_path.into(),
        }
    }

    #[cfg(unix)]
    pub fn set_permission_error<P: Into<PathBuf>>(source: io::Error, path: P) -> Self {
        Self::SetPermissions {
            source,
            path: path.into(),
        }
    }

    pub fn config_parsing<P: Into<PathBuf>, S: Into<String>>(config_path: P, source: S) -> Self {
        Self::ConfigFileParse {
            source: source.into(),
            path: config_path.into(),
        }
    }

    pub fn auth_parsing<P: Into<PathBuf>, S: Into<String>>(auth_path: P, source: S) -> Self {
        Self::AuthFileParse {
            source: source.into(),
            path: auth_path.into(),
        }
    }

    pub fn tool_cache_parsing<P: Into<PathBuf>, S: Into<String>>(path: P, source: S) -> Self {
        Self::ToolCacheParse {
            source: source.into(),
            path: path.into(),
        }
    }

    pub fn request_failed(source: reqwest::Error) -> Self {
        Self::RequestFailed { source }
    }

    pub fn unexpected_response_body<S: Into<String>, S2: Into<String>, S3: Into<String>>(
        source: S,
        response_body: S2,
        url: S3,
    ) -> Self {
        Self::UnexpectedResponseBody {
            source: source.into(),
            response_body: response_body.into(),
            url: url.into(),
        }
    }

    pub fn no_compatible_version_found(tool: &ToolSpec, available_versions: Vec<Version>) -> Self {
        Self::NoCompatibleVersionFound {
            tool: tool.clone(),
            available_versions,
        }
    }

    pub fn invalid_release_asset<S: Into<String>>(
        tool: &ToolSpec,
        version: &Version,
        message: S,
    ) -> Self {
        Self::InvalidReleaseAsset {
            tool: tool.clone(),
            version: version.clone(),
            message: message.into(),
        }
    }
}

impl fmt::Display for ForemanError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::IO { source, message } => {
                if let Some(message) = message {
                    write!(f, "{}: {}", message, source)
                } else {
                    write!(f, "io error: {}", source)
                }
            }
            Self::Read { source, path } => write!(
                f,
                "an error happened trying to read {}: {}",
                path.display(),
                source
            ),
            Self::CreateFile { source, path } => write!(
                f,
                "an error happened trying to create file {}: {}",
                path.display(),
                source
            ),
            Self::Write { source, path } => write!(
                f,
                "an error happened trying to write {}: {}",
                path.display(),
                source
            ),
            Self::Copy {
                source,
                source_path,
                destination_path,
            } => write!(
                f,
                "an error happened copying {} to {}: {}",
                source_path.display(),
                destination_path.display(),
                source
            ),
            #[cfg(unix)]
            Self::SetPermissions { source, path } => write!(
                f,
                "an error happened trying to set permissions on {}: {}",
                path.display(),
                source
            ),
            Self::ConfigFileParse { source, path } => write!(
                f,
                "unable to parse Foreman configuration file (at {}): {}\n\n{}",
                path.display(),
                source,
                FOREMAN_CONFIG_HELP
            ),
            Self::AuthFileParse { source, path } => write!(
                f,
                "unable to parse Foreman authentication file (at {}): {}\n\n{}",
                path.display(),
                source,
                FOREMAN_AUTH_HELP
            ),
            Self::ToolCacheParse { source, path } => {
                write!(
                    f,
                    "unable to parse Foreman tool cache file (at {}): {}",
                    path.display(),
                    source
                )
            }
            Self::RequestFailed { source } => write!(f, "request failed: {}", source),
            Self::UnexpectedResponseBody {
                source,
                response_body,
                url,
            } => write!(
                f,
                "unexpected response body: {}\nRequest from `{}`\n\nReceived body:\n{}",
                source, url, response_body
            ),
            Self::NoCompatibleVersionFound {
                tool,
                available_versions,
            } => {
                write!(
                    f,
                    "no compatible version of {} was found for version requirement {}{}",
                    tool.source(),
                    tool.version(),
                    if available_versions.is_empty() {
                        "".to_owned()
                    } else {
                        format!(
                            ". Available versions:\n* {}",
                            available_versions
                                .iter()
                                .map(|version| version.to_string())
                                .collect::<Vec<_>>()
                                .join("\n* ")
                        )
                    }
                )
            }
            Self::InvalidReleaseAsset {
                tool,
                version,
                message,
            } => write!(
                f,
                "invalid release asset for {} ({}): {}",
                tool.source(),
                version,
                message
            ),
            Self::ToolNotInstalled {
                name,
                current_path,
                config_file,
            } => write!(
                f,
                "'{}' is not a known Foreman tool, but Foreman was invoked \
                with its name.\n\nTo use this tool from {}, declare it in a \
                'foreman.toml' file in the current directory or a parent \
                directory.\n\n{}",
                name,
                current_path.display(),
                config_file,
            ),
        }
    }
}

const FOREMAN_CONFIG_HELP: &str = r#"A Foreman configuration file looks like this:

[tools] # list the tools you want to install under this header

# each tool is on its own line, the tool name is on the left
# side of `=` and the right side tells Foreman where to find
# it and which version to download
tool_name = { github = "user/repository-name", version = "1.0.0" }

# tools hosted on gitlab follows the same structure, except
# `github` is replaced with `gitlab`

# Examples:
stylua = { github = "JohnnyMorganz/StyLua", version = "0.11.3" }
darklua = { gitlab = "seaofvoices/darklua", version = "0.7.0" }"#;

const FOREMAN_AUTH_HELP: &str = r#"A Foreman authentication file looks like this:

# For authenticating with GitHub.com, put a personal access token here under the
# `github` key. This is useful if you hit GitHub API rate limits or if you need
# to access private tools.

github = "YOUR_TOKEN_HERE"

# For authenticating with GitLab.com, put a personal access token here under the
# `gitlab` key. This is useful if you hit GitLab API rate limits or if you need
# to access private tools.

gitlab = "YOUR_TOKEN_HERE""#;
