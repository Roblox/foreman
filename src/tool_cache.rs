use std::{
    collections::{BTreeSet, HashMap},
    env::consts::EXE_SUFFIX,
    io::Cursor,
    path::PathBuf,
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
    process,
    tool_provider::{Release, ToolProvider},
};

fn choose_asset(release: &Release, platform_keywords: &[&str]) -> Option<usize> {
    log::trace!(
        "Checking for name with compatible os/arch pair from platform-derived list: {:?}",
        platform_keywords
    );
    let asset_index = platform_keywords.iter().find_map(|keyword| {
        release
            .assets
            .iter()
            .position(|asset| asset.name.contains(keyword))
    })?;

    log::debug!(
        "Found matching artifact: {}",
        release.assets[asset_index].name
    );
    Some(asset_index)
}

/// Contains the current state of all of the tools that Foreman manages.
#[derive(Debug, PartialEq, Serialize, Deserialize)]
pub struct ToolCache {
    pub tools: HashMap<CiString, ToolEntry>,
    #[serde(skip)]
    paths: ForemanPaths,
}

impl ToolCache {
    pub fn new(paths: &ForemanPaths) -> Self {
        Self {
            tools: Default::default(),
            paths: paths.clone(),
        }
    }

    pub fn run(&self, tool: &ToolSpec, version: &Version, args: Vec<String>) -> ForemanResult<i32> {
        let tool_path = self.get_tool_exe_path(tool, version);

        log::debug!("Running tool {} ({})", tool, tool_path.display());

        let code = process::run(&tool_path, args).map_err(|err| {
            ForemanError::io_error_with_context(
                err,
                format!(
                    "an error happened trying to run `{}` at `{}` (this is an error in Foreman)",
                    tool,
                    tool_path.display()
                ),
            )
        })?;
        Ok(code)
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
        let releases = provider.get_releases(tool.path(), tool.host())?;

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

                let asset_index = choose_asset(&release, platform_keywords())?;

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
            let file_names: Vec<String> = archive.file_names().map(str::to_owned).collect();
            let file_index = choose_zip_file(&file_names, tool.path());
            let mut file = archive.by_index(file_index).map_err(|err| {
                ForemanError::invalid_release_asset(
                    tool,
                    version,
                    format!("unable to obtain file from zip archive ({})", err),
                )
            })?;
            log::trace!("Picked artifact file {}", file.name());

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
        let path = paths.index_file();
        log::debug!("load tool cache from {}", path.display());

        let mut tool_cache = fs::try_read(&path)?
            .map(|contents| {
                serde_json::from_slice(&contents)
                    .map_err(|err| ForemanError::tool_cache_parsing(&path, err.to_string()))
            })
            .unwrap_or_else(|| Ok(Self::new(paths)))?;

        tool_cache.paths = paths.clone();
        Ok(tool_cache)
    }

    fn save(&self) -> ForemanResult<()> {
        let serialized =
            serde_json::to_string_pretty(self).expect("unable to serialize tool cache");
        fs::write(self.paths.index_file(), serialized)
    }

    fn get_tool_exe_path(&self, tool: &ToolSpec, version: &Version) -> PathBuf {
        let mut tool_path = self.paths.tools_dir();
        let exe_name = tool_identifier_to_exe_name(tool, version);
        tool_path.push(exe_name);
        tool_path
    }
}

#[derive(Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct ToolEntry {
    pub versions: BTreeSet<Version>,
}

