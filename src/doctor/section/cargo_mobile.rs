use super::{Item, Section};
use crate::{
    doctor::Unrecoverable,
    util::{self, cli::VERSION_SHORT},
};
use once_cell_regex::regex;

#[cfg(target_os = "macos")]
fn check_os() -> Item {
    Item::from_result(
        util::run_and_search(
            &mut bossy::Command::impure_parse("system_profiler SPSoftwareDataType"),
            regex!(r"macOS (?P<version>.*)"),
            |_output, caps| caps.name("version").unwrap().as_str().to_owned(),
        )
        .map(|version| format!("macOS v{}", version)),
    )
}

#[cfg(target_os = "linux")]
fn check_os() -> Item {
    todo!()
}

fn check_rust() -> Result<String, String> {
    util::RustVersion::check()
        .map_err(|err| err.to_string())
        .and_then(|version| {
            version
                .valid()
                .then(|| format!("rustc v{}", version.to_string()))
                .ok_or_else(|| {
                    format!(
                        "iOS linking is broken on rustc v{}; please update to 1.49.0 or later",
                        version
                    )
                })
        })
}

pub fn check() -> Result<Section, Unrecoverable> {
    let section = Section::new(format!("cargo-mobile {}", VERSION_SHORT));
    Ok(match util::install_dir() {
        Ok(install_dir) => section
            .with_item(util::installed_commit_msg().map(|msg| {
                msg.map(util::format_commit_msg)
                    .unwrap_or_else(|| "Installed commit message isn't present".to_string())
            }))
            .with_item(if install_dir.exists() {
                Ok(format!(
                    "Installed at {:?}",
                    util::contract_home(&install_dir)?,
                ))
            } else {
                Err(format!(
                    "The cargo-mobile installation directory is missing! Checked at {:?}",
                    install_dir,
                ))
            }),
        Err(err) => section.with_failure(err),
    }
    .with_item(check_os())
    .with_item(check_rust()))
}
