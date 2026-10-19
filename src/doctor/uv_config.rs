use std::env;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::sync::atomic::AtomicBool;

use toml_edit::{DocumentMut, Item, Table};

use crate::error::{Error, Result};
use crate::process;

use super::{Report, Severity, uv_env_enabled};

pub(super) struct UvConfiguration {
    pub(super) document: DocumentMut,
    pub(super) path: PathBuf,
    pub(super) standalone: bool,
}

enum UvDiscovery {
    Config(Box<UvConfiguration>),
    Missing,
    Invalid,
}

impl UvConfiguration {
    pub(super) fn setting(&self, name: &str) -> Option<&toml_edit::Item> {
        if self.standalone {
            self.document.get(name)
        } else {
            self.document
                .get("tool")
                .and_then(|tool| tool.get("uv"))
                .and_then(|uv| uv.get(name))
        }
    }

    pub(super) fn label(&self, name: &str) -> String {
        if self.standalone {
            format!("{} {name}", self.path.display())
        } else {
            format!("[tool.uv].{name} in {}", self.path.display())
        }
    }
}

pub(super) fn uv_configuration(
    root: &Path,
    project_document: &DocumentMut,
    uv_binary: Option<&Path>,
    cancelled: &AtomicBool,
    report: &mut Report,
) -> Result<Option<Vec<UvConfiguration>>> {
    if let Some(path) = env::var_os("UV_CONFIG_FILE") {
        let path = root.join(path);
        return match read_uv_configuration_file(
            &path, uv_binary, cancelled, report, true,
        )? {
            UvDiscovery::Config(config) => {
                if let Some(binary) = uv_binary
                    && !probe_effective_uv_configuration(
                        binary, root, cancelled, report,
                    )?
                {
                    return Ok(None);
                }
                Ok(Some(vec![*config]))
            }
            UvDiscovery::Missing | UvDiscovery::Invalid => Ok(None),
        };
    }
    if uv_env_enabled("UV_NO_CONFIG") {
        return Ok(Some(Vec::new()));
    }
    let mut configurations = Vec::new();
    for directory in root.ancestors() {
        let uv_path = directory.join("uv.toml");
        match read_uv_configuration_file(
            &uv_path, uv_binary, cancelled, report, false,
        )? {
            UvDiscovery::Missing => {}
            UvDiscovery::Config(config) => {
                configurations.push(*config);
                break;
            }
            UvDiscovery::Invalid => return Ok(None),
        }
        let metadata = directory.join("pyproject.toml");
        let document = if directory == root {
            project_document.clone()
        } else {
            match fs::metadata(&metadata) {
                Ok(entry) if entry.is_file() => {}
                Ok(_) => {
                    report.line(Severity::Error, format!("dependencies: {} must be a readable file, not a directory or special file; repair the uv configuration", metadata.display()))?;
                    return Ok(None);
                }
                Err(error) if error.kind() == std::io::ErrorKind::NotFound => {
                    if metadata.is_symlink() {
                        report.line(Severity::Error, format!("dependencies: cannot read {}: dangling symlink; repair the uv configuration", metadata.display()))?;
                        return Ok(None);
                    }
                    continue;
                }
                Err(error) => {
                    report.line(Severity::Error, format!("dependencies: cannot inspect {}: {error}; repair the uv configuration", metadata.display()))?;
                    return Ok(None);
                }
            }
            let contents = match fs::read_to_string(&metadata) {
                Ok(contents) => contents,
                Err(error) if error.kind() == std::io::ErrorKind::NotFound => {
                    if metadata.is_symlink() {
                        report.line(Severity::Error, format!("dependencies: cannot read {}: dangling symlink; repair the uv configuration", metadata.display()))?;
                        return Ok(None);
                    }
                    continue;
                }
                Err(error) => {
                    report.line(Severity::Error, format!("dependencies: cannot read {}: {error}; repair the uv configuration", metadata.display()))?;
                    return Ok(None);
                }
            };
            match contents.parse::<DocumentMut>() {
                Ok(document) => document,
                Err(error) => {
                    report.line(
                        Severity::Error,
                        format!(
                            "dependencies: invalid {}: {error}; correct {}",
                            metadata.display(),
                            metadata.display()
                        ),
                    )?;
                    return Ok(None);
                }
            }
        };
        if document
            .get("tool")
            .and_then(|tool| tool.get("uv"))
            .is_some_and(|uv| uv.as_table_like().is_none())
        {
            report.line(
                Severity::Error,
                format!(
                    "dependencies: [tool.uv] in {} must be a table",
                    metadata.display()
                ),
            )?;
            return Ok(None);
        }
        if document.get("tool").and_then(|tool| tool.get("uv")).is_some() {
            configurations.push(UvConfiguration {
                document,
                path: metadata,
                standalone: false,
            });
            break;
        }
    }
    if let Some(path) = user_uv_configuration_path() {
        match read_uv_configuration_file(
            &path, uv_binary, cancelled, report, false,
        )? {
            UvDiscovery::Config(config) => configurations.push(*config),
            UvDiscovery::Invalid => return Ok(None),
            UvDiscovery::Missing => {}
        }
    }
    for path in system_uv_configuration_paths() {
        match read_uv_configuration_file(
            &path, uv_binary, cancelled, report, false,
        )? {
            UvDiscovery::Config(config) => {
                configurations.push(*config);
                break;
            }
            UvDiscovery::Invalid => return Ok(None),
            UvDiscovery::Missing => {}
        }
    }
    if let Some(binary) = uv_binary
        && let Some(project_config) =
            configurations.iter().find(|config| !config.standalone)
        && !probe_uv_configuration(
            binary,
            &project_config.path,
            project_config.document.get("tool").and_then(|tool| tool.get("uv")),
            None,
            cancelled,
            report,
        )?
    {
        return Ok(None);
    }
    if let Some(binary) = uv_binary
        && !probe_effective_uv_configuration(binary, root, cancelled, report)?
    {
        return Ok(None);
    }
    Ok(Some(configurations))
}

