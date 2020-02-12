//! Contains all of the paths that Foreman needs to deal with.

use std::{fs, io, path::PathBuf};

pub fn base_dir() -> PathBuf {
    let mut dir = dirs::home_dir().unwrap();
    dir.push(".foreman");
    dir
}

pub fn tools_dir() -> PathBuf {
    let mut dir = base_dir();
    dir.push("tools");
    dir
}

pub fn bin_dir() -> PathBuf {
    let mut dir = base_dir();
    dir.push("bin");
    dir
}

pub fn user_config() -> PathBuf {
    let mut path = base_dir();
    path.push("foreman.toml");
    path
}

pub fn create() -> io::Result<()> {
    fs::create_dir_all(base_dir())?;
    fs::create_dir_all(bin_dir())?;
    fs::create_dir_all(tools_dir())?;

    Ok(())
}
