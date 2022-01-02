//! Slice of GitHub's API that Foreman consumes.

use reqwest::{
    blocking::Client,
    header::{ACCEPT, AUTHORIZATION, USER_AGENT},
};
use serde::{Deserialize, Serialize};

use crate::auth_store::AuthStore;

use super::{Release, ReleaseAsset, ToolProviderImpl};

#[derive(Debug, Default)]
pub struct GithubProvider {}

impl ToolProviderImpl for GithubProvider {
    fn get_releases(&self, repo: &str) -> reqwest::Result<Vec<Release>> {
        log::debug!("Downloading github releases for {}", repo);

        let client = Client::new();

        let url = format!("https://api.github.com/repos/{}/releases", repo);
        let mut builder = client.get(&url).header(USER_AGENT, "Roblox/foreman");

        let auth_store = AuthStore::load().unwrap();
        if let Some(token) = &auth_store.github {
            builder = builder.header(AUTHORIZATION, format!("token {}", token));
        }

        let response_body = builder.send()?.text()?;

        let releases: Vec<GithubRelease> = match serde_json::from_str(&response_body) {
            Ok(releases) => releases,
            Err(err) => {
                log::error!("Unexpected GitHub API response: {}", response_body);
                panic!("{}", err);
            }
        };

        Ok(releases.into_iter().map(Into::into).collect())
    }

    fn download_asset(&self, url: &str) -> reqwest::Result<Vec<u8>> {
        log::debug!("Downloading release asset {}", url);

        let client = Client::new();

        let mut builder = client
            .get(url)
            .header(USER_AGENT, "Roblox/foreman")
            // Setting `Accept` is required to make the GitHub API return the actual
            // release asset instead of JSON metadata about the release.
            .header(ACCEPT, "application/octet-stream");

        let auth_store = AuthStore::load().unwrap();
        if let Some(token) = &auth_store.github {
            builder = builder.header(AUTHORIZATION, format!("token {}", token));
        }

        let mut response = builder.send()?;

        let mut output = Vec::new();
        response.copy_to(&mut output)?;
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
