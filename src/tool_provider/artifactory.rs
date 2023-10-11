//! Slice of GitHub's API that Foreman consumes.

use super::{Release, ReleaseAsset, ToolProviderImpl};
use crate::{
    error::{ForemanError, ForemanResult},
    paths::ForemanPaths,
};
use artiaa_auth;
use reqwest::{
    blocking::Client,
    header::{AUTHORIZATION, USER_AGENT},
};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use url::Url;

const ARTIFACTORY_API_KEY_HEADER: &str = "X-JFrog-Art-Api";

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
    fn get_releases(&self, repo: &str, host: &Url) -> ForemanResult<Vec<Release>> {
        let client = Client::new();

        let url = format!("{}artifactory/api/storage/{}", host, repo);
        let params = vec![("list", ""), ("deep", "1")];
        let mut builder = client
            .get(&url)
            .header(USER_AGENT, "Roblox/foreman")
            .query(&params);

        let tokens = artiaa_auth::Tokens::load(&self.paths.artiaa_path()?)
            .map_err(|error| ForemanError::ArtiAAError { error })?;

        if let Some(credentials) = tokens.get_credentials(host) {
            builder = builder.header(ARTIFACTORY_API_KEY_HEADER, credentials.token.to_string());
        }
        log::debug!("Downloading artifactory releases for {}", repo);
        let response_body = builder
            .send()
            .map_err(ForemanError::request_failed)?
            .text()
            .map_err(ForemanError::request_failed)?;

        let response: ArtifactoryResponse =
            serde_json::from_str(&response_body).map_err(|err| {
                ForemanError::unexpected_response_body(err.to_string(), response_body, url)
            })?;

        let mut release_map: HashMap<&str, Vec<ArtifactoryAsset>> = HashMap::new();
        for file in &response.files {
            let uri = file.uri.split("/");
            // file.uri should look something like /<version>/<artifact-name>, so uri will be ["", <version>, <artifact-name]
            // we should skip files that do not follow the expected path
            let Some((version, asset_name)) = get_version_and_asset_name(uri) else {
                log::debug!(
                    "Skipping '{}', does not match expected file path <version>/<asset_name>",
                    file.uri
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
            builder = builder.header(AUTHORIZATION, format!("bearer {}", credentials.token));
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
