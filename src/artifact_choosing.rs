#[cfg(target_os = "windows")]
static PLATFORM_KEYWORDS: &[&str] = &["win32", "win64", "windows"];

#[cfg(target_os = "macos")]
static PLATFORM_KEYWORDS: &[&str] = &["macos", "darwin"];

#[cfg(target_os = "linux")]
static PLATFORM_KEYWORDS: &[&str] = &["linux"];

pub fn platform_keywords() -> &'static [&'static str] {
    PLATFORM_KEYWORDS
}
