mod aliaser;
mod artifact_choosing;
mod auth_store;
mod ci_string;
mod config;
mod fs;
mod paths;
mod tool_cache;
mod tool_provider;

use std::{env, error::Error, io, process};

use structopt::StructOpt;

use crate::{
    aliaser::add_self_alias, auth_store::AuthStore, config::ConfigFile, tool_cache::ToolCache,
};

#[derive(Debug)]
struct ToolInvocation {
    name: String,
    args: Vec<String>,
}

impl ToolInvocation {
    fn from_env() -> Option<Self> {
        let app_path = env::current_exe().unwrap();
        let name = app_path.file_stem()?.to_str()?.to_owned();

        // That's us!
        if name == "foreman" {
            return None;
        }

        let args = env::args().skip(1).collect();

        Some(Self { name, args })
    }
}

fn main() -> Result<(), Box<dyn Error>> {
    let env = env_logger::Env::new().default_filter_or("foreman=info");
    env_logger::Builder::from_env(env)
        .format_module_path(false)
        .format_timestamp(None)
        .format_indent(Some(8))
        .init();

    paths::create()?;

    if let Some(invocation) = ToolInvocation::from_env() {
        let config = ConfigFile::aggregate()?;

        if let Some(tool_spec) = config.tools.get(&invocation.name) {
            log::debug!("Found tool spec {}", tool_spec);

            let mut tool_cache = ToolCache::load()?;
            let maybe_version = tool_cache.download_if_necessary(tool_spec);

            if let Some(version) = maybe_version {
                let exit_code = ToolCache::run(tool_spec, &version, invocation.args);

                if exit_code != 0 {
                    process::exit(exit_code);
                }
            }

            return Ok(());
        } else {
            eprintln!(
                "'{}' is not a known Foreman tool, but Foreman was invoked with its name.",
                invocation.name
            );
            eprintln!("You may not have this tool installed here, or your install may be broken.");

            return Ok(());
        }
    }

    actual_main()?;
    Ok(())
}

#[derive(Debug, StructOpt)]
struct Options {
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

fn actual_main() -> io::Result<()> {
    let options = Options::from_args();

    match options.subcommand {
        Subcommand::Install => {
            let config = ConfigFile::aggregate()?;

            log::trace!("Installing from gathered config: {:#?}", config);

            let mut cache = ToolCache::load()?;

            for (tool_alias, tool_spec) in &config.tools {
                cache.download_if_necessary(tool_spec);
                add_self_alias(tool_alias);
            }
        }
        Subcommand::List => {
            println!("Installed tools:");

            let cache = ToolCache::load()?;

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

            AuthStore::set_github_token(&token)?;

            println!("GitHub auth saved successfully.");
        }
        Subcommand::GitLabAuth(subcommand) => {
            let token = prompt_auth_token(
                subcommand.token,
                "GitLab",
                "https://docs.gitlab.com/ee/user/profile/personal_access_tokens.html",
            )?;

            AuthStore::set_gitlab_token(&token)?;

            println!("GitLab auth saved successfully.");
        }
    }

    Ok(())
}

fn prompt_auth_token(token: Option<String>, provider: &str, help: &str) -> io::Result<String> {
    match token {
        Some(token) => Ok(token),
        None => {
            println!("GitHub auth saved successfully.");
            println!(
                "Foreman authenticates to {} using Personal Access Tokens.",
                provider
            );
            println!("{}", help);
            println!();

            loop {
                let token =
                    rpassword::read_password_from_tty(Some(&format!("{} Token: ", provider)))?;

                if token.is_empty() {
                    println!("Token must be non-empty.");
                } else {
                    break Ok(token);
                }
            }
        }
    }
}
