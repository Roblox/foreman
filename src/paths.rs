//! Contains all of the paths that Foreman needs to deal with.

use std::{
    io,
    path::{Path, PathBuf},
};

use crate::{auth_store::DEFAULT_AUTH_CONFIG, fs};

static DEFAULT_USER_CONFIG: &str = include_str!("../resources/default-foreman.toml");

const FOREMAN_PATH_ENV_VARIABLE: &str = "FOREMAN_HOME";

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ForemanPaths {
    root_dir: PathBuf,
}

impl ForemanPaths {
    pub fn from_env() -> Option<Self> {
        std::env::var(FOREMAN_PATH_ENV_VARIABLE)
            .map(PathBuf::from)
            .ok()
            .and_then(|path| {
                if path.is_dir() {
                    Some(Self { root_dir:path })
                } else {
                    if path.exists() {
                        log::warn!(
                            "path specified using {} `{}` is not a directory. Using default path `~/.foreman`",
                            FOREMAN_PATH_ENV_VARIABLE,
                            path.display()
                        );
                    } else {
                        log::warn!(
                            "path specified using {} `{}` does not exist. Using default path `~/.foreman`",
                            FOREMAN_PATH_ENV_VARIABLE,
                            path.display()
                        );
                    }
                    None
                }
            })
    }

    pub fn new(root_dir: PathBuf) -> Self {
        Self { root_dir }
    }

    pub fn root_dir(&self) -> PathBuf {
        self.root_dir.clone()
    }

    pub fn from_root<P: AsRef<Path>>(&self, path: P) -> PathBuf {
        let mut dir = self.root_dir();
        dir.push(path);
        dir
    }

    pub fn tools_dir(&self) -> PathBuf {
        self.from_root("tools")
    }

    pub fn bin_dir(&self) -> PathBuf {
        self.from_root("bin")
    }

    pub fn auth_store(&self) -> PathBuf {
        self.from_root("auth.toml")
    }

    pub fn user_config(&self) -> PathBuf {
        self.from_root("foreman.toml")
    }

    pub fn index_file(&self) -> PathBuf {
        self.from_root("tool-cache.toml")
    }

    pub fn create_all(&self) -> io::Result<()> {
        fs::create_dir_all(self.root_dir())?;
        fs::create_dir_all(self.bin_dir())?;
        fs::create_dir_all(self.tools_dir())?;

        let config = self.user_config();
        if let Err(err) = fs::metadata(&config) {
            if err.kind() == io::ErrorKind::NotFound {
                fs::write(&config, DEFAULT_USER_CONFIG)?;
            } else {
                return Err(err);
            }
        }

        let auth = self.auth_store();
        if let Err(err) = fs::metadata(&auth) {
            if err.kind() == io::ErrorKind::NotFound {
                fs::write(&auth, DEFAULT_AUTH_CONFIG)?;
            } else {
                return Err(err);
            }
        }

        Ok(())
    }
}

impl Default for ForemanPaths {
    fn default() -> Self {
        let mut root_dir = dirs::home_dir().expect("unable to get home directory");
        root_dir.push(".foreman");
        Self::new(root_dir)
    }
}
