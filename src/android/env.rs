use super::ndk;
use crate::{
    env::{Env as CoreEnv, Error as CoreError, ExplicitEnv},
    util::{
        self,
        cli::{Report, Reportable},
    },
};
use std::path::{Path, PathBuf};
use thiserror::Error;

#[derive(Debug, Error)]
pub enum Error {
    #[error(transparent)]
    CoreEnvError(#[from] CoreError),
    // TODO: we should be nice and provide a platform-specific suggestion
    #[error("Have you installed the Android SDK? The `ANDROID_SDK_ROOT` environment variable isn't set, and is required: {0}")]
    AndroidSdkRootNotSet(#[from] std::env::VarError),
    #[error("Have you installed the Android SDK? The `ANDROID_SDK_ROOT` environment variable is set, but doesn't point to an existing directory.")]
    AndroidSdkRootNotADir,
    #[error(transparent)]
    NdkEnvError(#[from] ndk::Error),
}

impl Reportable for Error {
    fn report(&self) -> Report {
        match self {
            Self::CoreEnvError(err) => err.report(),
            Self::NdkEnvError(err) => err.report(),
            _ => Report::error("Failed to initialize Android environment", self),
        }
    }
}

impl Error {
    pub fn sdk_or_ndk_issue(&self) -> bool {
        !matches!(self, Self::CoreEnvError(_))
    }
}

#[derive(Debug, Error)]
pub enum VersionError {
    #[error("Failed to open {path:?}: {source}")]
    OpenFailed {
        path: PathBuf,
        source: std::io::Error,
    },
    #[error("Failed to parse {path:?}: {source}")]
    ParseFailed {
        path: PathBuf,
        source: java_properties::PropertiesError,
    },
    #[error("No version number was present in {path:?}.")]
    VersionMissing { path: PathBuf },
    #[error("Properties at {path:?} contained a version component {component:?} that wasn't a valid number: {source}")]
    ComponentNotNumerical {
        path: PathBuf,
        component: String,
        source: std::num::ParseIntError,
    },
    #[error("Version {version:?} in properties file {path:?} didn't have as many components as expected.")]
    TooFewComponents { path: PathBuf, version: String },
}

#[derive(Debug)]
pub struct Env {
    base: CoreEnv,
    sdk_root: PathBuf,
    pub ndk: ndk::Env,
}

impl Env {
    pub fn new() -> Result<Self, Error> {
        let base = CoreEnv::new()?;
        let sdk_root = std::env::var("ANDROID_SDK_ROOT")
            .map_err(Error::AndroidSdkRootNotSet)
            .map(PathBuf::from)
            .and_then(|sdk_root| {
                if sdk_root.is_dir() {
                    Ok(sdk_root)
                } else {
                    Err(Error::AndroidSdkRootNotADir)
                }
            })
            .or_else(|err| {
                if let Some(android_home) = std::env::var("ANDROID_HOME")
                    .ok()
                    .map(PathBuf::from)
                    .filter(|android_home| android_home.is_dir())
                {
                    log::warn!("`ANDROID_SDK_ROOT` isn't set; falling back to `ANDROID_HOME`, which is deprecated");
                    Ok(android_home)
                } else {
                    Err(err)
                }
            })
            .or_else(|err| {
                if let Some(android_home) = std::env::var("ANDROID_HOME")
                    .ok()
                    .map(PathBuf::from)
                    .filter(|android_home| android_home.is_dir())
                {
                    log::warn!("`ANDROID_SDK_ROOT` isn't set; falling back to `ANDROID_HOME`, which is deprecated");
                    Ok(android_home)
                } else {
                    Err(err)
                }
            })?;
        Ok(Self {
            base,
            sdk_root,
            ndk: ndk::Env::new()?,
        })
    }

    pub fn path(&self) -> &str {
        self.base.path()
    }

    pub fn sdk_root(&self) -> &str {
        self.sdk_root.as_path().to_str().unwrap()
    }

    // TODO: factor out the logic shared with `Ndk::version`
    pub fn sdk_version(&self) -> Result<util::VersionTriple, VersionError> {
        let path = Path::new(self.sdk_root()).join("tools/source.properties");
        let props = {
            let file = std::fs::File::open(&path).map_err(|source| VersionError::OpenFailed {
                path: path.clone(),
                source,
            })?;
            java_properties::read(file).map_err(|source| VersionError::ParseFailed {
                path: path.clone(),
                source,
            })?
        };
        let revision = props
            .get("Pkg.Revision")
            .ok_or_else(|| VersionError::VersionMissing { path: path.clone() })?;
        let components = revision
            .split('.')
            .take(3)
            .map(|component| {
                component
                    .parse::<u32>()
                    .map_err(|source| VersionError::ComponentNotNumerical {
                        path: path.clone(),
                        component: component.to_owned(),
                        source,
                    })
            })
            .collect::<Result<Vec<_>, _>>()?;
        if components.len() == 3 {
            Ok(util::VersionTriple {
                major: components[0],
                minor: components[1],
                patch: components[2],
            })
        } else {
            Err(VersionError::TooFewComponents {
                path,
                version: revision.to_owned(),
            })
        }
    }
}

impl ExplicitEnv for Env {
    fn explicit_env(&self) -> Vec<(&str, &std::ffi::OsStr)> {
        let mut envs = self.base.explicit_env();
        envs.extend(&[
            ("ANDROID_SDK_ROOT", self.sdk_root.as_ref()),
            ("NDK_HOME", self.ndk.home().as_ref()),
        ]);
        envs
    }
}
