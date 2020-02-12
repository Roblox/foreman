use std::{collections::HashMap, env, fs, io};

use semver::VersionReq;
use serde::{Deserialize, Serialize};

use crate::paths;

#[derive(Debug, Serialize, Deserialize)]
pub struct ConfigFile {
    pub tools: HashMap<String, ToolSpec>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct ToolSpec {
    pub source: String,
    pub version: VersionReq,
}

impl ConfigFile {
    pub fn new() -> Self {
        Self {
            tools: HashMap::new(),
        }
    }

    fn fill_from(&mut self, other: ConfigFile) {
        for (tool_name, tool_source) in other.tools {
            if !self.tools.contains_key(&tool_name) {
                self.tools.insert(tool_name, tool_source);
            }
        }
    }

    pub fn aggregate() -> io::Result<ConfigFile> {
        let mut config = ConfigFile::new();

        let base_dir = env::current_dir()?;
        let mut current_dir = base_dir.as_path();

        loop {
            let config_path = current_dir.join("foreman.toml");

            match fs::read(&config_path) {
                Ok(contents) => {
                    let config_source = toml::from_slice(&contents).unwrap();
                    config.fill_from(config_source);
                }
                Err(err) => {
                    if err.kind() != io::ErrorKind::NotFound {
                        return Err(err);
                    }
                }
            }

            if let Some(parent) = current_dir.parent() {
                current_dir = parent;
            } else {
                break;
            }
        }

        let home_config_path = paths::user_config();
        match fs::read(&home_config_path) {
            Ok(contents) => {
                let config_source = toml::from_slice(&contents).unwrap();
                config.fill_from(config_source);
            }
            Err(err) => {
                if err.kind() != io::ErrorKind::NotFound {
                    return Err(err);
                }
            }
        }

        Ok(config)
    }
}
