use path_abs::PathAbs;
use path_slash::PathExt;
use std::{
    fmt::{self, Display},
    io,
    path::{Path, PathBuf},
};
use thiserror::Error;

#[cfg(target_os = "windows")]
use path_slash::PathBufExt;

#[derive(Debug, Error)]
#[error("Failed to get user's home directory!")]
pub struct NoHomeDir;

pub fn home_dir() -> Result<PathBuf, NoHomeDir> {
    home::home_dir().ok_or(NoHomeDir)
}

pub fn expand_home(path: impl AsRef<Path>) -> Result<PathBuf, NoHomeDir> {
    let home = home_dir()?;
    let path = path.as_ref();
    if let Ok(path) = path.strip_prefix("~") {
        Ok(home.join(path))
    } else {
        Ok(path.to_owned())
    }
}

pub fn install_dir() -> Result<PathBuf, NoHomeDir> {
    home_dir().map(|home| home.join(concat!(".", env!("CARGO_PKG_NAME"))))
}

pub fn checkouts_dir() -> Result<PathBuf, NoHomeDir> {
    install_dir().map(|install_dir| install_dir.join("checkouts"))
}

pub fn temp_dir() -> PathBuf {
    std::env::temp_dir().join("com.brainiumstudios.cargo-mobile")
}

#[derive(Debug)]
pub struct PathNotPrefixed {
    path: PathBuf,
    prefix: PathBuf,
}

impl Display for PathNotPrefixed {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "Path {:?} didn't have prefix {:?}.",
            self.path, self.prefix
        )
    }
}

pub fn prefix_path(root: impl AsRef<Path>, path: impl AsRef<Path>) -> PathBuf {
    root.as_ref().join(path)
}

pub fn unprefix_path(
    root: impl AsRef<Path>,
    path: impl AsRef<Path>,
) -> Result<PathBuf, PathNotPrefixed> {
    let root = root.as_ref();
    let path = path.as_ref();
    path.strip_prefix(root)
        .map(|path| path.to_owned())
        .map_err(|_| PathNotPrefixed {
            path: path.to_owned(),
            prefix: root.to_owned(),
        })
}

/// If `path` is a [DOS device path](https://docs.microsoft.com/en-us/dotnet/standard/io/file-path-formats#dos-device-paths) `dedos_maybe` returns a new path with the "\\\\?\\" prefix trimmed otherwise it is an identity function.
/// Calls `path.to_str()` internally and in case that is a `None` or `path` does not have the prefix it just returns a new `PathBuf` from `path`.
fn dedos_maybe(path: &Path) -> PathBuf {
    match path.to_str() {
        Some(s) => match s.strip_prefix("\\\\?\\") {
            Some(t) => PathBuf::from(t),
            None => path.to_path_buf(),
        },
        None => path.to_path_buf(),
    }
}

/// If `path` contains any `\\` they will be replaced with `/`.
/// Calls `path.to_slash()` internally and in case that is a `None` it just returns `path.to_path_buf()`.
fn slash_maybe(path: &Path) -> PathBuf {
    match path.to_slash() {
        Some(s) => PathBuf::from(s),
        None => path.to_path_buf(),
    }
}

/// Composes `slash_maybe` and `dedos_maybe`.
/// So far, the sanest approach to tackling Windows path specification horrors...
pub fn unwin_maybe(path: &Path) -> PathBuf {
    slash_maybe(&dedos_maybe(path))
}

fn common_root(abs_src: &Path, abs_dest: &Path) -> PathBuf {
    let mut dest_root = abs_dest.to_owned();
    loop {
        if abs_src.starts_with(&dest_root) {
            return dest_root;
        } else {
            if !dest_root.pop() {
                unreachable!("`abs_src` and `abs_dest` have no common root");
            }
        }
    }
}

/// Transforms `abs_path` to be relative to `abs_relative_to`.
#[cfg(not(target_os = "windows"))]
pub fn relativize_path(abs_path: impl AsRef<Path>, abs_relative_to: impl AsRef<Path>) -> PathBuf {
    let (abs_path, abs_relative_to) = (abs_path.as_ref(), abs_relative_to.as_ref());
    assert!(abs_path.is_absolute());
    assert!(abs_relative_to.is_absolute());
    let (path, relative_to) = {
        let common_root = common_root(abs_path, abs_relative_to);
        let path = abs_path.strip_prefix(&common_root).unwrap();
        let relative_to = abs_relative_to.strip_prefix(&common_root).unwrap();
        (path, relative_to)
    };
    let mut rel_path = PathBuf::new();
    for _ in 0..relative_to.iter().count() {
        rel_path.push("..");
    }
    let rel_path = rel_path.join(path);
    log::info!(
        "{:?} relative to {:?} is {:?}",
        abs_path,
        abs_relative_to,
        rel_path
    );
    rel_path
}

/// Transforms `abs_path` to be relative to `abs_relative_to`.
#[cfg(target_os = "windows")]
pub fn relativize_path(abs_path: impl AsRef<Path>, abs_relative_to: impl AsRef<Path>) -> PathBuf {
    let (abs_path, abs_relative_to) = (abs_path.as_ref(), abs_relative_to.as_ref());
    assert!(abs_path.is_absolute());
    assert!(abs_relative_to.is_absolute());
    let (path, relative_to) = {
        let common_root = common_root(abs_path, abs_relative_to);
        let path = abs_path.strip_prefix(&common_root).unwrap();
        let relative_to = abs_relative_to.strip_prefix(&common_root).unwrap();
        (path, relative_to)
    };
    let mut rel_path = PathBuf::new();
    // NOTE
    //   the original loop
    //     for _ in 0..relative_to.iter().count() { rel_path.push(".."); }
    //   is pushing 1 excess ".." on my windows wormhole
    // FIX
    //   now substracting path component count
    for _ in 0..relative_to.iter().count() - path.iter().count() {
        rel_path.push("..");
    }
    // NOTE to_slash() changes \\ 2 / ...
    // ...required 4 rootDirRel in build.bradle.kts (ya on windows)
    let rel_path = PathBuf::from(rel_path.join(path).to_slash().unwrap());
    log::info!(
        "{:?} relative to {:?} is {:?}",
        abs_path,
        abs_relative_to,
        rel_path
    );
    rel_path
}

#[derive(Debug)]
pub enum NormalizationError {
    CanonicalizationFailed {
        path: PathBuf,
        cause: io::Error,
    },
    PathAbsFailed {
        path: PathBuf,
        cause: path_abs::Error,
    },
}

impl Display for NormalizationError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::CanonicalizationFailed { path, cause } => write!(
                f,
                "Failed to canonicalize existing path {:?}: {}",
                path, cause
            ),
            Self::PathAbsFailed { path, cause } => write!(
                f,
                "Failed to normalize non-existent path {:?}: {}",
                path, cause
            ),
        }
    }
}

pub fn normalize_path(path: impl AsRef<Path>) -> Result<PathBuf, NormalizationError> {
    let path = path.as_ref();
    if path.exists() {
        path.canonicalize()
            .map_err(|cause| NormalizationError::CanonicalizationFailed {
                path: path.to_owned(),
                cause,
            })
    } else {
        PathAbs::new(path)
            .map_err(|cause| NormalizationError::PathAbsFailed {
                path: path.to_owned(),
                cause,
            })
            .map(|abs| abs.as_path().to_owned())
    }
}

pub fn under_root(
    path: impl AsRef<Path>,
    root: impl AsRef<Path>,
) -> Result<bool, NormalizationError> {
    normalize_path(root.as_ref().join(path)).map(|norm| norm.starts_with(root))
}
