use crate::{
    android,
    util::cli::TextWrapper,
    util::prompt,
    util::repo::Repo,
};
#[cfg(target_os = "macos")]
use crate::{
    android::adb,
    apple::{deps::xcode_plugin, ios_deploy, system_profile, teams},
    env::Env,
    opts,
    util::{self, repo::Status},
};
use colored::Colorize;
use once_cell_regex::regex;
use std::{
    env as stdenv,
    fmt::Debug,
    fs,
    path::{Path, PathBuf},
};

use thiserror::Error;

//TODO: This but nicer. Maybe with DoctorStatus or something.
fn fail_bullet(error: DoctorError) -> ReportBullet {
    ReportBullet::Failure { error }
}
fn warn_bullet(bullet: String) -> ReportBullet {
    ReportBullet::Warning { bullet }
}
fn success_bullet(bullet: String) -> ReportBullet {
    ReportBullet::Success { bullet }
}
fn report_bullet(command_result: Result<String, DoctorError>) -> ReportBullet {
    match command_result {
        Ok(bullet) => ReportBullet::Success { bullet },
        Err(error) => ReportBullet::Failure { error },
    }
}

fn command(command: &str) -> Result<String, DoctorError> {
    Ok(bossy::Command::impure_parse(command)
        .run_and_wait_for_output()?
        .stdout_str()?
        .trim_end()
        .to_owned())
}

#[derive(Debug)]
enum ReportBullet {
    Success { bullet: String },
    Warning { bullet: String },
    Failure { error: DoctorError },
}

fn has_error(bullets: &Vec<ReportBullet>) -> bool {
    bullets.iter().fold(false, |acc, bullet| {
        acc || match bullet {
            ReportBullet::Failure { error: _ } => true,
            _ => false,
        }
    })
}

fn has_warning(bullets: &Vec<ReportBullet>) -> bool {
    bullets.iter().fold(false, |acc, bullet| {
        acc || match bullet {
            ReportBullet::Warning { bullet: _ } => true,
            _ => false,
        }
    })
}

