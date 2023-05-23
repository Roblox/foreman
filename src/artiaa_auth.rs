use std::{collections::HashMap, path::Path};

use reqwest::Url;
use serde::{Deserialize, Serialize};

use crate::{
    error::{ForemanError, ForemanResult},
    fs,
};

#[derive(Debug, Default, Serialize, Deserialize, PartialEq, Eq)]
pub struct Credentials {
    username: String,
    token: String,
}

/// Contains stored user tokens that Foreman can use to download tools.
#[derive(Debug, Default, Serialize, Deserialize, PartialEq, Eq)]
pub struct Tokens {
    tokens: HashMap<String, Credentials>,
}

impl Tokens {
    #[allow(dead_code)]
    pub fn load(path: &Path) -> ForemanResult<Self> {
        if let Some(contents) = fs::try_read(path)? {
            let tokens: Tokens = serde_json::from_slice(&contents)
                .map_err(|error| ForemanError::auth_parsing(path, error.to_string()))?;

            Ok(tokens)
        } else {
            log::debug!("Artifactory tokens config not found");
            Ok(Tokens::default())
        }
    }

    #[allow(dead_code)]
    pub fn get_credentials(&self, url: &Url) -> Option<&Credentials> {
        if let Some(domain) = url.domain() {
            self.tokens.get(domain)
        } else {
            log::warn!(
                "Could not find credentials for artifactory url with invalid domain: {}",
                url
            );
            None
        }
    }
}

#[cfg(test)]
mod test {
    use std::str::FromStr;

    use super::*;
    use jsonschema_valid::Config;
    use serde_json::Value;
    use tempfile::{tempdir, TempDir};

    const SCHEMA: &str = include_str!("../resources/artiaa-format.json");

    const EXAMPLE_FILE: &str = r#"{
        "tokens": {
            "example.com": {
                "username": "example_user",
                "token": "123456"
            },
            "artifactory.example.com": {
                "username": "artifactory_user",
                "token": "abcdef"
            }
        }
    }"#;

    fn write_test_config(contents: &str) -> TempDir {
        let folder = tempdir().unwrap();
        fs::write(folder.path().join("tokens.json"), contents).unwrap();

        folder
    }

    #[test]
    fn load_file() {
        let folder = write_test_config(EXAMPLE_FILE);

        Tokens::load(folder.path().join("tokens.json").as_ref()).unwrap();
    }

    #[test]
    fn read_credential() {
        let folder = write_test_config(EXAMPLE_FILE);
        let tokens = Tokens::load(folder.path().join("tokens.json").as_ref()).unwrap();

        let url = Url::from_str("https://example.com").unwrap();
        assert_eq!(
            tokens.get_credentials(&url).unwrap(),
            &Credentials {
                username: "example_user".to_string(),
                token: "123456".to_string(),
            }
        );
        let artifactory_url = Url::from_str("https://artifactory.example.com").unwrap();
        assert_eq!(
            tokens.get_credentials(&artifactory_url).unwrap(),
            &Credentials {
                username: "artifactory_user".to_string(),
                token: "abcdef".to_string(),
            }
        );
    }

    #[test]
    fn read_with_domain() {
        let folder = write_test_config(EXAMPLE_FILE);
        let tokens = Tokens::load(folder.path().join("tokens.json").as_ref()).unwrap();

        let url = Url::from_str("https://example.com").unwrap();
        assert_eq!(
            tokens.get_credentials(&url).unwrap(),
            &Credentials {
                username: "example_user".to_string(),
                token: "123456".to_string(),
            }
        )
    }

    #[test]
    fn read_url_not_found() {
        let folder = write_test_config(EXAMPLE_FILE);
        let tokens: Tokens = Tokens::load(folder.path().join("tokens.json").as_ref()).unwrap();

        let url = Url::from_str("https://other-example.com").unwrap();
        assert!(tokens.get_credentials(&url).is_none())
    }

    #[test]
    fn read_invalid_domain() {
        let folder = write_test_config(EXAMPLE_FILE);
        let tokens: Tokens = Tokens::load(folder.path().join("tokens.json").as_ref()).unwrap();

        let url = Url::from_str("file://path/to/file").unwrap();
        assert!(tokens.get_credentials(&url).is_none())
    }

    #[test]
    fn valid_file_conforms_to_schema() {
        let schema: Value = serde_json::from_str(SCHEMA).unwrap();
        let example: Value = serde_json::from_str(EXAMPLE_FILE).unwrap();
        let cfg = Config::from_schema(&schema, None).unwrap();

        assert!(cfg.validate_schema().is_ok());
        assert!(cfg.validate(&example).is_ok());
    }
}
