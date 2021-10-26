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

static PACKAGES: &[&str] = &["xcodegen", "ios-deploy"];
static COCOAPODS: &str = "cocoapods";
static COCOAPODS_INSTALLED_NAME: &str = "pod";

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

pub fn install(package: &'static str, reinstall_deps: opts::ReinstallDeps) -> Result<bool, Error> {
    install_with_installed_name(package, package, reinstall_deps)
}

pub fn install_with_installed_name(
    package: &'static str,
    installed_name: &'static str,
    reinstall_deps: opts::ReinstallDeps,
) -> Result<bool, Error> {
    if !package_found(installed_name)? || reinstall_deps.yes() {
        println!("Installing `{}`...", package);
        // reinstall works even if it's not installed yet, and will upgrade
        // if it's already installed!
        bossy::Command::impure_parse("brew reinstall")
            .with_arg(package)
            .run_and_wait()
            .map_err(|source| Error::InstallFailed { package, source })?;
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
    let cocoapods_installed_with_brew = cocoapods_installed_with_brew(COCOAPODS);
    let all_brew_packages = if cocoapods_installed_with_brew {
        [PACKAGES, &[COCOAPODS]].concat()
    } else {
        PACKAGES.to_vec()
    };
    for package in &all_brew_packages {
        install(package, reinstall_deps)?;
    }
    let outdated = Outdated::load(&all_brew_packages)?;
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
            for package in outdated.iter(all_brew_packages) {
                bossy::Command::impure_parse("brew upgrade")
                    .with_arg(package)
                    .run_and_wait()
                    .map_err(|source| Error::InstallFailed { package, source })?;
            }
        }
    }
    {
        if cocoapods_installed_with_brew {
            install_with_installed_name(COCOAPODS, COCOAPODS_INSTALLED_NAME, reinstall_deps)?;
        } else {
            if !package_found(COCOAPODS_INSTALLED_NAME)? || reinstall_deps.yes() {
                install_gem(COCOAPODS)?;
            }
        }
        let outdated_gems = bossy::Command::impure_parse("gem outdated")
            .with_arg(COCOAPODS)
            .with_stderr_null()
            .run_and_wait_for_string()
            .map_err(|source| Error::InstallFailed {
                package: COCOAPODS,
                source,
            })?;
        if outdated_gems.contains(COCOAPODS) && non_interactive.no() {
            let answer = loop {
                if let Some(answer) = prompt::yes_no(
                    format!(
                        "Would you like the {:?} dependency to be updated for you?",
                        COCOAPODS
                    ),
                    Some(prompt::YesOrNo::Yes),
                )? {
                    break answer;
                }
            };
            if answer.yes() {
                install_gem(COCOAPODS)?;
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

fn install_gem(package: &'static str) -> Result<(), Error> {
    println!("Installing `{}`...", package);
    bossy::Command::impure_parse("sudo gem install")
        .with_arg(package)
        .with_stderr_null()
        .run_and_wait()
        .map_err(|source| Error::InstallFailed {
            package: package,
            source,
        })?;
    Ok(())
}

fn cocoapods_installed_with_brew(package: &str) -> bool {
    bossy::Command::impure_parse("brew list")
        .with_arg(package)
        .run_and_wait_for_output()
        .is_ok()
}
