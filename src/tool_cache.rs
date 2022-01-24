use std::{
    collections::{BTreeSet, HashMap},
    env::consts::EXE_SUFFIX,
    io::{self, BufWriter, Cursor},
    path::PathBuf,
    process,
};

use semver::Version;
use serde::{Deserialize, Serialize};
use zip::ZipArchive;

use crate::{
    artifact_choosing::platform_keywords,
    ci_string::CiString,
    config::ToolSpec,
    fs::{self, File},
    paths::ForemanPaths,
    tool_provider::ToolProvider,
};

/// Contains the current state of all of the tools that Foreman manages.
#[derive(Debug, Default, Serialize, Deserialize)]
pub struct ToolCache {
    pub tools: HashMap<CiString, ToolEntry>,
    #[serde(skip)]
    paths: ForemanPaths,
}

impl ToolCache {
    #[must_use]
    pub fn run(&self, tool: &ToolSpec, version: &Version, args: Vec<String>) -> i32 {
        log::debug!("Running tool {}", tool);

        let tool_path = self.get_tool_exe_path(tool, version);

        let status = process::Command::new(&tool_path)
            .args(args)
            .status()
            .map_err(|e| {
                format!(
                    "an error happened trying to run `{}` at `{}`: {}\n\nThis is an error in Foreman.",
                    tool,
                    tool_path.display(),
                    e
                )
            })
            .unwrap();

        status.code().unwrap_or(1)
    }

    pub fn download_if_necessary(
        &mut self,
        tool: &ToolSpec,
        providers: &ToolProvider,
    ) -> Option<Version> {
        if let Some(tool_entry) = self.tools.get(&tool.cache_key()) {
            log::debug!("Tool has some versions installed");

            let matching_version = tool_entry
                .versions
                .iter()
                .rev()
                .find(|version| tool.version().matches(version));

            if let Some(version) = matching_version {
                return Some(version.clone());
            }
        }

        self.download(tool, providers)
    }

    pub fn download(&mut self, tool: &ToolSpec, providers: &ToolProvider) -> Option<Version> {
        log::info!("Downloading {}", tool);

        let provider = providers.get(&tool.provider());
        let releases = provider.get_releases(tool.source()).unwrap();

        // Filter down our set of releases to those that are valid versions and
        // have release assets for our current platform.
        let mut semver_releases: Vec<_> = releases
            .into_iter()
            .filter_map(|release| {
                log::trace!("Evaluating tag {}", release.tag_name);

                let version = Version::parse(&release.tag_name).ok().or_else(|| {
                    if !release.tag_name.starts_with('v') {
                        log::debug!(
                            "Release tag name did not start with 'v'! {}",
                            release.tag_name
                        );
                        return None;
                    }

                    Version::parse(&release.tag_name[1..]).ok()
                })?;

                let asset_index = release.assets.iter().position(|asset| {
                    platform_keywords()
                        .iter()
                        .any(|keyword| asset.name.contains(keyword))
                })?;

                Some((version, asset_index, release))
            })
            .collect();

        // Releases should come back chronological, but we want strictly
        // descending version numbers.
        semver_releases.sort_by(|a, b| b.0.cmp(&a.0));

        let version_req = tool.version();
        let matching_release = semver_releases
            .into_iter()
            .find(|(version, _asset_index, _release)| version_req.matches(version));

        if let Some((version, asset_index, release)) = matching_release {
            log::trace!("Picked version {}", version);

            let url = &release.assets[asset_index].url;
            let buffer = provider.download_asset(url).unwrap();

            log::trace!("Extracting downloaded artifact");
            let mut archive = ZipArchive::new(Cursor::new(&buffer)).unwrap();
            let mut file = archive.by_index(0).unwrap();

            let tool_path = self.get_tool_exe_path(tool, &version);

            let mut output = BufWriter::new(File::create(&tool_path).unwrap());
            io::copy(&mut file, &mut output).unwrap();

            // On Unix systems, mark the tool as executable.
            #[cfg(unix)]
            {
                use std::os::unix::fs::PermissionsExt;

                fs::set_permissions(&tool_path, fs::Permissions::from_mode(0o777)).unwrap();
            }

            log::trace!("Updating tool cache");
            let tool_entry = self.tools.entry(tool.cache_key()).or_default();
            tool_entry.versions.insert(version.clone());
            self.save().unwrap();

            Some(version)
        } else {
            log::error!(
                "No compatible version of {} was found for version requirement {}",
                tool.source(),
                version_req
            );

            None
        }
    }

    pub fn load(paths: &ForemanPaths) -> io::Result<Self> {
        match fs::read(paths.index_file()) {
            Ok(contents) => Ok(serde_json::from_slice(&contents).unwrap()),
            Err(err) => {
                if err.kind() == io::ErrorKind::NotFound {
                    Ok(Default::default())
                } else {
                    Err(err)
                }
            }
        }
        .map(|mut tool_cache: ToolCache| {
            tool_cache.paths = paths.clone();
            tool_cache
        })
    }

    fn save(&self) -> io::Result<()> {
        let serialized = serde_json::to_string_pretty(self).unwrap();
        fs::write(self.index_file(), serialized)
    }

    fn get_tool_exe_path(&self, tool: &ToolSpec, version: &Version) -> PathBuf {
        let mut tool_path = self.paths.tools_dir();
        let exe_name = tool_identifier_to_exe_name(tool, version);
        tool_path.push(exe_name);
        tool_path
    }

    fn index_file(&self) -> PathBuf {
        let mut path = self.paths.root_dir();
        path.push("tool-cache.json");
        path
    }
}

#[derive(Debug, Default, Serialize, Deserialize)]
pub struct ToolEntry {
    pub versions: BTreeSet<Version>,
}

fn tool_identifier_to_exe_name(tool: &ToolSpec, version: &Version) -> String {
    let mut name = format!("{}-{}{}", tool.cache_key().0, version, EXE_SUFFIX);
    name = name.replace('/', "__");
    name.replace('\\', "__")
}
