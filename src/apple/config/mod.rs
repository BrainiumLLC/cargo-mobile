mod raw;

pub use self::raw::*;

use crate::{
    config::app::App,
    util::{self, cli::Report, VersionDouble, VersionDoubleError},
};
use serde::{Deserialize, Serialize};
use std::{
    fmt::{self, Display},
    path::PathBuf,
};

static DEFAULT_PROJECT_DIR: &str = "gen/apple";
const DEFAULT_IOS_VERSION: VersionDouble = VersionDouble::new(9, 0);
const DEFAULT_MACOS_VERSION: VersionDouble = VersionDouble::new(11, 0);

#[derive(Debug, Default, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub struct Platform {
    features: Option<Vec<String>>,
    frameworks: Option<Vec<String>>,
    vendor_frameworks: Option<Vec<String>>,
    vendor_sdks: Option<Vec<String>>,
    asset_catalogs: Option<Vec<PathBuf>>,
    pods: Option<Vec<PathBuf>>,
    additional_targets: Option<Vec<PathBuf>>,
}

impl Platform {
    pub fn no_default_features(&self) -> bool {
        self.features.is_some()
    }

    pub fn features(&self) -> Option<&[String]> {
        self.features.as_deref()
    }

    pub fn frameworks(&self) -> &[String] {
        self.frameworks.as_deref().unwrap_or_else(|| &[])
    }

    pub fn vendor_frameworks(&self) -> &[String] {
        self.vendor_frameworks.as_deref().unwrap_or_else(|| &[])
    }

    pub fn vendor_sdks(&self) -> &[String] {
        self.vendor_sdks.as_deref().unwrap_or_else(|| &[])
    }

    pub fn asset_catalogs(&self) -> Option<&[PathBuf]> {
        self.asset_catalogs.as_deref()
    }

    pub fn pods(&self) -> Option<&[PathBuf]> {
        self.pods.as_deref()
    }
  
    pub fn additional_targets(&self) -> Option<&[PathBuf]> {
        self.additional_targets.as_deref()
    }
}

const fn default_true() -> bool {
    true
}

#[derive(Debug, Deserialize)]
pub struct Metadata {
    #[serde(default = "default_true")]
    supported: bool,
    #[serde(default)]
    ios: Platform,
    #[serde(default)]
    macos: Platform,
}

impl Default for Metadata {
    fn default() -> Self {
        Self {
            supported: true,
            ios: Default::default(),
            macos: Default::default(),
        }
    }
}

impl Metadata {
    pub const fn supported(&self) -> bool {
        self.supported
    }

    pub fn ios(&self) -> &Platform {
        &self.ios
    }

    pub fn macos(&self) -> &Platform {
        &self.macos
    }
}

#[derive(Debug)]
pub enum ProjectDirInvalid {
    NormalizationFailed {
        project_dir: String,
        cause: util::NormalizationError,
    },
    OutsideOfAppRoot {
        project_dir: String,
        root_dir: PathBuf,
    },
}

impl Display for ProjectDirInvalid {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::NormalizationFailed { project_dir, cause } => write!(
                f,
                "Xcode project dir {:?} couldn't be normalized: {}",
                project_dir, cause
            ),
            Self::OutsideOfAppRoot {
                project_dir,
                root_dir,
            } => write!(
                f,
                "Xcode project dir {:?} is outside of the app root dir {:?}",
                project_dir, root_dir,
            ),
        }
    }
}

#[derive(Debug)]
pub enum Error {
    DevelopmentTeamMissing,
    DevelopmentTeamEmpty,
    ProjectDirInvalid(ProjectDirInvalid),
    IosVersionInvalid(VersionDoubleError),
    MacOsVersionInvalid(VersionDoubleError),
}

impl Error {
    pub fn report(&self, msg: &str) -> Report {
        match self {
            Self::DevelopmentTeamMissing => Report::error(
                msg,
                format!("`{}.development-team` must be specified", super::NAME),
            ),
            Self::DevelopmentTeamEmpty => {
                Report::error(msg, format!("`{}.development-team` is empty", super::NAME))
            }
            Self::ProjectDirInvalid(err) => Report::error(
                msg,
                format!("`{}.project-dir` invalid: {}", super::NAME, err),
            ),
            Self::IosVersionInvalid(err) => Report::error(
                msg,
                format!("`{}.ios-version` invalid: {}", super::NAME, err),
            ),
            Self::MacOsVersionInvalid(err) => Report::error(
                msg,
                format!("`{}.macos-version` invalid: {}", super::NAME, err),
            ),
        }
    }
}

