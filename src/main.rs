mod aliaser;
mod artifact_choosing;
mod artifactory_auth_store;
mod artifactory_path;
mod auth_store;
mod ci_string;
mod config;
mod error;
mod fs;
mod paths;
mod process;
mod tool_cache;
mod tool_provider;

use std::{
    env,
    ffi::OsStr,
    io::{stdout, Write},
};

use artifactory_auth_store::ArtifactoryAuthStore;
use paths::ForemanPaths;
use structopt::StructOpt;

use crate::{
    aliaser::add_self_alias,
    auth_store::AuthStore,
    config::ConfigFile,
    error::{ForemanError, ForemanResult},
    tool_cache::ToolCache,
    tool_provider::ToolProvider,
};

#[derive(Debug)]
struct ToolInvocation {
    name: String,
    args: Vec<String>,
}

impl ToolInvocation {
    fn from_env() -> ForemanResult<Option<Self>> {
        let app_path = env::current_exe().map_err(|err| {
            ForemanError::io_error_with_context(err, "unable to obtain foreman executable location")
        })?;
        let name = if let Some(name) = app_path
            .file_stem()
            .and_then(OsStr::to_str)
            .map(ToOwned::to_owned)
        {
            name
        } else {
            return Ok(None);
        };

        // That's us!
        if name == "foreman" {
            return Ok(None);
        }

        let args = env::args().skip(1).collect();

        Ok(Some(Self { name, args }))
    }

    fn run(self, paths: &ForemanPaths) -> ForemanResult<()> {
        let config = ConfigFile::aggregate(paths)?;

        if let Some(tool_spec) = config.tools.get(&self.name) {
            log::debug!("Found tool spec {}", tool_spec);

            let mut tool_cache = ToolCache::load(paths)?;
            let providers = ToolProvider::new(paths);
            let version = tool_cache.download_if_necessary(tool_spec, &providers)?;

            let exit_code = tool_cache.run(tool_spec, &version, self.args)?;

            if exit_code != 0 {
                std::process::exit(exit_code);
            }

            Ok(())
        } else {
            let current_dir = env::current_dir().map_err(|err| {
                ForemanError::io_error_with_context(
                    err,
                    "unable to obtain the current working directory",
                )
            })?;
            Err(ForemanError::ToolNotInstalled {
                name: self.name,
                current_path: current_dir,
                config_file: config,
            })
        }
    }
}

fn main() {
    let paths = ForemanPaths::from_env().unwrap_or_default();

    if let Err(error) = paths.create_all() {
        exit_with_error(error);
    }

    let result = ToolInvocation::from_env().and_then(|maybe_invocation| {
        if let Some(invocation) = maybe_invocation {
            let env = env_logger::Env::new().default_filter_or("foreman=info");
            env_logger::Builder::from_env(env)
                .format_module_path(false)
                .format_timestamp(None)
                .format_indent(Some(8))
                .init();

            invocation.run(&paths)
        } else {
            actual_main(paths)
        }
    });

    if let Err(error) = result {
        exit_with_error(error);
    }
}

fn exit_with_error(error: ForemanError) -> ! {
    eprintln!("{}", error);
    std::process::exit(1);
}

#[derive(Debug, StructOpt)]
struct Options {
    /// Logging verbosity. Supply multiple for more verbosity, up to -vvv
    #[structopt(short, parse(from_occurrences), global = true)]
    pub verbose: u8,

    #[structopt(subcommand)]
    subcommand: Subcommand,
}

#[derive(Debug, StructOpt)]
enum Subcommand {
    /// Install tools defined by foreman.toml.
    Install,

    /// List installed tools.
    List,

    /// Set the GitHub Personal Access Token that Foreman should use with the
    /// GitHub API.
    ///
    /// This token can also be configured by editing ~/.foreman/auth.toml.
    #[structopt(name = "github-auth")]
    GitHubAuth(GitHubAuthCommand),

    /// Set the GitLab Personal Access Token that Foreman should use with the
    /// GitLab API.
    ///
    /// This token can also be configured by editing ~/.foreman/auth.toml.
    #[structopt(name = "gitlab-auth")]
    GitLabAuth(GitLabAuthCommand),

    /// Set the Artifactory Token that Foreman should use with the
    /// Artifactory API.
    #[structopt(name = "artifactory-auth")]
    ArtifactoryAuth(ArtifactoryAuthCommand),

    /// Create a path to publish to artifactory
    ///
    /// Foreman does not support uploading binaries to artifactory directly, but it can generate the path where it would expect to find a given artifact. Use this command to generate paths that can be input to generic artifactory upload solutions.
    #[structopt(name = "generate-artifactory-path")]
    GenerateArtifactoryPath(GenerateArtifactoryPathCommand),
}

#[derive(Debug, StructOpt)]
struct GitHubAuthCommand {
    /// GitHub personal access token that Foreman should use.
    ///
    /// If not specified, Foreman will prompt for it.
    token: Option<String>,
}

#[derive(Debug, StructOpt)]
struct GitLabAuthCommand {
    /// GitLab personal access token that Foreman should use.
    ///
    /// If not specified, Foreman will prompt for it.
    token: Option<String>,
}

#[derive(Debug, StructOpt)]
struct ArtifactoryAuthCommand {
    url: Option<String>,
    token: Option<String>,
}

