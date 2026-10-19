use std::fs;
use std::io;
use std::path::{Path, PathBuf};

use crate::error::{Error, Result};

pub(crate) fn select(explicit: Option<&Path>) -> Result<PathBuf> {
    let start = explicit.unwrap_or_else(|| Path::new("."));
    find(start, explicit.is_some())
}

pub(crate) fn discover(start: &Path) -> Result<PathBuf> {
    find(start, false)
}

fn find(start: &Path, exact: bool) -> Result<PathBuf> {
    let directory = fs::canonicalize(start).map_err(|source| {
        Error::io("resolve project directory", start, source)
    })?;
    if !directory.is_dir() {
        return Err(Error::Project(format!(
            "project path is not a directory: {}",
            directory.display()
        )));
    }
    let candidates: Vec<&Path> = if exact {
        vec![directory.as_path()]
    } else {
        directory.ancestors().collect()
    };
    for candidate in candidates {
        let metadata = candidate.join("pyproject.toml");
        match fs::metadata(&metadata) {
            Ok(entry) if entry.is_file() => return Ok(candidate.to_owned()),
            Ok(_) => {
                return Err(Error::Project(format!(
                    "project metadata is not a file: {}",
                    metadata.display()
                )));
            }
            Err(source) if source.kind() == io::ErrorKind::NotFound => {}
            Err(source) => {
                return Err(Error::io(
                    "inspect project metadata",
                    &metadata,
                    source,
                ));
            }
        }
    }
    if exact {
        Err(Error::Project(format!(
            "project directory has no pyproject.toml: {}",
            directory.display()
        )))
    } else {
        Err(Error::NoProject { directory })
    }
}

pub(crate) fn target(path: &Path) -> Result<PathBuf> {
    let absolute = std::path::absolute(path)
        .map_err(|source| Error::io("resolve", path, source))?;
    let mut ancestor = absolute.as_path();
    let mut missing = Vec::new();
    while !ancestor
        .try_exists()
        .map_err(|source| Error::io("inspect", ancestor, source))?
    {
        let name = ancestor.file_name().ok_or_else(|| {
            Error::Project(format!(
                "cannot resolve {}; use a path with an existing parent",
                path.display()
            ))
        })?;
        missing.push(name.to_owned());
        ancestor = ancestor.parent().ok_or_else(|| {
            Error::Project("missing project parent directory".into())
        })?;
    }
    let mut root = fs::canonicalize(ancestor)
        .map_err(|source| Error::io("resolve", ancestor, source))?;
    for name in missing.into_iter().rev() {
        root.push(name);
    }
    Ok(root)
}
