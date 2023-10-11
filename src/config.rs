use crate::{
    ci_string::CiString,
    error::{ConfigFileParseError, ConfigFileParseResult, ForemanError},
    fs,
    paths::ForemanPaths,
    tool_provider::Provider,
};
use semver::VersionReq;
use std::{
    collections::{BTreeMap, HashMap},
    env, fmt,
};
use toml::Value;
use url::Url;

const GITHUB: &'static str = "https://github.com";
const GITLAB: &'static str = "https://gitlab.com";

#[derive(Debug, Clone, PartialEq)]
pub struct ToolSpec {
    host: Url,
    path: String,
    version: VersionReq,
    protocol: Protocol,
}
#[derive(Debug, Clone, PartialEq)]
pub enum Protocol {
    Github,
    Gitlab,
    Artifactory,
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

            // Extraneous fields in a tool spec definition should not be allowed
            if !map.is_empty() {
                return Err(ConfigFileParseError::Tool {
                    tool: value.to_string(),
                });
            }

            let host = host_source.source.to_owned();
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
            Protocol::Artifactory => CiString(format!("{}@{}", self.host, self.path)),
        }
    }

    pub fn source(&self) -> String {
        let provider = match self.protocol {
            Protocol::Github => "github.com",
            Protocol::Gitlab => "gitlab.com",
            Protocol::Artifactory => "artifactory.com",
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
            Protocol::Artifactory => Provider::Artifactory,
        }
    }

    pub fn host(&self) -> &Url {
        &self.host
    }
}

impl fmt::Display for ToolSpec {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}@{}", self.source(), self.version())
    }
}

#[derive(Debug, PartialEq)]
pub struct ConfigFile {
    pub tools: BTreeMap<String, ToolSpec>,
    pub hosts: HashMap<String, Host>,
}

#[derive(Debug, PartialEq)]
pub struct Host {
    source: Url,
    protocol: Protocol,
}

impl Host {
    pub fn new(source: Url, protocol: Protocol) -> Self {
        Self { source, protocol }
    }

    pub fn from_value(value: &Value) -> ConfigFileParseResult<Self> {
        if let Value::Table(mut map) = value.clone() {
            let source_string = map
                .remove("source")
                .ok_or_else(|| ConfigFileParseError::Host {
                    host: value.to_string(),
                })?
                .as_str()
                .ok_or_else(|| ConfigFileParseError::Host {
                    host: value.to_string(),
                })?
                .to_string();

            let source = Url::parse(&source_string).map_err(|_| ConfigFileParseError::Host {
                host: value.to_string(),
            })?;
            let protocol_value =
                map.remove("protocol")
                    .ok_or_else(|| ConfigFileParseError::Host {
                        host: value.to_string(),
                    })?;

            if !map.is_empty() {
                return Err(ConfigFileParseError::Host {
                    host: value.to_string(),
                });
            }

            let protocol_str =
                protocol_value
                    .as_str()
                    .ok_or_else(|| ConfigFileParseError::Host {
                        host: value.to_string(),
                    })?;

            let protocol = match protocol_str {
                "github" => Protocol::Github,
                "gitlab" => Protocol::Gitlab,
                "artifactory" => Protocol::Artifactory,
                _ => {
                    return Err(ConfigFileParseError::InvalidProtocol {
                        protocol: protocol_str.to_string(),
                    })
                }
            };

            Ok(Self { source, protocol })
        } else {
            Err(ConfigFileParseError::Host {
                host: value.to_string(),
            })
        }
    }
}

impl ConfigFile {
    pub fn new_with_defaults() -> Self {
        Self {
            tools: BTreeMap::new(),
            hosts: HashMap::from([
                (
                    "source".to_string(),
                    Host::new(Url::parse(GITHUB).unwrap(), Protocol::Github),
                ),
                (
                    "github".to_string(),
                    Host::new(Url::parse(GITHUB).unwrap(), Protocol::Github),
                ),
                (
                    "gitlab".to_string(),
                    Host::new(Url::parse(GITLAB).unwrap(), Protocol::Gitlab),
                ),
            ]),
        }
    }

