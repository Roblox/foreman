use std::{
    collections::HashMap,
    env, fmt,
    ops::{Deref, DerefMut},
};

use semver::VersionReq;
use serde::{
    de::{self, MapAccess, Visitor},
    Deserialize, Serialize,
};

use crate::{
    ci_string::CiString, error::ForemanError, fs, paths::ForemanPaths, tool_provider::Provider,
};

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(untagged)]
pub enum ToolSpec {
    Github {
        // alias to `source` for backward compatibilty
        #[serde(alias = "source")]
        github: String,
        version: VersionReq,
    },
    Gitlab {
        gitlab: String,
        version: VersionReq,
    },
}

impl ToolSpec {
    pub fn cache_key(&self) -> CiString {
        match self {
            ToolSpec::Github { github, .. } => CiString(github.clone()),
            ToolSpec::Gitlab { gitlab, .. } => CiString(format!("gitlab@{}", gitlab)),
        }
    }

    pub fn source(&self) -> &str {
        match self {
            ToolSpec::Github { github: source, .. } | ToolSpec::Gitlab { gitlab: source, .. } => {
                source
            }
        }
    }

    pub fn version(&self) -> &VersionReq {
        match self {
            ToolSpec::Github { version, .. } | ToolSpec::Gitlab { version, .. } => version,
        }
    }

    pub fn provider(&self) -> Provider {
        match self {
            ToolSpec::Github { .. } => Provider::Github,
            ToolSpec::Gitlab { .. } => Provider::Gitlab,
        }
    }
}

impl fmt::Display for ToolSpec {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "{}.com/{}@{}",
            match self {
                ToolSpec::Github { .. } => "github",
                ToolSpec::Gitlab { .. } => "gitlab",
            },
            self.source(),
            self.version(),
        )
    }
}

#[derive(Debug, Serialize)]
pub struct ConfigFileTools(HashMap<String, ToolSpec>);

impl ConfigFileTools {
    pub fn new() -> ConfigFileTools {
        Self(HashMap::new())
    }
}

impl Deref for ConfigFileTools {
    type Target = HashMap<String, ToolSpec>;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl DerefMut for ConfigFileTools {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.0
    }
}

#[derive(Debug, Serialize, Deserialize)]
pub struct ConfigFile {
    pub tools: ConfigFileTools,
}

impl ConfigFile {
    pub fn new() -> Self {
        Self {
            tools: ConfigFileTools::new(),
        }
    }

    fn fill_from(&mut self, other: ConfigFile) {
        for (tool_name, tool_source) in other.tools.0 {
            self.tools.entry(tool_name).or_insert(tool_source);
        }
    }

    pub fn aggregate(paths: &ForemanPaths) -> Result<ConfigFile, ForemanError> {
        let mut config = ConfigFile::new();

        let base_dir = env::current_dir().map_err(|err| {
            ForemanError::io_error_with_context(
                err,
                "unable to obtain the current working directory",
            )
        })?;
        let mut current_dir = base_dir.as_path();

        loop {
            let config_path = current_dir.join("foreman.toml");

            if let Some(contents) = fs::try_read(&config_path)? {
                let config_source = toml::from_slice(&contents)
                    .map_err(|err| ForemanError::config_parsing(&config_path, err.to_string()))?;
                log::debug!(
                    "aggregating content from config file at {}",
                    config_path.display()
                );
                config.fill_from(config_source);
            }

            if let Some(parent) = current_dir.parent() {
                current_dir = parent;
            } else {
                break;
            }
        }

        let home_config_path = paths.user_config();
        if let Some(contents) = fs::try_read(&home_config_path)? {
            let config_source = toml::from_slice(&contents)
                .map_err(|err| ForemanError::config_parsing(&home_config_path, err.to_string()))?;
            log::debug!(
                "aggregating content from config file at {}",
                home_config_path.display()
            );
            config.fill_from(config_source);
        }

        Ok(config)
    }
}

impl fmt::Display for ConfigFile {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        writeln!(f, "Available Tools:")?;
        for (name, spec) in self.tools.iter() {
            writeln!(f, "\t {} => {}", name, spec)?;
        }
        Ok(())
    }
}

struct ConfigFileVisitor;

impl<'de> Visitor<'de> for ConfigFileVisitor {
    type Value = ConfigFileTools;

    fn expecting(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
        formatter.write_str("a map with non-duplicate keys")
    }

    fn visit_map<A>(self, mut map: A) -> Result<Self::Value, A::Error>
    where
        A: MapAccess<'de>,
    {
        let mut tools = HashMap::new();

        while let Some((key, value)) = map.next_entry()? {
            if tools.contains_key(&key) {
                // item already existed inside the config
                // throw an error as this is unlikely to be the users intention
                return Err(de::Error::custom(format!("duplicate tool `{key}`")));
            }

            tools.insert(key, value);
        }

        Ok(ConfigFileTools(tools))
    }
}

impl<'de> Deserialize<'de> for ConfigFileTools {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        let tools = deserializer.deserialize_map(ConfigFileVisitor)?;

        Ok(tools)
    }
}

#[cfg(test)]
mod test {
    use super::*;

    fn new_github<S: Into<String>>(github: S, version: VersionReq) -> ToolSpec {
        ToolSpec::Github {
            github: github.into(),
            version,
        }
    }

    fn new_gitlab<S: Into<String>>(github: S, version: VersionReq) -> ToolSpec {
        ToolSpec::Gitlab {
            gitlab: github.into(),
            version,
        }
    }

    fn version(string: &str) -> VersionReq {
        VersionReq::parse(string).unwrap()
    }

    mod deserialization {
        use super::*;

        #[test]
        fn github_from_source_field() {
            let github: ToolSpec =
                toml::from_str(&[r#"source = "user/repo""#, r#"version = "0.1.0""#].join("\n"))
                    .unwrap();
            assert_eq!(github, new_github("user/repo", version("0.1.0")));
        }

        #[test]
        fn github_from_github_field() {
            let github: ToolSpec =
                toml::from_str(&[r#"github = "user/repo""#, r#"version = "0.1.0""#].join("\n"))
                    .unwrap();
            assert_eq!(github, new_github("user/repo", version("0.1.0")));
        }

        #[test]
        fn gitlab_from_gitlab_field() {
            let gitlab: ToolSpec =
                toml::from_str(&[r#"gitlab = "user/repo""#, r#"version = "0.1.0""#].join("\n"))
                    .unwrap();
            assert_eq!(gitlab, new_gitlab("user/repo", version("0.1.0")));
        }

        #[test]
        fn duplicate_tools() {
            let err = toml::from_str::<ConfigFileTools>(
                r#"tool = { github = "user/repo", version = "0.1.0" }
			tool = { github = "user2/repo2", version = "0.2.0" }"#,
            )
            .unwrap_err();

            assert_eq!(err.to_string(), "duplicate tool `tool` at line 1 column 1");
        }
    }

    #[test]
    fn tool_cache_entry_is_backward_compatible() {
        let github = new_github("user/repo", version("7.0.0"));
        assert_eq!(github.cache_key(), "user/repo".into());
    }

    #[test]
    fn tool_cache_entry_is_different_for_github_and_gitlab_identical_projects() {
        let github = new_github("user/repo", version("7.0.0"));
        let gitlab = new_gitlab("user/repo", version("7.0.0"));
        assert_ne!(github.cache_key(), gitlab.cache_key());
    }
}
