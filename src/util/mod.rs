mod cargo;
pub mod cli;
mod git;
pub mod ln;
mod path;
pub mod prompt;

pub use self::{cargo::*, git::*, path::*};

use self::cli::{Report, Reportable};
use crate::os::{self, command_path};
use once_cell_regex::{exports::regex::Captures, exports::regex::Regex, regex};
use serde::{ser::Serializer, Deserialize, Serialize};
use std::{
    error::Error as StdError,
    fmt::{self, Debug, Display},
    io::{self, Write},
    path::{Path, PathBuf},
};
use thiserror::Error;

pub fn list_display(list: &[impl Display]) -> String {
    if list.len() == 1 {
        list[0].to_string()
    } else if list.len() == 2 {
        format!("{} and {}", list[0], list[1])
    } else {
        let mut display = String::new();
        for (idx, item) in list.iter().enumerate() {
            let formatted = if idx + 1 == list.len() {
                // this is the last item
                format!("and {}", item)
            } else {
                format!("{}, ", item)
            };
            display.push_str(&formatted);
        }
        display
    }
}

pub fn reverse_domain(domain: &str) -> String {
    domain.split('.').rev().collect::<Vec<_>>().join(".")
}

pub fn rustup_add(triple: &str) -> bossy::Result<bossy::ExitStatus> {
    bossy::Command::impure("rustup")
        .with_args(&["target", "add", triple])
        .run_and_wait()
}

#[derive(Debug)]
pub enum HostTargetTripleError {
    CommandFailed(RunAndSearchError),
}

impl Reportable for HostTargetTripleError {
    fn report(&self) -> Report {
        match self {
            Self::CommandFailed(err) => Report::error("Failed to detect host target triple", err),
        }
    }
}

pub fn host_target_triple() -> Result<String, HostTargetTripleError> {
    // TODO: add fast paths
    run_and_search(
        &mut bossy::Command::impure_parse("rustc --verbose --version"),
        regex!(r"host: ([\w-]+)"),
        |_text, caps| {
            let triple = caps[1].to_owned();
            log::info!("detected host target triple {:?}", triple);
            triple
        },
    )
    .map_err(HostTargetTripleError::CommandFailed)
}

#[derive(Debug, Error)]
pub enum VersionTripleError {
    #[error("Failed to parse major version from {version:?}: {source}")]
    MajorInvalid {
        version: String,
        source: std::num::ParseIntError,
    },
    #[error("Failed to parse minor version from {version:?}: {source}")]
    MinorInvalid {
        version: String,
        source: std::num::ParseIntError,
    },
    #[error("Failed to parse patch version from {version:?}: {source}")]
    PatchInvalid {
        version: String,
        source: std::num::ParseIntError,
    },
    #[error(
        "Failed to parse version string {version:?}: string must be in format <major>[.minor][.patch]"
    )]
    VersionStringInvalid { version: String },
}

macro_rules! parse {
    ($key:expr, $err:ident, $variant:ident, $field:ident) => {
        |caps: &Captures<'_>, context: &str| {
            caps[$key].parse::<u32>().map_err(|source| $err::$variant {
                $field: context.to_owned(),
                source,
            })
        }
    };
}

// Generic version triple
#[derive(Clone, Copy, Debug, Eq, Ord, PartialEq, PartialOrd)]
pub struct VersionTriple {
    pub major: u32,
    pub minor: u32,
    pub patch: u32,
}

impl Display for VersionTriple {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}.{}.{}", self.major, self.minor, self.patch)
    }
}

impl Serialize for VersionTriple {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        serializer.collect_str(self)
    }
}

impl VersionTriple {
    pub const fn new(major: u32, minor: u32, patch: u32) -> Self {
        Self {
            major,
            minor,
            patch,
        }
    }