#[cfg(not(windows))]
fn user_uv_configuration_path() -> Option<PathBuf> {
    env::var_os("XDG_CONFIG_HOME")
        .map(PathBuf::from)
        .or_else(|| {
            env::var_os("HOME").map(|home| PathBuf::from(home).join(".config"))
        })
        .map(|directory| directory.join("uv/uv.toml"))
}

#[cfg(windows)]
fn user_uv_configuration_path() -> Option<PathBuf> {
    env::var_os("APPDATA")
        .map(|directory| PathBuf::from(directory).join("uv/uv.toml"))
}

#[cfg(not(windows))]
fn system_uv_configuration_paths() -> Vec<PathBuf> {
    let directories = env::var_os("XDG_CONFIG_DIRS")
        .map(|paths| env::split_paths(&paths).collect::<Vec<_>>())
        .filter(|paths| !paths.is_empty())
        .unwrap_or_else(|| vec![PathBuf::from("/etc/xdg")]);
    directories
        .into_iter()
        .map(|directory| directory.join("uv/uv.toml"))
        .chain([PathBuf::from("/etc/uv/uv.toml")])
        .collect()
}

#[cfg(windows)]
fn system_uv_configuration_paths() -> Vec<PathBuf> {
    env::var_os("PROGRAMDATA")
        .map(|directory| vec![PathBuf::from(directory).join("uv/uv.toml")])
        .unwrap_or_default()
}

fn read_uv_configuration_file(
    path: &Path,
    uv_binary: Option<&Path>,
    cancelled: &AtomicBool,
    report: &mut Report,
    explicit: bool,
) -> Result<UvDiscovery> {
    let contents = match fs::metadata(path) {
        Ok(metadata) if metadata.is_file() => fs::read_to_string(path),
        Ok(_) => {
            let action = if explicit {
                "check UV_CONFIG_FILE and select a regular file"
            } else {
                "replace it with a regular file"
            };
            report.line(Severity::Error, format!("dependencies: cannot read {}: must be a readable file, not a directory or special file; {action}", path.display()))?;
            return Ok(UvDiscovery::Invalid);
        }
        Err(error) => Err(error),
    };
    let contents = match contents {
        Ok(contents) => contents,
        Err(error)
            if !explicit
                && error.kind() == std::io::ErrorKind::NotFound
                && !path.is_symlink() =>
        {
            return Ok(UvDiscovery::Missing);
        }
        Err(error) => {
            let action = if explicit {
                "check UV_CONFIG_FILE and its file permissions"
            } else {
                "check the file and its permissions"
            };
            report.line(
                Severity::Error,
                format!(
                    "dependencies: cannot read {}: {error}; {action}",
                    path.display()
                ),
            )?;
            return Ok(UvDiscovery::Invalid);
        }
    };
    let document = match contents.parse::<DocumentMut>() {
        Ok(document) => document,
        Err(error) => {
            report.line(
                Severity::Error,
                format!(
                    "dependencies: invalid {}: {error}; correct {}",
                    path.display(),
                    path.display()
                ),
            )?;
            return Ok(UvDiscovery::Invalid);
        }
    };
    let mut invalid = false;
    for field in [
        "conflicts",
        "workspace",
        "sources",
        "dev-dependencies",
        "default-groups",
        "dependency-groups",
        "managed",
        "package",
        "build-backend",
        "environments",
        "required-environments",
    ] {
        if document.get(field).is_some() {
            let destination = if field == "dependency-groups" {
                "[tool.uv.dependency-groups] (or [dependency-groups] for group declarations)"
            } else {
                "[tool.uv]"
            };
            let explicit_action = if explicit {
                "; unset UV_CONFIG_FILE to use project settings"
            } else {
                ""
            };
            report.line(
                Severity::Error,
                format!(
                    "dependencies: {field} is only allowed in {destination} in pyproject.toml, not {}; move it there and remove it from this file{explicit_action}",
                    path.display()
                ),
            )?;
            invalid = true;
        }
    }
    if invalid {
        return Ok(UvDiscovery::Invalid);
    }
    if let Some(binary) = uv_binary
        && !probe_uv_configuration(
            binary,
            path,
            None,
            Some(&document),
            cancelled,
            report,
        )?
    {
        return Ok(UvDiscovery::Invalid);
    }
    Ok(UvDiscovery::Config(Box::new(UvConfiguration {
        document,
        path: path.to_path_buf(),
        standalone: true,
    })))
}

