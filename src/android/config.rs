use crate::{
    config::app::App,
    util::{self, cli::Report},
};
use serde::{Deserialize, Serialize};
use std::{
    fmt::{self, Display},
    path::PathBuf,
};

const DEFAULT_MIN_SDK_VERSION: u32 = 24;
const DEFAULT_VULKAN_VALIDATION: bool = true;
static DEFAULT_PROJECT_DIR: &str = "gen/android";

const fn default_true() -> bool {
    true
}

#[derive(Debug, Deserialize)]
pub struct AssetPackInfo {
    pub name: String,
    pub delivery_type: String,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub struct Metadata {
    #[serde(default = "default_true")]
    supported: bool,
    features: Option<Vec<String>>,
    app_sources: Option<Vec<String>>,
    app_plugins: Option<Vec<String>>,
    project_dependencies: Option<Vec<String>>,
    app_dependencies: Option<Vec<String>>,
    app_dependencies_platform: Option<Vec<String>>,
    asset_packs: Option<Vec<AssetPackInfo>>,
}

impl Default for Metadata {
    fn default() -> Self {
        Self {
            supported: true,
            features: None,
            app_sources: None,
            app_plugins: None,
            project_dependencies: None,
            app_dependencies: None,
            app_dependencies_platform: None,
            asset_packs: None,
        }
    }
}

impl Metadata {
    pub const fn supported(&self) -> bool {
        self.supported
    }

    pub fn no_default_features(&self) -> bool {
        self.features.is_some()
    }

    pub fn features(&self) -> Option<&[String]> {
        self.features.as_deref()
    }

    pub fn app_sources(&self) -> &[String] {
        self.app_sources.as_deref().unwrap_or_else(|| &[])
    }

    pub fn app_plugins(&self) -> Option<&[String]> {
        self.app_plugins.as_deref()
    }

    pub fn project_dependencies(&self) -> Option<&[String]> {
        self.project_dependencies.as_deref()
    }

    pub fn app_dependencies(&self) -> Option<&[String]> {
        self.app_dependencies.as_deref()
    }

    pub fn app_dependencies_platform(&self) -> Option<&[String]> {
        self.app_dependencies_platform.as_deref()
    }

    pub fn asset_packs(&self) -> Option<&[AssetPackInfo]> {
        self.asset_packs.as_deref()
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
    ContainsSpaces {
        project_dir: String,
    },
}

impl Display for ProjectDirInvalid {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::NormalizationFailed { project_dir, cause } => {
                write!(f, "{:?} couldn't be normalized: {}", project_dir, cause)
            }
            Self::OutsideOfAppRoot {
                project_dir,
                root_dir,
            } => write!(
                f,
                "{:?} is outside of the app root {:?}",
                project_dir, root_dir,
            ),
            Self::ContainsSpaces { project_dir } => write!(
                f,
                "{:?} contains spaces, which the NDK is remarkably intolerant of",
                project_dir
            ),
        }
    }
}

#[derive(Debug)]
pub enum Error {
    ProjectDirInvalid(ProjectDirInvalid),
}

impl Error {
    pub fn report(&self, msg: &str) -> Report {
        match self {
            Self::ProjectDirInvalid(err) => Report::error(
                msg,
                format!("`{}.project-dir` invalid: {}", super::NAME, err),
            ),
        }
    }
}

#[derive(Clone, Debug, Default, Deserialize, Serialize)]
#[serde(rename_all = "kebab-case")]
pub struct Raw {
    min_sdk_version: Option<u32>,
    vulkan_validation: Option<bool>,
    project_dir: Option<String>,
    no_default_features: Option<bool>,
    features: Option<Vec<String>>,
}

#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "kebab-case")]
pub struct Config {
    #[serde(skip_serializing)]
    app: App,
    min_sdk_version: u32,
    vulkan_validation: bool,
    project_dir: PathBuf,
}

impl Config {
    pub fn from_raw(app: App, raw: Option<Raw>) -> Result<Self, Error> {
        let raw = raw.unwrap_or_default();

        let min_sdk_version = raw.min_sdk_version.unwrap_or_else(|| {
            log::info!(
                "`{}.min-sdk-version` not set; defaulting to {}",
                super::NAME,
                DEFAULT_MIN_SDK_VERSION
            );
            DEFAULT_MIN_SDK_VERSION
        });

        let vulkan_validation = raw.vulkan_validation.unwrap_or_else(|| {
            log::info!(
                "`{}.vulkan-validation` not set; defaulting to {}",
                super::NAME,
                DEFAULT_VULKAN_VALIDATION
            );
            DEFAULT_VULKAN_VALIDATION
        });

        let project_dir = if let Some(project_dir) = raw.project_dir {
            if project_dir == DEFAULT_PROJECT_DIR {
                log::warn!(
                    "`{}.project-dir` is set to the default value; you can remove it from your config",
                    super::NAME
                );
            }
            if util::under_root(&project_dir, app.root_dir()).map_err(|cause| {
                Error::ProjectDirInvalid(ProjectDirInvalid::NormalizationFailed {
                    project_dir: project_dir.clone(),
                    cause,
                })
            })? {
                if !project_dir.contains(' ') {
                    Ok(project_dir.into())
                } else {
                    Err(Error::ProjectDirInvalid(
                        ProjectDirInvalid::ContainsSpaces { project_dir },
                    ))
                }
            } else {
                Err(Error::ProjectDirInvalid(
                    ProjectDirInvalid::OutsideOfAppRoot {
                        project_dir,
                        root_dir: app.root_dir().to_owned(),
                    },
                ))
            }
        } else {
            log::info!(
                "`{}.project-dir` not set; defaulting to {:?}",
                super::NAME,
                DEFAULT_PROJECT_DIR
            );
            Ok(DEFAULT_PROJECT_DIR.into())
        }?;

        Ok(Self {
            app,
            min_sdk_version,
            vulkan_validation,
            project_dir,
        })
    }

    pub fn app(&self) -> &App {
        &self.app
    }

    pub fn so_name(&self) -> String {
        format!("lib{}.so", self.app().name_snake())
    }

    pub fn min_sdk_version(&self) -> u32 {
        self.min_sdk_version
    }

    pub fn project_dir(&self) -> PathBuf {
        self.app
            .prefix_path(&self.project_dir)
            .join(self.app().name())
    }

    pub fn project_dir_exists(&self) -> bool {
        self.project_dir().is_dir()
    }
}
