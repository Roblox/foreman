mod aliaser;
mod artifact_choosing;
mod config;
mod github;
mod paths;
mod tool_cache;

use std::{env, error::Error, io, path::Path};

use structopt::StructOpt;

use crate::{aliaser::add_self_alias, config::ConfigFile, tool_cache::ToolCache};

#[derive(Debug)]
struct ToolInvocation {
    name: String,
    args: Vec<String>,
}

impl ToolInvocation {
    fn from_args() -> Option<Self> {
        let mut all_args = env::args();

        let app_path = all_args.next()?;
        let as_path = Path::new(&app_path);
        let name = as_path.file_stem()?.to_str()?.to_owned();

        // That's us!
        if name == "foreman" {
            return None;
        }

        let args = all_args.collect();

        Some(Self { name, args })
    }
}

fn main() -> Result<(), Box<dyn Error>> {
    let env = env_logger::Env::new().default_filter_or("foreman=info");
    env_logger::Builder::from_env(env)
        .format_module_path(false)
        .format_timestamp(None)
        .init();

    paths::create()?;

    if let Some(invocation) = ToolInvocation::from_args() {
        let config = ConfigFile::aggregate()?;

        if let Some(tool_spec) = config.tools.get(&invocation.name) {
            log::debug!("Found tool spec {}@{}", tool_spec.source, tool_spec.version);

            let maybe_version =
                ToolCache::download_if_necessary(&tool_spec.source, &tool_spec.version);

            if let Some(version) = maybe_version {
                ToolCache::run(&tool_spec.source, &version, invocation.args);
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
enum Options {
    /// Install tools defined by foreman.toml.
    Install,

    /// List installed tools.
    List,
}

fn actual_main() -> io::Result<()> {
    let options = Options::from_args();

    match options {
        Options::Install => {
            let config = ConfigFile::aggregate()?;

            log::trace!("Installing from gathered config: {:#?}", config);

            for (tool_alias, tool_spec) in &config.tools {
                ToolCache::download_if_necessary(&tool_spec.source, &tool_spec.version);
                add_self_alias(tool_alias);
            }
        }
        Options::List => {
            println!("Installed tools:");

            let cache = ToolCache::load().unwrap();

            for (tool_source, tool) in &cache.tools {
                println!("  {}", tool_source);

                for version in &tool.versions {
                    println!("    - {}", version);
                }
            }
        }
    }

    Ok(())
}
