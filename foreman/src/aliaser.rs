use std::{
    env::{self, consts::EXE_SUFFIX},
    path::Path,
};

use crate::{
    error::{ForemanError, ForemanResult},
    fs,
};

pub fn add_self_alias(name: &str, bin_path: &Path) -> ForemanResult<()> {
    let foreman_path = env::current_exe().map_err(|err| {
        ForemanError::io_error_with_context(err, "unable to obtain foreman executable location")
    })?;
    let mut alias_path = bin_path.to_owned();
    alias_path.push(format!("{}{}", name, EXE_SUFFIX));

    fs::copy(foreman_path, alias_path).map(|_| ())
}
