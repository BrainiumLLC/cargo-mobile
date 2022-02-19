use crate::util::{
    ln::{Clobber, Error, ErrorCause, LinkType, TargetStyle},
    prefix_path,
};
use std::{
    borrow::Cow,
    fs::{remove_dir_all, remove_file},
    os::windows::ffi::OsStrExt,
    path::Path,
};
use windows::{
    runtime::{self, Handle as _},
    Win32::{
        Foundation::{CloseHandle, ERROR_PRIVILEGE_NOT_HELD, HANDLE, PWSTR},
        Storage::FileSystem::{
            CreateFileW, FILE_ACCESS_FLAGS, FILE_FLAG_BACKUP_SEMANTICS, FILE_FLAG_DELETE_ON_CLOSE,
            FILE_FLAG_OPEN_REPARSE_POINT, FILE_SHARE_READ, OPEN_EXISTING,
        },
        System::SystemServices::GENERIC_READ,
    },
};

pub fn force_symlink(
    source: impl AsRef<Path>,
    target: impl AsRef<Path>,
    target_style: TargetStyle,
) -> Result<(), Error> {
    let (source, target) = (source.as_ref(), target.as_ref());
    let error = |cause: ErrorCause| {
        Error::new(
            LinkType::Symbolic,
            Clobber::FileOnly,
            source.to_owned(),
            target.to_owned(),
            target_style,
            cause,
        )
    };
    let target = if target_style == TargetStyle::Directory {
        let file_name = if let Some(file_name) = source.file_name() {
            file_name
        } else {
            return Err(error(ErrorCause::MissingFileName));
        };
        Cow::Owned(target.join(file_name))
    } else {
        Cow::Borrowed(target)
    };
    let is_directory = target
        .parent()
        .map(|parent| prefix_path(parent, source).is_dir())
        .unwrap_or(false);
    if is_symlink(&target) {
        delete_symlink(&target).map_err(|err| error(ErrorCause::IOError(err.into())))?;
    } else if target.is_file() {
        remove_file(&target).map_err(|err| error(ErrorCause::IOError(err)))?;
    } else if target.is_dir() {
        remove_dir_all(&target).map_err(|err| error(ErrorCause::IOError(err)))?;
    }
    let result = if is_directory {
        std::os::windows::fs::symlink_dir(source, target)
    } else {
        std::os::windows::fs::symlink_file(source, target)
    };
    result.map_err(|err| {
        if err.raw_os_error() == Some(ERROR_PRIVILEGE_NOT_HELD.0 as i32) {
            error(ErrorCause::SymlinkNotAllowed)
        } else {
            error(ErrorCause::IOError(err))
        }
    })?;
    Ok(())
}

pub fn force_symlink_relative(
    abs_source: impl AsRef<Path>,
    abs_target: impl AsRef<Path>,
    target_style: TargetStyle,
) -> Result<(), Error> {
    let (abs_source, abs_target) = (abs_source.as_ref(), abs_target.as_ref());
    let rel_source = crate::util::relativize_path(abs_source, abs_target);
    if target_style == TargetStyle::Directory && rel_source.file_name().is_none() {
        if let Some(file_name) = abs_source.file_name() {
            force_symlink(rel_source, abs_target.join(file_name), TargetStyle::File)
        } else {
            Err(Error::new(
                LinkType::Symbolic,
                Clobber::FileOnly,
                rel_source,
                abs_target.to_owned(),
                target_style,
                ErrorCause::MissingFileName,
            ))
        }
    } else {
        force_symlink(rel_source, abs_target, target_style)
    }
}

fn delete_symlink(filename: &Path) -> Result<(), runtime::Error> {
    let filename = filename
        .as_os_str()
        .encode_wide()
        .chain([0])
        .collect::<Vec<_>>();
    let handle = FileHandle(unsafe {
        CreateFileW(
            PWSTR(filename.as_ptr() as _),
            FILE_ACCESS_FLAGS(GENERIC_READ),
            FILE_SHARE_READ,
            std::ptr::null(),
            OPEN_EXISTING,
            FILE_FLAG_BACKUP_SEMANTICS | FILE_FLAG_OPEN_REPARSE_POINT | FILE_FLAG_DELETE_ON_CLOSE,
            HANDLE(0),
        )
    });
    if handle.is_invalid() {
        return Err(runtime::Error::from_win32());
    }
    Ok(())
}

fn is_symlink(filename: &Path) -> bool {
    if let Ok(metadata) = std::fs::symlink_metadata(filename) {
        metadata.file_type().is_symlink()
    } else {
        false
    }
}

struct FileHandle(HANDLE);

impl FileHandle {
    fn is_invalid(&self) -> bool {
        self.0.is_invalid()
    }
}

impl Drop for FileHandle {
    fn drop(&mut self) {
        if !self.is_invalid() {
            unsafe { CloseHandle(self.0) };
        }
    }
}
