pub mod aab;
pub mod adb;
pub mod apk;
mod bundletool;
pub mod cli;
pub mod config;
pub mod device;
pub mod env;
mod jnilibs;
pub mod ndk;
pub(crate) mod project;
mod source_props;
pub mod target;

pub static NAME: &str = "android";
