//! Slice of Artifactory's API that Foreman consumes.

use super::{Release, ReleaseAsset, ToolProviderImpl};
use crate::{
    error::{ForemanError, ForemanResult},
    paths::ForemanPaths,
};
use artiaa_auth;
use reqwest::{
    blocking::{Client, RequestBuilder},
    header::{AUTHORIZATION, USER_AGENT},
    StatusCode,
};
use semver::{BuildMetadata, Op, Version, VersionReq};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use url::Url;

#[derive(Debug)]
pub struct ArtifactoryProvider {
    paths: ForemanPaths,
}

impl ArtifactoryProvider {
    pub fn new(paths: ForemanPaths) -> Self {
        Self { paths }
    }
}

impl ToolProviderImpl for ArtifactoryProvider {
    fn get_releases(
        &self,
        repo: &str,
        host: &Url,
        version_req: &VersionReq,
    ) -> ForemanResult<Vec<Release>> {
        let client = Client::new();

        let tokens = artiaa_auth::Tokens::load(&self.paths.artiaa_path()?)
            .map_err(|error| ForemanError::ArtiAAError { error })?;
        let credentials = tokens.get_credentials(host);

        let get = |url: &str| {
            let mut builder = client.get(url).header(USER_AGENT, "Roblox/foreman");
            if let Some(credentials) = credentials {
                builder = builder.header(AUTHORIZATION, format!("Bearer {}", credentials.token));
            }
            builder
        };

        let url = format!("{}artifactory/api/storage/{}", host, repo);

        // An exact version pin only needs the files in that version's folder. Artifactory can
        // serve folder info anonymously while requiring authentication to list a whole repo.
        let file_uris = if let Some(version) = exact_version(version_req) {
            log::debug!("Downloading artifactory release {} for {}", version, repo);
            list_exact_release_files(&get, &url, &version)?
        } else {
            log::debug!("Downloading artifactory releases for {}", repo);
            let response = get(&url)
                .query(&[("list", ""), ("deep", "1")])
                .send()
                .map_err(ForemanError::request_failed)?;

            if response.status() == StatusCode::UNAUTHORIZED
                || response.status() == StatusCode::FORBIDDEN
            {
                log::warn!(
                    "Listing releases of {} requires Artifactory authentication. Pin an exact version (e.g. \"=1.2.3\") or run `foreman artifactory-auth {} <token>`",
                    repo,
                    host.domain().unwrap_or_else(|| host.as_str())
                );
            }

            let response_body = response.text().map_err(ForemanError::request_failed)?;

            let response: ArtifactoryResponse =
                serde_json::from_str(&response_body).map_err(|err| {
                    ForemanError::unexpected_response_body(err.to_string(), response_body, url)
                })?;

            response.files.into_iter().map(|file| file.uri).collect()
        };

        let mut release_map: HashMap<&str, Vec<ArtifactoryAsset>> = HashMap::new();
        for file_uri in &file_uris {
            let uri = file_uri.split("/");
            // file_uri should look something like /<version>/<artifact-name>, so uri will be ["", <version>, <artifact-name]
            // we should skip files that do not follow the expected path
            let Some((version, asset_name)) = get_version_and_asset_name(uri) else {
                log::debug!(
                    "Skipping '{}', does not match expected file path <version>/<asset_name>",
                    file_uri
                );
                continue;
            };

            let asset_url = format!("{}artifactory/{}/{}/{}", host, repo, version, asset_name);

            let asset = ArtifactoryAsset {
                url: asset_url,
                name: asset_name.to_string(),
            };

            release_map.entry(version).or_insert(Vec::new()).push(asset);
        }

        let releases: Vec<ArtifactoryRelease> = release_map
            .into_iter()
            .map(|(version, assets)| ArtifactoryRelease {
                tag_name: version.to_string(),
                assets,
            })
            .collect();

        Ok(releases.into_iter().map(Into::into).collect())
    }