/// Choose the file in the release archive that is most likely the tool
/// executable: releases may also contain files like licenses or readmes,
/// and the executable is not necessarily the first entry (#97).
fn choose_zip_file(file_names: &[String], tool_path: &str) -> usize {
    let base_name = |name: &str| {
        name.trim_end_matches('/')
            .rsplit(['/', '\\'])
            .next()
            .unwrap_or(name)
            .to_lowercase()
    };
    let is_dir = |name: &str| name.ends_with('/');

    // an entry named after the tool wins, e.g. `asphalt` or `asphalt.exe`
    // for the tool `jackTabsCode/asphalt`
    let tool_name = base_name(tool_path.rsplit(['/', '\\']).next().unwrap_or(tool_path));
    if let Some(index) = file_names.iter().position(|name| {
        !is_dir(name) && {
            let base = base_name(name);
            base == tool_name || base == format!("{}{}", tool_name, EXE_SUFFIX)
        }
    }) {
        return index;
    }

    // otherwise, the first entry that does not look like documentation
    let looks_like_documentation = |name: &str| {
        let base = base_name(name);
        [
            "license",
            "licence",
            "copying",
            "notice",
            "readme",
            "changelog",
        ]
        .iter()
        .any(|prefix| base.starts_with(prefix))
            || [".md", ".txt", ".rst", ".html", ".pdf"]
                .iter()
                .any(|extension| base.ends_with(extension))
    };
    if let Some(index) = file_names
        .iter()
        .position(|name| !is_dir(name) && !looks_like_documentation(name))
    {
        return index;
    }

    // fall back to the previous behavior
    0
}

fn tool_identifier_to_exe_name(tool: &ToolSpec, version: &Version) -> String {
    let mut name = format!("{}-{}{}", tool.cache_key().0, version, EXE_SUFFIX);
    name = name.replace('/', "__");
    name.replace('\\', "__")
}

#[cfg(test)]
mod test {
    use tempfile::tempdir;

    use crate::tool_provider::ReleaseAsset;

    use super::*;

    // Regression test for #97: the executable is not necessarily the first
    // entry of the release archive
    #[test]
    fn choose_zip_file_prefers_the_tool_name() {
        let names = |names: &[&str]| names.iter().map(|s| s.to_string()).collect::<Vec<_>>();

        // the release that surfaced #97 contained a license before the binary
        let asphalt = names(&["LICENSE.md", "README.md", "asphalt"]);
        assert_eq!(choose_zip_file(&asphalt, "jackTabsCode/asphalt"), 2);

        // Windows executables carry the .exe suffix
        #[cfg(windows)]
        {
            let windows = names(&["LICENSE", "asphalt.exe"]);
            assert_eq!(choose_zip_file(&windows, "jackTabsCode/asphalt"), 1);
        }

        // matching is case-insensitive and ignores directories in the archive
        let nested = names(&["asphalt-1.0/", "asphalt-1.0/LICENSE", "asphalt-1.0/Asphalt"]);
        assert_eq!(choose_zip_file(&nested, "jackTabsCode/asphalt"), 2);

        // without a name match, documentation files are skipped
        let renamed = names(&["LICENSE.txt", "readme.md", "tool-x86_64-linux"]);
        assert_eq!(choose_zip_file(&renamed, "user/tool"), 2);

        // single-entry archives keep working like before
        let single = names(&["rojo"]);
        assert_eq!(choose_zip_file(&single, "rojo-rbx/rojo"), 0);

        // if everything looks like documentation, fall back to the first entry
        let odd = names(&["README.md", "LICENSE"]);
        assert_eq!(choose_zip_file(&odd, "user/tool"), 0);
    }

    // Regression test for LUAFDN-1041, based on the release that surfaced it
    #[test]
    fn select_correct_asset() {
        let release = Release {
            prerelease: false,
            tag_name: "v0.5.2".to_string(),
            assets: vec![
                ReleaseAsset {
                    name: "tool-linux.zip".to_string(),
                    url: "https://example.com/some/repo/releases/assets/1".to_string(),
                },
                ReleaseAsset {
                    name: "tool-macos-arm64.zip".to_string(),
                    url: "https://example.com/some/repo/releases/assets/2".to_string(),
                },
                ReleaseAsset {
                    name: "tool-macos-x86_64.zip".to_string(),
                    url: "https://example.com/some/repo/releases/assets/3".to_string(),
                },
                ReleaseAsset {
                    name: "tool-win64.zip".to_string(),
                    url: "https://example.com/some/repo/releases/assets/4".to_string(),
                },
            ],
        };
        assert_eq!(
            choose_asset(&release, &["win32", "win64", "windows"]),
            Some(3)
        );
        assert_eq!(
            choose_asset(
                &release,
                &["macos-x86_64", "darwin-x86_64", "macos", "darwin"]
            ),
            Some(2)
        );
        assert_eq!(
            choose_asset(
                &release,
                &[
                    "macos-arm64",
                    "darwin-arm64",
                    "macos-x86_64",
                    "darwin-x86_64",
                    "macos",
                    "darwin",
                ]
            ),
            Some(1)
        );
        assert_eq!(choose_asset(&release, &["linux"]), Some(0));
    }

