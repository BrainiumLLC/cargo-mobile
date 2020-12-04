// #![cfg(target_os = "macos")]
#![forbid(unsafe_code)]

#[cfg(target_os = "macos")]
use cargo_mobile::{
    apple::{cli::Input, NAME},
    util::cli::exec,
};

#[cfg(target_os = "macos")]
fn main() {
    exec::<Input>(NAME)
}

#[cfg(not(target_os = "macos"))]
fn main() {
    panic!("Not supported outside of macos");
}
