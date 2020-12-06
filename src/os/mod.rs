#![allow(unsafe_code)]

#[cfg(target_os = "macos")]
mod macos;

#[cfg(target_os = "macos")]
pub use self::macos::*;

#[cfg(target_os = "linux")]
mod linux;

#[cfg(target_os = "linux")]
pub use self::linux::*;

#[cfg(all(not(target_os = "macos"), not(target_os = "linux")))]
compile_error!("Host platform not yet supported by cargo-mobile! We'd love if you made a PR to add support for this platform ❤️");
