use super::{Error, Item, Section};
use crate::util::{self, cli::VERSION_SHORT};
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

fn check_rust() -> Item {
    match util::RustVersion::check() {
        Ok(version) => {
            if version.valid() {
                Item::victory(format!("rustc v{}", version.to_string()))
            } else {
                Item::failure(Error::RustVersionInvalid { version })
            }
        }
        Err(err) => Item::failure(err),
    }
}

pub fn check() -> Section {
    let section = Section::new(format!("cargo-mobile {}", VERSION_SHORT));
    match util::install_dir() {
        Ok(install_dir) => section
            .with_item(match util::installed_commit_msg().transpose() {
                Some(result) => Item::from_result(result.map(util::format_commit_msg)),
                None => Item::victory("Installed commit message isn't present"),
            })
            .with_item(if install_dir.exists() {
                // TODO: don't unwrap here
                Item::victory(format!(
                    "Installed at {:?}",
                    util::contract_home(install_dir).unwrap()
                ))
            } else {
                Item::failure(format!(
                    "The cargo-mobile installation directory is missing! Checked at {:?}",
                    install_dir.to_str().unwrap()
                ))
            }),
        Err(err) => section.with_item(Item::failure(err)),
    }
    .with_item(check_os())
    .with_item(check_rust())
}
