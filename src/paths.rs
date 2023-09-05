//! Contains all of the paths that Foreman needs to deal with.

use std::path::{Path, PathBuf};

use crate::{auth_store::DEFAULT_AUTH_CONFIG, error::ForemanError, fs};

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

    fn from_root<P: AsRef<Path>>(&self, path: P) -> PathBuf {
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
        self.from_root("tool-cache.json")
    }

    pub fn create_all(&self) -> Result<(), ForemanError> {
        fs::create_dir_all(self.root_dir())?;
        fs::create_dir_all(self.bin_dir())?;
        fs::create_dir_all(self.tools_dir())?;

        let config = self.user_config();
        fs::write_if_not_found(&config, DEFAULT_USER_CONFIG)?;

        let auth = self.auth_store();
        fs::write_if_not_found(&auth, DEFAULT_AUTH_CONFIG)?;

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

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn root_dir() {
        let root = PathBuf::from("/foreman");
        let paths = ForemanPaths::new(root.clone());

        assert_eq!(paths.root_dir(), root);
    }

    #[test]
    fn tools_dir() {
        let mut directory = PathBuf::from("/foreman");
        let paths = ForemanPaths::new(directory.clone());
        directory.push("tools");

        assert_eq!(directory, paths.tools_dir());
    }

    #[test]
    fn bin_dir() {
        let mut directory = PathBuf::from("/foreman");
        let paths = ForemanPaths::new(directory.clone());
        directory.push("bin");

        assert_eq!(directory, paths.bin_dir());
    }

    #[test]
    fn auth_store() {
        let mut directory = PathBuf::from("/foreman");
        let paths = ForemanPaths::new(directory.clone());
        directory.push("auth.toml");

        assert_eq!(directory, paths.auth_store());
    }

    #[test]
    fn user_config() {
        let mut directory = PathBuf::from("/foreman");
        let paths = ForemanPaths::new(directory.clone());
        directory.push("foreman.toml");

        assert_eq!(directory, paths.user_config());
    }

    #[test]
    fn index_file() {
        let mut directory = PathBuf::from("/foreman");
        let paths = ForemanPaths::new(directory.clone());
        directory.push("tool-cache.json");

        assert_eq!(directory, paths.index_file());
    }
}
