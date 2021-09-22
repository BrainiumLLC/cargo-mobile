use super::{
    adb,
    config::Config,
    env::Env,
    jnilibs::{self, JniLibs},
    target::{BuildError, Target},
};
use crate::{
    env::ExplicitEnv as _,
    opts::{FilterLevel, NoiseLevel, Profile},
    util::{
        self,
        cli::{Report, Reportable},
    },
};
use std::fmt::{self, Display};

fn gradlew(config: &Config, env: &Env) -> bossy::Command {
    let gradlew_path = config.project_dir().join("gradlew");
    bossy::Command::pure(&gradlew_path)
        .with_env_vars(env.explicit_env())
        .with_arg("--project-dir")
        .with_arg(config.project_dir())
}

#[cfg(target_os = "linux")]
struct BundletoolJarInfo {
    version: &'static str,
}

#[cfg(target_os = "linux")]
impl BundletoolJarInfo {
    const fn new(version: &'static str) -> Self {
        Self { version }
    }
    fn jar(&self) -> String {
        format!("bundletool-all-{}.jar", self.version)
    }
    fn jar_path(&self) -> String {
        format!("gen/android/{}", self.jar())
    }
    fn download_url(&self) -> String {
        format!(
            "https://github.com/google/bundletool/releases/download/{}/{}",
            self.version,
            self.jar()
        )
    }
    fn run_command(&self) -> String {
        format!("java -jar {}", self.jar_path())
    }
}

#[cfg(target_os = "linux")]
const BUNDLE_TOOL_JAR_INFO: BundletoolJarInfo = BundletoolJarInfo::new("1.8.0");

#[cfg(target_os = "macos")]
fn bundletool_command() -> String {
    "bundletool".to_string()
}

#[cfg(target_os = "linux")]
fn bundletool_command() -> String {
    BUNDLE_TOOL_JAR_INFO.run_command()
}

#[cfg(target_os = "macos")]
fn install_bundletool() -> Result<(), BundletoolInstallError> {
    crate::apple::deps::install("bundletool", Default::default())
        .map_err(BundletoolInstallError::MacOSInstallFailed)?;

    Ok(())
}

#[cfg(target_os = "linux")]
fn install_bundletool() -> Result<(), BundletoolInstallError> {
    if !std::path::Path::new(&BUNDLE_TOOL_JAR_INFO.jar_path()).exists() {
        let response = ureq::get(&BUNDLE_TOOL_JAR_INFO.download_url())
            .call()
            .map_err(BundletoolInstallError::DownloadFailed)?;
        let mut out = std::fs::File::create(&BUNDLE_TOOL_JAR_INFO.jar())
            .map_err(BundletoolInstallError::JarFileCreationFailed)?;
        std::io::copy(&mut response.into_reader(), &mut out)
            .map_err(BundletoolInstallError::CopyToFileFailed)?;
        std::fs::rename(
            &BUNDLE_TOOL_JAR_INFO.jar(),
            &BUNDLE_TOOL_JAR_INFO.jar_path(),
        )
        .map_err(BundletoolInstallError::MoveFileFailed)?;
    }
    Ok(())
}

#[derive(Debug)]
pub enum ApkBuildError {
    LibSymlinkCleaningFailed(jnilibs::RemoveBrokenLinksError),
    LibBuildFailed(BuildError),
    AssembleFailed(bossy::Error),
}

impl Reportable for ApkBuildError {
    fn report(&self) -> Report {
        match self {
            Self::LibSymlinkCleaningFailed(err) => err.report(),
            Self::LibBuildFailed(err) => err.report(),
            Self::AssembleFailed(err) => Report::error("Failed to assemble APK", err),
        }
    }
}

#[derive(Debug)]
pub enum AabBuildError {
    BuildFailed(bossy::Error),
}

impl Reportable for AabBuildError {
    fn report(&self) -> Report {
        match self {
            Self::BuildFailed(err) => Report::error("Failed to build AAB", err),
        }
    }
}

#[derive(Debug)]
pub enum ApksBuildError {
    CleanFailed(bossy::Error),
    BuildFromAabFailed(bossy::Error),
}

