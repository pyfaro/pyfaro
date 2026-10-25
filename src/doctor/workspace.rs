use std::fs;
use std::path::{Path, PathBuf};

use glob::Pattern;
use toml_edit::DocumentMut;

use crate::error::Result;
use crate::tools;

use super::{Report, Severity};

pub(super) fn inspect_workspace(
    root: &Path,
    document: &DocumentMut,
    report: &mut Report,
) -> Result<bool> {
    if document
        .get("tool")
        .and_then(|tool| tool.get("uv"))
        .is_some_and(|uv| uv.as_table_like().is_none())
    {
        report.line(
            Severity::Error,
            "configuration: [tool.uv] in pyproject.toml must be a table",
        )?;
        return Ok(true);
    }
    if document
        .get("tool")
        .and_then(|tool| tool.get("uv"))
        .and_then(|uv| uv.get("package"))
        .is_some_and(|package| package.as_bool().is_none())
    {
        report.line(Severity::Error, "configuration: [tool.uv] package must be a boolean; correct pyproject.toml")?;
        return Ok(true);
    }
    if let Some(managed) = document
        .get("tool")
        .and_then(|tool| tool.get("uv"))
        .and_then(|uv| uv.get("managed"))
    {
        match managed.as_bool() {
            Some(false) => {
                report.line(Severity::Error, "configuration: [tool.uv] managed = false disables uv project management; remove it before using pyfaro project workflows")?;
                return Ok(true);
            }
            Some(true) => {}
            None => {
                report.line(Severity::Error, "configuration: [tool.uv] managed must be a boolean; correct pyproject.toml")?;
                return Ok(true);
            }
        }
    }
    for directory in root.ancestors() {
        let metadata = directory.join("pyproject.toml");
        let candidate = if directory == root {
            document.clone()
        } else {
            match fs::metadata(&metadata) {
                Ok(entry) if entry.is_file() => {}
                Ok(_) => {
                    report.line(Severity::Error, format!("configuration: {} must be a readable file, not a directory or special file; check workspace metadata", metadata.display()))?;
                    return Ok(true);
                }
                Err(error) if error.kind() == std::io::ErrorKind::NotFound => {
                    if metadata.is_symlink() {
                        report.line(Severity::Error, format!("configuration: cannot read {}: dangling symlink; repair workspace metadata", metadata.display()))?;
                        return Ok(true);
                    }
                    continue;
                }
                Err(error) => {
                    report.line(Severity::Error, format!("configuration: cannot inspect {}: {error}; check workspace metadata", metadata.display()))?;
                    return Ok(true);
                }
            }
            let source = match fs::read_to_string(&metadata) {
                Ok(source) => source,
                Err(error) if error.kind() == std::io::ErrorKind::NotFound => {
                    if metadata.is_symlink() {
                        report.line(Severity::Error, format!("configuration: cannot read {}: dangling symlink; repair workspace metadata", metadata.display()))?;
                        return Ok(true);
                    }
                    continue;
                }
                Err(error) => {
                    report.line(Severity::Error, format!("configuration: cannot read {}: {error}; check workspace metadata", metadata.display()))?;
                    return Ok(true);
                }
            };
            match source.parse::<DocumentMut>() {
                Ok(candidate) => candidate,
                Err(error) => {
                    report.line(Severity::Error, format!("configuration: cannot parse {}: {error}; check workspace metadata", metadata.display()))?;
                    return Ok(true);
                }
            }
        };
        let workspace = candidate
            .get("tool")
            .and_then(|tool| tool.get("uv"))
            .and_then(|uv| uv.get("workspace"));
        let Some(workspace) = workspace else {
            if directory == root {
                continue;
            }
            return Ok(false);
        };
        if workspace.as_table_like().is_none() {
            report.line(Severity::Error, format!("configuration: [tool.uv] workspace in {} must be a table; correct pyproject.toml", metadata.display()))?;
            return Ok(true);
        }
        let member = if directory == root {
            true
        } else {
            let included = match workspace_matches(
                root, directory, workspace, "members", report,
            )? {
                Some(included) => included,
                None => return Ok(true),
            };
            if !included {
                return Ok(false);
            }
            let excluded = match workspace_matches(
                root, directory, workspace, "exclude", report,
            )? {
                Some(excluded) => excluded,
                None => return Ok(true),
            };
            !excluded
        };
        if member {
            report.line(Severity::Error, format!("configuration: uv workspace at {} is not supported by pyfaro doctor; use a standalone project", metadata.display()))?;
            return Ok(true);
        }
        return Ok(false);
    }
    Ok(false)
}

