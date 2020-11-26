use crate::util::cli::{Report, Reportable};
use std::{
    ffi::OsStr,
    fmt::{self, Debug, Display},
    path::Path,
};

pub trait ExplicitEnv: Debug {
    fn explicit_env(&self) -> Vec<(&str, &OsStr)>;
}

#[derive(Debug)]
pub enum Error {
    HomeNotSet(std::env::VarError),
    PathNotSet(std::env::VarError),
    #[cfg(target_os = "windows")]
    SystemRootNotSet(std::env::VarError),
    #[cfg(target_os = "windows")]
    OsNotSet(std::env::VarError),
    #[cfg(target_os = "windows")]
    JavaHomeNotSet(std::env::VarError),
}

impl Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::HomeNotSet(err) => write!(
                f,
                "The `HOME` environment variable isn't set, which is pretty weird: {}",
                err
            ),
            Self::PathNotSet(err) => write!(
                f,
                "The `PATH` environment variable isn't set, which is super weird: {}",
                err
            ),
            #[cfg(target_os = "windows")]
            Self::SystemRootNotSet(err) => write!(
                f,
                "The `SYSTEMROOT` environment variable isn't set, which is super weird: {}",
                err
            ),
            #[cfg(target_os = "windows")]
            Self::OsNotSet(err) => write!(
                f,
                "The `OS` environment variable isn't set, which is super weird: {}",
                err
            ),
            #[cfg(target_os = "windows")]
            Self::JavaHomeNotSet(err) => write!(
                f,
                "The `JAVA_HOME` environment variable isn't set, which is super weird: {}",
                err
            ),
        }
    }
}

impl Reportable for Error {
    fn report(&self) -> Report {
        Report::error("Failed to initialize base environment", self)
    }
}

#[derive(Debug)]
pub struct Env {
    home: String,
    path: String,
    term: Option<String>,
    ssh_auth_sock: Option<String>,
    #[cfg(target_os = "windows")]
    system_root: String,
    #[cfg(target_os = "windows")]
    os: String,
    #[cfg(target_os = "windows")]
    java_home: String,
}

impl Env {
    #[cfg(not(target_os = "windows"))]
    pub fn new() -> Result<Self, Error> {
        let home = std::env::var("HOME").map_err(Error::HomeNotSet)?;
        let path = std::env::var("PATH").map_err(Error::PathNotSet)?;
        let term = std::env::var("TERM").ok();
        let ssh_auth_sock = std::env::var("SSH_AUTH_SOCK").ok();
        Ok(Self {
            home,
            path,
            term,
            ssh_auth_sock,
        })
    }

    #[cfg(target_os = "windows")]
    pub fn new() -> Result<Self, Error> {
        let home = std::env::var("USERPROFILE").map_err(Error::HomeNotSet)?;
        let path = std::env::var("PATH").map_err(Error::PathNotSet)?;
        let system_root = std::env::var("SYSTEMROOT").map_err(Error::SystemRootNotSet)?;
        let os = std::env::var("OS").map_err(Error::OsNotSet)?;
        let java_home = std::env::var("JAVA_HOME").map_err(Error::JavaHomeNotSet)?;
        let term = std::env::var("TERM").ok();
        let ssh_auth_sock = std::env::var("SSH_AUTH_SOCK").ok();
        Ok(Self {
            home,
            path,
            term,
            ssh_auth_sock,
            system_root,
            os,
            java_home,
        })
    }

    pub fn path(&self) -> &str {
        &self.path
    }

    pub fn prepend_to_path(mut self, path: impl AsRef<Path>) -> Self {
        self.path = format!("{}:{}", path.as_ref().display(), self.path);
        self
    }
}

impl ExplicitEnv for Env {
    #[cfg(not(target_os = "windows"))]
    fn explicit_env(&self) -> Vec<(&str, &std::ffi::OsStr)> {
        let mut env = vec![("HOME", self.home.as_ref()), ("PATH", self.path.as_ref())];
        if let Some(term) = self.term.as_ref() {
            env.push(("TERM", term.as_ref()));
        }
        if let Some(ssh_auth_sock) = self.ssh_auth_sock.as_ref() {
            env.push(("SSH_AUTH_SOCK", ssh_auth_sock.as_ref()));
        }
        env
    }

    #[cfg(target_os = "windows")]
    fn explicit_env(&self) -> Vec<(&str, &std::ffi::OsStr)> {
        let mut env = vec![
            ("USERPROFILE", self.home.as_ref()),
            ("PATH", self.path.as_ref()),
            ("SYSTEMROOT", self.system_root.as_ref()),
            ("OS", self.os.as_ref()),
            ("JAVA_HOME", self.java_home.as_ref()),
        ];
        if let Some(term) = self.term.as_ref() {
            env.push(("TERM", term.as_ref()));
        }
        if let Some(ssh_auth_sock) = self.ssh_auth_sock.as_ref() {
            env.push(("SSH_AUTH_SOCK", ssh_auth_sock.as_ref()));
        }
        env
    }
}
