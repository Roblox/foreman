# Foreman Changelog

## Unreleased

## 1.2.0 (2023-08-02)
- Take into account architecture when downloading binaries for Windows to fix incorrect download of windows-aarch64 assets on win64 hosts ([#71](https://github.com/Roblox/foreman/pull/71))
- Support all Tier 1 Rust supported platforms {windows, linux, macos}-{x86_64, i686, aarch64} ([#71](https://github.com/Roblox/foreman/pull/71))

## 1.1.0 (2022-09-28)

- Support `macos-aarch64` as an artifact name for Apple Silicon (arm64) binaries ([#60](https://github.com/Roblox/foreman/pull/60))
- Take into account architecture when downloading binaries for linux ([#59](https://github.com/Roblox/foreman/pull/59))

## 1.0.5 (2022-05-25)

- Fix artifact selection bug affecting mac os tool downloads ([#53](https://github.com/Roblox/foreman/pull/53))

## 1.0.4 (2022-05-13)

- Introduce improved error output on using uninstalled tools ([#51](https://github.com/Roblox/foreman/pull/51))
- Add support for Apple Silicon (arm64) binaries ([#46](https://github.com/Roblox/foreman/pull/46))

## 1.0.3 (2022-02-04)

- Report correct exit code ([#41](https://github.com/Roblox/foreman/pull/41))
- Improve error handling to reduces crashes and add more useful error messages ([#40](https://github.com/Roblox/foreman/pull/40))
- Add environment variable to override Foreman home directory ([#39](https://github.com/Roblox/foreman/pull/39))
- Support tools hosted on GitLab ([#31](https://github.com/Roblox/foreman/pull/31))
  - Updated config format to support both GitHub and GitLab tools
  - Added `foreman gitlab-auth` command for authenticating with GitLab.
- Logging improvements ([#30](https://github.com/Roblox/foreman/pull/30))
	- Add commandline option to increase logging level (`-v`, `-vv`, etc)
	- Add an INFO-level log explaining when a release version tag name doesn't match expected convention.
	- Default logging to INFO level. Fixes ([#27]https://github.com/Roblox/foreman/issues/27).

## 1.0.2 (2020-05-20)
- Fixed Foreman not propagating error codes from underlying tools. ([#20](https://github.com/Roblox/foreman/pull/20))

## 1.0.1
- Metadata fix for crates.io release

## 1.0.0
- No changes since 0.6.0.
- Initial release on crates.io.

## 0.6.0
- Added `foreman github-auth` command for authenticating with GitHub.

## 0.5.1
- On Unix systems, tools now always have permissions of 777.
	- This ensures that they're executable, even when the containing archives fail to preserve permissions.

## 0.5.0
- Initial release
- Version number chosen so that Foreman can take over the [foreman](https://crates.io/crates/foreman) crate on crates.io and leave existing versions alone.
