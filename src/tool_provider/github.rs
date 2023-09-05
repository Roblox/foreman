//! Slice of GitHub's API that Foreman consumes.

use reqwest::{
    blocking::Client,
    header::{ACCEPT, AUTHORIZATION, USER_AGENT},
};
use serde::{Deserialize, Serialize};

use crate::{
    auth_store::AuthStore,
    error::{ForemanError, ForemanResult},
    paths::ForemanPaths,
};

use super::{Release, ReleaseAsset, ToolProviderImpl};

#[derive(Debug)]
pub struct GithubProvider {
    paths: ForemanPaths,
}

impl GithubProvider {
    pub fn new(paths: ForemanPaths) -> Self {
        Self { paths }
    }
}

impl ToolProviderImpl for GithubProvider {
    fn get_releases(&self, repo: &str) -> ForemanResult<Vec<Release>> {
        let client = Client::new();

        let url = format!("https://api.github.com/repos/{}/releases", repo);
        let mut builder = client.get(&url).header(USER_AGENT, "Roblox/foreman");

        let auth_store = AuthStore::load(&self.paths.auth_store())?;
        if let Some(token) = &auth_store.github {
            builder = builder.header(AUTHORIZATION, format!("token {}", token));
        }

        log::debug!("Downloading github releases for {}", repo);
        let response_body = builder
            .send()
            .map_err(ForemanError::request_failed)?
            .text()
            .map_err(ForemanError::request_failed)?;

        let releases: Vec<GithubRelease> = serde_json::from_str(&response_body).map_err(|err| {
            ForemanError::unexpected_response_body(err.to_string(), response_body, url)
        })?;

        Ok(releases.into_iter().map(Into::into).collect())
    }

    fn download_asset(&self, url: &str) -> ForemanResult<Vec<u8>> {
        let client = Client::new();

        let mut builder = client
            .get(url)
            .header(USER_AGENT, "Roblox/foreman")
            // Setting `Accept` is required to make the GitHub API return the actual
            // release asset instead of JSON metadata about the release.
            .header(ACCEPT, "application/octet-stream");

        let auth_store = AuthStore::load(&self.paths.auth_store())?;
        if let Some(token) = &auth_store.github {
            builder = builder.header(AUTHORIZATION, format!("token {}", token));
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
struct GithubRelease {
    pub tag_name: String,
    pub prerelease: bool,
    pub assets: Vec<GithubAsset>,
}

#[derive(Debug, Serialize, Deserialize)]
struct GithubAsset {
    pub url: String,
    pub name: String,
}

impl From<GithubRelease> for Release {
    fn from(release: GithubRelease) -> Self {
        Release {
            tag_name: release.tag_name,
            prerelease: release.prerelease,
            assets: release.assets.into_iter().map(Into::into).collect(),
        }
    }
}

impl From<GithubAsset> for ReleaseAsset {
    fn from(asset: GithubAsset) -> Self {
        ReleaseAsset {
            url: asset.url,
            name: asset.name,
        }
    }
}
