use std::{io, fmt::{Display, self}, ffi::{OsString, OsStr}, path::{PathBuf, Path}};

use self::windows::{detect_type_editor, FileType};

pub (super) mod info;
mod windows;

#[derive(Debug)]
pub enum DetectEditorError {
    NoDefaultEditorSet,
    ExecFieldMissing,
}


impl Display for DetectEditorError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::NoDefaultEditorSet => write!(f, "No default editor is set: registry queries for \".rs\" and \".txt\" both failed"),
            Self::ExecFieldMissing => write!(f, "Exec field on desktop entry was not found"),
        }
    }
}

#[derive(Debug)]
pub enum OpenFileError {
    LaunchFailed(bossy::Error),
    CommandParsingFailed,
}

impl Display for OpenFileError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::LaunchFailed(e) => write!(f, "Launch failed: {}", e),
            Self::CommandParsingFailed => write!(f, "Command parsing failed"),
        }
    }
}

#[derive(Debug)]
pub struct Application {
    exec_command: OsString,
}


impl Application {
    pub fn detect_editor() -> Result<Self, DetectEditorError> {
        if let Ok(command) = detect_type_editor(FileType::Rust) {
            return Ok(Self{exec_command: command})
        } else {
            return match detect_type_editor(FileType::Text) {
                Ok(c) => Ok(Self{exec_command: c}),
                Err(_) => Err(DetectEditorError::NoDefaultEditorSet)
            }
        }
    }

    pub fn open_file(&self, path: impl AsRef<Path>) -> Result<(), OpenFileError> {
        let path = path.as_ref();
        let command_parts = vec![&self.exec_command, path.as_os_str()];

        bossy::Command::impure(&command_parts[0])
            .with_args(&command_parts[1..])
            .run_and_detach()
            .map_err(OpenFileError::LaunchFailed)
    }
}

pub fn open_file_with(
    application: impl AsRef<OsStr>,
    path: impl AsRef<OsStr>,
) -> bossy::Result<()> {
    let application = application.as_ref();
    let path = path.as_ref();

    let command_parts = vec![application.to_os_string(), path.to_os_string()];

    bossy::Command::impure(&command_parts[0])
            .with_args(&command_parts[1..])
            .run_and_detach()
}

// We use "sh" in order to access "command -v", as that is a bultin command on sh.
// Linux does not require a binary "command" in path, so this seems the way to go.
#[cfg(target_os = "windows")]
pub fn command_path(name: &str) -> bossy::Result<bossy::Output> {
    bossy::Command::impure("powershell")
        .with_args(&["-Command", name])
        .run_and_wait_for_output()
}
