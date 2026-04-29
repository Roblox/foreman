use crate::{
    error::{ForemanError, ForemanResult},
    fs,
};
use keyring_core::api::CredentialStore;
use serde::{Deserialize, Serialize};
use std::{
    path::Path,
    sync::{Arc, Once},
};
use toml_edit::{value, Document, TomlError};
pub static DEFAULT_AUTH_CONFIG: &str = include_str!("../resources/default-auth.toml");

const KEYRING_SERVICE: &str = "foreman";

fn ensure_credential_store() {
    static INIT: Once = Once::new();
    INIT.call_once(|| match create_platform_store() {
        Ok(store) => {
            keyring_core::set_default_store(store);
            log::debug!("Initialized OS credential store");
        }
        Err(e) => {
            log::debug!("OS credential store unavailable: {}", e);
        }
    });
}

fn require_credential_store() -> ForemanResult<()> {
    ensure_credential_store();
    if keyring_core::get_default_store().is_none() {
        return Err(ForemanError::keyring_error(
            "OS credential store is not supported on this platform. \
             Use `foreman github-auth` or `foreman gitlab-auth` to store tokens in auth.toml instead.",
        ));
    }
    Ok(())
}

#[cfg(target_os = "macos")]
fn create_platform_store() -> Result<Arc<CredentialStore>, String> {
    let store: Arc<CredentialStore> =
        apple_native_keyring_store::keychain::Store::new().map_err(|e| e.to_string())?;
    Ok(store)
}

#[cfg(windows)]
fn create_platform_store() -> Result<Arc<CredentialStore>, String> {
    let store: Arc<CredentialStore> =
        windows_native_keyring_store::Store::new().map_err(|e| e.to_string())?;
    Ok(store)
}

#[cfg(all(not(target_os = "macos"), not(windows)))]
fn create_platform_store() -> Result<Arc<CredentialStore>, String> {
    Err("OS credential store not supported on this platform".to_string())
}

/// Contains stored user tokens that Foreman can use to download tools.
#[derive(Debug, Default, Serialize, Deserialize)]
pub struct AuthStore {
    pub github: Option<String>,
    pub gitlab: Option<String>,
}

impl AuthStore {
    pub fn load(path: &Path) -> ForemanResult<Self> {
        let github = Self::resolve_token("github", path)?;
        let gitlab = Self::resolve_token("gitlab", path)?;

        if github.is_none() && gitlab.is_none() {
            log::debug!("Found no credentials");
        }

        Ok(Self { github, gitlab })
    }

    fn resolve_token(key: &str, auth_file: &Path) -> ForemanResult<Option<String>> {
        if let Some(token) = get_token_from_keyring(key) {
            log::debug!("Found {} credentials from OS keyring", key);
            return Ok(Some(token));
        }

        if let Some(contents) = fs::try_read(auth_file)? {
            let file_store: AuthStore = toml::from_slice(&contents)
                .map_err(|error| ForemanError::auth_parsing(auth_file, error.to_string()))?;

            let token = match key {
                "github" => file_store.github,
                "gitlab" => file_store.gitlab,
                _ => None,
            };

            if token.is_some() {
                log::debug!("Found {} credentials from auth.toml", key);
            }
            return Ok(token);
        }

        Ok(None)
    }

    pub fn set_github_token(auth_file: &Path, token: &str) -> ForemanResult<()> {
        Self::set_token_in_file(auth_file, "github", token)
    }

    pub fn set_gitlab_token(auth_file: &Path, token: &str) -> ForemanResult<()> {
        Self::set_token_in_file(auth_file, "gitlab", token)
    }

    fn set_token_in_file(auth_file: &Path, key: &str, token: &str) -> ForemanResult<()> {
        let contents =
            fs::try_read_to_string(auth_file)?.unwrap_or_else(|| DEFAULT_AUTH_CONFIG.to_owned());

        let mut store: Document = contents
            .parse()
            .map_err(|err: TomlError| ForemanError::auth_parsing(auth_file, err.to_string()))?;
        store[key] = value(token);

        let serialized = store.to_string();
        fs::write(auth_file, serialized)
    }

    pub fn set_token_secure(provider: &str, token: &str) -> ForemanResult<()> {
        require_credential_store()?;
        set_token_in_keyring(provider, token)
    }

    pub fn delete_token_secure(provider: &str) -> ForemanResult<()> {
        require_credential_store()?;
        delete_token_from_keyring(provider)
    }

    pub fn delete_all_tokens_secure() -> ForemanResult<()> {
        require_credential_store()?;
        let mut errors = Vec::new();
        if let Err(e) = delete_token_from_keyring("github") {
            errors.push(format!("github: {}", e));
        }
        if let Err(e) = delete_token_from_keyring("gitlab") {
            errors.push(format!("gitlab: {}", e));
        }
        if errors.is_empty() {
            Ok(())
        } else {
            Err(ForemanError::keyring_error(errors.join("; ")))
        }
    }

    pub fn migrate_to_keyring(auth_file: &Path) -> ForemanResult<(bool, bool)> {
        require_credential_store()?;
        let contents = match fs::try_read(auth_file)? {
            Some(c) => c,
            None => return Ok((false, false)),
        };

        let file_store: AuthStore = toml::from_slice(&contents)
            .map_err(|error| ForemanError::auth_parsing(auth_file, error.to_string()))?;

        let mut github_migrated = false;
        let mut gitlab_migrated = false;

        if let Some(token) = &file_store.github {
            set_token_in_keyring("github", token)?;
            github_migrated = true;
        }

        if let Some(token) = &file_store.gitlab {
            set_token_in_keyring("gitlab", token)?;
            gitlab_migrated = true;
        }

        if github_migrated || gitlab_migrated {
            Self::clear_migrated_tokens(auth_file, github_migrated, gitlab_migrated)?;
        }

        Ok((github_migrated, gitlab_migrated))
    }

    fn clear_migrated_tokens(
        auth_file: &Path,
        clear_github: bool,
        clear_gitlab: bool,
    ) -> ForemanResult<()> {
        let contents =
            fs::try_read_to_string(auth_file)?.unwrap_or_else(|| DEFAULT_AUTH_CONFIG.to_owned());

        let mut store: Document = contents
            .parse()
            .map_err(|err: TomlError| ForemanError::auth_parsing(auth_file, err.to_string()))?;

        if clear_github {
            store.remove("github");
        }
        if clear_gitlab {
            store.remove("gitlab");
        }

        fs::write(auth_file, store.to_string())
    }
}

fn get_token_from_keyring(key: &str) -> Option<String> {
    ensure_credential_store();
    match keyring_core::Entry::new(KEYRING_SERVICE, key) {
        Ok(entry) => match entry.get_password() {
            Ok(token) if !token.is_empty() => Some(token),
            _ => None,
        },
        Err(_) => None,
    }
}

fn set_token_in_keyring(key: &str, token: &str) -> ForemanResult<()> {
    ensure_credential_store();
    let entry = keyring_core::Entry::new(KEYRING_SERVICE, key)
        .map_err(|e| ForemanError::keyring_error(e.to_string()))?;
    entry
        .set_password(token)
        .map_err(|e| ForemanError::keyring_error(e.to_string()))
}

fn delete_token_from_keyring(key: &str) -> ForemanResult<()> {
    ensure_credential_store();
    let entry = keyring_core::Entry::new(KEYRING_SERVICE, key)
        .map_err(|e| ForemanError::keyring_error(e.to_string()))?;
    entry
        .delete_credential()
        .map_err(|e| ForemanError::keyring_error(e.to_string()))
}
