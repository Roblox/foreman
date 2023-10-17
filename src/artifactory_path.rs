use crate::error::{ForemanError, ForemanResult};
use semver::Version;
use std::io::{Error, ErrorKind};

// Redundant operating systems that Foreman recognizes are not included;
static VALID_OS: &[&str] = &["windows", "macos", "linux"];
static VALID_ARCH: &[&str] = &["x86_64", "arm64", "aarch64", "i686"];

pub fn generate_artifactory_path<S: Into<String>>(
    repo: S,
    tool_name: S,
    version: S,
    operating_system: S,
    architecture: Option<S>,
) -> ForemanResult<String> {
    let repo = repo.into();
    let tool_name = tool_name.into();
    let version = version.into();
    let operating_system = operating_system.into();

    check_valid_os(&operating_system)?;
    check_valid_version(&version)?;
    let mut full_tool_name = format!("{}-{}-{}", tool_name, version, operating_system);
    if let Some(architecture) = architecture {
        let architecture = architecture.into();
        check_valid_arch(&architecture)?;
        full_tool_name.push('-');
        full_tool_name.push_str(&architecture);
    }

    full_tool_name.push_str(".zip");

    Ok(format!(
        "artifactory/{}/{}/{}/{}",
        repo, tool_name, version, full_tool_name
    ))
}

fn check_valid_os(operating_system: &str) -> ForemanResult<()> {
    if !VALID_OS.contains(&operating_system) {
        return Err(ForemanError::io_error_with_context(
            Error::new(ErrorKind::InvalidInput, "Invalid Argument"),
            format!(
                "Invalid operating system: {}. Please input a valid operating system: {}",
                operating_system,
                VALID_OS.join(", ")
            ),
        ));
    } else {
        Ok(())
    }
}

fn check_valid_arch(architecture: &str) -> ForemanResult<()> {
    if !VALID_ARCH.contains(&architecture) {
        return Err(ForemanError::io_error_with_context(
            Error::new(ErrorKind::InvalidInput, "Invalid Argument"),
            format!(
                "Invalid architecture: {}. Please input a valid architecture: {}",
                architecture,
                VALID_ARCH.join(", ")
            ),
        ));
    } else {
        Ok(())
    }
}

fn check_valid_version(version: &str) -> ForemanResult<()> {
    if !version.starts_with('v') {
        return Err(ForemanError::io_error_with_context(
            Error::new(ErrorKind::InvalidInput, "Invalid Argument"),
            format!("Invalid version: {}. Versions must start with a v", version),
        ));
    }

    if let Err(err) = Version::parse(&version[1..]) {
        Err(ForemanError::io_error_with_context(
            Error::new(ErrorKind::InvalidInput, "Invalid Argument"),
            format!("Invalid version: {}. Error: {}", version, err),
        ))
    } else {
        Ok(())
    }
}

#[cfg(test)]
mod test {
    use super::generate_artifactory_path;

    #[test]
    fn simple_path() {
        let path = generate_artifactory_path("repo", "tool_name", "v0.1.0", "macos", None).unwrap();
        assert_eq!(
            path,
            "artifactory/repo/tool_name/v0.1.0/tool_name-v0.1.0-macos.zip"
        );
    }

    #[test]
    fn simple_path_with_arch() {
        let path = generate_artifactory_path("repo", "tool_name", "v0.1.0", "macos", Some("arm64"))
            .unwrap();
        assert_eq!(
            path,
            "artifactory/repo/tool_name/v0.1.0/tool_name-v0.1.0-macos-arm64.zip"
        );
    }

    #[test]
    fn invalid_version_no_v() {
        let path = generate_artifactory_path("repo", "tool_name", "0.1.0", "macos", Some("arm64"))
            .unwrap_err();
        assert_eq!(
            path.to_string(),
            "Invalid version: 0.1.0. Versions must start with a v: Invalid Argument".to_string()
        );
    }
    #[test]
    fn invalid_version_incomplete() {
        let path = generate_artifactory_path("repo", "tool_name", "v0.1", "macos", Some("arm64"))
            .unwrap_err();
        assert_eq!(
            path.to_string(),
            "Invalid version: v0.1. Error: unexpected end of input while parsing minor version number: Invalid Argument".to_string()
        );
    }

    #[test]
    fn invalid_operating_system() {
        let path =
            generate_artifactory_path("repo", "tool_name", "v0.1.0", "fake_os", Some("arm64"))
                .unwrap_err();
        assert_eq!(
            path.to_string(),
            "Invalid operating system: fake_os. Please input a valid operating system: windows, macos, linux: Invalid Argument".to_string()
        );
    }

    #[test]
    fn invalid_architecture() {
        let path =
            generate_artifactory_path("repo", "tool_name", "v0.1.0", "macos", Some("fake_arch"))
                .unwrap_err();
        assert_eq!(
            path.to_string(),
            "Invalid architecture: fake_arch. Please input a valid architecture: x86_64, arm64, aarch64, i686: Invalid Argument".to_string()
        );
    }
}
