#![allow(unsafe_code)]

#[cfg(target_os = "macos")]
mod macos;

#[cfg(target_os = "macos")]
pub use self::macos::*;

#[cfg(target_os = "windows")]
mod windows;

#[cfg(target_os = "windows")]
pub use self::windows::*;

#[cfg(not(target_os = "macos"))]
#[cfg(not(target_os = "windows"))]
compile_error!("Host platform not yet supported by cargo-mobile! We'd love if you made a PR to add support for this platform ❤️");
