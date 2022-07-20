use super::app;
#[cfg(target_os = "macos")]
use crate::apple;
use crate::{
    android,
    util::cli::{Report, Reportable, TextWrapper},
};
use serde::{Deserialize, Serialize};

use std::{
    fmt::{self, Display},
    fs, io,
    path::{Path, PathBuf},
};

#[derive(Debug)]
pub enum PromptError {
    AppFailed(app::PromptError),
    #[cfg(target_os = "macos")]
    AppleFailed(apple::config::PromptError),
}

impl Reportable for PromptError {
    fn report(&self) -> Report {
        match self {
            Self::AppFailed(err) => {
                Report::error(format!("Failed to prompt for `{}` config", app::KEY), err)
            }
            #[cfg(target_os = "macos")]
            Self::AppleFailed(err) => Report::error(
                format!("Failed to prompt for `{}` config", apple::NAME),
                err,
            ),
        }
    }
}

#[derive(Debug)]
pub enum DetectError {
    AppFailed(app::DetectError),
    #[cfg(target_os = "macos")]
    AppleFailed(apple::config::DetectError),
}

impl Reportable for DetectError {
    fn report(&self) -> Report {
        match self {
            Self::AppFailed(err) => {
                Report::error(format!("Failed to detect `{}` config", app::KEY), err)
            }
            #[cfg(target_os = "macos")]
            Self::AppleFailed(err) => {
                Report::error(format!("Failed to detect `{}` config", apple::NAME), err)
            }
        }
    }
}

#[derive(Debug)]
pub enum LoadError {
    DiscoverFailed(io::Error),
    ReadFailed {
        path: PathBuf,
        cause: io::Error,
    },
    ParseFailed {
        path: PathBuf,
        cause: toml::de::Error,
    },
}

impl Display for LoadError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::DiscoverFailed(err) => write!(
                f,
                "Failed to canonicalize path while searching for config file: {}",
                err
            ),
            Self::ReadFailed { path, cause } => {
                write!(f, "Failed to read config file at {:?}: {}", path, cause)
            }
            Self::ParseFailed { path, cause } => {
                write!(f, "Failed to parse config file at {:?}: {}", path, cause)
            }
        }
    }
}

#[derive(Debug)]
pub enum WriteError {
    SerializeFailed(toml::ser::Error),
    WriteFailed(io::Error),
}

impl Reportable for WriteError {
    fn report(&self) -> Report {
        match self {
            Self::SerializeFailed(err) => Report::error("Failed to serialize config", err),
            Self::WriteFailed(err) => Report::error("Failed to write config", err),
        }
    }
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(rename_all = "kebab-case")]
pub struct Raw {
    pub app: app::Raw,
    #[cfg(target_os = "macos")]
    pub apple: Option<apple::config::Raw>,
    pub android: Option<android::config::Raw>,
    pub env: Option<toml::value::Table>,
}

impl Raw {
    pub fn prompt(wrapper: &TextWrapper) -> Result<Self, PromptError> {
        let app = app::Raw::prompt(wrapper).map_err(PromptError::AppFailed)?;
        #[cfg(target_os = "macos")]
        let apple = apple::config::Raw::prompt(wrapper).map_err(PromptError::AppleFailed)?;
        Ok(Self {
            app,
            #[cfg(target_os = "macos")]
            apple: Some(apple),
            android: None,
            env: None,
        })
    }

    pub fn detect(wrapper: &TextWrapper) -> Result<Self, DetectError> {
        let app = app::Raw::detect(wrapper).map_err(DetectError::AppFailed)?;
        #[cfg(target_os = "macos")]
        let apple = apple::config::Raw::detect().map_err(DetectError::AppleFailed)?;
        Ok(Self {
            app,
            #[cfg(target_os = "macos")]
            apple: Some(apple),
            android: None,
            env: None,
        })
    }

    pub fn discover_root(cwd: impl AsRef<Path>) -> io::Result<Option<PathBuf>> {
        let file_name = super::file_name();
        let mut path = cwd.as_ref().canonicalize()?.join(&file_name);
        log::info!("looking for config file at {:?}", path);
        while !path.exists() {
            if let Some(parent) = path.parent().and_then(Path::parent) {
                path = parent.join(&file_name);
                log::info!("looking for config file at {:?}", path);
            } else {
                log::info!("no config file was ever found");
                return Ok(None);
            }
        }
        log::info!("found config file at {:?}", path);
        path.pop();
        Ok(Some(path))
    }

    pub fn load(cwd: impl AsRef<Path>) -> Result<Option<(PathBuf, Self)>, LoadError> {
        Self::discover_root(cwd)
            .map_err(LoadError::DiscoverFailed)?
            .map(|root_dir| {
                let path = root_dir.join(super::file_name());
                let bytes = fs::read(&path).map_err(|cause| LoadError::ReadFailed {
                    path: path.clone(),
                    cause,
                })?;
                toml::from_slice::<Self>(&bytes)
                    .map(|raw| (root_dir, raw))
                    .map_err(|cause| LoadError::ParseFailed {
                        path: path.clone(),
                        cause,
                    })
            })
            .transpose()
    }

    pub fn write(&self, root_dir: &Path) -> Result<(), WriteError> {
        let bytes = toml::to_vec(self).map_err(WriteError::SerializeFailed)?;
        let path = root_dir.join(super::file_name());
        log::info!("writing config to {:?}", path);
        fs::write(path, bytes).map_err(WriteError::WriteFailed)
    }
}
