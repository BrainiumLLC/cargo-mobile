pub mod app;
pub mod metadata;
mod raw;

use self::{app::App, raw::*};
#[cfg(target_os = "macos")]
use crate::apple;
use crate::{
    android,
    opts::NonInteractive,
    templating,
    util::cli::{Report, Reportable, TextWrapper},
};
use serde::Serialize;
use std::{
    fmt::Debug,
    io,
    path::{Path, PathBuf},
};

pub fn file_name() -> String {
    format!("{}.toml", crate::NAME)
}

#[derive(Debug)]
pub enum FromRawError {
    AppConfigInvalid(app::Error),
    #[cfg(target_os = "macos")]
    AppleConfigInvalid(apple::config::Error),
    AndroidConfigInvalid(android::config::Error),
}

impl FromRawError {
    pub fn report(&self, msg: &str) -> Report {
        match self {
            Self::AppConfigInvalid(err) => err.report(msg),
            #[cfg(target_os = "macos")]
            Self::AppleConfigInvalid(err) => err.report(msg),
            Self::AndroidConfigInvalid(err) => err.report(msg),
        }
    }
}

#[derive(Debug)]
pub enum GenError {
    PromptFailed(PromptError),
    DetectFailed(DetectError),
    CanonicalizeFailed(io::Error),
    FromRawFailed(FromRawError),
    WriteFailed(WriteError),
}

impl Reportable for GenError {
    fn report(&self) -> Report {
        let msg = "Failed to generate config";
        match self {
            Self::PromptFailed(err) => err.report(),
            Self::DetectFailed(err) => err.report(),
            Self::CanonicalizeFailed(err) => {
                Report::error(msg, format!("Failed to canonicalize root dir: {}", err))
            }
            Self::FromRawFailed(err) => err.report(msg),
            Self::WriteFailed(err) => err.report(),
        }
    }
}

#[derive(Debug)]
pub enum LoadOrGenError {
    LoadFailed(LoadError),
    FromRawFailed { path: PathBuf, cause: FromRawError },
    GenFailed(GenError),
}

impl Reportable for LoadOrGenError {
    fn report(&self) -> Report {
        match self {
            Self::LoadFailed(err) => Report::error("Failed to load config", err),
            Self::FromRawFailed { path, cause } => {
                let msg = format!("Config file at {:?} invalid", path);
                cause.report(&msg)
            }
            Self::GenFailed(err) => err.report(),
        }
    }
}

#[derive(Clone, Copy, Debug)]
pub enum Origin {
    FreshlyMinted,
    Loaded,
}

impl Origin {
    pub fn freshly_minted(self) -> bool {
        matches!(self, Self::FreshlyMinted)
    }
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "kebab-case")]
pub struct Config {
    app: App,
    #[cfg(target_os = "macos")]
    apple: apple::config::Config,
    android: android::config::Config,
    env: Option<toml::value::Table>,
}

impl Config {
    fn from_raw(root_dir: PathBuf, raw: Raw) -> Result<Self, FromRawError> {
        let app = App::from_raw(root_dir, raw.app).map_err(FromRawError::AppConfigInvalid)?;
        #[cfg(target_os = "macos")]
        let apple = apple::config::Config::from_raw(app.clone(), raw.apple)
            .map_err(FromRawError::AppleConfigInvalid)?;
        let android = android::config::Config::from_raw(app.clone(), raw.android)
            .map_err(FromRawError::AndroidConfigInvalid)?;
        Ok(Self {
            app,
            #[cfg(target_os = "macos")]
            apple,
            android,
            env: raw.env,
        })
    }

    fn gen(
        cwd: impl AsRef<Path>,
        non_interactive: NonInteractive,
        wrapper: &TextWrapper,
    ) -> Result<Self, GenError> {
        let raw = if non_interactive.no() {
            Raw::prompt(wrapper).map_err(GenError::PromptFailed)
        } else {
            Raw::detect(wrapper).map_err(GenError::DetectFailed)
        }?;
        let root_dir = cwd
            .as_ref()
            .canonicalize()
            .map_err(GenError::CanonicalizeFailed)?;
        let config =
            Self::from_raw(root_dir.clone(), raw.clone()).map_err(GenError::FromRawFailed)?;
        log::info!("generated config: {:#?}", config);
        raw.write(&root_dir).map_err(GenError::WriteFailed)?;
        Ok(config)
    }

    pub fn load_or_gen(
        cwd: impl AsRef<Path>,
        non_interactive: NonInteractive,
        wrapper: &TextWrapper,
    ) -> Result<(Self, Origin), LoadOrGenError> {
        let cwd = cwd.as_ref();
        if let Some((root_dir, raw)) = Raw::load(cwd).map_err(LoadOrGenError::LoadFailed)? {
            Self::from_raw(root_dir.clone(), raw)
                .map(|config| (config, Origin::Loaded))
                .map_err(|cause| LoadOrGenError::FromRawFailed {
                    path: root_dir,
                    cause,
                })
        } else {
            Self::gen(cwd, non_interactive, wrapper)
                .map(|config| (config, Origin::FreshlyMinted))
                .map_err(LoadOrGenError::GenFailed)
        }
    }

    pub fn path(&self) -> PathBuf {
        self.app().root_dir().join(file_name())
    }

    pub fn app(&self) -> &App {
        &self.app
    }

    #[cfg(target_os = "macos")]
    pub fn apple(&self) -> &apple::config::Config {
        &self.apple
    }

    pub fn android(&self) -> &android::config::Config {
        &self.android
    }

    pub fn env(&self) -> &Option<toml::value::Table> {
        &self.env
    }

    pub fn build_a_bike(&self) -> bicycle::Bicycle {
        templating::init(Some(self))
    }
}
