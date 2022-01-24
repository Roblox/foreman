use std::{
    env::{self, consts::EXE_SUFFIX},
    path::Path,
};

use crate::fs;

pub fn add_self_alias(name: &str, bin_path: &Path) {
    let foreman_path = env::current_exe().unwrap();
    let mut alias_path = bin_path.to_owned();
    alias_path.push(format!("{}{}", name, EXE_SUFFIX));

    fs::copy(foreman_path, alias_path).unwrap();
}
