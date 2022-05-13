mod aliaser;
mod artifact_choosing;
mod auth_store;
mod ci_string;
mod config;
mod error;
mod fs;
mod paths;
mod tool_cache;
mod tool_provider;

use std::{env, ffi::OsStr, process};

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
                process::exit(exit_code);
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
    process::exit(1);
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

            for (tool_alias, tool_spec) in &config.tools {
                let providers = ToolProvider::new(&paths);
                cache.download_if_necessary(tool_spec, &providers)?;
                add_self_alias(tool_alias, &paths.bin_dir())?;
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
    }

    Ok(())
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
                    rpassword::read_password_from_tty(Some(&format!("{} Token: ", provider)))
                        .map_err(|err| {
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
