use std::{
    convert::From,
    ffi::OsStr,
    fmt::{self, Display},
    path::Path,
};

#[derive(Debug)]
pub enum DetectEditorError {
    LookupFailed,
}

impl Display for DetectEditorError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::LookupFailed => write!(f, "editor lookup failed"),
        }
    }
}

#[derive(Debug)]
pub enum OpenFileError {
    BossyFailed(bossy::Error),
}

impl Display for OpenFileError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::BossyFailed(err) => write!(f, "bossy failed: {:?}", err),
        }
    }
}

impl From<bossy::Error> for OpenFileError {
    fn from(error: bossy::Error) -> Self {
        OpenFileError::BossyFailed(error)
    }
}

#[derive(Debug)]
pub struct Application {
    url: String,
}

impl Application {
    pub fn detect_editor() -> Result<Self, DetectEditorError> {
        panic!("not implemented");
    }

    pub fn open_file(&self, _path: impl AsRef<Path>) -> Result<(), OpenFileError> {
        panic!("not implemented");
    }
}

pub fn open_file_with(
    _application: impl AsRef<OsStr>,
    _path: impl AsRef<OsStr>,
) -> bossy::Result<()> {
    panic!("not implemented");
}
