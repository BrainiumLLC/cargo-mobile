use super::{Item, Section};
use crate::{
    android::{self, adb},
    env::Env,
};

pub fn check() -> Section {
    let section = Section::new("Connected devices");

    #[cfg(target_os = "macos")]
    let section = {
        use crate::apple::ios_deploy;
        // TODO: don't unwrap
        let env = Env::new().unwrap();
        match ios_deploy::device_list(&env) {
            Ok(list) => section.with_items(list.into_iter().map(Item::victory)),
            Err(err) => section.with_item(Item::failure(format!(
                "Failed to get iOS device list: {}",
                err
            ))),
        }
    };

    let section = if let Ok(env) = android::env::Env::new() {
        match adb::device_list(&env) {
            Ok(list) => section.with_items(list.into_iter().map(Item::victory)),
            // TODO: impl Display for this error
            Err(err) => section.with_item(Item::failure(format!(
                "Failed to get Android device list: {:?}",
                err
            ))),
        }
    } else {
        section
    };

    if section.is_empty() {
        section.with_item(Item::victory("No connected devices were found"))
    } else {
        section
    }
}
