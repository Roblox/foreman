//Original source from https://github.com/LPGhatguy/aftman/blob/d3f8d1fac4c89d9163f8f3a0c97fa33b91294fea/src/process/windows.rs

//! On Windows, we use command_group to spawn processes in a job group that will
//! be automatically cleaned up when this process exits.

use std::io::{Error, ErrorKind};
use std::path::Path;
use std::process::Command;

use command_group::CommandGroup;

pub fn run(exe_path: &Path, args: Vec<String>) -> Result<i32, Error> {
    // On Windows, using a job group here will cause the subprocess to terminate
    // automatically when Aftman is terminated.
    let mut child = Command::new(exe_path)
        .args(args)
        .group_spawn()
        .map_err(|_| {
            Error::new(
                ErrorKind::Other,
                format!("Could not spawn {}", exe_path.display()),
            )
        })?;
    let status = child.wait()?;
    Ok(status.code().unwrap_or(1))
}