    pub fn from_caps<'a>(caps: &'a Captures<'a>) -> Result<(Self, &'a str), VersionTripleError> {
        let version_str = &caps["version"];
        Ok((
            Self {
                major: parse!("major", VersionTripleError, MajorInvalid, version)(
                    &caps,
                    version_str,
                )?,
                minor: parse!("minor", VersionTripleError, MinorInvalid, version)(
                    &caps,
                    version_str,
                )?,
                patch: parse!("patch", VersionTripleError, PatchInvalid, version)(
                    &caps,
                    version_str,
                )?,
            },
            version_str,
        ))
    }

    pub fn from_split(
        split: &mut std::str::Split<&str>,
        version: &str,
    ) -> Result<Self, VersionTripleError> {
        Ok(VersionTriple {
            major: split.next().unwrap().parse().map_err(|source| {
                VersionTripleError::MajorInvalid {
                    version: version.to_owned(),
                    source,
                }
            })?,
            minor: split.next().unwrap().parse().map_err(|source| {
                VersionTripleError::MinorInvalid {
                    version: version.to_owned(),
                    source,
                }
            })?,
            patch: split.next().unwrap().parse().map_err(|source| {
                VersionTripleError::PatchInvalid {
                    version: version.to_owned(),
                    source,
                }
            })?,
        })
    }

    pub fn from_str(v: &str) -> Result<Self, VersionTripleError> {
        match v.split(".").count() {
            1 => Ok(VersionTriple {
                major: v
                    .parse()
                    .map_err(|source| VersionTripleError::MajorInvalid {
                        version: v.to_owned(),
                        source,
                    })?,
                minor: 0,
                patch: 0,
            }),
            2 => {
                let mut s = v.split(".");
                Ok(VersionTriple {
                    major: s.next().unwrap().parse().map_err(|source| {
                        VersionTripleError::MajorInvalid {
                            version: v.to_owned(),
                            source,
                        }
                    })?,
                    minor: s.next().unwrap().parse().map_err(|source| {
                        VersionTripleError::MinorInvalid {
                            version: v.to_owned(),
                            source,
                        }
                    })?,
                    patch: 0,
                })
            }
            3 => {
                let mut s = v.split(".");
                Self::from_split(&mut s, v)
            }
            _ => Err(VersionTripleError::VersionStringInvalid {
                version: v.to_owned(),
            }),
        }
    }
}

#[derive(Debug, Error)]
pub enum VersionDoubleError {
    #[error("Failed to parse major version from {version:?}: {source}")]
    MajorInvalid {
        version: String,
        source: std::num::ParseIntError,
    },
    #[error("Failed to parse minor version from {version:?}: {source}")]
    MinorInvalid {
        version: String,
        source: std::num::ParseIntError,
    },
    #[error(
        "Failed to parse version string {version:?}: string must be in format <major>[.minor]"
    )]
    VersionStringInvalid { version: String },
}

// Generic version double
#[derive(Clone, Copy, Debug, Eq, Ord, PartialEq, PartialOrd)]
pub struct VersionDouble {
    pub major: u32,
    pub minor: u32,
}

impl Display for VersionDouble {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}.{}", self.major, self.minor)
    }
}

impl Serialize for VersionDouble {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        serializer.collect_str(self)
    }
}

impl VersionDouble {
    pub const fn new(major: u32, minor: u32) -> Self {
        Self { major, minor }
    }

