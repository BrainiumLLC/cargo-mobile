mod xdg;

use std::{
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
            Self::LookupFailed => write!(f, "Lookup failed"),
        }
    }
}

#[derive(Debug)]
pub enum OpenFileError {
    LaunchFailed,
}

impl Display for OpenFileError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::LaunchFailed => write!(f, "Launch failed"),
        }
    }
}

#[derive(Debug)]
pub struct Application {
    exec_command: String,
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
        let path_str = path.to_string_lossy(); // Ugh, lossy

        let command = xdg::build_command(&self.exec_command, &path_str)
            .ok_or(OpenFileError::LaunchFailed)?;

        // I'm having a problem creating a dettached process with bossy,
        // but this should work and be well supported on linux systems,
        // as "sh" and no "nohup" are pretty standard.
        bossy::Command::impure("sh")
            .add_args(&["-c", &format!("nohup {} > /dev/null &", command)])
            .set_stdout(bossy::Stdio::null())
            .set_stderr(bossy::Stdio::null())
            .run_and_wait()
            .map_err(|_| OpenFileError::LaunchFailed)?;

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
    let app_str = application.as_ref().to_string_lossy();
    let path_str = path.as_ref().to_string_lossy();

    for dir in xdg::get_xdg_data_dirs() {
        let dir = dir.join("applications");
        if let Some(entry) = xdg::find_entry_by_app_name(&dir, &app_str) {
            let exec_str = entry.section("Desktop Entry")
                .attr("Exec")
                // If there is no attribute Exec we may as well try our luck with the app_str.
                // The main reason is that I don't want to change the function return type.
                .unwrap_or(&app_str);

            bossy::Command::impure("sh")
                .add_args(&["-c", &format!("nohup \"{}\" \"{}\" > /dev/null &", exec_str, path_str)])
                .set_stdout(bossy::Stdio::null())
                .set_stderr(bossy::Stdio::null())
                .run_and_wait()?;
            break;
        }
    }
    Ok(())
}
