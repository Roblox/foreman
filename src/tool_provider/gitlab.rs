//! Slice of Gitlab's API that Foreman consumes.

use reqwest::{
    blocking::Client,
    header::{ACCEPT, USER_AGENT},
};
use serde::{Deserialize, Serialize};

use crate::auth_store::AuthStore;

use super::{Release, ReleaseAsset, ToolProviderImpl};

#[derive(Debug, Default)]
pub struct GitlabProvider {}

impl ToolProviderImpl for GitlabProvider {
    fn get_releases(&self, repo: &str) -> reqwest::Result<Vec<Release>> {
        log::debug!("Downloading gitlab releases for {}", repo);

        let client = Client::new();

        let url = format!(
            "https://gitlab.com/api/v4/projects/{}/releases",
            urlencoding::encode(repo)
        );
        let mut builder = client.get(&url).header(USER_AGENT, "Roblox/foreman");

        let auth_store = AuthStore::load().unwrap();
        if let Some(token) = &auth_store.gitlab {
            builder = builder.header("PRIVATE-TOKEN", token);
        }
        let response_body = builder.send()?.text()?;

        let releases: Vec<GitlabRelease> = match serde_json::from_str(&response_body) {
            Ok(releases) => releases,
            Err(err) => {
                log::error!("Unexpected GitLab API response: {}", response_body);
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
            // Setting `Accept` is required to make the GitLab API return the actual
            // release asset instead of JSON metadata about the release.
            .header(ACCEPT, "application/octet-stream");

        let auth_store = AuthStore::load().unwrap();
        if let Some(token) = &auth_store.gitlab {
            builder = builder.header("PRIVATE-TOKEN", token);
        }

        let mut response = builder.send()?;

        let mut output = Vec::new();
        response.copy_to(&mut output)?;
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
