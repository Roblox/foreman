//Orignal source from https://github.com/LPGhatguy/aftman/blob/d3f8d1fac4c89d9163f8f3a0c97fa33b91294fea/src/process/mod.rs

#[cfg(windows)]
mod windows;

#[cfg(windows)]
pub use windows::run;

#[cfg(unix)]
mod unix;

#[cfg(unix)]
pub use unix::run;
