use std::{io, path::Path};

use serde::{Deserialize, Serialize};
use toml_edit::{value, Document};

use crate::fs;

pub static DEFAULT_AUTH_CONFIG: &str = include_str!("../resources/default-auth.toml");

/// Contains stored user tokens that Foreman can use to download tools.
#[derive(Debug, Default, Serialize, Deserialize)]
pub struct AuthStore {
    pub github: Option<String>,
    pub gitlab: Option<String>,
}

impl AuthStore {
    pub fn load(path: &Path) -> io::Result<Self> {
        log::debug!("Loading auth store...");

        match fs::read(path) {
            Ok(contents) => {
                let store: AuthStore = toml::from_slice(&contents).unwrap();

                let mut found_credentials = false;
                if store.github.is_some() {
                    log::debug!("Found GitHub credentials");
                    found_credentials = true;
                }
                if store.gitlab.is_some() {
                    log::debug!("Found GitLab credentials");
                    found_credentials = true;
                }
                if !found_credentials {
                    log::debug!("Found no credentials");
                }

                Ok(store)
            }
            Err(err) => {
                if err.kind() == io::ErrorKind::NotFound {
                    Ok(AuthStore::default())
                } else {
                    Err(err)
                }
            }
        }
    }

    pub fn set_github_token(auth_file: &Path, token: &str) -> io::Result<()> {
        Self::set_token(auth_file, "github", token)
    }

    pub fn set_gitlab_token(auth_file: &Path, token: &str) -> io::Result<()> {
        Self::set_token(auth_file, "gitlab", token)
    }

    fn set_token(auth_file: &Path, key: &str, token: &str) -> io::Result<()> {
        let contents = match fs::read_to_string(auth_file) {
            Ok(contents) => contents,
            Err(err) => {
                if err.kind() == io::ErrorKind::NotFound {
                    DEFAULT_AUTH_CONFIG.to_owned()
                } else {
                    return Err(err);
                }
            }
        };

        let mut store: Document = contents.parse().unwrap();
        store[key] = value(token);

        let serialized = store.to_string();
        fs::write(auth_file, serialized)
    }
}
