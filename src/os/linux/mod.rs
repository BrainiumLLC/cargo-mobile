mod xdg;

use std::{
    ffi::{OsStr, OsString},
    fmt::{self, Display},
    path::{Path, PathBuf},
};

#[derive(Debug)]
pub enum DetectEditorError {
    NoDefaultEditorSet,
    FreeDesktopEntryNotFound,
    FreeDesktopEntryParseError,
}

impl Display for DetectEditorError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::NoDefaultEditorSet => write!(f, "No default editor is set: xdg-mime queries for \"text/rust\" and \"text/plain\" both failed"),
            Self::FreeDesktopEntryNotFound => write!(f, "Entry Not Found: xdg-mime returned an entry name that could not be found"),
            Self::FreeDesktopEntryParseError => write!(f, "Entry Parse Error: xdg-mime returned an entry that could not be parsed"),
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
    icon: Option<OsString>,
    xdg_entry_path: PathBuf,
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
                return Err(DetectEditorError::NoDefaultEditorSet);
            }
        };

        for dir in xdg::get_xdg_data_dirs() {
            // Look at the applications folder of that dir
            let dir = dir.join("applications");
            if let Some(result) = xdg::find_entry_in_dir(&dir, &entry) {
                let parsed = xdg::parse(&result)
                    .ok_or(DetectEditorError::FreeDesktopEntryParseError)?;
                return Ok(Self {
                    exec_command: parsed
                        .section("Desktop Entry")
                        .attr("Exec")
                        .ok_or(DetectEditorError::FreeDesktopEntryParseError)?
                        .into(),
                    icon: parsed
                        .section("Desktop Entry")
                        .attr("Icon")
                        .map(|s| s.into()),
                    xdg_entry_path: dir.join(result),
                })
            }
        }

        Err(DetectEditorError::FreeDesktopEntryNotFound)
    }

    pub fn open_file(&self, path: impl AsRef<Path>) -> Result<(), OpenFileError> {
        let path = path.as_ref();

        let maybe_icon = if let Some(icon_str) = &self.icon {
            Some(icon_str.as_os_str())
        } else {
            None
        };

        // Parse the xdg command field with all the needed data
        let command_parts = xdg::parse_command(
            &self.exec_command,
            path.as_os_str(),
            maybe_icon,
            Some(&self.xdg_entry_path)
        );

        if !command_parts.is_empty() {
            // If command_parts has at least one element this works. If it has a single
            // element, &command_parts[1..] should be an empty slice (&[]) and bossy
            // `add_args` does not add any argument on that case, although the docs
            // do not make it obvious.
            bossy::Command::impure(&command_parts[0])
                .add_args(&command_parts[1..])
                .run_and_detach()
                .map_err(|e| OpenFileError::LaunchFailed(e))?;
        }

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
        if let Some((entry, entry_path)) = xdg::find_entry_by_app_name(&dir, &app_str) {
            let command_parts = if let Some(str_entry) = entry.section("Desktop Entry").attr("Exec") {
                // If we have the entry, we return it as an OsString
                let osstring_entry: OsString = str_entry.into();
                xdg::parse_command(
                    &osstring_entry,
                    path_str,
                    entry.section("Desktop Entry").attr("Icon").map(|s| s.as_ref()),
                    Some(&entry_path),
                )
            } else {
                // If there is no attribute Exec we may as well try our luck with the app_str.
                // The main reason is that I don't want to change the function return type.
                // It returns a bossy error, not a parse error. If a command with that name
                // exists, it
                vec![app_str.to_os_string()]
            };

            if !command_parts.is_empty() {
                bossy::Command::impure(&command_parts[0])
                    .add_args(&command_parts[1..])
                    .run_and_detach()?;
            }
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