impl Reportable for ApksBuildError {
    fn report(&self) -> Report {
        match self {
            Self::CleanFailed(err) => Report::error("Failed to clean old APKS", err),
            Self::BuildFromAabFailed(err) => Report::error("Failed to build APKS from AAB", err),
        }
    }
}

#[derive(Debug)]
pub enum BundletoolInstallError {
    #[cfg(target_os = "macos")]
    MacOSInstallFailed(crate::apple::deps::Error),
    DownloadFailed(ureq::Error),
    JarFileCreationFailed(std::io::Error),
    CopyToFileFailed(std::io::Error),
    MoveFileFailed(std::io::Error),
}

impl Reportable for BundletoolInstallError {
    fn report(&self) -> Report {
        match self {
            #[cfg(target_os = "macos")]
            Self::MacOSInstallFailed(err) => Report::error("Failed to install `bundletool`", err),
            Self::DownloadFailed(err) => Report::error("Failed to download `bundletool`", err),
            Self::JarFileCreationFailed(err) => {
                Report::error("Failed to create bundletool.jar", err)
            }
            Self::CopyToFileFailed(err) => {
                Report::error("Failed to copy content into bundletool.jar", err)
            }
            Self::MoveFileFailed(err) => Report::error("Failed to move bundletool.jar", err),
        }
    }
}

#[derive(Debug)]
pub enum ApkInstallError {
    InstallFailed(bossy::Error),
    InstallFromAabFailed(bossy::Error),
}

impl Reportable for ApkInstallError {
    fn report(&self) -> Report {
        match self {
            Self::InstallFailed(err) => Report::error("Failed to install APK", err),
            Self::InstallFromAabFailed(err) => Report::error("Failed to install APK from AAB", err),
        }
    }
}

#[derive(Debug)]
pub enum RunError {
    ApkBuildFailed(ApkBuildError),
    ApkInstallFailed(ApkInstallError),
    StartFailed(bossy::Error),
    WakeScreenFailed(bossy::Error),
    LogcatFailed(bossy::Error),
    BundletoolInstallFailed(BundletoolInstallError),
    AabBuildError(AabBuildError),
    ApksFromAabBuildError(ApksBuildError),
}

impl Reportable for RunError {
    fn report(&self) -> Report {
        match self {
            Self::ApkBuildFailed(err) => err.report(),
            Self::ApkInstallFailed(err) => err.report(),
            Self::StartFailed(err) => Report::error("Failed to start app on device", err),
            Self::WakeScreenFailed(err) => Report::error("Failed to wake device screen", err),
            Self::LogcatFailed(err) => Report::error("Failed to log output", err),
            Self::BundletoolInstallFailed(err) => err.report(),
            Self::AabBuildError(err) => err.report(),
            Self::ApksFromAabBuildError(err) => err.report(),
        }
    }
}

#[derive(Debug)]
pub enum StacktraceError {
    PipeFailed(util::PipeError),
}

impl Reportable for StacktraceError {
    fn report(&self) -> Report {
        match self {
            Self::PipeFailed(err) => Report::error("Failed to pipe stacktrace output", err),
        }
    }
}

#[derive(Debug, Eq, Ord, PartialEq, PartialOrd)]
pub struct Device<'a> {
    serial_no: String,
    name: String,
    model: String,
    target: &'a Target<'a>,
}

impl<'a> Display for Device<'a> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.name)?;
        if self.model != self.name {
            write!(f, " ({})", self.model)?;
        }
        Ok(())
    }
}

impl<'a> Device<'a> {
    pub(super) fn new(
        serial_no: String,
        name: String,
        model: String,
        target: &'a Target<'a>,
    ) -> Self {
        Self {
            serial_no,
            name,
            model,
            target,
        }
    }

    pub fn target(&self) -> &'a Target<'a> {
        self.target
    }

    fn adb(&self, env: &Env) -> bossy::Command {
        adb::adb(env, &self.serial_no)
    }

