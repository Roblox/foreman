#[cfg(all(target_os = "windows", target_arch = "x86"))]
static PLATFORM_KEYWORDS: &[&str] = &["win32", "windows"];

#[cfg(all(target_os = "windows", not(target_arch = "x86")))]
static PLATFORM_KEYWORDS: &[&str] = &[
	"win64",
	"windows",
	"win32",
];

#[cfg(all(target_os = "macos", target_arch = "x86_64"))]
static PLATFORM_KEYWORDS: &[&str] = &["macos-x86_64", "darwin-x86_64", "macos", "darwin"];

#[cfg(all(target_os = "macos", target_arch = "aarch64"))]
static PLATFORM_KEYWORDS: &[&str] = &[
    "macos-arm64",
    "darwin-arm64",
    "macos-x86_64",
    "darwin-x86_64",
    "macos",
    "darwin",
];

#[cfg(target_os = "linux")]
static PLATFORM_KEYWORDS: &[&str] = &["linux"];

pub fn platform_keywords() -> &'static [&'static str] {
    PLATFORM_KEYWORDS
}