    fn download_asset(&self, url: &str) -> ForemanResult<Vec<u8>> {
        let client = Client::new();
        let artifactory_url = Url::parse(url).unwrap();

        let mut builder = client.get(url).header(USER_AGENT, "Roblox/foreman");

        let tokens = artiaa_auth::Tokens::load(&self.paths.artiaa_path()?).unwrap();
        if let Some(credentials) = tokens.get_credentials(&artifactory_url) {
            builder = builder.header(AUTHORIZATION, format!("Bearer {}", credentials.token));
        }

        log::debug!("Downloading release asset {}", url);
        let mut response = builder.send().map_err(ForemanError::request_failed)?;

        let mut output = Vec::new();
        response
            .copy_to(&mut output)
            .map_err(ForemanError::request_failed)?;
        Ok(output)
    }
}

/// Returns the version a requirement pins exactly, such as `=1.2.3` or `=1.2.3-rc.1`.
fn exact_version(version_req: &VersionReq) -> Option<Version> {
    let [comparator] = version_req.comparators.as_slice() else {
        return None;
    };

    if comparator.op != Op::Exact {
        return None;
    }

    Some(Version {
        major: comparator.major,
        minor: comparator.minor?,
        patch: comparator.patch?,
        pre: comparator.pre.clone(),
        build: BuildMetadata::EMPTY,
    })
}

/// Lists the files in `version`'s folder as /<version>/<asset_name> URIs. The folder may be named
/// with or without a leading `v`.
fn list_exact_release_files<F>(get: &F, url: &str, version: &Version) -> ForemanResult<Vec<String>>
where
    F: Fn(&str) -> RequestBuilder,
{
    for tag_name in [format!("v{}", version), version.to_string()] {
        let folder_url = format!("{}/{}", url, tag_name);
        let response = get(&folder_url)
            .send()
            .map_err(ForemanError::request_failed)?;

        if response.status() == StatusCode::NOT_FOUND {
            log::debug!("No artifactory release folder at {}", folder_url);
            continue;
        }

        let response_body = response.text().map_err(ForemanError::request_failed)?;
        let folder: ArtifactoryFolderInfo =
            serde_json::from_str(&response_body).map_err(|err| {
                ForemanError::unexpected_response_body(err.to_string(), response_body, folder_url)
            })?;

        return Ok(folder
            .children
            .into_iter()
            .filter(|child| !child.folder)
            .map(|child| format!("/{}{}", tag_name, child.uri))
            .collect());
    }

    Ok(Vec::new())
}

fn get_version_and_asset_name<'a, I>(mut uri: I) -> Option<(&'a str, &'a str)>
where
    I: Iterator<Item = &'a str>,
{
    let Some(empty_string) = uri.next() else {
        return None;
    };

    if empty_string != "" {
        return None;
    }

    let Some(version) = uri.next() else {
        return None;
    };

    let Some(asset_name) = uri.next() else {
        return None;
    };

    if uri.next().is_some() {
        return None;
    }

    Some((version, asset_name))
}

#[derive(Debug, Serialize, Deserialize)]
struct ArtifactoryResponse {
    files: Vec<ArtifactoryResponseFiles>,
}
#[derive(Debug, Serialize, Deserialize)]
struct ArtifactoryResponseFiles {
    uri: String,
}
#[derive(Debug, Serialize, Deserialize)]
struct ArtifactoryFolderInfo {
    children: Vec<ArtifactoryFolderChild>,
}
#[derive(Debug, Serialize, Deserialize)]
struct ArtifactoryFolderChild {
    uri: String,
    folder: bool,
}
#[derive(Debug)]
struct ArtifactoryRelease {
    tag_name: String,
    assets: Vec<ArtifactoryAsset>,
}
#[derive(Debug)]
struct ArtifactoryAsset {
    url: String,
    name: String,
}

impl From<ArtifactoryRelease> for Release {
    fn from(release: ArtifactoryRelease) -> Self {
        Release {
            tag_name: release.tag_name,
            prerelease: false,
            assets: release.assets.into_iter().map(Into::into).collect(),
        }
    }
}

impl From<ArtifactoryAsset> for ReleaseAsset {
    fn from(asset: ArtifactoryAsset) -> Self {
        ReleaseAsset {
            url: asset.url,
            name: asset.name,
        }
    }
}
