use std::{
    ffi::OsStr,
    fmt::{self, Display},
    path::{Path, PathBuf},
};

pub type OsErrorCode = isize;

#[derive(Debug)]
pub enum DetectEditorError {
    LookupFailed(OsErrorCode),
}

impl Display for DetectEditorError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::LookupFailed(err) => write!(f, "{}", err),
        }
    }
}

#[derive(Debug)]
pub enum OpenFileError {
    PathToUrlFailed { path: PathBuf },
    LaunchFailed(OsErrorCode),
}

impl Display for OpenFileError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::PathToUrlFailed { path } => {
                write!(f, "Failed to convert path {:?} into a `CFURL`.", path)
            }
            Self::LaunchFailed(status) => write!(f, "Status code {}", status),
        }
    }
}

#[derive(Debug)]
pub struct Application {
}

impl Application {
    pub fn detect_editor() -> Result<Self, DetectEditorError> {
        unimplemented!()
    }

    pub fn open_file(&self, _path: impl AsRef<Path>) -> Result<(), OpenFileError> {
        unimplemented!()
    }
}

pub fn open_file_with(
    application: impl AsRef<OsStr>,
    path: impl AsRef<OsStr>,
) -> bossy::Result<()> {
    // I have to test whether this is really it
    bossy::Command::impure("nohup")
        .with_args(&[application.as_ref(), path.as_ref()])
        .set_stdout(bossy::Stdio::null())
        .set_stderr(bossy::Stdio::null())
        .run_and_wait()?;
    Ok(())
}
