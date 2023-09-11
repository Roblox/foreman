use crate::{
    ci_string::CiString,
    error::{ConfigFileParseError, ConfigFileParseResult, ForemanError},
    fs,
    paths::ForemanPaths,
    tool_provider::Provider,
};
use semver::VersionReq;
use serde::{Deserialize, Serialize};
use std::{
    collections::{BTreeMap, HashMap},
    env, fmt,
};
use toml::Value;
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ToolSpec {
    host: String,
    path: String,
    version: VersionReq,
    protocol: Protocol,
}
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum Protocol {
    Github,
    Gitlab,
}

impl ToolSpec {
    pub fn from_value(
        value: &Value,
        host_map: &HashMap<String, Host>,
    ) -> ConfigFileParseResult<Self> {
        if let Value::Table(mut map) = value.clone() {
            let version_value =
                map.remove("version")
                    .ok_or_else(|| ConfigFileParseError::Tool {
                        tool: value.to_string(),
                    })?;
            let version_str = version_value
                .as_str()
                .ok_or_else(|| ConfigFileParseError::Tool {
                    tool: value.to_string(),
                })?;
            let version =
                VersionReq::parse(version_str).map_err(|_| ConfigFileParseError::Tool {
                    tool: value.to_string(),
                })?;

            let (path_val, host_source) = host_map
                .iter()
                .find_map(|(potential_host, host_source)| {
                    if let Some(path) = map.remove(potential_host) {
                        Some((path, host_source))
                    } else {
                        None
                    }
                })
                .ok_or_else(|| ConfigFileParseError::Tool {
                    tool: value.to_string(),
                })?;

            // Extraneous fields should in a tool spec definition should not be allowed
            if !map.is_empty() {
                return Err(ConfigFileParseError::Tool {
                    tool: value.to_string(),
                });
            }

            let host = host_source.source.to_string();
            let path = path_val
                .as_str()
                .ok_or_else(|| ConfigFileParseError::Tool {
                    tool: value.to_string(),
                })?
                .to_string();

            let protocol = host_source.protocol.clone();

            Ok(Self {
                host,
                path,
                version,
                protocol,
            })
        } else {
            Err(ConfigFileParseError::Tool {
                tool: value.to_string(),
            })
        }
    }

    pub fn cache_key(&self) -> CiString {
        match self.protocol {
            Protocol::Github => CiString(format!("{}", self.path)),
            Protocol::Gitlab => CiString(format!("gitlab@{}", self.path)),
        }
    }

    pub fn source(&self) -> String {
        let provider = match self.protocol {
            Protocol::Github => "github.com",
            Protocol::Gitlab => "gitlab.com",
        };

        format!("{}/{}", provider, self.path)
    }

    pub fn path(&self) -> &str {
        self.path.as_str()
    }

    pub fn version(&self) -> &VersionReq {
        &self.version
    }

    pub fn provider(&self) -> Provider {
        match self.protocol {
            Protocol::Github => Provider::Github,
            Protocol::Gitlab => Provider::Gitlab,
        }
    }
}

impl fmt::Display for ToolSpec {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}@{}", self.source(), self.version())
    }
}

#[derive(Debug, Serialize, Deserialize)]
pub struct ConfigFile {
    pub tools: BTreeMap<String, ToolSpec>,
    pub hosts: HashMap<String, Host>,
}

#[derive(Debug, Serialize, Deserialize, PartialEq)]
pub struct Host {
    source: String,
    protocol: Protocol,
}

impl Host {
    pub fn new(source: String, protocol: Protocol) -> Self {
        Self { source, protocol }
    }
}

impl ConfigFile {
    pub fn new_with_defaults() -> Self {
        Self {
            tools: BTreeMap::new(),
            hosts: HashMap::from([
                (
                    "source".to_string(),
                    Host::new("https://github.com".to_string(), Protocol::Github),
                ),
                (
                    "github".to_string(),
                    Host::new("https://github.com".to_string(), Protocol::Github),
                ),
                (
                    "gitlab".to_string(),
                    Host::new("https://gitlab.com".to_string(), Protocol::Gitlab),
                ),
            ]),
        }
    }

