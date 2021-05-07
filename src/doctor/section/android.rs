use super::Section;
use crate::{android::env::Env, util};

pub fn check() -> Section {
    let section = Section::new("Android developer tools");
    match Env::new() {
        Ok(env) => section
            .with_item(
                env.sdk_version()
                    .map(|sdk_version| {
                        format!(
                            "SDK v{} installed at {:?}",
                            sdk_version,
                            // TODO: don't unwrap this (...though it's basically fatal anyway)
                            util::contract_home(env.sdk_root()).unwrap(),
                        )
                    })
                    .map_err(|err| format!("Failed to get SDK version: {}", err)),
            )
            .with_item(
                env.ndk
                    .version()
                    .map(|ndk_version| {
                        format!(
                            "NDK {} installed at {:?}",
                            ndk_version,
                            util::contract_home(env.ndk.home()).unwrap(),
                        )
                    })
                    .map_err(|err| format!("Failed to get NDK version: {}", err)),
            ),
        Err(err) => section.with_failure(err),
    }
}
