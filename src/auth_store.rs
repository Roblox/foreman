use std::io;

use serde::{Deserialize, Serialize};
use toml_edit::{value, Document};

use crate::{fs, paths};

pub static DEFAULT_AUTH_CONFIG: &str = include_str!("../resources/default-auth.toml");

/// Contains stored user tokens that Foreman can use to download tools.
#[derive(Debug, Default, Serialize, Deserialize)]
pub struct AuthStore {
    pub github: Option<String>,
}

impl AuthStore {
    pub fn load() -> io::Result<Self> {
        log::debug!("Loading auth store...");

        match fs::read(paths::auth_store()) {
            Ok(contents) => {
                let store: AuthStore = toml::from_slice(&contents).unwrap();

                if store.github.is_some() {
                    log::debug!("Found GitHub credentials");
                } else {
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
        store["github"] = value(token);

        let serialized = store.to_string();
        fs::write(paths::auth_store(), serialized)
    }
}
