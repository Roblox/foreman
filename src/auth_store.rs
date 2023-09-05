use std::path::Path;

use serde::{Deserialize, Serialize};
use toml_edit::{value, Document, TomlError};

use crate::{
    error::{ForemanError, ForemanResult},
    fs,
};

pub static DEFAULT_AUTH_CONFIG: &str = include_str!("../resources/default-auth.toml");

/// Contains stored user tokens that Foreman can use to download tools.
#[derive(Debug, Default, Serialize, Deserialize)]
pub struct AuthStore {
    pub github: Option<String>,
    pub gitlab: Option<String>,
}

impl AuthStore {
    pub fn load(path: &Path) -> ForemanResult<Self> {
        if let Some(contents) = fs::try_read(path)? {
            log::debug!("Loading auth store");
            let store: AuthStore = toml::from_slice(&contents)
                .map_err(|error| ForemanError::auth_parsing(path, error.to_string()))?;

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
        } else {
            log::debug!("Auth store not found");
            Ok(AuthStore::default())
        }
    }

    pub fn set_github_token(auth_file: &Path, token: &str) -> ForemanResult<()> {
        Self::set_token(auth_file, "github", token)
    }

    pub fn set_gitlab_token(auth_file: &Path, token: &str) -> ForemanResult<()> {
        Self::set_token(auth_file, "gitlab", token)
    }

    fn set_token(auth_file: &Path, key: &str, token: &str) -> ForemanResult<()> {
        let contents =
            fs::try_read_to_string(auth_file)?.unwrap_or_else(|| DEFAULT_AUTH_CONFIG.to_owned());

        let mut store: Document = contents
            .parse()
            .map_err(|err: TomlError| ForemanError::auth_parsing(auth_file, err.to_string()))?;
        store[key] = value(token);

        let serialized = store.to_string();
        fs::write(auth_file, serialized)
    }
}
