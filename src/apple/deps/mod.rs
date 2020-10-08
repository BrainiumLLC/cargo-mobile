mod update;
mod xcode_plugin;

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

static PACKAGES: &[Package] = &[
    Package::from_name("xcodegen"),
    Package::from_name("ios-deploy"),
    Package::from_name_and_tap("zld", "michaeleisel/zld"),
];

#[derive(Clone, Copy, Debug)]
struct Package {
    name: &'static str,
    tap: Option<&'static str>,
}

impl Package {
    const fn from_name(name: &'static str) -> Self {
        Self { name, tap: None }
    }

    const fn from_name_and_tap(name: &'static str, tap: &'static str) -> Self {
        Self { name, tap: Some(tap) }
    }

    fn present(&self) -> Result<bool, Error> {
        let present = util::command_present(self.name)
            .map_err(|source| Error::PresenceCheckFailed { package: self.name, source })?;
        log::info!("`{}` command {}", self.name, if present { "present" } else { "absent" });
        Ok(present)
    }

    fn install(&self) -> Result<(), Error> {
        println!("Installing `{}`...", self.name);
        if let Some(tap) = self.tap {
            bossy::Command::impure_parse("brew tap")
                .with_arg(tap)
                .run_and_wait()
                .map_err(|source| Error::TapFailed { tap, source })?;
        }
        // reinstall works even if it's not installed yet, and will upgrade
        // if it's already installed!
        bossy::Command::impure_parse("brew reinstall")
            .with_arg(self.name)
            .run_and_wait()
            .map_err(|source| Error::InstallFailed { package: self.name, source })?;
        Ok(())
    }

    fn upgrade(&self) -> Result<(), Error> {
        bossy::Command::impure_parse("brew upgrade")
            .with_arg(self.name)
            .run_and_wait()
            .map_err(|source| Error::InstallFailed { package: self.name, source })?;
        Ok(())
    }
}

#[derive(Debug, Error)]
pub enum Error {
    #[error(transparent)]
    OutdatedFailed(#[from] OutdatedError),
    #[error("Failed to check for presence of `{package}`: {source}")]
    PresenceCheckFailed {
        package: &'static str,
        source: bossy::Error,
    },
    #[error("Failed to tap `{tap}`: {source}")]
    TapFailed {
        tap: &'static str,
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

pub fn install(
    wrapper: &TextWrapper,
    non_interactive: opts::NonInteractive,
    skip_dev_tools: opts::SkipDevTools,
    reinstall_deps: opts::ReinstallDeps,
    host_rustflags: &mut Vec<String>,
) -> Result<(), Error> {
    for package in PACKAGES {
        if !package.present()? || reinstall_deps.yes() {
            package.install()?;
        }
    }
    // Speed up linking substantially on macOS
    host_rustflags.push("-Clink-arg=-fuse-ld=/usr/local/bin/zld".to_owned());
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
                package.upgrade()?;
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
