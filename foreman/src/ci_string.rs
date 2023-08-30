use std::{
    fmt,
    hash::{Hash, Hasher},
};

use serde::{Deserialize, Serialize};

/// Case-insensitive string.
///
/// A string that acts case-insensitive when compared or hashed, but preserves
/// its case.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(transparent)]
pub struct CiString(pub String);

impl From<&str> for CiString {
    fn from(string: &str) -> Self {
        Self(string.to_owned())
    }
}

impl fmt::Display for CiString {
    fn fmt(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
        formatter.write_str(&self.0)
    }
}

impl Hash for CiString {
    fn hash<H: Hasher>(&self, state: &mut H) {
        let mut buf = [0; 4];
        for outer in self.0.chars() {
            for normalized in outer.to_lowercase() {
                normalized.encode_utf8(&mut buf);
                state.write(&buf);
            }
        }

        state.write_u8(0xff);
    }
}

impl PartialEq for CiString {
    fn eq(&self, other: &Self) -> bool {
        self.0
            .chars()
            .flat_map(char::to_lowercase)
            .eq(other.0.chars().flat_map(char::to_lowercase))
    }
}

impl Eq for CiString {}
