use std::{
    env,
    fs::OpenOptions,
    io::{self, Write},
    path::{Path, PathBuf},
    process::Command,
};

pub fn is_in_path<P: AsRef<Path>>(path: P) -> bool {
    let target = match path.as_ref().canonicalize() {
        Ok(p) => p,
        Err(_) => return false,
    };

    if let Some(paths) = env::var_os("PATH") {
        for entry in env::split_paths(&paths) {
            if entry.is_dir() {
                if let Ok(canon) = entry.canonicalize() {
                    if canon == target {
                        return true;
                    }
                }
            }
        }
    }

    false
}

pub fn add_to_path<P: AsRef<Path>>(path: P) -> io::Result<()> {
    // Ensure directory exists before canonicalizing
    std::fs::create_dir_all(&path)?;

    let canon = path.as_ref()
        .canonicalize()
        .map_err(|e| io::Error::new(io::ErrorKind::Other,
            format!("failed to canonicalize '{}': {}", path.as_ref().display(), e)))?;

    _add_to_path(&canon)
}

#[cfg(target_os = "windows")]
fn _add_to_path(dir: &Path) -> io::Result<()> {
    // To modify for all users use `/M` and require admin
    let dir_str = dir.display().to_string();
    let value = format!("%PATH%;{}", dir_str);
    Command::new("setx")
        .args(&["PATH", &value])
        .status()
        .map(|_| ())
        .map_err(|e| io::Error::new(io::ErrorKind::Other, e))
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