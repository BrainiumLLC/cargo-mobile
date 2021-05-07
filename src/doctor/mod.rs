mod section;

use self::section::Section;
use crate::{
    env::{self, Env},
    util::{self, cli::TextWrapper},
};
use thiserror::Error;

// This should only be used for errors that we *really* don't expect and/or
// that violate core assumptions made throughout the program.
#[derive(Debug, Error)]
pub enum Unrecoverable {
    // Only encountered if the most basic environment variables are absent or
    // unreadable
    #[error(transparent)]
    EnvInitFailed(#[from] env::Error),
    // Only encountered if A) the user has no home directory, or B) either the
    // home or some other path isn't valid UTF-8
    #[error("Failed to prettify path: {0}")]
    ContractHomeFailed(#[from] util::ContractHomeError),
}

#[derive(Debug)]
pub struct Doctor {
    sections: Vec<Section>,
}

impl Doctor {
    pub fn check() -> Result<Self, Unrecoverable> {
        let env = Env::new()?;
        Ok(Self {
            sections: vec![
                section::cargo_mobile::check()?,
                #[cfg(target_os = "macos")]
                section::apple::check(),
                section::android::check(&env)?,
                section::device_list::check(&env),
            ],
        })
    }

    pub fn print(&self, wrapper: &TextWrapper) {
        for section in &self.sections {
            println!();
            section.print(wrapper);
        }
    }
}
