use super::Section;
use crate::{
    android::{self, adb},
    env::Env,
};

pub fn check(env: &Env) -> Section {
    let section = Section::new("Connected devices");

    #[cfg(target_os = "macos")]
    let section = {
        use crate::apple::ios_deploy;
        match ios_deploy::device_list(env) {
            Ok(list) => section.with_victories(list),
            Err(err) => section.with_failure(format!("Failed to get iOS device list: {}", err)),
        }
    };

    let section = if let Ok(android_env) = android::env::Env::from_env(env.clone()) {
        match adb::device_list(&android_env) {
            Ok(list) => section.with_victories(list),
            Err(err) => section.with_failure(format!("Failed to get Android device list: {}", err)),
        }
    } else {
        section
    };

    if section.is_empty() {
        section.with_victory("No connected devices were found")
    } else {
        section
    }
}