    pub fn from_value(value: Value) -> ConfigFileParseResult<Self> {
        let mut config = ConfigFile::new_with_defaults();

        if let Value::Table(top_level) = &value {
            if let Some(hosts) = &top_level.get("hosts") {
                if let Value::Table(hosts) = hosts {
                    for (host, toml) in hosts {
                        let host_source =
                            Host::from_value(&toml).map_err(|_| ConfigFileParseError::Tool {
                                tool: value.to_string(),
                            })?;
                        config.hosts.insert(host.to_owned(), host_source);
                    }
                }
            }

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
    const ARTIFACTORY: &'static str = "https://artifactory.com";
    use super::*;

    fn new_github<S: Into<String>>(github: S, version: VersionReq) -> ToolSpec {
        ToolSpec {
            host: Url::parse(GITHUB).unwrap(),
            path: github.into(),
            version: version,
            protocol: Protocol::Github,
        }
    }

    fn new_gitlab<S: Into<String>>(gitlab: S, version: VersionReq) -> ToolSpec {
        ToolSpec {
            host: Url::parse(GITLAB).unwrap(),
            path: gitlab.into(),
            version: version,
            protocol: Protocol::Gitlab,
        }
    }

    fn new_artifactory<S: Into<String>>(host: S, path: S, version: VersionReq) -> ToolSpec {
        ToolSpec {
            host: Url::parse(host.into().as_str()).unwrap(),
            path: path.into(),
            version: version,
            protocol: Protocol::Artifactory,
        }
    }

    fn new_config(tools: BTreeMap<String, ToolSpec>, hosts: HashMap<String, Host>) -> ConfigFile {
        let mut config = ConfigFile::new_with_defaults();
        config.fill_from(ConfigFile { tools, hosts });
        config
    }

    fn version(string: &str) -> VersionReq {
        VersionReq::parse(string).unwrap()
    }

    fn new_host(source: Url, protocol: Protocol) -> Host {
        Host { source, protocol }
    }

    fn default_hosts() -> HashMap<String, Host> {
        HashMap::from([
            (
                "source".to_string(),
                Host::new(Url::parse(GITHUB).unwrap(), Protocol::Github),
            ),
            (
                "github".to_string(),
                Host::new(Url::parse(GITHUB).unwrap(), Protocol::Github),
            ),
            (
                "gitlab".to_string(),
                Host::new(Url::parse(GITLAB).unwrap(), Protocol::Gitlab),
            ),
        ])
    }

    fn artifactory_host() -> HashMap<String, Host> {
        let mut hosts = default_hosts();
        hosts.insert(
            "artifactory".to_string(),
            Host::new(Url::parse(ARTIFACTORY).unwrap(), Protocol::Artifactory),
        );
        hosts
    }

    mod deserialization {

        use super::*;

        #[test]
        fn github_from_source_field() {
            let value: Value =
                toml::from_str(&[r#"source = "user/repo""#, r#"version = "0.1.0""#].join("\n"))
                    .unwrap();
            let github = ToolSpec::from_value(&value, &default_hosts()).unwrap();

            dbg!("{github}");
            assert_eq!(github, new_github("user/repo", version("0.1.0")));
        }

        #[test]
        fn github_from_github_field() {
            let value: Value =
                toml::from_str(&[r#"github = "user/repo""#, r#"version = "0.1.0""#].join("\n"))
                    .unwrap();
            let github = ToolSpec::from_value(&value, &default_hosts()).unwrap();
            assert_eq!(github, new_github("user/repo", version("0.1.0")));
        }

        #[test]
        fn gitlab_from_gitlab_field() {
            let value: Value =
                toml::from_str(&[r#"gitlab = "user/repo""#, r#"version = "0.1.0""#].join("\n"))
                    .unwrap();
            let gitlab = ToolSpec::from_value(&value, &default_hosts()).unwrap();
            assert_eq!(gitlab, new_gitlab("user/repo", version("0.1.0")));
        }

        #[test]
        fn artifactory_from_artifactory_field() {
            let value: Value = toml::from_str(
                &[
                    r#"artifactory = "generic-rbx-local-tools/rotriever/""#,
                    r#"version = "0.5.4""#,
                ]
                .join("\n"),
            )
            .unwrap();

            let artifactory = ToolSpec::from_value(&value, &artifactory_host()).unwrap();
            assert_eq!(
                artifactory,
                new_artifactory(
                    "https://artifactory.com",
                    "generic-rbx-local-tools/rotriever/",
                    version("0.5.4")
                )
            );
        }

        #[test]
        fn host_artifactory() {
            let value: Value = toml::from_str(
                &[
                    r#"source = "https://artifactory.com""#,
                    r#"protocol = "artifactory""#,
                ]
                .join("\n"),
            )
            .unwrap();

            let host = Host::from_value(&value).unwrap();
            assert_eq!(
                host,
                new_host(
                    Url::parse("https://artifactory.com").unwrap(),
                    Protocol::Artifactory
                )
            )
        }

        #[test]
        fn extraneous_fields_tools() {
            let value: Value = toml::from_str(
                &[
                    r#"rbx_artifactory = "generic-rbx-local-tools/rotriever/""#,
                    r#"path = "generic-rbx-local-tools/rotriever/""#,
                    r#"version = "0.5.4""#,
                ]
                .join("\n"),
            )
            .unwrap();

            let artifactory = ToolSpec::from_value(&value, &artifactory_host()).unwrap_err();
            assert_eq!(
                artifactory,
                ConfigFileParseError::Tool {
                    tool: [
                        r#"path = "generic-rbx-local-tools/rotriever/""#,
                        r#"rbx_artifactory = "generic-rbx-local-tools/rotriever/""#,
                        r#"version = "0.5.4""#,
                        r#""#,
                    ]
                    .join("\n")
                    .to_string()
                }
            )
        }

        #[test]
        fn extraneous_fields_host() {
            let value: Value = toml::from_str(
                &[
                    r#"source = "https://artifactory.com""#,
                    r#"protocol = "artifactory""#,
                    r#"extra = "field""#,
                ]
                .join("\n"),
            )
            .unwrap();

            let err = Host::from_value(&value).unwrap_err();
            assert_eq!(
                err,
                ConfigFileParseError::Host {
                    host: [
                        r#"extra = "field""#,
                        r#"protocol = "artifactory""#,
                        r#"source = "https://artifactory.com""#,
                        r#""#,
                    ]
                    .join("\n")
                    .to_string()
                }
            )
        }
        #[test]
        fn config_file_with_hosts() {
            let value: Value = toml::from_str(&[
                r#"[hosts]"#,
                r#"artifactory = {source = "https://artifactory.com", protocol = "artifactory"}"#,
                r#""#,
                r#"[tools]"#,
                r#"tool = {artifactory = "path/to/tool", version = "1.0.0"}"#,
            ].join("\n"))
            .unwrap();

            let config = ConfigFile::from_value(value).unwrap();
            assert_eq!(
                config,
                new_config(
                    BTreeMap::from([(
                        "tool".to_string(),
                        ToolSpec {
                            host: Url::parse("https://artifactory.com").unwrap(),
                            path: "path/to/tool".to_string(),
                            version: VersionReq::parse("1.0.0").unwrap(),
                            protocol: Protocol::Artifactory
                        }
                    )]),
                    HashMap::from([(
                        "artifactory".to_string(),
                        Host {
                            source: Url::parse("https://artifactory.com").unwrap(),
                            protocol: Protocol::Artifactory
                        }
                    )])
                )
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