    pub fn from_str(v: &str) -> Result<Self, VersionDoubleError> {
        match v.split(".").count() {
            1 => Ok(VersionDouble {
                major: v
                    .parse()
                    .map_err(|source| VersionDoubleError::MajorInvalid {
                        version: v.to_owned(),
                        source,
                    })?,
                minor: 0,
            }),
            2 => {
                let mut s = v.split(".");
                Ok(VersionDouble {
                    major: s.next().unwrap().parse().map_err(|source| {
                        VersionDoubleError::MajorInvalid {
                            version: v.to_owned(),
                            source,
                        }
                    })?,
                    minor: s.next().unwrap().parse().map_err(|source| {
                        VersionDoubleError::MinorInvalid {
                            version: v.to_owned(),
                            source,
                        }
                    })?,
                })
            }
            _ => Err(VersionDoubleError::VersionStringInvalid {
                version: v.to_owned(),
            }),
        }
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub struct Pod {
    name: String,
    version: Option<String>,
}

#[derive(Debug, Error)]
pub enum RustVersionError {
    #[error("Failed to check rustc version: {0}")]
    CommandFailed(#[from] RunAndSearchError),
    #[error(transparent)]
    TripleInvalid(#[from] VersionTripleError),
    #[error("Failed to parse rustc release year from {date:?}: {source}")]
    YearInvalid {
        date: String,
        source: std::num::ParseIntError,
    },
    #[error("Failed to parse rustc release month from {date:?}: {source}")]
    MonthInvalid {
        date: String,
        source: std::num::ParseIntError,
    },
    #[error("Failed to parse rustc release day from {date:?}: {source}")]
    DayInvalid {
        date: String,
        source: std::num::ParseIntError,
    },
}

impl Reportable for RustVersionError {
    fn report(&self) -> Report {
        Report::error("Failed to check Rust version", self)
    }
}

#[derive(Debug)]
pub struct RustVersionFlavor {
    pub flavor: String,
    pub candidate: Option<String>,
}

#[derive(Debug)]
pub struct RustVersionDetails {
    pub hash: String,
    pub date: (u32, u32, u32),
}

#[derive(Debug)]
pub struct RustVersion {
    pub triple: VersionTriple,
    pub flavor: Option<RustVersionFlavor>,
    /// This section can be absent if Rust is installed using something other
    /// than `rustup` (i.e. Homebrew)
    pub details: Option<RustVersionDetails>,
}

impl Display for RustVersion {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.triple)?;
        if let Some(flavor) = &self.flavor {
            write!(f, "-{}", flavor.flavor)?;
            if let Some(candidate) = &flavor.candidate {
                write!(f, ".{}", candidate)?;
            }
        }
        if let Some(details) = &self.details {
            write!(
                f,
                " ({} {}-{}-{})",
                details.hash, details.date.0, details.date.1, details.date.2
            )?;
        }
        Ok(())
    }
}

impl RustVersion {
    pub fn check() -> Result<Self, RustVersionError> {
        run_and_search(
            &mut bossy::Command::impure_parse("rustc --version"),
            regex!(
                r"rustc (?P<version>(?P<major>\d+)\.(?P<minor>\d+)\.(?P<patch>\d+)(-(?P<flavor>\w+)(.(?P<candidate>\d+))?)?)(?P<details> \((?P<hash>\w{9}) (?P<date>(?P<year>\d{4})-(?P<month>\d{2})-(?P<day>\d{2}))\))?"
            ),
            |_text, caps| {
                let (triple, _version_str) = VersionTriple::from_caps(&caps)?;
                let this = Self {
                    triple,
                    flavor: caps.name("flavor").map(|flavor| RustVersionFlavor {
                        flavor: flavor.as_str().to_owned(),
                        candidate: caps
                            .name("candidate")
                            .map(|candidate| candidate.as_str().to_owned()),
                    }),
                    details: caps
                        .name("details")
                        .map(|_details| -> Result<_, RustVersionError> {
                            let date_str = &caps["date"];
                            Ok(RustVersionDetails {
                                hash: caps["hash"].to_owned(),
                                date: (
                                    parse!("year", RustVersionError, YearInvalid, date)(
                                        &caps, date_str,
                                    )?,
                                    parse!("month", RustVersionError, MonthInvalid, date)(
                                        &caps, date_str,
                                    )?,
                                    parse!("day", RustVersionError, DayInvalid, date)(
                                        &caps, date_str,
                                    )?,
                                ),
                            })
                        })
                        .transpose()?,
                };
                log::info!("detected rustc version {}", this);
                Ok(this)
            },
        )?
    }

    pub fn valid(&self) -> bool {
        if cfg!(target_os = "macos") {
            const LAST_GOOD_STABLE: VersionTriple = VersionTriple::new(1, 45, 2);
            const NEXT_GOOD_STABLE: VersionTriple = VersionTriple::new(1, 49, 0);
            const FIRST_GOOD_NIGHTLY: (u32, u32, u32) = (2020, 10, 24);

            let old_good = self.triple <= LAST_GOOD_STABLE;
            let new_good = self.triple >= NEXT_GOOD_STABLE
                && self
                    .details
                    .as_ref()
                    .map(|details| details.date >= FIRST_GOOD_NIGHTLY)
                    .unwrap_or_else(|| {
                        log::warn!("output of `rustc --version` didn't contain date info; continuing with the assumption that the release date is at least 2020-10-24");
                        true
                    });

            old_good || new_good
        } else {
            true
        }
    }
}

pub fn prepend_to_path(path: impl Display, base_path: impl Display) -> String {
    format!("{}:{}", path, base_path)
}

pub fn command_present(name: &str) -> bossy::Result<bool> {
    command_path(name).map(|_path| true).or_else(|err| {
        if err.code().is_some() {
            Ok(false)
        } else {
            Err(err)
        }
    })
}

#[derive(Debug)]
pub enum PipeError {
    TxCommandFailed(bossy::Error),
    RxCommandFailed(bossy::Error),
    PipeFailed(io::Error),
    WaitFailed(bossy::Error),
}

impl Display for PipeError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::TxCommandFailed(err) => write!(f, "Failed to run sending command: {}", err),
            Self::RxCommandFailed(err) => write!(f, "Failed to run receiving command: {}", err),
            Self::PipeFailed(err) => write!(f, "Failed to pipe output: {}", err),
            Self::WaitFailed(err) => {
                write!(f, "Failed to wait for receiving command to exit: {}", err)
            }
        }
    }
}

