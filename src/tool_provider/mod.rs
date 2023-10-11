mod artifactory;
mod github;
mod gitlab;

use crate::{error::ForemanResult, paths::ForemanPaths};
use artifactory::ArtifactoryProvider;
use github::GithubProvider;
use gitlab::GitlabProvider;
use std::{collections::HashMap, fmt};
use url::Url;

pub trait ToolProviderImpl: fmt::Debug {
    fn get_releases(&self, repo: &str, host: &Url) -> ForemanResult<Vec<Release>>;

    fn download_asset(&self, url: &str) -> ForemanResult<Vec<u8>>;
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Provider {
    Github,
    Gitlab,
    Artifactory,
}

impl fmt::Display for Provider {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "{}",
            match self {
                Provider::Github => "GitHub",
                Provider::Gitlab => "GitLab",
                Provider::Artifactory => "Artifactory",
            }
        )
    }
}

#[derive(Debug)]
pub struct ToolProvider {
    providers: HashMap<Provider, Box<dyn ToolProviderImpl>>,
}

impl ToolProvider {
    pub fn new(paths: &ForemanPaths) -> Self {
        let mut providers: HashMap<Provider, Box<dyn ToolProviderImpl>> = HashMap::default();
        providers.insert(
            Provider::Github,
            Box::new(GithubProvider::new(paths.clone())),
        );
        providers.insert(
            Provider::Gitlab,
            Box::new(GitlabProvider::new(paths.clone())),
        );
        providers.insert(
            Provider::Artifactory,
            Box::new(ArtifactoryProvider::new(paths.clone())),
        );
        Self { providers }
    }

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