#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "kebab-case")]
pub struct Config {
    #[serde(skip_serializing)]
    app: App,
    development_team: String,
    project_dir: String,
    ios_version: VersionDouble,
    macos_version: VersionDouble,
    use_legacy_build_system: bool,
}

impl Config {
    pub fn from_raw(app: App, raw: Option<Raw>) -> Result<Self, Error> {
        let raw = raw.ok_or_else(|| Error::DevelopmentTeamMissing)?;

        if raw.development_team.is_empty() {
            return Err(Error::DevelopmentTeamEmpty);
        }

        let project_dir = raw
            .project_dir
            .map(|project_dir| {
                if project_dir == DEFAULT_PROJECT_DIR {
                    log::warn!("`{}.project-dir` is set to the default value; you can remove it from your config", super::NAME);
                }
                if util::under_root(&project_dir, app.root_dir())
                    .map_err(|cause| Error::ProjectDirInvalid(ProjectDirInvalid::NormalizationFailed {
                        project_dir: project_dir.clone(),
                        cause,
                    }))?
                {
                    Ok(project_dir)
                } else {
                    Err(Error::ProjectDirInvalid(ProjectDirInvalid::OutsideOfAppRoot {
                        project_dir,
                        root_dir: app.root_dir().to_owned(),
                    }))
                }
            }).unwrap_or_else(|| {
                log::info!(
                    "`{}.project-dir` not set; defaulting to {}",
                    super::NAME, DEFAULT_PROJECT_DIR
                );
                Ok(DEFAULT_PROJECT_DIR.to_owned())
            })?;
        Ok(Self {
            app,
            development_team: raw.development_team,
            project_dir,
            ios_version: raw
                .ios_version
                .map(|str| VersionDouble::from_str(&str))
                .transpose()
                .map_err(Error::IosVersionInvalid)?
                .unwrap_or(DEFAULT_IOS_VERSION),
            macos_version: raw
                .macos_version
                .map(|str| VersionDouble::from_str(&str))
                .transpose()
                .map_err(Error::IosVersionInvalid)?
                .unwrap_or(DEFAULT_MACOS_VERSION),
            use_legacy_build_system: raw.use_legacy_build_system.unwrap_or(true),
        })
    }

    pub fn app(&self) -> &App {
        &self.app
    }

    pub fn project_dir(&self) -> PathBuf {
        self.app.prefix_path(&self.project_dir)
    }

    pub fn project_dir_exists(&self) -> bool {
        self.project_dir().is_dir()
    }

    pub fn workspace_path(&self) -> PathBuf {
        let root_workspace = self
            .project_dir()
            .join(format!("{}.xcworkspace/", self.app.name()));
        if root_workspace.exists() {
            root_workspace
        } else {
            self.project_dir().join(format!(
                "{}.xcodeproj/project.xcworkspace/",
                self.app.name()
            ))
        }
    }

    pub fn archive_dir(&self) -> PathBuf {
        self.project_dir().join("build")
    }

    pub fn export_dir(&self) -> PathBuf {
        self.project_dir().join("build")
    }

    pub fn export_plist_path(&self) -> PathBuf {
        self.project_dir().join("ExportOptions.plist")
    }

    pub fn ipa_path(&self) -> Result<PathBuf, (PathBuf, PathBuf)> {
        let path = |tail: &str| self.export_dir().join(format!("{}.ipa", tail));
        let old = path(&self.scheme());
        // It seems like the format changed recently?
        let new = path(self.app.name());
        std::iter::once(&old)
            .chain(std::iter::once(&new))
            .filter(|path| {
                let found = path.is_file();
                log::info!("IPA {}found at {:?}", if found { "" } else { "not " }, path);
                found
            })
            .next()
            .cloned()
            .ok_or_else(|| (old, new))
    }

    pub fn app_path(&self) -> PathBuf {
        self.export_dir()
            .join(format!("Payload/{}.app", self.app.name()))
    }

    pub fn scheme(&self) -> String {
        format!("{}_iOS", self.app.name())
    }
}
