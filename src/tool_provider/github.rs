//! Slice of GitHub's API that Foreman consumes.

use reqwest::{
    blocking::Client,
    header::{ACCEPT, AUTHORIZATION, USER_AGENT},
};
use serde::{Deserialize, Serialize};

use super::{Release, ReleaseAsset, ToolProviderImpl};
use crate::{
    auth_store::AuthStore,
    error::{ForemanError, ForemanResult},
    paths::ForemanPaths,
};
use url::Url;

/// Parses the GitHub `Link` header to extract the "next" page URL.
/// Example header: `<https://api.github.com/...?page=2>; rel="next", <...>; rel="last"`
fn parse_next_link(link_header: &str) -> Option<String> {
    for part in link_header.split(',') {
        let mut url = None;
        let mut is_next = false;

        for segment in part.split(';') {
            let segment = segment.trim();
            if segment.starts_with('<') && segment.ends_with('>') {
                url = Some(segment[1..segment.len() - 1].to_string());
            } else if segment == r#"rel="next""# {
                is_next = true;
            }
        }

        if is_next {
            return url;
        }
    }
    None
}

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
    fn get_releases(&self, repo: &str, _host: &Url) -> ForemanResult<Vec<Release>> {
        let client = Client::new();
        let auth_store = AuthStore::load(&self.paths.auth_store())?;

        let mut all_releases: Vec<GithubRelease> = Vec::new();
        let mut next_url: Option<String> = Some(format!(
            "https://api.github.com/repos/{}/releases?per_page=100",
            repo
        ));

        while let Some(url) = next_url.take() {
            let mut builder = client.get(&url).header(USER_AGENT, "Roblox/foreman");

            if let Some(token) = &auth_store.github {
                builder = builder.header(AUTHORIZATION, format!("token {}", token));
            }

            log::debug!("Downloading github releases for {} (url: {})", repo, url);
            let response = builder.send().map_err(ForemanError::request_failed)?;

            // Parse the Link header for pagination
            next_url = response
                .headers()
                .get("link")
                .and_then(|h| h.to_str().ok())
                .and_then(parse_next_link);

            let response_body = response.text().map_err(ForemanError::request_failed)?;

            let releases: Vec<GithubRelease> =
                serde_json::from_str(&response_body).map_err(|err| {
                    ForemanError::unexpected_response_body(err.to_string(), response_body, url)
                })?;

            all_releases.extend(releases);
        }

        Ok(all_releases.into_iter().map(Into::into).collect())
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_next_link_with_next_and_last() {
        let header = r#"<https://api.github.com/repos/owner/repo/releases?per_page=100&page=2>; rel="next", <https://api.github.com/repos/owner/repo/releases?per_page=100&page=5>; rel="last""#;
        assert_eq!(
            parse_next_link(header),
            Some("https://api.github.com/repos/owner/repo/releases?per_page=100&page=2".to_string())
        );
    }

    #[test]
    fn parse_next_link_with_all_rels() {
        let header = r#"<https://api.github.com/repos/owner/repo/releases?per_page=100&page=1>; rel="prev", <https://api.github.com/repos/owner/repo/releases?per_page=100&page=3>; rel="next", <https://api.github.com/repos/owner/repo/releases?per_page=100&page=5>; rel="last", <https://api.github.com/repos/owner/repo/releases?per_page=100&page=1>; rel="first""#;
        assert_eq!(
            parse_next_link(header),
            Some("https://api.github.com/repos/owner/repo/releases?per_page=100&page=3".to_string())
        );
    }

    #[test]
    fn parse_next_link_last_page_no_next() {
        let header = r#"<https://api.github.com/repos/owner/repo/releases?per_page=100&page=4>; rel="prev", <https://api.github.com/repos/owner/repo/releases?per_page=100&page=1>; rel="first""#;
        assert_eq!(parse_next_link(header), None);
    }

    #[test]
    fn parse_next_link_empty_header() {
        assert_eq!(parse_next_link(""), None);
    }

    #[test]
    fn parse_next_link_only_next() {
        let header = r#"<https://api.github.com/repos/owner/repo/releases?per_page=100&page=2>; rel="next""#;
        assert_eq!(
            parse_next_link(header),
            Some("https://api.github.com/repos/owner/repo/releases?per_page=100&page=2".to_string())
        );
    }
}
