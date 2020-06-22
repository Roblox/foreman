//! Slice of GitHub's API that Foreman consumes.

use std::io::Write;

use reqwest::{
    blocking::Client,
    header::{ACCEPT, AUTHORIZATION, USER_AGENT},
};
use serde::{Deserialize, Serialize};

use crate::auth_store::AuthStore;

pub fn get_releases(repo: &str) -> reqwest::Result<Vec<Release>> {
    log::debug!("Downloading releases for {}", repo);

    let client = Client::new();

    let url = format!("https://api.github.com/repos/{}/releases", repo);
    let mut builder = client.get(&url).header(USER_AGENT, "Roblox/foreman");

    let auth_store = AuthStore::load().unwrap();
    if let Some(token) = &auth_store.github {
        builder = builder.header(AUTHORIZATION, format!("token {}", token));
    }

    let response_body = builder.send()?.text()?;

    let releases: Vec<Release> = match serde_json::from_str(&response_body) {
        Ok(releases) => releases,
        Err(err) => {
            log::error!("Unexpected GitHub API response: {}", response_body);
            panic!("{}", err);
        }
    };

    Ok(releases)
}

pub fn download_asset<W: Write>(url: &str, mut output: W) -> reqwest::Result<()> {
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
    response.copy_to(&mut output)?;

    Ok(())
}

#[derive(Debug, Serialize, Deserialize)]
pub struct Release {
    pub tag_name: String,
    pub prerelease: bool,
    pub assets: Vec<ReleaseAsset>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct ReleaseAsset {
    pub url: String,
    pub name: String,
}
