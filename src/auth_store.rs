use std::io;

use serde::{Deserialize, Serialize};
use toml_edit::{value, Document};

use crate::{fs, paths};

pub static DEFAULT_AUTH_CONFIG: &str = include_str!("../resources/default-auth.toml");

/// Contains stored user tokens that Foreman can use to download tools.
#[derive(Debug, Default, Serialize, Deserialize)]
pub struct AuthStore {
    pub github: Option<String>,
    pub gitlab: Option<String>,
}

impl AuthStore {
    pub fn load() -> io::Result<Self> {
        log::debug!("Loading auth store...");

        match fs::read(paths::auth_store()) {
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
                    return Ok(AuthStore::default());
                } else {
                    return Err(err);
                }
            }
        }
    }

    pub fn set_github_token(token: &str) -> io::Result<()> {
        Self::set_token("github", token)
    }

    pub fn set_gitlab_token(token: &str) -> io::Result<()> {
        Self::set_token("gitlab", token)
    }

    fn set_token(key: &str, token: &str) -> io::Result<()> {
        let contents = match fs::read_to_string(paths::auth_store()) {
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
        fs::write(paths::auth_store(), serialized)
    }
}
