use std::{
    env,
    fs::OpenOptions,
    io::{self, Write},
    path::{Path, PathBuf},
    process::Command,
};

pub fn add_to_path<P: AsRef<Path>>(path: P) -> io::Result<()> {
    // Ensure directory exists before canonicalizing
    std::fs::create_dir_all(&path)?;
    
    let canon = path.as_ref().canonicalize().map_err(|e| {
        io::Error::new(
            io::ErrorKind::Other,
            format!(
                "failed to canonicalize '{}': {}",
                path.as_ref().display(),
                e
            ),
        )
    })?;

    _add_to_path(&canon)
}

#[cfg(target_os = "windows")]
fn _add_to_path(dir: &Path) -> io::Result<()> {
    use dunce;
    use winreg::enums::{HKEY_CURRENT_USER, KEY_READ, KEY_WRITE, REG_EXPAND_SZ};
    use winreg::{RegKey, RegValue};

    let dir_str = dunce::canonicalize(dir)?;
    let dir_str = dir_str.display().to_string();

    // Open handle to the Env variable
    let hkcu = RegKey::predef(HKEY_CURRENT_USER);
    let environment = hkcu.open_subkey_with_flags("Environment", KEY_READ | KEY_WRITE)?;
    let reg_value: RegValue = match environment.get_raw_value("Path") {
        Ok(v) => v,
        Err(e) if e.kind() == io::ErrorKind::NotFound => RegValue {
            // Construct empty object if none exists
            bytes: vec![],
            vtype: REG_EXPAND_SZ,
        },
        Err(e) => return Err(e),
    };

    // Windows handles paths in UTF-16 so we must convert what we have to UTF-16 too.
    let utf16_bytes = reg_value.bytes.chunks_exact(2);
    let mut path_chars = Vec::new();
    for chunk in utf16_bytes {
        let code_point = u16::from_le_bytes([chunk[0], chunk[1]]);
        if code_point == 0 {
            break;
        }
        path_chars.push(code_point);
    }

    let mut current_path = String::from_utf16(&path_chars)
        .map_err(|_| io::Error::new(io::ErrorKind::InvalidData, "Non-UTF16 data"))?
        .trim_end_matches('\0') //Trim the null terminator
        .to_string();

    // Check if the path is already added in %PATH%
    for path in env::split_paths(&current_path) {
        if let Ok(path) = dunce::canonicalize(path) {
            if path.display().to_string() == dir_str {
                return Ok(());
            }
        }
    }

    // paths in %PATH% are seperated using `;`. This ensures proper formatting
    if !current_path.is_empty() && !current_path.ends_with(';') {
        current_path.push(';');
    }
    current_path.push_str(&dir_str);
    

    //Replace the old object with the new one
    environment.set_raw_value(
        "Path",
        &RegValue {
            bytes: current_path
                .encode_utf16()
                .flat_map(|c| c.to_le_bytes())
                .collect(),
            vtype: REG_EXPAND_SZ,
        },
    )
}

#[cfg(unix)]
fn _add_to_path(dir: &Path) -> io::Result<()> {
    // For macOS also paths.d may be modified so I suggest looking more into it. Though it requires elevated permissions

    let shell = env::var("SHELL").unwrap_or_default();
    let mut rc = PathBuf::from(env::var("HOME").unwrap());
    if shell.contains("zsh") {
        rc.push(".zshrc");
    } else {
        // Prefer login file with fallback
        rc.push(".bash_profile");
    }
    let mut file = OpenOptions::new().create(true).append(true).open(&rc)?;
    writeln!(file, "export PATH=\"{}:$PATH\"", dir.display())?;
    Ok(())
}

