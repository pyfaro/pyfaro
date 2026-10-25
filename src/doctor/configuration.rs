use std::env;
use std::fs;
use std::path::Path;

use pep440_rs::Version;
use toml_edit::DocumentMut;

use crate::error::Result;

use super::dependencies::valid_group_name;
use super::uv_config::uv_env_enabled;
use super::{Report, Severity};

pub(super) fn uses_pytest_9_configuration(
    root: &Path,
    document: &DocumentMut,
) -> bool {
    root.join("pytest.toml").exists()
        || root.join(".pytest.toml").exists()
        || document
            .get("tool")
            .and_then(|tool| tool.get("pytest"))
            .and_then(|pytest| pytest.as_table_like())
            .is_some_and(|pytest| {
                pytest.iter().any(|(key, _)| key != "ini_options")
            })
}

pub(super) fn inspect_configuration(
    root: &Path,
    document: &DocumentMut,
    report: &mut Report,
) -> Result<()> {
    for (section, files) in [
        ("ruff", &["ruff.toml", ".ruff.toml"][..]),
        ("ty", &["ty.toml"][..]),
        (
            "pytest",
            &[
                "pytest.toml",
                ".pytest.toml",
                "pytest.ini",
                ".pytest.ini",
                "tox.ini",
                "setup.cfg",
            ][..],
        ),
    ] {
        let configured = document
            .get("tool")
            .and_then(|tool| tool.get(section))
            .is_some_and(|config| {
                section != "pytest"
                    || config.get("ini_options").is_some()
                    || config.as_table_like().is_some_and(|table| {
                        table.iter().any(|(name, _)| name != "ini_options")
                    })
            });
        let mut present = Vec::new();
        for file in files {
            let path = root.join(file);
            let contains_configuration = match *file {
                "tox.ini" | "setup.cfg" => {
                    let section = if *file == "tox.ini" {
                        "[pytest]"
                    } else {
                        "[tool:pytest]"
                    };
                    let contents = match fs::metadata(&path) {
                        Ok(metadata) if metadata.is_file() => {
                            fs::read_to_string(&path)
                        }
                        Ok(_) => {
                            report.line(Severity::Error, format!("configuration: {} must be a readable file, not a directory or special file", path.display()))?;
                            continue;
                        }
                        Err(error) => Err(error),
                    };
                    match contents {
                        Ok(contents) => contents.lines().any(|line| {
                            line.trim()
                                .trim_start_matches('\u{feff}')
                                .strip_prefix(section)
                                .is_some_and(|rest| {
                                    let rest = rest.trim();
                                    rest.is_empty()
                                        || rest.starts_with('#')
                                        || rest.starts_with(';')
                                })
                        }),
                        Err(error)
                            if error.kind() == std::io::ErrorKind::NotFound =>
                        {
                            missing_tool_configuration(&path, report)?
                        }
                        Err(error) => {
                            report.line(Severity::Error, format!("configuration: cannot read {}: {error}; check the file and its permissions", path.display()))?;
                            false
                        }
                    }
                }
                _ => match fs::metadata(&path) {
                    Ok(metadata) if !metadata.is_file() => {
                        report.line(Severity::Error, format!("configuration: {} must be a readable file, not a directory or special file", path.display()))?;
                        false
                    }
                    Ok(_) => match fs::File::open(&path) {
                        Ok(_) => true,
                        Err(error) => {
                            report.line(Severity::Error, format!("configuration: cannot read {}: {error}; check the file and its permissions", path.display()))?;
                            false
                        }
                    },
                    Err(error)
                        if error.kind() == std::io::ErrorKind::NotFound =>
                    {
                        missing_tool_configuration(&path, report)?
                    }
                    Err(error) => {
                        report.line(Severity::Error, format!("configuration: cannot inspect {}: {error}; check the file and its permissions", path.display()))?;
                        false
                    }
                },
            };
            if contains_configuration {
                present.push(*file);
            }
        }
        if configured && !present.is_empty() {
            report.line(
                Severity::Error,
                format!(
                    "configuration: [tool.{section}] and {} both configure {section}; keep one source of configuration",
                    present.join(", ")
                ),
            )?;
        } else if present.len() > 1 {
            report.line(
                Severity::Error,
                format!(
                    "configuration: {} both configure {section}; keep one source of configuration",
                    present.join(" and ")
                ),
            )?;
        }
    }
    if let Some(pytest) = document
        .get("tool")
        .and_then(|tool| tool.get("pytest"))
        .and_then(|pytest| pytest.as_table_like())
    {
        let native = pytest.iter().any(|(name, _)| name != "ini_options");
        let ini_options = pytest
            .get("ini_options")
            .and_then(|options| options.as_table_like())
            .is_some_and(|options| options.iter().next().is_some());
        if native && ini_options {
            report.line(Severity::Error, "configuration: [tool.pytest] and [tool.pytest.ini_options] both configure pytest; keep one format")?;
        }
    }
    if document.get("tool").and_then(|tool| tool.get("uv")).is_some()
        && project_uv_toml_selected(root)
    {
        report.line(
            Severity::Warn,
            "configuration: uv.toml takes precedence over [tool.uv] configuration settings; project metadata remains active",
        )?;
    }
    Ok(())
}

fn project_uv_toml_selected(root: &Path) -> bool {
    env::var_os("UV_CONFIG_FILE").is_none()
        && !uv_env_enabled("UV_NO_CONFIG")
        && fs::symlink_metadata(root.join("uv.toml")).is_ok()
}

fn missing_tool_configuration(
    path: &Path,
    report: &mut Report,
) -> Result<bool> {
    if path.is_symlink() {
        report.line(Severity::Error, format!("configuration: cannot read {}: dangling symlink; repair or remove it", path.display()))?;
    }
    Ok(false)
}

pub(super) fn inspect_project_metadata(
    document: &DocumentMut,
    report: &mut Report,
) -> Result<()> {
    let Some(project) = document.get("project") else {
        return Ok(());
    };
    if project
        .get("name")
        .and_then(|name| name.as_str())
        .is_none_or(|name| name.trim().is_empty())
    {
        report.line(Severity::Error, "configuration: [project].name must be a nonempty string; correct pyproject.toml")?;
    } else if project
        .get("name")
        .and_then(|name| name.as_str())
        .is_some_and(|name| !valid_group_name(name))
    {
        report.line(Severity::Error, "configuration: [project].name must be a valid package name; correct pyproject.toml")?;
    }
    let dynamic_version = match project.get("dynamic") {
        Some(dynamic) => match dynamic.as_array() {
            Some(entries)
                if entries.iter().all(|entry| entry.as_str().is_some()) =>
            {
                entries.iter().any(|entry| entry.as_str() == Some("version"))
            }
            _ => {
                report.line(Severity::Error, "configuration: [project].dynamic must be an array of strings; correct pyproject.toml")?;
                false
            }
        },
        None => false,
    };
    match project.get("version") {
        Some(version)
            if version
                .as_str()
                .is_none_or(|value| value.parse::<Version>().is_err()) =>
        {
            report.line(Severity::Error, "configuration: [project].version must be a valid version string; correct pyproject.toml")?;
        }
        Some(_) if dynamic_version => {
            report.line(Severity::Error, "configuration: [project].version cannot also be dynamic; correct pyproject.toml")?;
        }
        None if !dynamic_version => {
            report.line(Severity::Error, "configuration: [project].version is required unless listed in dynamic; correct pyproject.toml")?;
        }
        _ => {}
    }
    Ok(())
}