    #[test]
    fn select_correct_asset_macos() {
        let release = Release {
            prerelease: false,
            tag_name: "v0.5.2".to_string(),
            assets: vec![
                ReleaseAsset {
                    name: "tool-linux.zip".to_string(),
                    url: "https://example.com/some/repo/releases/assets/1".to_string(),
                },
                ReleaseAsset {
                    name: "tool-macos-x86_64.zip".to_string(),
                    url: "https://example.com/some/repo/releases/assets/2".to_string(),
                },
                ReleaseAsset {
                    name: "tool-macos-aarch64.zip".to_string(),
                    url: "https://example.com/some/repo/releases/assets/3".to_string(),
                },
            ],
        };
        assert_eq!(
            choose_asset(
                &release,
                &[
                    "macos-arm64",
                    "darwin-arm64",
                    "macos-aarch64",
                    "darwin-aarch64",
                    "macos",
                    "darwin",
                ]
            ),
            Some(2)
        );
    }

    #[test]
    fn select_correct_asset_linux() {
        let release = Release {
            prerelease: false,
            tag_name: "v0.5.2".to_string(),
            assets: vec![
                ReleaseAsset {
                    name: "tool-linux-aarch64.zip".to_string(),
                    url: "https://example.com/some/repo/releases/assets/1".to_string(),
                },
                ReleaseAsset {
                    name: "tool-linux-x86_64.zip".to_string(),
                    url: "https://example.com/some/repo/releases/assets/2".to_string(),
                },
                ReleaseAsset {
                    name: "tool-macos-x86_64.zip".to_string(),
                    url: "https://example.com/some/repo/releases/assets/3".to_string(),
                },
                ReleaseAsset {
                    name: "tool-win64.zip".to_string(),
                    url: "https://example.com/some/repo/releases/assets/4".to_string(),
                },
            ],
        };
        assert_eq!(choose_asset(&release, &["linux"]), Some(0));
        assert_eq!(choose_asset(&release, &["linux-x86_64", "linux"]), Some(1));
        assert_eq!(
            choose_asset(&release, &["linux-arm64", "linux-aarch64", "linux"]),
            Some(0)
        );
    }

    mod load {
        use super::*;

        #[test]
        fn use_default_when_tool_cache_file_does_not_exist() {
            let foreman_root = tempdir().expect("unable to create temporary directory");
            let paths = ForemanPaths::new(foreman_root.into_path());

            let cache = ToolCache::load(&paths).unwrap();

            assert_eq!(cache, ToolCache::new(&paths));
        }

        #[test]
        fn reads_the_content_from_the_cache_file() {
            let foreman_root = tempdir().expect("unable to create temporary directory");
            let paths = ForemanPaths::new(foreman_root.into_path());

            fs::write(
                paths.index_file(),
                r#"
            {
                "tools": {
                    "username/toolname": {
                        "versions": [
                            "0.1.0"
                        ]
                    }
                }
            }
            "#,
            )
            .unwrap();

            let cache = ToolCache::load(&paths).unwrap();

            let mut expected_cache = ToolCache::new(&paths);

            expected_cache.tools.insert(
                "username/toolname".into(),
                ToolEntry {
                    versions: {
                        let mut tree = BTreeSet::new();
                        tree.insert(Version::parse("0.1.0").unwrap());
                        tree
                    },
                },
            );

            assert_eq!(cache, expected_cache);
        }
    }

    mod save {
        use super::*;

        #[test]
        fn snapshot_default_tool_cache() {
            let foreman_root = tempdir().expect("unable to create temporary directory");
            let paths = ForemanPaths::new(foreman_root.into_path());

            let cache = ToolCache::new(&paths);

            cache.save().unwrap();

            let content = fs::try_read_to_string(paths.index_file())
                .unwrap()
                .expect("unable to find tool cache file");

            insta::assert_snapshot!(&content, @r###"
            {
              "tools": {}
            }
            "###);
        }
    }
}