    fn suffix(profile: Profile) -> &'static str {
        match profile {
            Profile::Debug => profile.as_str(),
            // TODO: how to handle signed APKs?
            Profile::Release => "release-unsigned",
        }
    }

    fn apk_path(config: &Config, profile: Profile, flavor: &str) -> std::path::PathBuf {
        let build_ty = profile.as_str();
        let suffix = Self::suffix(profile);
        config.project_dir().join(format!(
            "app/build/outputs/apk/{}/{}/app-{}-{}.apk",
            flavor, build_ty, flavor, suffix,
        ))
    }

    fn apks_path(config: &Config, profile: Profile, flavor: &str) -> std::path::PathBuf {
        let build_ty = profile.as_str();
        let suffix = Self::suffix(profile);
        config.project_dir().join(format!(
            "app/build/outputs/apk/{}/{}/app-{}-{}.apks",
            flavor, build_ty, flavor, suffix,
        ))
    }

    fn aab_path(config: &Config, profile: Profile, flavor: &str) -> std::path::PathBuf {
        let build_ty = profile.as_str();
        let suffix = Self::suffix(profile);
        config.project_dir().join(format!(
            "app/build/outputs/bundle/{}{}/app-{}-{}.aab",
            flavor, build_ty, flavor, suffix
        ))
    }

    fn build_apk(
        &self,
        config: &Config,
        env: &Env,
        noise_level: NoiseLevel,
        profile: Profile,
    ) -> Result<(), ApkBuildError> {
        use heck::CamelCase as _;
        JniLibs::remove_broken_links(config).map_err(ApkBuildError::LibSymlinkCleaningFailed)?;
        let flavor = self.target.arch.to_camel_case();
        let build_ty = profile.as_str().to_camel_case();
        gradlew(config, env)
            .with_arg(format!("assemble{}{}", flavor, build_ty))
            .with_arg(match noise_level {
                NoiseLevel::Polite => "--warn",
                NoiseLevel::LoudAndProud => "--info",
                NoiseLevel::FranklyQuitePedantic => "--debug",
            })
            .run_and_wait()
            .map_err(ApkBuildError::AssembleFailed)?;
        Ok(())
    }

    fn install_apk(
        &self,
        config: &Config,
        env: &Env,
        profile: Profile,
    ) -> Result<(), ApkInstallError> {
        let flavor = self.target.arch;
        let apk_path = Self::apk_path(config, profile, flavor);
        self.adb(env)
            .with_arg("install")
            .with_arg(apk_path)
            .run_and_wait()
            .map_err(ApkInstallError::InstallFailed)?;
        Ok(())
    }

    fn clean_apks(&self, config: &Config, profile: Profile) -> Result<(), ApksBuildError> {
        let flavor = self.target.arch;
        let apks_path = Self::apks_path(config, profile, flavor);
        bossy::Command::impure_parse("rm -f")
            .with_parsed_args(
                apks_path
                    .to_str()
                    .unwrap_or_else(|| panic!("path {:?} contained invalid utf-8", apks_path)),
            )
            .run_and_wait()
            .map_err(ApksBuildError::CleanFailed)?;
        Ok(())
    }

    fn build_aab(&self, config: &Config, env: &Env, profile: Profile) -> Result<(), AabBuildError> {
        use heck::CamelCase as _;
        let flavor = self.target.arch.to_camel_case();
        let build_ty = profile.as_str().to_camel_case();
        gradlew(config, env)
            .with_arg(format!(":app:bundle{}{}", flavor, build_ty))
            .run_and_wait()
            .map_err(AabBuildError::BuildFailed)?;
        Ok(())
    }

    fn build_apks_from_aab(&self, config: &Config, profile: Profile) -> Result<(), ApksBuildError> {
        let flavor = self.target.arch;
        let apks_path = Self::apks_path(config, profile, flavor);
        let aab_path = Self::aab_path(config, profile, flavor);
        bossy::Command::impure_parse(bundletool_command())
            .with_parsed_args("build-apks")
            .with_parsed_args(format!(
                "--bundle={}",
                aab_path
                    .to_str()
                    .unwrap_or_else(|| panic!("path {:?} contained invalid utf-8", aab_path))
            ))
            .with_parsed_args(format!(
                "--output={}",
                apks_path
                    .to_str()
                    .unwrap_or_else(|| panic!("path {:?} contained invalid utf-8", apks_path))
            ))
            .with_parsed_args("--connected-device")
            .run_and_wait()
            .map_err(ApksBuildError::BuildFromAabFailed)?;
        Ok(())
    }

    fn install_apk_from_aab(
        &self,
        config: &Config,
        profile: Profile,
    ) -> Result<(), ApkInstallError> {
        let flavor = self.target.arch;
        let apks_path = Self::apks_path(config, profile, flavor);
        bossy::Command::impure_parse(bundletool_command())
            .with_parsed_args("install-apks")
            .with_parsed_args(format!(
                "--apks={}",
                apks_path
                    .to_str()
                    .unwrap_or_else(|| panic!("path {:?} contained invalid utf-8", apks_path))
            ))
            .run_and_wait()
            .map_err(ApkInstallError::InstallFromAabFailed)?;
        Ok(())
    }

    fn wake_screen(&self, env: &Env) -> bossy::Result<()> {
        self.adb(env)
            .with_args(&["shell", "input", "keyevent", "KEYCODE_WAKEUP"])
            .run_and_wait()?;
        Ok(())
    }

    pub fn run(
        &self,
        config: &Config,
        env: &Env,
        noise_level: NoiseLevel,
        profile: Profile,
        filter_level: Option<FilterLevel>,
        build_app_bundle: bool,
    ) -> Result<(), RunError> {
        if build_app_bundle {
            install_bundletool().map_err(RunError::BundletoolInstallFailed)?;
            self.clean_apks(config, profile)
                .map_err(RunError::ApksFromAabBuildError)?;
            self.build_aab(config, env, profile)
                .map_err(RunError::AabBuildError)?;
            self.build_apks_from_aab(config, profile)
                .map_err(RunError::ApksFromAabBuildError)?;
            self.install_apk_from_aab(config, profile)
                .map_err(RunError::ApkInstallFailed)?;
        } else {
            self.build_apk(config, env, noise_level, profile)
                .map_err(RunError::ApkBuildFailed)?;
            self.install_apk(config, env, profile)
                .map_err(RunError::ApkInstallFailed)?;
        }
        let activity = format!(
            "{}.{}/android.app.NativeActivity",
            config.app().reverse_domain(),
            config.app().name_snake(),
        );
        self.adb(env)
            .with_args(&["shell", "am", "start", "-n", &activity])
            .run_and_wait()
            .map_err(RunError::StartFailed)?;
        self.wake_screen(env).map_err(RunError::WakeScreenFailed)?;
        let filter = format!(
            "{}:{}",
            config.app().name(),
            filter_level
                .unwrap_or(match noise_level {
                    NoiseLevel::Polite => FilterLevel::Warn,
                    NoiseLevel::LoudAndProud => FilterLevel::Info,
                    NoiseLevel::FranklyQuitePedantic => FilterLevel::Verbose,
                })
                .logcat()
        );
        adb::adb(env, &self.serial_no)
            .with_args(&["logcat", "-v", "color", "-s", &filter])
            .run_and_wait()
            .map_err(RunError::LogcatFailed)?;
        Ok(())
    }

    pub fn stacktrace(&self, config: &Config, env: &Env) -> Result<(), StacktraceError> {
        // -d = print and exit
        let logcat_command = adb::adb(env, &self.serial_no).with_args(&["logcat", "-d"]);
        let stack_command = bossy::Command::pure("ndk-stack")
            .with_env_vars(env.explicit_env())
            .with_env_var(
                "PATH",
                util::prepend_to_path(env.ndk.home().display(), env.path()),
            )
            .with_arg("-sym")
            .with_arg(
                config
                    .app()
                    // ndk-stack can't seem to handle spaces in args, no matter
                    // how I try to quote or escape them... so, instead of
                    // mandating that the entire path not contain spaces, we'll
                    // just use a relative path!
                    .unprefix_path(jnilibs::path(config, *self.target))
                    .expect("developer error: jnilibs subdir not prefixed"),
            );
        if !util::pipe(logcat_command, stack_command).map_err(StacktraceError::PipeFailed)? {
            println!("  -- no stacktrace --");
        }
        Ok(())
    }
}
