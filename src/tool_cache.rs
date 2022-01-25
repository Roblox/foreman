use std::{
    collections::{BTreeSet, HashMap},
    env::consts::EXE_SUFFIX,
    io::Cursor,
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
    error::{ForemanError, ForemanResult},
    fs,
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
    pub fn run(&self, tool: &ToolSpec, version: &Version, args: Vec<String>) -> ForemanResult<i32> {
        let tool_path = self.get_tool_exe_path(tool, version);

        log::debug!("Running tool {} ({})", tool, tool_path.display());

        let status = process::Command::new(&tool_path)
            .args(args)
            .status()
            .map_err(|err| {
                ForemanError::io_error_with_context(err,
                    format!(
                        "an error happened trying to run `{}` at `{}` (this is an error in Foreman)",
                        tool,
                        tool_path.display()
                    )
                )
            })?;

        Ok(status.code().unwrap_or(1))
    }

    pub fn download_if_necessary(
        &mut self,
        tool: &ToolSpec,
        providers: &ToolProvider,
    ) -> ForemanResult<Version> {
        if let Some(tool_entry) = self.tools.get(&tool.cache_key()) {
            log::debug!("Tool has some versions installed");

            let matching_version = tool_entry
                .versions
                .iter()
                .rev()
                .find(|version| tool.version().matches(version));

            if let Some(version) = matching_version {
                return Ok(version.clone());
            }
        }

        self.download(tool, providers)
    }

    pub fn download(
        &mut self,
        tool: &ToolSpec,
        providers: &ToolProvider,
    ) -> ForemanResult<Version> {
        log::info!("Downloading {}", tool);

        let provider = providers.get(&tool.provider());
        let releases = provider.get_releases(tool.source())?;

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
            .iter()
            .find(|(version, _asset_index, _release)| version_req.matches(version));

        if let Some((version, asset_index, release)) = matching_release {
            log::trace!("Picked version {}", version);

            let url = &release.assets[*asset_index].url;
            let buffer = provider.download_asset(url)?;

            log::trace!("Extracting downloaded artifact");
            let mut archive = ZipArchive::new(Cursor::new(&buffer)).map_err(|err| {
                ForemanError::invalid_release_asset(
                    tool,
                    version,
                    format!("unable to open zip archive ({})", err),
                )
            })?;
            let mut file = archive.by_index(0).map_err(|err| {
                ForemanError::invalid_release_asset(
                    tool,
                    version,
                    format!("unable to obtain file from zip archive ({})", err),
                )
            })?;

            let tool_path = self.get_tool_exe_path(tool, version);

            fs::copy_from_reader(&mut file, &tool_path)?;

            // On Unix systems, mark the tool as executable.
            #[cfg(unix)]
            {
                use std::os::unix::fs::PermissionsExt;

                fs::set_permissions(&tool_path, fs::Permissions::from_mode(0o777))?;
            }

            log::trace!("Updating tool cache");
            let tool_entry = self.tools.entry(tool.cache_key()).or_default();
            tool_entry.versions.insert(version.clone());
            self.save()?;

            Ok(version.clone())
        } else {
            Err(ForemanError::no_compatible_version_found(
                tool,
                semver_releases
                    .into_iter()
                    .map(|(version, _asset_index, _release)| version)
                    .collect(),
            ))
        }
    }

    pub fn load(paths: &ForemanPaths) -> ForemanResult<Self> {
        let mut tool_cache = fs::try_read(paths.index_file())?
            .map(|contents| {
                serde_json::from_slice(&contents).map_err(|err| {
                    ForemanError::tool_cache_parsing(paths.index_file(), err.to_string())
                })
            })
            .unwrap_or_else(|| Ok(Self::default()))?;

        tool_cache.paths = paths.clone();
        Ok(tool_cache)
    }

    fn save(&self) -> ForemanResult<()> {
        let serialized =
            serde_json::to_string_pretty(self).expect("unable to serialize tool cache");
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
