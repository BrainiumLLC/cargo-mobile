use super::{Item, Section};
use crate::{android::env::Env, util};

pub fn check() -> Section {
    let section = Section::new("Android developer tools");
    match Env::new() {
        Ok(env) => section
            .with_item(match env.sdk_version() {
                Ok(sdk_version) => Item::victory(format!(
                    "SDK v{} installed at {:?}",
                    sdk_version,
                    // TODO: don't unwrap this (...though it's basically fatal anyway)
                    util::contract_home(env.sdk_root()).unwrap(),
                )),
                Err(err) => Item::failure(format!("Failed to get SDK version: {}", err)),
            })
            .with_item(match env.ndk.version() {
                Ok(ndk_version) => Item::victory(format!(
                    "NDK {} installed at {:?}",
                    ndk_version,
                    util::contract_home(env.ndk.home()).unwrap(),
                )),
                Err(err) => Item::failure(format!("Failed to get NDK version: {}", err)),
            }),
        Err(err) => section.with_item(Item::failure(err)),
    }
}
