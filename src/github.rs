//! Slice of GitHub's API that Foreman consumes.

use std::io::Write;

use reqwest::{blocking::Client, header::USER_AGENT};
use serde::{Deserialize, Serialize};

pub fn get_releases(repo: &str) -> reqwest::Result<Vec<Release>> {
    log::debug!("Downloading releases for {}", repo);

    let client = Client::new();

    let url = format!("https://api.github.com/repos/{}/releases", repo);
    let releases: Vec<Release> = client
        .get(&url)
        .header(USER_AGENT, "rojo-rbx/foreman")
        .send()?
        .json()?;

    Ok(releases)
}

pub fn download_asset<W: Write>(url: &str, mut output: W) -> reqwest::Result<()> {
    log::debug!("Downloading release asset {}", url);

    let client = Client::new();

    let mut response = client
        .get(url)
        .header(USER_AGENT, "rojo-rbx/foreman")
        .send()?;

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
    pub name: String,
    pub browser_download_url: String,
}
