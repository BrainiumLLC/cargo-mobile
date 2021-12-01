use thiserror::Error;

#[derive(Debug, Error)]
pub enum Error {
    #[cfg(not(target_os = "macos"))]
    #[error("Failed to check if `git-lfs` is present: {0}")]
    CheckFailed(#[source] bossy::Error),
    #[cfg(not(target_os = "macos"))]
    #[error("Git LFS isn't installed; please install it and try again")]
    // TODO: this should be an action request
    InstallNeeded,
    #[cfg(target_os = "macos")]
    #[error(transparent)]
    BrewFailed(#[from] crate::apple::deps::Error),
    #[error("Failed to run `git lfs install`: {0}")]
    InstallFailed(#[source] bossy::Error),
}

pub fn ensure_present() -> Result<(), Error> {
    #[cfg(not(target_os = "macos"))]
    {
        if !crate::util::command_present("git-lfs").map_err(Error::CheckFailed)? {
            return Err(Error::InstallNeeded);
        }
    }
    #[cfg(target_os = "macos")]
    {
        use crate::apple::deps;
        // This only installs if not already present, so there's no need for us
        // to check here.
        if deps::PackageSpec::brew("git-lfs")
            .install(Default::default(), &mut deps::GemCache::new())
            .map_err(Error::from)?
        {
            println!("Running `git lfs install` for you...");
        }
    }
    bossy::Command::impure_parse("git lfs install")
        .run_and_wait()
        .map_err(Error::InstallFailed)?;
    Ok(())
}
