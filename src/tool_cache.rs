use std::{
    collections::{BTreeSet, HashMap},
    env::consts::EXE_SUFFIX,
    io::{self, BufWriter, Cursor},
    path::PathBuf,
    process,
};

use semver::{Version, VersionReq};
use serde::{Deserialize, Serialize};
use zip::ZipArchive;

use crate::{
    artifact_choosing::platform_keywords,
    ci_string::CiString,
    fs::{self, File},
    github, paths,
};

fn index_file() -> PathBuf {
    let mut path = paths::base_dir();
    path.push("tool-cache.json");
    path
}

/// Contains the current state of all of the tools that Foreman manages.
#[derive(Debug, Default, Serialize, Deserialize)]
pub struct ToolCache {
    pub tools: HashMap<CiString, ToolEntry>,
}

impl ToolCache {
    pub fn run(source: &str, version: &Version, args: Vec<String>) -> i32 {
        log::debug!("Running tool {}@{}", source, version);

        let mut tool_path = paths::tools_dir();
        let exe_name = tool_identifier_to_exe_name(source, version);
        tool_path.push(exe_name);

        let status = process::Command::new(tool_path)
            .args(args)
            .status()
            .unwrap();

        status.code().unwrap_or(1)
    }

    pub fn download_if_necessary(source: &str, version_req: &VersionReq) -> Option<Version> {
        let cache = Self::load().unwrap();

        if let Some(tool) = cache.tools.get(&CiString(source.to_owned())) {
            log::debug!("Tool has some versions installed");

            let matching_version = tool
                .versions
                .iter()
                .rev()
                .find(|version| version_req.matches(version));

            if let Some(version) = matching_version {
                return Some(version.clone());
            }
        }

        Self::download(source, version_req)
    }

    pub fn download(source: &str, version_req: &VersionReq) -> Option<Version> {
        log::info!("Downloading {}@{}", source, version_req);

        let releases = github::get_releases(source).unwrap();

        // Filter down our set of releases to those that are valid versions and
        // have release assets for our current platform.
        let mut semver_releases: Vec<_> = releases
            .into_iter()
            .filter_map(|release| {
                log::trace!("Evaluating tag {}", release.tag_name);

                let version = Version::parse(&release.tag_name).ok().or_else(|| {
                    if !release.tag_name.starts_with('v') {
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

        let matching_release = semver_releases
            .into_iter()
            .find(|(version, _asset_index, _release)| version_req.matches(version));

        if let Some((version, asset_index, release)) = matching_release {
            log::trace!("Picked version {}", version);

            let url = &release.assets[asset_index].url;
            let mut buffer = Vec::new();
            github::download_asset(url, &mut buffer).unwrap();

            log::trace!("Extracting downloaded artifact");
            let mut archive = ZipArchive::new(Cursor::new(&buffer)).unwrap();
            let mut file = archive.by_index(0).unwrap();

            let mut tool_path = paths::tools_dir();
            let exe_name = tool_identifier_to_exe_name(source, &version);
            tool_path.push(exe_name);

            let mut output = BufWriter::new(File::create(&tool_path).unwrap());
            io::copy(&mut file, &mut output).unwrap();

            // On Unix systems, mark the tool as executable.
            #[cfg(unix)]
            {
                use std::os::unix::fs::PermissionsExt;

                fs::set_permissions(&tool_path, fs::Permissions::from_mode(0o777)).unwrap();
            }

            log::trace!("Updating tool cache");
            let mut cache = Self::load().unwrap();
            let tool = cache.tools.entry(CiString(source.to_owned())).or_default();
            tool.versions.insert(version.clone());
            cache.save().unwrap();

            Some(version)
        } else {
            log::error!(
                "No compatible version of {} was found for version requirement {}",
                source,
                version_req
            );

            None
        }
    }

    pub fn load() -> io::Result<Self> {
        match fs::read(index_file()) {
            Ok(contents) => Ok(serde_json::from_slice(&contents).unwrap()),
            Err(err) => {
                if err.kind() == io::ErrorKind::NotFound {
                    Ok(Default::default())
                } else {
                    Err(err)
                }
            }
        }
    }

    fn save(&self) -> io::Result<()> {
        let serialized = serde_json::to_string_pretty(self).unwrap();
        fs::write(index_file(), serialized)
    }
}

#[derive(Debug, Default, Serialize, Deserialize)]
pub struct ToolEntry {
    pub versions: BTreeSet<Version>,
}

fn tool_identifier_to_exe_name(source: &str, version: &Version) -> String {
    let mut name = format!("{}-{}{}", source, version, EXE_SUFFIX);
    name = name.replace('/', "__");
    name.replace('\\', "__")
}