#[derive(Debug, StructOpt)]
struct GenerateArtifactoryPathCommand {
    repo: String,
    tool_name: String,
    version: String,
    operating_system: String,
    architecture: Option<String>,
}

fn actual_main(paths: ForemanPaths) -> ForemanResult<()> {
    let options = Options::from_args();

    {
        let log_filter = match options.verbose {
            0 => "warn,foreman=info",
            1 => "info,foreman=debug",
            2 => "info,foreman=trace",
            _ => "trace",
        };

        let env = env_logger::Env::default().default_filter_or(log_filter);

        env_logger::Builder::from_env(env)
            .format_module_path(false)
            .format_target(false)
            .format_timestamp(None)
            .format_indent(Some(8))
            .init();
    }

    match options.subcommand {
        Subcommand::Install => {
            let config = ConfigFile::aggregate(&paths)?;

            log::trace!("Installing from gathered config: {:#?}", config);

            let mut cache = ToolCache::load(&paths)?;

            let providers = ToolProvider::new(&paths);

            let tools_not_downloaded: Vec<String> = config
                .tools
                .iter()
                .filter_map(|(tool_alias, tool_spec)| {
                    cache
                        .download_if_necessary(tool_spec, &providers)
                        .and_then(|_| add_self_alias(tool_alias, &paths.bin_dir()))
                        .err()
                        .map(|err| {
                            log::error!(
                                "The following error occurred while trying to download tool \"{}\":\n{}",
                                tool_alias,
                                err
                            );
                            tool_alias.to_string()
                        })
                })
                .collect();

            if !tools_not_downloaded.is_empty() {
                return Err(ForemanError::ToolsNotDownloaded {
                    tools: tools_not_downloaded,
                });
            }

            if config.tools.is_empty() {
                log::info!(
                    concat!(
                        "foreman did not find any tools to install.\n\n",
                        "You can define system-wide tools in:\n  {}\n",
                        "or create a 'foreman.toml' file in your project directory.",
                    ),
                    paths.user_config().display()
                );
            }
        }
        Subcommand::List => {
            println!("Installed tools:");

            let cache = ToolCache::load(&paths)?;

            for (tool_source, tool) in &cache.tools {
                println!("  {}", tool_source);

                for version in &tool.versions {
                    println!("    - {}", version);
                }
            }
        }
        Subcommand::GitHubAuth(subcommand) => {
            let token = prompt_auth_token(
                    subcommand.token,
                    "GitHub",
                    "https://help.github.com/en/github/authenticating-to-github/creating-a-personal-access-token-for-the-command-line",
                )?;

            AuthStore::set_github_token(&paths.auth_store(), &token)?;

            println!("GitHub auth saved successfully.");
        }
        Subcommand::GitLabAuth(subcommand) => {
            let token = prompt_auth_token(
                subcommand.token,
                "GitLab",
                "https://docs.gitlab.com/ee/user/profile/personal_access_tokens.html",
            )?;

            AuthStore::set_gitlab_token(&paths.auth_store(), &token)?;

            println!("GitLab auth saved successfully.");
        }
        Subcommand::GenerateArtifactoryPath(subcommand) => {
            let artifactory_path = artifactory_path::generate_artifactory_path(
                subcommand.repo,
                subcommand.tool_name,
                subcommand.version,
                subcommand.operating_system,
                subcommand.architecture,
            )?;
            println!("{}", artifactory_path);
        }
        Subcommand::ArtifactoryAuth(subcommand) => {
            let url = prompt_url(subcommand.url)?;

            let token = prompt_auth_token(
                subcommand.token,
                "Artifactory",
                "https://jfrog.com/help/r/jfrog-platform-administration-documentation/access-tokens",
            )?;

            ArtifactoryAuthStore::set_token(&paths.artiaa_path()?, &url, &token)?;
        }
    }

    Ok(())
}

fn prompt_url(url: Option<String>) -> Result<String, ForemanError> {
    match url {
        Some(url) => Ok(url),
        None => {
            println!("Artifactory auth saved successfully.");
            println!("Foreman requires a specific URL to authenticate to Artifactory.");
            println!();

            loop {
                let mut input = String::new();

                print!("Artifactory URL: ");
                stdout().flush().map_err(|err| {
                    ForemanError::io_error_with_context(
                        err,
                        "an error happened trying to flush stdout",
                    )
                })?;
                std::io::stdin().read_line(&mut input).map_err(|err| {
                    ForemanError::io_error_with_context(err, "an error happened trying to read url")
                })?;

                if input.is_empty() {
                    println!("Token must be non-empty.");
                } else {
                    break Ok(input);
                }
            }
        }
    }
}

fn prompt_auth_token(
    token: Option<String>,
    provider: &str,
    help: &str,
) -> Result<String, ForemanError> {
    match token {
        Some(token) => Ok(token),
        None => {
            println!("{} auth saved successfully.", provider);
            println!(
                "Foreman authenticates to {} using Personal Access Tokens.",
                provider
            );
            println!("{}", help);
            println!();

            loop {
                let token =
                    rpassword::prompt_password(format!("{} Token: ", provider)).map_err(|err| {
                        ForemanError::io_error_with_context(
                            err,
                            "an error happened trying to read password",
                        )
                    })?;

                if token.is_empty() {
                    println!("Token must be non-empty.");
                } else {
                    break Ok(token);
                }
            }
        }
    }
}
