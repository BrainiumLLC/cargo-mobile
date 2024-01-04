#![allow(unsafe_code)]

#[cfg(target_os = "macos")]
mod macos;

#[cfg(target_os = "macos")]
pub use self::macos::*;

#[cfg(target_os = "linux")]
mod linux;

#[cfg(target_os = "linux")]
pub use self::linux::*;

#[cfg(target_os = "windows")]
mod windows;

#[cfg(target_os = "windows")]
pub use self::windows::*;

// TODO: we should probably expose common functionality throughout `os` in a
// less ad-hoc way... since it's really easy to accidentally break things.
#[derive(Debug)]
pub struct Info {
    pub name: String,
    pub version: String,
}

impl Info {
    pub fn check() -> Result<Self, impl std::error::Error> {
        self::info::check()
    }
}
