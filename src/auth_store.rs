use std::io;

use serde::{Deserialize, Serialize};

use crate::{fs, paths};

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
}
