use std::{
    path::{Path, PathBuf},
    env,
};
use freedesktop_entry_parser::Entry as FreeDesktopEntry;
use freedesktop_entry_parser::parse_entry;
use regex::*;

// Detects which .desktop file contains the data on how to handle a given
// mime type (like: "with which program do I open a text/rust file?")
pub fn query_mime_entry(mime_type: &str) -> Option<PathBuf> {
    let out = bossy::Command::impure("xdg-mime")
        .add_args(&["query", "default", mime_type])
        .run_and_wait_for_output()
        .ok()?;
    if let Ok(out_str) = out.stdout_str() {
        log::debug!("query_mime_entry got output \"{}\"", out_str);
        if !out_str.is_empty() {
            return Some(out_str.trim().into())
        }
    }
    None
}

// Returns the first entry on that directory whose filename is equal to target.
//
// This spec is what makes me believe the search is recursive:
// https://specifications.freedesktop.org/desktop-entry-spec/desktop-entry-spec-latest.html
// This other one does not give that idea:
// https://specifications.freedesktop.org/menu-spec/latest/ar01s02.html
pub fn find_entry_in_dir(dir_path: &Path, target: &Path) -> Option<PathBuf> {
    for entry in dir_path.read_dir().ok()? {
        if let Ok(entry) = entry {
            // If it is a file with that same _filename_ (not full path)
            if entry.path().is_file() && entry.file_name() == target {
                return Some(entry.path().into())
            } else if entry.path().is_dir() {
                // I think if there are any dirs on that directory we have to
                // recursively search on them
                if let Some(result) = find_entry_in_dir(&entry.path(), target) {
                    return Some(result);
                }
            }
        }
    }
    None
}

// Creates a command from the path to a freedesktop "Desktop Entry" file.
// These kind of files are generally named "something.desktop"
pub fn command_from_freedesktop_entry(entry: &Path) -> Option<String> {
    let parsed = parse_entry(entry).ok()?;

    let exec_command = parsed.section("Desktop Entry")
        .attr("Exec")?
        .into();

    Some(exec_command)
}

pub fn find_entry_by_app_name(dir_path: &Path, app_name: &str) -> Option<FreeDesktopEntry> {
    for entry in dir_path.read_dir().ok()? {
        if let Ok(entry) = entry {
            // If it is a file we open it
            if entry.path().is_file() {
                if let Ok(parsed) = parse_entry(entry.path()) {
                    if let Some(name) = parsed.section("Desktop Entry").attr("Name") {
                        if name == app_name {
                            return Some(parsed);
                        }
                    }
                }
            } else if entry.path().is_dir() {
                // Recursively keep searching if it is a directory
                if let Some(result) = find_entry_by_app_name(&entry.path(), app_name) {
                    return Some(result);
                }
            }
        }
    }
    None
}

// The exec field of the FreeDesktop entry may contain some flags that need to
// be replaced by parameters or even other stuff. The other things are still
// not implemented
pub fn build_command(command: &str, argument: &str) -> Option<String> {
    let mut command = command.to_string();

    let arg_re = regex!(r"%u|%U|%f|%F");
    while let Some(mat) = arg_re.find(&command) {
        let start = mat.start();
        let end = mat.end();
        command.replace_range(start..end, argument);
    }

    Some(command)
}

// Returns a vector of all the relevant xdg desktop application entries
// Check out:
// https://wiki.archlinux.org/index.php/XDG_Base_Directory
// That explains the default values and the relevant variables.
pub fn get_xdg_data_dirs() -> Vec<PathBuf> {
    let mut result = Vec::new();

    if let Ok(home) = env::var("HOME") {
        let home: PathBuf = home.into();
        let xdg_data_home: PathBuf = if let Ok(var) = env::var("XDG_DATA_HOME") {
            var.into()
        } else {
            home.join(".local/share") // The default
        };
        result.push(xdg_data_home);
    }

    if let Ok(var) = env::var("XDG_DATA_DIRS") {
        let entries = var.split(":").map(|dirname| PathBuf::from(dirname));
        result.extend(entries);
    } else {
        // These are the default ones we'll use in case the var is not set
        result.push(PathBuf::from("/usr/local/share"));
        result.push(PathBuf::from("/usr/share"));
    };

    result
}