#[derive(Debug, Error)]
pub enum DoctorException {
    #[error(transparent)]
    NoHomeDir(#[from] util::NoHomeDir),
}

#[derive(Debug, Error)]
enum DoctorError {
    #[error("Failed to check installed macOS version")]
    OsCheckFailed(#[from] bossy::Error),
    #[error("Output contained invalid UTF-8: {0}")]
    InvalidUtf8(#[from] std::str::Utf8Error),
    #[error("Unable to find: {search_expr:?} in {details:?}")]
    RegexSearchFailed {
        search_expr: String,
        details: String,
    },
    #[error("Environment variable not set.")]
    VarError(#[from] stdenv::VarError),
    #[error(transparent)]
    CommandSearchFailed(#[from] util::RunAndSearchError),
    #[error("Failed to check Rust version")]
    RustVersionFailed(#[from] util::RustVersionError),
    #[error("iOS linking is broken on Rust versions later than 1.45.2 (d3fb005a3 2020-07-31) and earlier than 1.49.0-nightly (ffa2e7ae8 2020-10-24), but you're on {version}!\n    - Until this is resolved by Rust 1.49.0, please do one of the following:\n        A) downgrade to 1.45.2:\n           `rustup install stable-2020-08-03 && rustup default stable-2020-08-03`\n        B) update to a recent nightly:\n           `rustup update nightly && rustup default nightly`")]
    RustVersionInvalid { version: util::RustVersion },
    #[error("Commit message error")]
    InstalledCommitMsgFailed(#[from] util::InstalledCommitMsgError),
    #[error("Unknown Error")]
    MiscError { error: String },
}

#[derive(Debug, Clone, Copy)]
enum DoctorStatus {
    Success,
    Warning,
    Failure,
}
impl DoctorStatus {
    pub fn new(has_error: bool, has_warning: bool) -> Self {
        match (has_error, has_warning) {
            (false, false) => Self::Success,
            (false, true) => Self::Warning,
            (true, _) => Self::Failure,
        }
    }

    //TODO: cooler symbols
    pub fn get_report_symbol(self) -> &'static str {
        macro_rules! bracketify {
            ($s:literal) => {
                concat!("[", $s, "] ")
            };
        }
        match self {
            Self::Success => bracketify!("✔"),
            Self::Warning => bracketify!("!"),
            Self::Failure => bracketify!("X"),
        }
    }

    pub fn get_report_color(self) -> colored::Color {
        match self {
            Self::Success => colored::Color::BrightGreen,
            Self::Warning => colored::Color::BrightMagenta,
            Self::Failure => colored::Color::BrightRed,
        }
    }
}
#[derive(Debug)]
struct DoctorReport {
    status: DoctorStatus,
    headline: String,
    bullets: Vec<ReportBullet>,
}
impl DoctorReport {
    pub fn print(&self, wrapper: &TextWrapper) {
        static INDENT: &str = "    - ";
        let hanging_indent = INDENT.replace('-', " ");
        let bullet_wrapper = wrapper
            .clone()
            .initial_indent(INDENT)
            .subsequent_indent(&hanging_indent);
        let report_color = self.status.get_report_color();

        println!(
            "{}",
            wrapper.fill(&format!(
                "{} {}",
                self.status.get_report_symbol().color(report_color),
                &self.headline.color(report_color)
            ))
        );

        for report_bullet in &self.bullets {
            println!(
                "{}",
                bullet_wrapper.fill(&match report_bullet {
                    ReportBullet::Success { bullet } => bullet.to_owned().normal(),
                    ReportBullet::Warning { bullet } => bullet
                        .to_owned()
                        .color(colored::Color::BrightMagenta)
                        .bold(),
                    ReportBullet::Failure { error } => format!("{:?}", error)
                        .color(colored::Color::BrightRed)
                        .bold(),
                })
            );
        }
        println!();
    }
}
type DoctorResult = Result<DoctorReport, DoctorException>;

#[derive(Debug)]
pub struct ReportSummary {
    reports: Vec<DoctorReport>,
}
impl ReportSummary {
    pub fn print(&self, wrapper: &TextWrapper) {
        for report in &self.reports {
            report.print(wrapper);
        }
    }
}

fn check_cargo_mobile() -> DoctorResult {
    let install_dir = util::install_dir()?;
    let installation_not_found_error = !install_dir.exists();

    let commit_message = report_bullet(
        util::installed_commit_msg()
            .map(|message| message.unwrap_or_else(|| "commit info file not found".to_owned()))
            .map_err(|error| DoctorError::from(error)),
    );
    let version = report_bullet(command("cargo mobile -V"));

    let mut bullets = vec![commit_message];
    if installation_not_found_error {
        bullets.push(ReportBullet::Warning {
            bullet: "The /.cargo-mobile directory could not be found!".to_string(),
        });
        bullets.push(ReportBullet::Warning {
            bullet: format!("Unable to locate at {}", install_dir.to_str().unwrap()),
        });
    }

    Ok(DoctorReport {
        status: DoctorStatus::new(installation_not_found_error, false),
        headline: match version {
            ReportBullet::Success { bullet } => bullet,
            _ => "cargo mobile".to_owned(),
        },
        bullets,
    })
}

fn check_rust() -> DoctorResult {
    let rust_version = util::RustVersion::check();

    Ok(match rust_version {
        Ok(version) => {
            let valid = version.valid();
            DoctorReport {
                status: DoctorStatus::new(!valid, false),
                headline: format!("rustc {}", version.to_string()),
                bullets: match valid {
                    true => vec![],
                    false => vec![fail_bullet(DoctorError::RustVersionInvalid { version })],
                },
            }
        }
        Err(error) => DoctorReport {
            status: DoctorStatus::new(true, false),
            headline: format!("rustc version could not be found."),
            bullets: vec![ReportBullet::Failure {
                error: DoctorError::from(error),
            }],
        },
    })
}

fn check_os() -> DoctorResult {
    let mut bullets = match stdenv::consts::OS {
        "macos" => {
            let macos_version = util::run_and_search(
                &mut bossy::Command::impure_parse("system_profiler SPSoftwareDataType"),
                regex!(r"macOS (?P<version>.*)"),
                |_output, caps| caps.name("version").unwrap().as_str().to_owned(),
            );

            vec![match macos_version {
                Ok(version) => success_bullet(version),
                Err(error) => fail_bullet(DoctorError::from(error)),
            }]
        }
        &_ => unreachable!("cargo mobile only compiles on macOS. How are you here?"),
    };

    let os_not_currently_supported = stdenv::consts::OS != "macos";
    if os_not_currently_supported {
        bullets.push(warn_bullet(
            "A valid macOS installation is required for iOS and iPadOS development.".to_string(),
        ));
    }

    Ok(DoctorReport {
        status: DoctorStatus::new(os_not_currently_supported, false),
        headline: format!("Operating System: {}", stdenv::consts::OS),
        bullets,
    })
}

fn check_apple() -> DoctorResult {
    let is_macos = stdenv::consts::OS == "macos";

    let xcode_version =
        report_bullet(command("xcodebuild -version").map(|result| result.replace("\n", " ")));
    let xcode_select_version = report_bullet(command("xcode-select -version"));
    let ios_deploy_version = report_bullet(command("ios-deploy --version"));

    let development_teams =
        teams::find_development_teams().map_err(|_err| "No Apple Developer Teams were found");
    let plugin_valid = validate_xcode_plugin();

    validate_xcode_select(); // This could be done after reports are printed instead of before (Is there a way to do it mid-report?)

    let mut bullets = vec![
        xcode_version,
        xcode_select_version,
        ios_deploy_version,
        plugin_valid.1,
    ];

    let dev_team_warning = development_teams.is_err();
    if dev_team_warning {
        bullets.push(fail_bullet(DoctorError::MiscError {
            error: "No Apple Developer Teams were found!".to_owned(),
        }));
    } else {
        for team in &development_teams.unwrap() {
            bullets.push(success_bullet(format!(
                "Dev Team: {} ({})",
                team.name, team.id
            )));
        }
    }

    Ok(DoctorReport {
        status: DoctorStatus::new(
            has_error(&bullets) || !is_macos || !plugin_valid.0,
            has_warning(&bullets),
        ),
        headline: "Apple Developer Tools".to_owned(),
        bullets,
    })
}

fn check_android() -> DoctorResult {
    let android_env = android::env::Env::new();
    let valid = android_env.is_ok();

    if valid {
        let env = android_env.unwrap();

        let sdk_root = env.sdk_root().to_owned();
        let sdk_source_properties =
            fs::read_to_string(Path::new(&sdk_root).join("tools/source.properties")).unwrap();
        let sdk_version = regex!(r"Pkg.Revision=(?P<version>[0-9.]*)")
            .captures(&sdk_source_properties)
            .map(|caps| caps.name("version").unwrap().as_str())
            .ok_or_else(|| DoctorError::RegexSearchFailed {
                search_expr: "Pkg.Revision=(?P<version>[0-9.]*)".to_owned(),
                details: "Unable to locate Android SDK version".to_owned(),
            })
            .unwrap();

        let ndk_env = env.ndk;
        let ndk_home = ndk_env.home().to_str().unwrap().to_owned();
        let ndk_source_properties =
            fs::read_to_string(ndk_env.home().join("source.properties")).unwrap();
        let ndk_version = regex!(r"Pkg.Revision = (?P<version>[0-9.]*)")
            .captures(&ndk_source_properties)
            .map(|caps| caps.name("version").unwrap().as_str())
            .ok_or_else(|| DoctorError::RegexSearchFailed {
                search_expr: "Pkg.Revision = (?P<version>[0-9.]*)".to_owned(),
                details: "Unable to locate Android NDK version".to_owned(),
            })
            .unwrap();

        Ok(DoctorReport {
            status: DoctorStatus::new(!valid, false),
            headline: "Android Build Tools".to_owned(),
            bullets: vec![
                success_bullet(format!("SDK: {} ({})", sdk_root, sdk_version)),
                success_bullet(format!("NDK: {} ({})", ndk_home, ndk_version)),
            ],
        })
    } else {
        Ok(DoctorReport {
            status: DoctorStatus::new(!valid, false),
            headline: "Android Build Tools".to_owned(),
            bullets: vec![warn_bullet(
                "Failed to find ANDROID_SDK_ROOT or ANDROID_HOME environment variable!".to_owned(),
            )],
        })
    }
}

fn check_device_list() -> DoctorResult {
    let apple_env = Env::new().unwrap();
    let android_env = android::env::Env::new();
    let mut device_list = vec![];

    //Populate list with Apple devices (will panic if ios-deploy is not installed)
    if command("ios-deploy --version").is_ok() {
    ios_deploy::device_list(&apple_env)
        .map(|list| {
            for device in list {
                device_list.push(success_bullet(format!("{}", device)));
            }
        })
        .unwrap_or_default();
    }

    // Populate list with Android devices
    match android_env {
        Ok(env) => {
            adb::device_list(&env)
                .map(|list| {
                    for device in list {
                        device_list.push(success_bullet(format!("{}", device)));
                    }
                })
                .unwrap_or_default();
        }
        _ => (),
    }

    let no_devices_warning = device_list.is_empty();
    if no_devices_warning {
        device_list.push(warn_bullet("No connected devices were found".to_owned()));
    }

    Ok(DoctorReport {
        status: DoctorStatus::new(false, no_devices_warning),
        headline: "Connected Devices".to_string(),
        bullets: device_list,
    })
}

fn validate_xcode_plugin() -> (bool, ReportBullet) {
    // Check validity of Rust.ideplugin for Xcode (it'd be keen if there was an easier way)
    let xcode_version = system_profile::DeveloperTools::new().unwrap().version;
    let xcode_plugins_dir = xcode_plugin::xcode_library_dir()
        .unwrap_or_else(|_error| PathBuf::new())
        .join("Plug-ins");
    let xcode_app_dir = xcode_plugin::xcode_app_dir().unwrap_or_default();
    let xcode_lang_res_dir =
        xcode_app_dir.join("SharedFrameworks/SourceModel.framework/Versions/A/Resources");
    let spec_dst = if xcode_version.0 >= 11 {
        xcode_lang_res_dir.join("LanguageSpecifications")
    } else {
        xcode_app_dir.join("Specifications")
    }
    .join("Rust.xclangspec");
    let meta_dst = xcode_lang_res_dir.join("LanguageMetadata/Xcode.SourceCodeLanguage.Rust.plist");

    let plugin_valid = !xcode_plugin::check_plugin(
        opts::ReinstallDeps::No,
        xcode_version,
        &Repo::checkouts_dir("rust-xcode-plugin").unwrap(),
        &xcode_plugins_dir,
        &spec_dst,
        &meta_dst,
    )
    .unwrap_or_else(|_err| Status::Stale)
    .stale();

    let plugin_bullet = match plugin_valid {
        true => success_bullet("Xcode Rust.ideplugin is up to date.".to_owned()),
        false => fail_bullet(DoctorError::MiscError {
            error: "Xcode Rust.ideplugin is out of date!".to_owned(),
        }), // What steps should be recommended?
    };

    (plugin_valid, plugin_bullet)
}

fn validate_xcode_select() {
    // Check xcode-select path, make sure it aligns with xcode_app_dir
    let xcode_app_dir = xcode_plugin::xcode_app_dir()
        .unwrap_or_else(|_err| PathBuf::from("xcode app dir not found"));
    let xcode_select_path = command("xcode-select -p").unwrap_or_else(|_err| "".to_owned());
    let valid_path = xcode_select_path.contains(xcode_app_dir.to_str().unwrap());

    if !valid_path {
        println!("xcode-select path was not in the Xcode app directory:");
        println!("Xcode App Directory: {}", xcode_app_dir.display());
        println!("Xcode Select:        {}", xcode_select_path);

        let updated_path = xcode_app_dir.join("Developer");
        println!("Recommended:         {}", updated_path.display());

        if let Some(answer) = prompt::yes_no(
            "Would you like to update your Xcode-select path?",
            Some(prompt::YesOrNo::Yes),
        )
        .unwrap()
        {
            match answer {
                prompt::YesOrNo::Yes => {
                    command(&format!(
                        "xcode-select -s {}",
                        updated_path.to_str().unwrap()
                    ));
                    ()
                }
                prompt::YesOrNo::No => (),
            }
        }
    }
}

pub fn exec() -> Result<ReportSummary, DoctorException> {
    Ok(ReportSummary {
        reports: vec![
            check_cargo_mobile()?,
            check_rust()?,
            check_os()?,
            check_apple()?,
            check_android()?,
            check_device_list()?,
        ],
    })
}