fn workspace_matches(
    project: &Path,
    workspace_root: &Path,
    workspace: &toml_edit::Item,
    key: &str,
    report: &mut Report,
) -> Result<Option<bool>> {
    if project.to_str().is_none() {
        report.line(Severity::Error, format!("configuration: cannot determine uv workspace membership for {}: path is not UTF-8", project.display()))?;
        return Ok(None);
    }
    let Some(workspace_text) = workspace_root.to_str() else {
        report.line(Severity::Error, format!("configuration: cannot determine uv workspace membership for {}: path is not UTF-8", workspace_root.display()))?;
        return Ok(None);
    };
    let mut matched = false;
    if let Some(value) = workspace.get(key) {
        let Some(patterns) = value.as_array() else {
            report.line(Severity::Error, format!("configuration: invalid workspace glob in {}: [tool.uv.workspace].{key} must be an array of strings", workspace_root.join("pyproject.toml").display()))?;
            return Ok(None);
        };
        for value in patterns {
            let Some(raw) = value.as_str() else {
                report.line(Severity::Error, format!("configuration: invalid workspace glob in {}: [tool.uv.workspace].{key} must be an array of strings", workspace_root.join("pyproject.toml").display()))?;
                return Ok(None);
            };
            let mut normalized = PathBuf::new();
            for component in Path::new(raw).components() {
                match component {
                    std::path::Component::CurDir => {}
                    std::path::Component::ParentDir => {
                        if normalized
                            .file_name()
                            .is_some_and(|part| part != "..")
                        {
                            normalized.pop();
                        } else {
                            normalized.push("..");
                        }
                    }
                    other => normalized.push(other.as_os_str()),
                }
            }
            let absolute =
                PathBuf::from(Pattern::escape(workspace_text)).join(normalized);
            let Some(absolute_glob) = absolute.to_str() else {
                report.line(Severity::Error, format!("configuration: cannot determine uv workspace membership for {}: glob path is not UTF-8", absolute.display()))?;
                return Ok(None);
            };
            let pattern = match Pattern::new(absolute_glob) {
                Ok(pattern) => pattern,
                Err(error) => {
                    report.line(Severity::Error, format!("configuration: invalid workspace glob {raw:?} in {}: {error}; correct [tool.uv.workspace].{key}", workspace_root.join("pyproject.toml").display()))?;
                    return Ok(None);
                }
            };
            matched |= pattern.matches_path(project);
        }
    }
    Ok(Some(matched))
}

pub(super) fn inspect_layout(root: &Path, report: &mut Report) -> Result<()> {
    for directory in ["src", "tests"] {
        let path = root.join(directory);
        if fs::symlink_metadata(&path).is_ok() && !path.is_dir() {
            report.line(
                Severity::Error,
                format!("layout: {} must be a directory; move or remove the conflicting file", path.display()),
            )?;
        }
    }
    let venv = tools::project_environment(root);
    if fs::symlink_metadata(&venv).is_ok() && !venv.is_dir() {
        report.line(Severity::Error, format!("layout: {} must be a directory; move or remove the conflicting file", venv.display()))?;
    }
    let interpreter = tools::project_python(root);
    if venv.is_dir() && !interpreter.is_file() {
        report.line(
            Severity::Error,
            format!(
                "layout: {} has no Python interpreter; repair it with pyfaro sync",
                venv.display()
            ),
        )?;
    }
    #[cfg(unix)]
    if interpreter.is_file() {
        use std::os::unix::fs::PermissionsExt;

        use crate::error::Error;

        let permissions = fs::metadata(&interpreter)
            .map_err(|error| Error::io("inspect", &interpreter, error))?
            .permissions();
        if permissions.mode() & 0o111 == 0 {
            report.line(
                Severity::Error,
                format!(
                    "layout: {} is not executable; repair it with pyfaro sync",
                    interpreter.display()
                ),
            )?;
        }
    }
    Ok(())
}