pub fn pipe(mut tx_command: bossy::Command, rx_command: bossy::Command) -> Result<bool, PipeError> {
    let tx_output = tx_command
        .run_and_wait_for_output()
        .map_err(PipeError::TxCommandFailed)?;
    if !tx_output.stdout().is_empty() {
        let mut rx_command = rx_command
            .with_stdin_piped()
            .with_stdout(bossy::Stdio::inherit())
            .run()
            .map_err(PipeError::RxCommandFailed)?;
        let pipe_result = rx_command
            .stdin()
            .expect("developer error: `rx_command` stdin not captured")
            .write_all(tx_output.stdout())
            .map_err(PipeError::PipeFailed);
        let wait_result = rx_command.wait_for_output().map_err(PipeError::WaitFailed);
        // We try to wait even if the pipe failed, but the pipe error has higher
        // priority than the wait error, since it's likely to be more relevant.
        pipe_result?;
        wait_result?;
        Ok(true)
    } else {
        Ok(false)
    }
}

#[derive(Debug, Error)]
pub enum RunAndSearchError {
    #[error(transparent)]
    CommandFailed(#[from] bossy::Error),
    #[error("{command:?} output failed to match regex: {output:?}")]
    SearchFailed { command: String, output: String },
}

pub fn run_and_search<T>(
    command: &mut bossy::Command,
    re: &Regex,
    f: impl FnOnce(&str, Captures<'_>) -> T,
) -> Result<T, RunAndSearchError> {
    let command_string = command.display().to_owned();
    Ok(command
        .run_and_wait_for_str(|output| {
            re.captures(output)
                .ok_or_else(|| RunAndSearchError::SearchFailed {
                    command: command_string,
                    output: output.to_owned(),
                })
                .map(|caps| f(output, caps))
        })
        .map_err(RunAndSearchError::from)??)
}

#[derive(Debug, Error)]
pub enum CaptureGroupError {
    #[error("Capture group {group:?} missing from string {string:?}")]
    InvalidCaptureGroup { group: String, string: String },
}

pub fn get_string_for_group(
    caps: &Captures<'_>,
    group: &str,
    string: &str,
) -> Result<String, CaptureGroupError> {
    Ok(caps
        .name(group)
        .ok_or_else(|| CaptureGroupError::InvalidCaptureGroup {
            group: group.to_string(),
            string: string.to_string(),
        })?
        .as_str()
        .to_owned())
}

#[derive(Debug)]
pub enum OpenInEditorError {
    DetectFailed(os::DetectEditorError),
    OpenFailed(os::OpenFileError),
}

impl Display for OpenInEditorError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::DetectFailed(err) => write!(f, "Failed to detect editor: {}", err),
            Self::OpenFailed(err) => write!(f, "Failed to open path in edtior: {}", err),
        }
    }
}

