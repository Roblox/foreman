//! Slice of GitHub's API that Foreman consumes.

use crate::{
    error::{ForemanError, ForemanResult},
    paths::ForemanPaths,
};

use super::{Release, ToolProviderImpl};

#[derive(Debug)]
#[allow(unused)]
pub struct ArtifactoryProvider {
    paths: ForemanPaths,
}

impl ArtifactoryProvider {
    pub fn new(paths: ForemanPaths) -> Self {
        Self { paths }
    }
}
#[allow(unused)]
impl ToolProviderImpl for ArtifactoryProvider {
    fn get_releases(&self, repo: &str) -> ForemanResult<Vec<Release>> {
        Err(ForemanError::Other {
            message: "Artifactory is not yet supported. Please use Github or Gitlab as your source"
                .to_owned(),
        })
    }

    fn download_asset(&self, url: &str) -> ForemanResult<Vec<u8>> {
        Err(ForemanError::Other {
            message: "Artifactory is not yet supported. Please use Github or Gitlab as your source"
                .to_owned(),
        })
    }
}
