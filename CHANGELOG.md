# Foreman Changelog

## 0.5.1
- On Unix systems, tools now always have permissions of 777.
	- This ensures that they're executable, even when the containing archives fail to preserve permissions.

## 0.5.0
- Initial release
- Version number chosen so that Foreman can take over the [foreman](https://crates.io/crates/foreman) crate on crates.io and leave existing versions alone.