pub fn open_in_editor(path: impl AsRef<Path>) -> Result<(), OpenInEditorError> {
    let path = path.as_ref();
    os::Application::detect_editor()
        .map_err(OpenInEditorError::DetectFailed)?
        .open_file(path)
        .map_err(OpenInEditorError::OpenFailed)
}

#[derive(Debug, Error)]
pub enum InstalledCommitMsgError {
    #[error(transparent)]
    NoHomeDir(#[from] NoHomeDir),
    #[error("Failed to read version info from {path:?}: {source}")]
    ReadFailed { path: PathBuf, source: io::Error },
}

pub fn installed_commit_msg() -> Result<Option<String>, InstalledCommitMsgError> {
    let path = install_dir()?.join("commit");
    if path.is_file() {
        std::fs::read_to_string(&path)
            .map(Some)
            .map_err(|source| InstalledCommitMsgError::ReadFailed { path, source })
    } else {
        Ok(None)
    }
}

pub fn format_commit_msg(msg: String) -> String {
    format!("Contains commits up to {:?}", msg)
}

pub fn unwrap_either<T>(result: Result<T, T>) -> T {
    match result {
        Ok(t) | Err(t) => t,
    }
}

#[derive(Debug, Error)]
pub enum WithWorkingDirError<E>
where
    E: StdError,
{
    #[error("Failed to get current directory: {0}")]
    CurrentDirGetFailed(#[source] std::io::Error),
    #[error("Failed to set working directory {path:?}: {source}")]
    CurrentDirSetFailed {
        path: PathBuf,
        source: std::io::Error,
    },
    #[error(transparent)]
    CallbackFailed(#[from] E),
}

pub fn with_working_dir<T, E, IE>(
    working_dir: impl AsRef<Path>,
    f: impl FnOnce() -> Result<T, IE>,
) -> Result<T, WithWorkingDirError<E>>
where
    E: StdError,
    E: From<IE>,
{
    let working_dir = working_dir.as_ref();
    let current_dir = std::env::current_dir().map_err(WithWorkingDirError::CurrentDirGetFailed)?;
    std::env::set_current_dir(working_dir).map_err(|source| {
        WithWorkingDirError::CurrentDirSetFailed {
            path: working_dir.to_owned(),
            source,
        }
    })?;
    let result = f().map_err(E::from)?;
    std::env::set_current_dir(&current_dir).map_err(|source| {
        WithWorkingDirError::CurrentDirSetFailed {
            path: current_dir.into(),
            source,
        }
    })?;
    Ok(result)
}

#[derive(Clone, Debug, Deserialize, PartialEq)]
#[serde(untagged)]
pub enum OneOrMany<T: Debug> {
    One(T),
    Many(Vec<T>),
}

impl<T: Debug> From<OneOrMany<T>> for Vec<T> {
    fn from(from: OneOrMany<T>) -> Self {
        match from {
            OneOrMany::One(val) => vec![val],
            OneOrMany::Many(vec) => vec,
        }
    }
}

impl<T: Debug> Serialize for OneOrMany<T> {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        let serialized_str = match self {
            Self::One(one) => format!("{:?}", one),
            Self::Many(vec) => format!("{:?}", vec),
        };
        serializer.serialize_str(&serialized_str)
    }
}
