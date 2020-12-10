mod xdg;

use std::{
    ffi::{OsStr, OsString},
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
            Self::LookupFailed => write!(f, "Lookup failed"),
        }
    }
}

#[derive(Debug)]
pub enum OpenFileError {
    LaunchFailed(bossy::Error),
}

impl Display for OpenFileError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::LaunchFailed(e) => write!(f, "Launch failed: {}", e),
        }
    }
}

#[derive(Debug)]
pub struct Application {
    exec_command: OsString,
}

impl Application {
    pub fn detect_editor() -> Result<Self, DetectEditorError> {
        let entry = {
            // Try a rust code editor, then a plain text editor, if neither are available,
            // then return an error.
            if let Some(rust_entry) = xdg::query_mime_entry("text/rust") {
                rust_entry
            } else if let Some(plain_entry) = xdg::query_mime_entry("text/plain") {
                plain_entry
            } else {
                return Err(DetectEditorError::LookupFailed);
            }
        };

        for dir in xdg::get_xdg_data_dirs() {
            // Look at the applications folder of that dir
            let dir = dir.join("applications");
            if let Some(result) = xdg::find_entry_in_dir(&dir, &entry) {
                return Ok(Self {
                    exec_command: xdg::command_from_freedesktop_entry(&result)
                        .ok_or(DetectEditorError::LookupFailed)?
                })
            }
        }

        Err(DetectEditorError::LookupFailed)
    }

    pub fn open_file(&self, path: impl AsRef<Path>) -> Result<(), OpenFileError> {
        let path = path.as_ref();

        let command = xdg::build_command(&self.exec_command, path.as_os_str());

        // I'm having a problem creating a dettached process with bossy,
        // but this should work and be well supported on linux systems,
        // as "sh" and no "nohup" are pretty standard.
        bossy::Command::impure(command)
            .run_and_detach()
            .map_err(|e| OpenFileError::LaunchFailed(e))?;

        Ok(())
    }
}

pub fn open_file_with(
    application: impl AsRef<OsStr>,
    path: impl AsRef<OsStr>,
) -> bossy::Result<()> {
    // I really dislike this lossy conversions
    // I feel "less" bad about it after I learned firefox also does it
    // https://support.mozilla.org/en-US/kb/utf-8-only-file-paths
    // But it still isn't perfect.
    let app_str = application.as_ref();
    let path_str = path.as_ref();

    for dir in xdg::get_xdg_data_dirs() {
        let dir = dir.join("applications");
        if let Some(entry) = xdg::find_entry_by_app_name(&dir, &app_str) {

            let command = if let Some(str_entry) = entry.section("Desktop Entry").attr("Exec") {
                // If we have the entry, we return it as an OsString
                let osstring_entry: OsString = str_entry.into();
                xdg::build_command(&osstring_entry, path_str)
            } else {
                // If there is no attribute Exec we may as well try our luck with the app_str.
                // The main reason is that I don't want to change the function return type.
                // It returns a bossy error, not a parse error. If a command with that name
                // exists, it
                app_str.to_os_string()
            };

            bossy::Command::impure(command)
                .run_and_detach()?;
            break;
        }
    }
    Ok(())
}

// We use "sh" in order to access "command -v", as that is a bultin command on sh.
// Linux does not require a binary "command" in path, so this seems the way to go.
#[cfg(target_os = "linux")]
pub fn command_path(name: &str) -> bossy::Result<bossy::Output> {
    bossy::Command::impure("sh")
        .with_args(&["-c", &format!("command -v {}", name)])
        .run_and_wait_for_output()
}
