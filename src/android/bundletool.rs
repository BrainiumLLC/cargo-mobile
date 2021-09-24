use crate::android::config::Config;
use std::path::PathBuf;

pub const BUNDLE_TOOL_JAR_INFO: BundletoolJarInfo = BundletoolJarInfo::new("1.8.0");

pub struct BundletoolJarInfo {
    version: &'static str,
}

impl BundletoolJarInfo {
    const fn new(version: &'static str) -> Self {
        Self { version }
    }

    fn jar(&self) -> String {
        format!("bundletool-all-{}.jar", self.version)
    }

    pub fn jar_path(&self, config: &Config) -> PathBuf {
        config.project_dir().join(self.jar())
    }

    pub fn download_url(&self) -> String {
        format!(
            "https://github.com/google/bundletool/releases/download/{}/{}",
            self.version,
            self.jar()
        )
    }

    pub fn run_command(&self, config: &Config) -> bossy::Command {
        bossy::Command::impure_parse("java -jar").with_arg(self.jar_path(config))
    }
}
