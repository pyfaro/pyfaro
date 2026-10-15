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
            .map_err(|error| Error::io("replace", &self.path, error.error))?;
        Ok(())
    }

    pub fn restore(&self) -> Result<()> {
        if let Some(contents) = &self.contents {
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
