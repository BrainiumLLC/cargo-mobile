use crate::{
    opts,
    util::{
        self,
        cli::{Report, Reportable},
        NoHomeDir,
    },
};
#[cfg(not(target_os = "macos"))]
use std::path::PathBuf;

#[cfg(not(target_os = "macos"))]
pub const BUNDLE_TOOL_JAR_INFO: BundletoolJarInfo = BundletoolJarInfo { version: "1.8.0" };

#[cfg(not(target_os = "macos"))]
pub struct BundletoolJarInfo {
    version: &'static str,
}

#[cfg(not(target_os = "macos"))]
impl BundletoolJarInfo {
    fn file_name(&self) -> String {
        format!("bundletool-all-{}.jar", self.version)
    }

    fn installation_path(&self) -> Result<PathBuf, NoHomeDir> {
        util::tools_dir().map(|tools_dir| tools_dir.join(self.file_name()))
    }

    fn download_url(&self) -> String {
        format!(
            "https://github.com/google/bundletool/releases/download/{}/{}",
            self.version,
            self.file_name()
        )
    }

    fn run_command(&self) -> Result<bossy::Command, NoHomeDir> {
        let installation_path = self.installation_path()?;
        Ok(bossy::Command::impure_parse("java -jar").with_arg(installation_path))
    }
}

pub fn command() -> Result<bossy::Command, NoHomeDir> {
    #[cfg(not(target_os = "macos"))]
    {
        BUNDLE_TOOL_JAR_INFO.run_command()
    }
    #[cfg(target_os = "macos")]
    {
        Ok(bossy::Command::impure("bundletool"))
    }
}

#[cfg(target_os = "macos")]
#[derive(Debug)]
pub struct InstallError(crate::apple::deps::Error);

#[cfg(target_os = "macos")]
impl Reportable for InstallError {
    fn report(&self) -> Report {
        Report::error("Failed to install `bundletool`", &self.0)
    }
}

#[cfg(not(target_os = "macos"))]
#[derive(Debug)]
pub enum InstallError {
    DownloadFailed(ureq::Error),
    JarFileCreationFailed {
        path: PathBuf,
        cause: std::io::Error,
    },
    CopyToFileFailed {
        path: PathBuf,
        cause: std::io::Error,
    },
    NoInstallationPath(util::NoHomeDir),
}

#[cfg(not(target_os = "macos"))]
impl Reportable for InstallError {
    fn report(&self) -> Report {
        match self {
            Self::DownloadFailed(err) => Report::error("Failed to download `bundletool`", err),
            Self::JarFileCreationFailed { path, cause } => Report::error(
                format!("Failed to create bundletool.jar at {:?}", path),
                cause,
            ),
            Self::CopyToFileFailed { path, cause } => Report::error(
                format!("Failed to copy content into bundletool.jar at {:?}", path),
                cause,
            ),
            Self::NoInstallationPath(err) => {
                Report::error("No valid instalalation path for `bundletool`", err)
            }
        }
    }
}

pub fn install(reinstall_deps: opts::ReinstallDeps) -> Result<(), InstallError> {
    #[cfg(not(target_os = "macos"))]
    {
        let jar_path = BUNDLE_TOOL_JAR_INFO
            .installation_path()
            .map_err(InstallError::NoInstallationPath)?;
        if !jar_path.exists() || reinstall_deps.yes() {
            let response = ureq::get(&BUNDLE_TOOL_JAR_INFO.download_url())
                .call()
                .map_err(InstallError::DownloadFailed)?;
            let tools_dir = util::tools_dir().expect("unable to find tools dir");
            std::fs::create_dir_all(&tools_dir).map_err(|cause| {
                InstallError::JarFileCreationFailed {
                    path: tools_dir,
                    cause,
                }
            })?;
            let mut out = std::fs::File::create(&jar_path).map_err(|cause| {
                InstallError::JarFileCreationFailed {
                    path: jar_path.clone(),
                    cause,
                }
            })?;
            std::io::copy(&mut response.into_reader(), &mut out).map_err(|cause| {
                InstallError::CopyToFileFailed {
                    path: jar_path,
                    cause,
                }
            })?;
        }
    }
    #[cfg(target_os = "macos")]
    {
        crate::apple::deps::install("bundletool", reinstall_deps).map_err(InstallError)?;
    }
    Ok(())
}
