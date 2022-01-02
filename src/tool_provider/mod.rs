mod github;
mod gitlab;

use std::{collections::HashMap, fmt};

use github::GithubProvider;
use gitlab::GitlabProvider;

pub trait ToolProviderImpl: fmt::Debug {
    fn get_releases(&self, repo: &str) -> reqwest::Result<Vec<Release>>;

    fn download_asset(&self, url: &str) -> reqwest::Result<Vec<u8>>;
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Provider {
    Github,
    Gitlab,
}

impl fmt::Display for Provider {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "{}",
            match self {
                Provider::Github => "GitHub",
                Provider::Gitlab => "GitLab",
            }
        )
    }
}

#[derive(Debug)]
pub struct ToolProvider {
    providers: HashMap<Provider, Box<dyn ToolProviderImpl>>,
}

impl Default for ToolProvider {
    fn default() -> Self {
        let mut providers: HashMap<Provider, Box<dyn ToolProviderImpl>> = HashMap::default();
        providers.insert(Provider::Github, Box::new(GithubProvider::default()));
        providers.insert(Provider::Gitlab, Box::new(GitlabProvider::default()));
        Self { providers }
    }
}

impl ToolProvider {
    pub fn get(&self, provider: &Provider) -> &dyn ToolProviderImpl {
        self.providers
            .get(provider)
            .unwrap_or_else(|| {
                panic!(
                    "unable to find tool provider implementation for {}",
                    provider
                )
            })
            .as_ref()
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Release {
    pub tag_name: String,
    pub prerelease: bool,
    pub assets: Vec<ReleaseAsset>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ReleaseAsset {
    pub url: String,
    pub name: String,
}
