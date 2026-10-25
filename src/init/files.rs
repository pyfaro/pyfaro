use std::fs::{self, Permissions};
use std::io::{self, Write};
use std::path::PathBuf;

use crate::error::{Error, Result};

/// Own only the two metadata files that initialization may change.
pub(super) struct Snapshot {
    path: PathBuf,
    contents: Option<Vec<u8>>,
    permissions: Option<Permissions>,
}

impl Snapshot {
    pub fn read(path: PathBuf) -> Result<Self> {
        let metadata = match fs::symlink_metadata(&path) {
            Ok(metadata) if metadata.is_file() => Some(metadata),
            Ok(_) => {
                return Err(Error::Project(format!(
                    "{} must be a regular file, not a symlink or directory",
                    path.display()
                )));
            }
            Err(error) if error.kind() == io::ErrorKind::NotFound => None,
            Err(error) => return Err(Error::io("inspect", &path, error)),
        };
        let contents = metadata
            .as_ref()
            .map(|_| {
                fs::read(&path).map_err(|error| Error::io("read", &path, error))
            })
            .transpose()?;
        Ok(Self {
            path,
            contents,
            permissions: metadata.map(|meta| meta.permissions()),
        })
    }

    pub fn text(&self) -> Result<Option<&str>> {
        self.contents
            .as_deref()
            .map(|bytes| {
                std::str::from_utf8(bytes).map_err(|error| {
                    Error::io(
                        "read UTF-8 from",
                        &self.path,
                        io::Error::new(io::ErrorKind::InvalidData, error),
                    )
                })
            })
            .transpose()
    }

    pub fn exists(&self) -> bool {
        self.contents.is_some()
    }

    pub fn verify_unchanged(&self) -> Result<()> {
        let current = Self::read(self.path.clone())?;
        if current.contents != self.contents {
            return Err(Error::Project(format!(
                "{} changed while planning; rerun init",
                self.path.display()
            )));
        }
        Ok(())
    }

    /// Fail before anything changes when the file cannot be replaced. Windows
    /// refuses to rename over a read-only file or one that another program
    /// holds open without sharing; Unix replaces both, keeping the mode.
    #[cfg(windows)]
    pub fn ensure_replaceable(&self) -> Result<()> {
        use std::fs::OpenOptions;
        use std::os::windows::fs::OpenOptionsExt;

        const DELETE: u32 = 0x0001_0000;
        const SHARE_ALL: u32 = 0b111;
        if !self.exists() {
            return Ok(());
        }
        if self.permissions.as_ref().is_some_and(Permissions::readonly) {
            return Err(Error::Project(format!(
                "{} is read-only; clear its read-only attribute and rerun init",
                self.path.display()
            )));
        }
        // Renaming over a file needs DELETE access to it.
        OpenOptions::new()
            .access_mode(DELETE)
            .share_mode(SHARE_ALL)
            .open(&self.path)
            .map(drop)
            .map_err(|error| self.replace_error(error))
    }

    #[cfg(not(windows))]
    #[allow(clippy::unused_self, clippy::unnecessary_wraps)]
    pub fn ensure_replaceable(&self) -> Result<()> {
        Ok(())
    }

    /// Name the usual Windows causes of a refused replacement.
    fn replace_error(&self, error: io::Error) -> Error {
        const ACCESS_DENIED: i32 = 5;
        const SHARING_VIOLATION: i32 = 32;
        const LOCK_VIOLATION: i32 = 33;
        if cfg!(windows)
            && matches!(
                error.raw_os_error(),
                Some(ACCESS_DENIED | SHARING_VIOLATION | LOCK_VIOLATION)
            )
        {
            return Error::Project(format!(
                "cannot replace {}: it is read-only or another program has it open ({error}); close that program and rerun init",
                self.path.display()
            ));
        }
        Error::io("replace", &self.path, error)
    }

    pub fn write(&self, contents: &[u8]) -> Result<()> {
        let parent = self.path.parent().ok_or_else(|| {
            Error::Project("missing project directory".into())
        })?;
        let mut file =
            tempfile::NamedTempFile::new_in(parent).map_err(|error| {
                Error::io("create temporary file in", parent, error)
            })?;
        if let Some(permissions) = &self.permissions {
            file.as_file().set_permissions(permissions.clone()).map_err(
                |error| {
                    Error::io("preserve permissions for", &self.path, error)
                },
            )?;
        }
        file.write_all(contents)
            .map_err(|error| Error::io("write", &self.path, error))?;
        file.as_file()
            .sync_all()
            .map_err(|error| Error::io("sync", &self.path, error))?;
        file.persist(&self.path)
            .map_err(|error| self.replace_error(error.error))?;
        Ok(())
    }

    pub fn restore(&self) -> Result<()> {
        if let Some(contents) = &self.contents {
            // An untouched file needs no rewrite, which a locked file refuses.
            if fs::read(&self.path).is_ok_and(|current| current == *contents) {
                return Ok(());
            }
            self.write(contents)
        } else {
            match fs::remove_file(&self.path) {
                Ok(()) => Ok(()),
                Err(error) if error.kind() == io::ErrorKind::NotFound => Ok(()),
                Err(error) => {
                    Err(Error::io("remove incomplete file", &self.path, error))
                }
            }
        }
    }
}
