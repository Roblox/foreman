# Foreman Changelog

## Unreleased
- Fixed Foreman not propagating error codes from underlying tools. ([#20](https://github.com/rojo-rbx/foreman/pull/20))

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