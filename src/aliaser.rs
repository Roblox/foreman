use std::{
    env::{self, consts::EXE_SUFFIX},
    fs,
};

use crate::paths;

pub fn add_self_alias(name: &str) {
    let foreman_path = env::args().next().unwrap();
    let mut alias_path = paths::bin_dir();
    alias_path.push(format!("{}{}", name, EXE_SUFFIX));

    fs::copy(foreman_path, alias_path).unwrap();
}