    pub fn from_value(value: Value) -> ConfigFileParseResult<Self> {
        let mut config = ConfigFile::new_with_defaults();

        if let Value::Table(top_level) = &value {
            if let Some(tools) = &top_level.get("tools") {
                if let Value::Table(tools) = tools {
                    for (tool, toml) in tools {
                        let tool_spec =
                            ToolSpec::from_value(&toml, &config.hosts).map_err(|_| {
                                ConfigFileParseError::Tool {
                                    tool: value.to_string(),
                                }
                            })?;
                        config.tools.insert(tool.to_owned(), tool_spec);
                    }
                }
            } else {
                return Err(ConfigFileParseError::MissingField {
                    field: "tools".to_string(),
                });
            }
            Ok(config)
        } else {
            Err(ConfigFileParseError::Tool {
                tool: value.to_string(),
            })
        }
    }

    fn fill_from(&mut self, other: ConfigFile) {
        for (tool_name, tool_source) in other.tools {
            self.tools.entry(tool_name).or_insert(tool_source);
        }

        for (host_name, host_source) in other.hosts {
            self.hosts.entry(host_name).or_insert(host_source);
        }
    }

    pub fn aggregate(paths: &ForemanPaths) -> Result<ConfigFile, ForemanError> {
        let mut config = ConfigFile::new_with_defaults();

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
                let new_config = ConfigFile::from_value(config_source)
                    .map_err(|err| ForemanError::config_parsing(&config_path, err.to_string()))?;
                config.fill_from(new_config);
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
            let new_config = ConfigFile::from_value(config_source)
                .map_err(|err| ForemanError::config_parsing(&home_config_path, err.to_string()))?;
            log::debug!(
                "aggregating content from config file at {}",
                home_config_path.display()
            );
            config.fill_from(new_config);
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

#[cfg(test)]
mod test {
    use super::*;

    fn new_github<S: Into<String>>(github: S, version: VersionReq) -> ToolSpec {
        ToolSpec {
            host: "https://github.com".to_string(),
            path: github.into(),
            version: version,
            protocol: Protocol::Github,
        }
    }

    fn new_gitlab<S: Into<String>>(gitlab: S, version: VersionReq) -> ToolSpec {
        ToolSpec {
            host: "https://gitlab.com".to_string(),
            path: gitlab.into(),
            version: version,
            protocol: Protocol::Gitlab,
        }
    }

    fn version(string: &str) -> VersionReq {
        VersionReq::parse(string).unwrap()
    }

    fn default_host() -> HashMap<String, Host> {
        HashMap::from([
            (
                "source".to_string(),
                Host::new("https://github.com".to_string(), Protocol::Github),
            ),
            (
                "github".to_string(),
                Host::new("https://github.com".to_string(), Protocol::Github),
            ),
            (
                "gitlab".to_string(),
                Host::new("https://gitlab.com".to_string(), Protocol::Gitlab),
            ),
        ])
    }

    mod deserialization {
        use super::*;

        #[test]
        fn github_from_source_field() {
            let value: Value =
                toml::from_str(&[r#"source = "user/repo""#, r#"version = "0.1.0""#].join("\n"))
                    .unwrap();
            let github = ToolSpec::from_value(&value, &default_host()).unwrap();

            dbg!("{github}");
            assert_eq!(github, new_github("user/repo", version("0.1.0")));
        }

        #[test]
        fn github_from_github_field() {
            let value: Value =
                toml::from_str(&[r#"github = "user/repo""#, r#"version = "0.1.0""#].join("\n"))
                    .unwrap();
            let github = ToolSpec::from_value(&value, &default_host()).unwrap();
            assert_eq!(github, new_github("user/repo", version("0.1.0")));
        }

        #[test]
        fn gitlab_from_gitlab_field() {
            let value: Value =
                toml::from_str(&[r#"gitlab = "user/repo""#, r#"version = "0.1.0""#].join("\n"))
                    .unwrap();
            let gitlab = ToolSpec::from_value(&value, &default_host()).unwrap();
            assert_eq!(gitlab, new_gitlab("user/repo", version("0.1.0")));
        }

        #[test]
        fn extraneous_fields_tools() {
            let value: Value = toml::from_str(
                &[
                    r#"github = "Roblox/rotriever""#,
                    r#"path = "Roblox/rotriever""#,
                    r#"version = "0.5.4""#,
                ]
                .join("\n"),
            )
            .unwrap();

            let artifactory = ToolSpec::from_value(&value, &default_host()).unwrap_err();
            assert_eq!(
                artifactory,
                ConfigFileParseError::Tool {
                    tool: [
                        r#"github = "Roblox/rotriever""#,
                        r#"path = "Roblox/rotriever""#,
                        r#"version = "0.5.4""#,
                        r#""#,
                    ]
                    .join("\n")
                    .to_string()
                }
            )
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
