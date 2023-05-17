#[cfg(all(target_os = "windows", target_arch = "x86_64"))]
static PLATFORM_KEYWORDS: &[&str] = &[
    "win64",
    "windows-x86_64",
    "windows"
];

#[cfg(all(target_os = "windows", target_arch = "i686"))]
static PLATFORM_KEYWORDS: &[&str] = &[
    "win32",
    "windows-i686"
];

#[cfg(all(target_os = "windows", target_arch = "aarch64"))]
static PLATFORM_KEYWORDS: &[&str] = &[
    "windows-aarch64"
];


#[cfg(all(target_os = "macos", target_arch = "x86_64"))]
static PLATFORM_KEYWORDS: &[&str] = &["macos-x86_64", "darwin-x86_64", "macos", "darwin"];

#[cfg(all(target_os = "macos", target_arch = "i686"))]
static PLATFORM_KEYWORDS: &[&str] = &["macos-i686", "darwin-i686"];

#[cfg(all(target_os = "macos", target_arch = "aarch64"))]
static PLATFORM_KEYWORDS: &[&str] = &[
    "macos-arm64",
    "darwin-arm64",
    "macos-aarch64",
    "darwin-aarch64",
    "macos",
    "darwin",
];

#[cfg(all(target_os = "linux", target_arch = "x86_64"))]
static PLATFORM_KEYWORDS: &[&str] = &["linux-x86_64", "linux"];

#[cfg(all(target_os = "linux", target_arch = "i686"))]
static PLATFORM_KEYWORDS: &[&str] = &["linux-i686"];

#[cfg(all(target_os = "linux", target_arch = "aarch64"))]
static PLATFORM_KEYWORDS: &[&str] = &["linux-arm64", "linux-aarch64"];

#[cfg(all(
    target_os = "linux",
    not(any(target_arch = "x86_64", target_arch = "aarch64", target_arch = "i686"))
))]
static PLATFORM_KEYWORDS: &[&str] = &["linux"];

pub fn platform_keywords() -> &'static [&'static str] {
    PLATFORM_KEYWORDS
}
