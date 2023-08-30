//! Slice of Gitlab's API that Foreman consumes.

use reqwest::{
    blocking::Client,
    header::{ACCEPT, USER_AGENT},
};
use serde::{Deserialize, Serialize};

use crate::{
    auth_store::AuthStore,
    error::{ForemanError, ForemanResult},
    paths::ForemanPaths,
};

use super::{Release, ReleaseAsset, ToolProviderImpl};

#[derive(Debug, Default)]
pub struct GitlabProvider {
    paths: ForemanPaths,
}

impl GitlabProvider {
    pub fn new(paths: ForemanPaths) -> Self {
        Self { paths }
    }
}

impl ToolProviderImpl for GitlabProvider {
    fn get_releases(&self, repo: &str) -> ForemanResult<Vec<Release>> {
        let client = Client::new();

        let url = format!(
            "https://gitlab.com/api/v4/projects/{}/releases",
            urlencoding::encode(repo)
        );
        let mut builder = client.get(&url).header(USER_AGENT, "Roblox/foreman");

        let auth_store = AuthStore::load(&self.paths.auth_store())?;
        if let Some(token) = &auth_store.gitlab {
            builder = builder.header("PRIVATE-TOKEN", token);
        }

        log::debug!("Downloading gitlab releases for {}", repo);
        let response_body = builder
            .send()
            .map_err(ForemanError::request_failed)?
            .text()
            .map_err(ForemanError::request_failed)?;

        let releases: Vec<GitlabRelease> = serde_json::from_str(&response_body).map_err(|err| {
            ForemanError::unexpected_response_body(err.to_string(), response_body, url)
        })?;

        Ok(releases.into_iter().map(Into::into).collect())
    }

    fn download_asset(&self, url: &str) -> ForemanResult<Vec<u8>> {
        let client = Client::new();

        let mut builder = client
            .get(url)
            .header(USER_AGENT, "Roblox/foreman")
            // Setting `Accept` is required to make the GitLab API return the actual
            // release asset instead of JSON metadata about the release.
            .header(ACCEPT, "application/octet-stream");

        let auth_store = AuthStore::load(&self.paths.auth_store())?;
        if let Some(token) = &auth_store.gitlab {
            builder = builder.header("PRIVATE-TOKEN", token);
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

#[derive(Debug, Serialize, Deserialize)]
struct GitlabRelease {
    pub name: String,
    pub tag_name: String,
    pub upcoming_release: bool,
    pub assets: ReleaseAssets,
}

#[derive(Debug, Serialize, Deserialize)]
struct ReleaseAssets {
    links: Vec<GitlabAsset>,
}

#[derive(Debug, Serialize, Deserialize)]
struct GitlabAsset {
    pub url: String,
    pub name: String,
}

impl From<GitlabRelease> for Release {
    fn from(release: GitlabRelease) -> Self {
        Release {
            tag_name: release.tag_name,
            prerelease: release.upcoming_release,
            assets: release.assets.links.into_iter().map(Into::into).collect(),
        }
    }
}

impl From<GitlabAsset> for ReleaseAsset {
    fn from(asset: GitlabAsset) -> Self {
        ReleaseAsset {
            url: asset.url,
            name: asset.name,
        }
    }
}