fn probe_uv_configuration(
    binary: &Path,
    path: &Path,
    project_uv: Option<&Item>,
    standalone: Option<&DocumentMut>,
    cancelled: &AtomicBool,
    report: &mut Report,
) -> Result<bool> {
    let stderr = tempfile::NamedTempFile::new().map_err(|error| {
        Error::io("create uv configuration output", path, error)
    })?;
    let isolated_project = if let Some(project_uv) = project_uv {
        let directory = tempfile::tempdir().map_err(|error| {
            Error::io("create uv configuration probe", path, error)
        })?;
        let mut document = DocumentMut::new();
        let mut tool = Table::new();
        tool.insert("uv", project_uv.clone());
        document.as_table_mut().insert("tool", Item::Table(tool));
        let metadata = directory.path().join("pyproject.toml");
        fs::write(&metadata, document.to_string()).map_err(|error| {
            Error::io("write uv configuration probe", &metadata, error)
        })?;
        Some(directory)
    } else {
        None
    };
    let isolated_standalone = if let Some(standalone) = standalone
        && standalone.get("required-version").is_some()
    {
        let directory = tempfile::tempdir().map_err(|error| {
            Error::io("create uv configuration probe", path, error)
        })?;
        let mut document = standalone.clone();
        document.as_table_mut().remove("required-version");
        let config = directory.path().join("uv.toml");
        fs::write(&config, document.to_string()).map_err(|error| {
            Error::io("write uv configuration probe", &config, error)
        })?;
        Some((directory, config))
    } else {
        None
    };
    let mut command = Command::new(binary);
    command.arg("--no-cache");
    if let Some(directory) = isolated_project.as_ref() {
        command.arg("--directory").arg(directory.path());
        command
            .env("XDG_CONFIG_HOME", directory.path())
            .env("XDG_CONFIG_DIRS", directory.path())
            .env("HOME", directory.path())
            .env("APPDATA", directory.path())
            .env("PROGRAMDATA", directory.path());
    } else {
        command.arg("--config-file").arg(
            isolated_standalone
                .as_ref()
                .map_or(path, |(_, config)| config.as_path()),
        );
    }
    command
        .arg("--offline")
        .arg("python")
        .arg("list")
        .arg("--only-installed")
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .env_remove("UV_WORKING_DIR")
        .env_remove("UV_PROJECT")
        .env_remove("UV_CONFIG_FILE")
        .env_remove("UV_NO_CONFIG")
        .stderr(
            stderr
                .reopen()
                .map_err(|error| Error::io("open", stderr.path(), error))?,
        );
    let status = process::run(command, cancelled)?;
    let diagnostic = process::read_diagnostic(stderr.path())?;
    if !status.success()
        || (isolated_project.is_some()
            && diagnostic.contains(
                "Failed to parse `pyproject.toml` during settings discovery",
            ))
    {
        report.line(Severity::Error, format!("dependencies: uv rejected {}: {diagnostic}; correct the uv configuration", path.display()))?;
        return Ok(false);
    }
    Ok(true)
}

fn probe_effective_uv_configuration(
    binary: &Path,
    root: &Path,
    cancelled: &AtomicBool,
    report: &mut Report,
) -> Result<bool> {
    let stderr = tempfile::NamedTempFile::new().map_err(|error| {
        Error::io("create uv configuration output", root, error)
    })?;
    let mut command = Command::new(binary);
    command
        .arg("--no-cache")
        .arg("--directory")
        .arg(root)
        .arg("--offline")
        .arg("python")
        .arg("list")
        .arg("--only-installed")
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .env_remove("UV_WORKING_DIR")
        .env_remove("UV_PROJECT")
        .stderr(
            stderr
                .reopen()
                .map_err(|error| Error::io("open", stderr.path(), error))?,
        );
    if process::run(command, cancelled)?.success() {
        return Ok(true);
    }
    let diagnostic = process::read_diagnostic(stderr.path())?;
    report.line(Severity::Error, format!("dependencies: uv rejected effective configuration for {}: {diagnostic}; correct the uv configuration", root.display()))?;
    Ok(false)
}
