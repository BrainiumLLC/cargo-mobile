use super::{installed_with_brew, installed_with_gem, PACKAGES};
use serde::Deserialize;
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
    pub fn load() -> Result<Self, OutdatedError> {
        #[derive(Deserialize)]
        struct Raw {
            formulae: Vec<Formula>,
        }

        let package_names = PACKAGES
            .iter()
            .map(|info| info.pkg_name)
            .collect::<Vec<_>>();

        let gem_outdated = bossy::Command::impure_parse("gem outdated")
            .run_and_wait_for_string()
            .map_err(OutdatedError::CommandFailed)?;
        let gem_outdated = gem_outdated
            .split("\n")
            .filter(|string| !string.is_empty())
            .collect::<Vec<_>>();
        let gem_needs_update = package_names
            .iter()
            .filter(|name| installed_with_gem(name) && gem_outdated.contains(name))
            .collect::<Vec<_>>();
        let mut gem_formulas = gem_needs_update
            .iter()
            .map(|string| parse_gem_outdated_string(string))
            .collect::<Result<Vec<_>, _>>()?;

        let mut brew_formulas = bossy::Command::impure_parse("brew outdated --json=v2")
            .run_and_wait_for_output()
            .map_err(OutdatedError::CommandFailed)
            .and_then(|output| serde_json::from_slice(output.stdout()).map_err(Into::into))
            .map(|Raw { formulae }| {
                formulae
                    .into_iter()
                    .filter(|formula| package_names.contains(&formula.name.as_str()))
                    .collect::<Vec<_>>()
            })?;

        brew_formulas.append(&mut gem_formulas);

        Ok(Self {
            packages: brew_formulas,
        })
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
    #[error("Failed to match string {string:?} using pattern {regex:?}")]
    MatchFailed { string: String, regex: regex::Regex },
    #[error("capture group {group} failed for string {string:?} using pattern {regex:?}")]
    InvalidCaptureGroup {
        string: String,
        regex: regex::Regex,
        group: usize,
    },
}

fn parse_gem_outdated_string(s: &str) -> Result<Formula, RegexError> {
    let regex =
        regex::Regex::new(r"(.*) \(([0-9]+.[0-9]+.[0-9]+) < ([0-9]+.[0-9]+.[0-9]+)\)").unwrap();
    let caps = regex.captures(s).ok_or_else(|| RegexError::MatchFailed {
        string: s.to_string(),
        regex: regex.clone(),
    })?;

    let group = 1;
    let name = caps
        .get(group)
        .ok_or_else(|| RegexError::InvalidCaptureGroup {
            string: s.to_string(),
            regex: regex.clone(),
            group,
        })?
        .as_str()
        .to_string();

    let group = 2;
    let installed_version = caps
        .get(group)
        .ok_or_else(|| RegexError::InvalidCaptureGroup {
            string: s.to_string(),
            regex: regex.clone(),
            group,
        })?
        .as_str()
        .parse()
        .unwrap();

    let group = 3;
    let current_version = caps
        .get(group)
        .ok_or_else(|| RegexError::InvalidCaptureGroup {
            string: s.to_string(),
            regex,
            group,
        })?
        .as_str()
        .parse()
        .unwrap();

    Ok(Formula {
        name,
        installed_versions: vec![installed_version],
        current_version,
    })
}
