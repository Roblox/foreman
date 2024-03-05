use crate::error::ForemanError;
use crate::{error::ForemanResult, fs};
use artiaa_auth::{error::ArtifactoryAuthError, Credentials};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

use std::{
    ops::{Deref, DerefMut},
    path::Path,
};
/// Contains stored user tokens that Foreman can use to download tools.
#[derive(Debug, Default, Serialize, Deserialize)]
pub struct ArtifactoryAuthStore {
    tokens: HashMap<String, Credentials>,
}

impl Deref for ArtifactoryAuthStore {
    type Target = HashMap<String, Credentials>;

    fn deref(&self) -> &Self::Target {
        &self.tokens
    }
}

impl DerefMut for ArtifactoryAuthStore {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.tokens
    }
}

impl ArtifactoryAuthStore {
    pub fn set_token(auth_file: &Path, key: &str, token: &str) -> ForemanResult<()> {
        let contents = fs::try_read_to_string(auth_file)?;

        let mut store: ArtifactoryAuthStore = if let Some(contents) = contents {
            serde_json::from_str(&contents).map_err(|err: serde_json::Error| {
                ForemanError::ArtiAAError {
                    error: ArtifactoryAuthError::auth_parsing(auth_file, err.to_string()),
                }
            })?
        } else {
            ArtifactoryAuthStore::default()
        };

        store.insert(
            key.to_owned(),
            Credentials {
                username: "".to_owned(),
                token: token.to_owned(),
            },
        );

        let serialized =
            serde_json::to_string_pretty(&store).map_err(|err: serde_json::Error| {
                ForemanError::ArtiAAError {
                    error: ArtifactoryAuthError::auth_parsing(auth_file, err.to_string()),
                }
            })?;

        if let Some(dir) = auth_file.parent() {
            fs::create_dir_all(dir)?;
            fs::write(auth_file, serialized)
        } else {
            Err(ForemanError::ArtiAAError {
                error: ArtifactoryAuthError::auth_parsing(
                    auth_file,
                    "Could not find parent directory of auth file".to_owned(),
                ),
            })
        }
    }
}
