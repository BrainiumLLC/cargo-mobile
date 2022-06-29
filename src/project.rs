use crate::{
    config::Config,
    templating::{self, FancyPackResolveError},
    util::{
        cli::{Report, Reportable},
        prompt, Git,
    },
};
use std::path::PathBuf;

#[derive(Debug)]
pub enum Error {
    GitInitFailed(bossy::Error),
    TemplatePackResolveFailed(FancyPackResolveError),
    ProcessingFailed {
        src: PathBuf,
        dest: PathBuf,
        cause: bicycle::ProcessingError,
    },
    PromptFailed(std::io::Error),
    OverwriteFilePermissionDenied,
}

impl Reportable for Error {
    fn report(&self) -> Report {
        match self {
            Self::GitInitFailed(err) => Report::error("Failed to initialize git", err),
            Self::TemplatePackResolveFailed(err) => {
                Report::error("Failed to resolve template pack", err)
            }
            Self::ProcessingFailed { src, dest, cause } => Report::error(
                format!(
                    "Base project template processing from src {:?} to dest {:?} failed",
                    src, dest,
                ),
                cause,
            ),
            Self::PromptFailed(err) => Report::error(
                "Failed to prompt to for permission to overwrite project files",
                err,
            ),
            Self::OverwriteFilePermissionDenied => {
                Report::error("Failed to get persmission to overwrite project files", "")
            }
        }
    }
}

pub fn gen(
    config: &Config,
    bike: &bicycle::Bicycle,
    filter: &templating::Filter,
    submodule_commit: Option<String>,
    dot_first_init_exists: bool,
) -> Result<(), Error> {
    println!("Generating base project...");
    let root = config.app().root_dir();
    let git = Git::new(&root);
    git.init().map_err(Error::GitInitFailed)?;
    let pack_chain = config
        .app()
        .template_pack()
        .resolve(git, submodule_commit.as_deref())
        .map_err(Error::TemplatePackResolveFailed)?;
    log::info!("template pack chain: {:#?}", pack_chain);
    for pack in pack_chain {
        log::info!("traversing template pack {:#?}", pack);
        if dot_first_init_exists {
            let to_overwrite = {
                let hbs = std::ffi::OsStr::new("hbs");
                walkdir::WalkDir::new(pack)
                    .into_iter()
                    .filter_map(|entry| entry.ok())
                    .map(|entry| entry.path().strip_prefix(pack).unwrap().to_owned())
                    .map(|path| {
                        if path.extension() == Some(hbs) {
                            PathBuf::from(path.file_stem().unwrap())
                        } else {
                            path
                        }
                    })
                    .filter(|path| path.exists() && !path.is_dir())
                    .collect::<Vec<_>>()
            };
            if !to_overwrite.is_empty() {
                log::warn!("first `cargo mobile init` expects a fresh project setup");
                if prompt::yes_no(
                    format!(
                        "the following files will be overwritten:\n{:#?}\nOverwrite files?",
                        to_overwrite
                    ),
                    Some(prompt::YesOrNo::Yes),
                )
                .map_err(Error::PromptFailed)?
                .unwrap_or(prompt::YesOrNo::No)
                .no()
                {
                    return Err(Error::OverwriteFilePermissionDenied);
                }
            }
        }
        bike.filter_and_process(&pack, &root, |_| (), filter.fun())
            .map_err(|cause| Error::ProcessingFailed {
                src: pack.to_owned(),
                dest: root.to_owned(),
                cause,
            })?;
    }
    Ok(())
}
