use super::{
    util::{self, CaptureGroupError},
    GemCache, PACKAGES,
};
use once_cell_regex::regex;
use serde::Deserialize;
use std::collections::hash_set::HashSet;
use thiserror::Error;

#[derive(Debug, Error)]
pub enum OutdatedError {
    #[error("Failed to check for outdated packages: {0}")]
    CommandFailed(#[from] bossy::Error),
    #[error("Failed to parse outdated package list: {0}")]
    ParseFailed(#[from] serde_json::Error),
    #[error(transparent)]
    RegexError(#[from] RegexError),
}

#[derive(Debug, Deserialize)]
struct Formula {
    name: String,
    installed_versions: Vec<String>,
    current_version: String,
    // pinned: bool,
    // pinned_version: Option<String>,
}

impl Formula {
    fn print_notice(&self) {
        if self.installed_versions.len() == 1 {
            println!(
                "  - `{}` is at {}; latest version is {}",
                self.name, self.installed_versions[0], self.current_version
            );
        } else {
            println!(
                "  - `{}` is at [{}]; latest version is {}",
                self.name,
                self.installed_versions.join(", "),
                self.current_version
            );
        }
    }
}

#[derive(Debug)]
pub struct Outdated {
    packages: Vec<Formula>,
}

impl Outdated {
    pub fn load(gem_cache: &mut GemCache) -> Result<Self, OutdatedError> {
        #[derive(Deserialize)]
        struct Raw {
            formulae: Vec<Formula>,
        }

        let package_names = PACKAGES
            .iter()
            .map(|info| info.pkg_name)
            .collect::<HashSet<_>>();

        let gem_outdated = bossy::Command::impure_parse("gem outdated")
            .run_and_wait_for_string()
            .map_err(OutdatedError::CommandFailed)?;

        let packages = bossy::Command::impure_parse("brew outdated --json=v2")
            .run_and_wait_for_output()
            .map_err(OutdatedError::CommandFailed)
            .and_then(|output| serde_json::from_slice(output.stdout()).map_err(Into::into))
            .map(|Raw { formulae }| {
                formulae
                    .into_iter()
                    .filter(|formula| package_names.contains(&formula.name.as_str()))
                    .map(Ok)
            })?
            .chain(
                gem_outdated
                    .lines()
                    .filter(|name| {
                        !name.is_empty()
                            && gem_cache.contains_unchecked(name)
                            && gem_outdated.contains(name)
                    })
                    .map(parse_gem_outdated_string),
            )
            .collect::<Result<_, _>>()?;

        Ok(Self { packages })
    }

    pub fn iter(&self) -> impl Iterator<Item = &'static str> + '_ {
        self.packages.iter().map(|formula| {
            PACKAGES
                .iter()
                .map(|info| &info.pkg_name)
                // Do a switcheroo to get static lifetimes, just for the dubious
                // goal of not needing to use `String` in `deps::Error`...
                .find(|package| **package == formula.name.as_str())
                .copied()
                .expect("developer error: outdated package list should be a subset of `PACKAGES`")
        })
    }

    pub fn is_empty(&self) -> bool {
        self.packages.is_empty()
    }

    pub fn print_notice(&self) {
        if !self.is_empty() {
            println!("Outdated dependencies:");
            for package in self.packages.iter() {
                package.print_notice();
            }
        } else {
            println!("Apple dependencies are up to date");
        }
    }
}

#[derive(Debug, Error)]
pub enum RegexError {
    #[error("Failed to match regex in string {revision:?}")]
    SearchFailed { revision: String },
    #[error(transparent)]
    InvalidCaptureGroup(#[from] CaptureGroupError),
}

fn parse_gem_outdated_string(revision: &str) -> Result<Formula, RegexError> {
    let caps = regex!(r"(?P<name>.+) \((?P<installed_version>.+) < (?P<latest_version>.+)\)")
        .captures(revision)
        .ok_or_else(|| RegexError::SearchFailed {
            revision: revision.to_owned(),
        })?;

    let name = util::get_string_for_group(&caps, "name", revision)
        .map_err(RegexError::InvalidCaptureGroup)?;
    let installed_version = util::get_string_for_group(&caps, "installed_version", revision)
        .map_err(RegexError::InvalidCaptureGroup)?;
    let current_version = util::get_string_for_group(&caps, "current_version", revision)
        .map_err(RegexError::InvalidCaptureGroup)?;

    Ok(Formula {
        name,
        installed_versions: vec![installed_version],
        current_version,
    })
}
