mod update;
pub(crate) mod xcode_plugin;

use self::update::{Outdated, OutdatedError};
use super::system_profile::{self, DeveloperTools};
use crate::{
    opts,
    util::{
        self,
        cli::{Report, TextWrapper},
        prompt,
    },
};
use thiserror::Error;

pub enum PackageSource {
    Brew,
    BrewOrGem,
}

pub struct PackageData {
    pub pkg_name: &'static str,
    pub bin_name: &'static str,
    pub package_source: PackageSource,
}
static PACKAGES: &[PackageData] = &[
    PackageData {
        pkg_name: "xcodegen",
        bin_name: "xcodegen",
        package_source: PackageSource::Brew,
    },
    PackageData {
        pkg_name: "ios-deploy",
        bin_name: "ios-deploy",
        package_source: PackageSource::Brew,
    },
    PackageData {
        pkg_name: "cocoapods",
        bin_name: "pod",
        package_source: PackageSource::BrewOrGem,
    },
];

#[derive(Debug, Error)]
pub enum Error {
    #[error(transparent)]
    OutdatedFailed(#[from] OutdatedError),
    #[error("Failed to check for presence of `{package}`: {source}")]
    PresenceCheckFailed {
        package: &'static str,
        source: bossy::Error,
    },
    #[error("Failed to install `{package}`: {source}")]
    InstallFailed {
        package: &'static str,
        source: bossy::Error,
    },
    #[error("Failed to prompt to install updates: {0}")]
    PromptFailed(#[from] std::io::Error),
    #[error(transparent)]
    VersionLookupFailed(#[from] system_profile::Error),
}

pub fn package_found(package: &'static str) -> Result<bool, Error> {
    let found = util::command_present(package)
        .map_err(|source| Error::PresenceCheckFailed { package, source })?;
    log::info!("package `{}` present: {}", package, found);
    Ok(found)
}

pub fn install(package: &PackageData, reinstall_deps: opts::ReinstallDeps) -> Result<bool, Error> {
    if !package_found(package.bin_name)? || reinstall_deps.yes() {
        println!("Installing `{}`...", package.pkg_name);
        match package.package_source {
            PackageSource::Brew => brew_reinstall(package.pkg_name)?,
            PackageSource::BrewOrGem => {
                let package_updated = update_package(package.pkg_name)?;
                if !package_updated {
                    bossy::Command::impure_parse("sudo gem install")
                        .with_arg(package.pkg_name)
                        .with_stderr_null()
                        .run_and_wait()
                        .map_err(|source| Error::InstallFailed {
                            package: package.pkg_name,
                            source,
                        })?;
                }
            }
        }
        Ok(true)
    } else {
        Ok(false)
    }
}

pub fn install_all(
    wrapper: &TextWrapper,
    non_interactive: opts::NonInteractive,
    skip_dev_tools: opts::SkipDevTools,
    reinstall_deps: opts::ReinstallDeps,
) -> Result<(), Error> {
    for package in PACKAGES {
        install(package, reinstall_deps)?;
    }
    let outdated = Outdated::load()?;
    outdated.print_notice();
    if !outdated.is_empty() && non_interactive.no() {
        let answer = loop {
            if let Some(answer) = prompt::yes_no(
                "Would you like these outdated dependencies to be updated for you?",
                Some(prompt::YesOrNo::Yes),
            )? {
                break answer;
            }
        };
        if answer.yes() {
            for package in outdated.iter() {
                let package_updated = update_package(package)?;
                if !package_updated {
                    panic!("developer error: attemped updating package {:?}, which has not been installed", package);
                }
            }
        }
    }
    // we definitely don't want to install this on CI...
    if skip_dev_tools.no() {
        let tool_info = DeveloperTools::new()?;
        let result = xcode_plugin::install(wrapper, reinstall_deps, tool_info.version);
        if let Err(err) = result {
            // philosophy: never be so sturbborn as to prevent use / progress
            Report::action_request(
                "Failed to install Rust Xcode plugin; this component is optional, so init will continue anyway, but Xcode debugging won't work until this is resolved!",
                err,
            )
            .print(wrapper);
        }
    }
    Ok(())
}

fn installed_with_brew(package: &str) -> bool {
    bossy::Command::impure_parse("brew list")
        .with_arg(package)
        .run_and_wait_for_output()
        .is_ok()
}

fn installed_with_gem(package: &str) -> bool {
    let output = bossy::Command::impure_parse("brew list")
        .with_arg(package)
        .run_and_wait_for_string();

    output.is_ok() && output.unwrap().contains(package)
}

fn brew_reinstall(package: &'static str) -> Result<(), Error> {
    // reinstall works even if it's not installed yet, and will upgrade
    // if it's already installed!
    bossy::Command::impure_parse("brew reinstall")
        .with_arg(package)
        .run_and_wait()
        .map_err(|source| Error::InstallFailed { package, source })?;
    Ok(())
}

fn update_package(package: &'static str) -> Result<bool, Error> {
    if installed_with_brew(package) {
        brew_reinstall(package)?;
        Ok(true)
    } else if installed_with_gem(package) {
        bossy::Command::impure_parse("gem update")
            .with_arg(package)
            .run_and_wait()
            .map_err(|source| Error::InstallFailed { package, source })?;
        Ok(true)
    } else {
        Ok(false)
    }
}
