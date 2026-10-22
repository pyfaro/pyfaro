use std::collections::HashMap;
use std::env;
use std::ffi::OsString;
use std::fs;
use std::io::Write;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::sync::atomic::AtomicBool;

use glob::Pattern;
use pep440_rs::{Version, VersionSpecifiers};
use toml_edit::{DocumentMut, Table};

use crate::cli::{DoctorArgs, Tool};
use crate::config::Config;
use crate::error::{Error, Result};
use crate::{process, project, tools};
mod uv_config;
use uv_config::{UvConfiguration, uv_configuration};

#[derive(Clone, Copy)]
enum Severity {
    Ok,
    Warn,
    Error,
}

impl Severity {
    fn label(self) -> &'static str {
        match self {
            Self::Ok => "OK",
            Self::Warn => "WARN",
            Self::Error => "ERROR",
        }
    }
}

struct Report {
    blocking: usize,
}

impl Report {
    fn line(
        &mut self,
        level: Severity,
        message: impl AsRef<str>,
    ) -> Result<()> {
        if matches!(level, Severity::Error) {
            self.blocking += 1;
        }
        writeln!(
            std::io::stdout().lock(),
            "{}: {}",
            level.label(),
            tools::single_line(message.as_ref())
        )
        .map_err(Error::Output)
    }
}

pub(crate) fn run(args: DoctorArgs) -> Result<()> {
    let cancelled = process::cancellation()?;
    let mut report = Report { blocking: 0 };
    let root = match project::select(args.project.as_deref()) {
        Ok(root) => {
            report
                .line(Severity::Ok, format!("project: {}", root.display()))?;
            Some(root)
        }
        Err(error) => {
            let hint = if matches!(error, Error::NoProject { .. }) {
                ""
            } else {
                "; use --project <PATH> to select a project"
            };
            report.line(Severity::Error, format!("project: {error}{hint}"))?;
            None
        }
    };
    let uv_probe =
        tools::inspect(Tool::Uv, root.as_deref(), args.uv.as_ref(), &cancelled);
    if matches!(uv_probe, Err(Error::Cancelled)) {
        return Err(Error::Cancelled);
    }
    let uv_binary = uv_probe.as_ref().ok().map(|(path, _)| path.as_path());

    let mut dependencies = None;
    let mut requires_python = None;
    let mut pytest_9_only = false;
    if let Some(root) = root.as_deref() {
        match Config::load_document(root) {
            Ok(document) => {
                report.line(
                    Severity::Ok,
                    "pyproject.toml: valid Pyfaro configuration",
                )?;
                inspect_configuration(root, &document, &mut report)?;
                inspect_project_metadata(&document, &mut report)?;
                pytest_9_only = uses_pytest_9_configuration(root, &document);
                if !inspect_workspace(root, &document, &mut report)? {
                    let declared = declared_dependencies(
                        root,
                        &document,
                        uv_binary,
                        &cancelled,
                        &mut report,
                    )?;
                    let valid_metadata = match uv_binary {
                        Some(binary) => uv_config::validate_project_metadata(
                            root,
                            binary,
                            &cancelled,
                            &mut report,
                        )?,
                        None => true,
                    };
                    if valid_metadata {
                        dependencies = declared;
                    }
                }
                requires_python = document
                    .get("project")
                    .and_then(|project| project.get("requires-python"))
                    .map(|value| value.as_str().map(str::to_owned));
            }
            Err(error) => report.line(
                Severity::Error,
                format!("configuration: {error}; correct pyproject.toml"),
            )?,
        }
        inspect_layout(root, &mut report)?;
    }

    let python = match tools::inspect_python(root.as_deref(), &cancelled) {
        Ok((path, version)) => {
            report.line(
                Severity::Ok,
                format!("Python: {} ({version})", path.display()),
            )?;
            if let Some(requirement) = requires_python.as_ref() {
                inspect_python_requirement(
                    requirement.as_deref(),
                    &version,
                    &mut report,
                )?;
            }
            if let Some(root) = root.as_deref() {
                inspect_python_pin(root, Some(&version), &mut report)?;
            }
            Some(path)
        }
        Err(Error::Cancelled) => return Err(Error::Cancelled),
        Err(error) => {
            report.line(Severity::Error, format!("Python: {error}"))?;
            if let Some(root) = root.as_deref() {
                inspect_python_pin(root, None, &mut report)?;
            }
            None
        }
    };
    match uv_probe {
        Ok((path, version)) => report.line(
            Severity::Ok,
            format!("uv: {} ({version})", path.display()),
        )?,
        Err(error) => report
            .line(Severity::Error, format!("uv: {}", failure_detail(error)))?,
    }
    for tool in [Tool::Ruff, Tool::Ty, Tool::Pytest] {
        let version = inspect_tool(
            tool,
            root.as_deref(),
            args.uv.as_ref(),
            &cancelled,
            &mut report,
            false,
        )?;
        if let Some(version) =
            version.filter(|_| matches!(tool, Tool::Pytest) && pytest_9_only)
        {
            let number = version.split_whitespace().nth(1).unwrap_or_default();
            let old = number
                .parse::<Version>()
                .ok()
                .and_then(|found| found.release().first().copied())
                .is_some_and(|major| major < 9);
            if old {
                report.line(Severity::Error, format!("configuration: pytest {number} cannot use pytest 9 configuration; upgrade to pytest >=9 or use INI configuration"))?;
            }
        }
    }
    if let (Some(root), Some(dependencies)) =
        (root.as_deref(), dependencies.as_ref())
    {
        inspect_dependencies(
            root,
            python.as_deref(),
            dependencies,
            &cancelled,
            &mut report,
        )?;
    }
    if report.blocking > 0 {
        Err(Error::DoctorFailed { count: report.blocking })
    } else {
        report.line(Severity::Ok, "doctor: no blocking issues")
    }
}

/// The tool name is already printed as the line prefix, so drop it from the
/// error text.
fn failure_detail(error: Error) -> String {
    match error {
        Error::Tool { reason, .. } => reason,
        other => other.to_string(),
    }
}

fn inspect_tool(
    tool: Tool,
    root: Option<&Path>,
    uv: Option<&OsString>,
    cancelled: &AtomicBool,
    report: &mut Report,
    required: bool,
) -> Result<Option<String>> {
    let name = tools::name(tool);
    match tools::inspect(tool, root, uv, cancelled) {
        Ok((path, version)) => {
            report.line(
                Severity::Ok,
                format!("{name}: {} ({version})", path.display()),
            )?;
            Ok(Some(version))
        }
        Err(Error::Cancelled) => Err(Error::Cancelled),
        Err(error) => {
            let detail = failure_detail(error);
            report.line(
                if required { Severity::Error } else { Severity::Warn },
                format!("{name}: {detail}"),
            )?;
            Ok(None)
        }
    }
}

fn uses_pytest_9_configuration(root: &Path, document: &DocumentMut) -> bool {
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

fn inspect_configuration(
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

fn inspect_project_metadata(
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

fn inspect_workspace(
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

fn inspect_layout(root: &Path, report: &mut Report) -> Result<()> {
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

fn inspect_python_requirement(
    requirement: Option<&str>,
    reported_version: &str,
    report: &mut Report,
) -> Result<()> {
    let Some(requirement) = requirement else {
        return report.line(
            Severity::Error,
            "configuration: [project].requires-python must be a version specifier string",
        );
    };
    let Ok(specifiers) = requirement.parse::<VersionSpecifiers>() else {
        return report.line(
            Severity::Error,
            format!("configuration: invalid requires-python {requirement:?}; correct pyproject.toml"),
        );
    };
    let version = reported_version
        .strip_prefix("Python ")
        .and_then(|rest| rest.split_whitespace().next())
        .and_then(|number| number.parse::<Version>().ok());
    if let Some(version) = version
        && !specifiers.contains(&version)
    {
        return report.line(
            Severity::Error,
            format!("Python: {reported_version} does not satisfy requires-python {requirement:?}; select a compatible interpreter"),
        );
    }
    Ok(())
}

fn inspect_python_pin(
    root: &Path,
    reported_version: Option<&str>,
    report: &mut Report,
) -> Result<()> {
    let path = root.join(".python-version");
    let contents = match fs::metadata(&path) {
        Ok(metadata) if metadata.is_file() => fs::read_to_string(&path),
        Ok(_) => {
            return report.line(Severity::Error, format!("Python: {} must be a readable file, not a directory or special file; repair the version pin", path.display()));
        }
        Err(error) => Err(error),
    };
    let request =
        match contents {
            Ok(request) => request,
            Err(error)
                if error.kind() == std::io::ErrorKind::NotFound
                    && !path.is_symlink() =>
            {
                return Ok(());
            }
            Err(error) => return report.line(
                Severity::Error,
                format!(
                    "Python: cannot read {}: {error}; repair the version pin",
                    path.display()
                ),
            ),
        };
    let request = request.trim();
    let numeric = request.strip_prefix("cpython@").unwrap_or(request);
    let components: Option<Vec<u64>> = numeric
        .split('.')
        .map(|part| {
            if part.is_empty()
                || !part.bytes().all(|byte| byte.is_ascii_digit())
            {
                None
            } else {
                part.parse().ok()
            }
        })
        .collect();
    let Some(components) =
        components.filter(|parts| (2..=3).contains(&parts.len()))
    else {
        return report.line(Severity::Warn, format!("Python: cannot compare .python-version request {request:?} with the selected interpreter; verify the pin"));
    };
    let Some(reported_version) = reported_version else {
        return Ok(());
    };
    let selected = reported_version
        .strip_prefix("Python ")
        .and_then(|rest| rest.split_whitespace().next())
        .and_then(|number| number.parse::<Version>().ok());
    if selected
        .is_some_and(|version| !version.release().starts_with(&components))
    {
        report.line(Severity::Error, format!("Python: {reported_version} does not match .python-version {request:?}; select a matching interpreter or update the pin"))?;
    }
    Ok(())
}

fn normalize_group_name(name: &str) -> String {
    let mut normalized = String::with_capacity(name.len());
    let mut separator = false;
    for character in name.chars() {
        if matches!(character, '-' | '_' | '.') {
            if !separator {
                normalized.push('-');
            }
            separator = true;
        } else {
            normalized.push(character.to_ascii_lowercase());
            separator = false;
        }
    }
    normalized
}

fn valid_group_name(name: &str) -> bool {
    let bytes = name.as_bytes();
    bytes.first().is_some_and(u8::is_ascii_alphanumeric)
        && bytes.last().is_some_and(u8::is_ascii_alphanumeric)
        && bytes.iter().all(|byte| {
            byte.is_ascii_alphanumeric() || matches!(byte, b'-' | b'_' | b'.')
        })
}

struct GroupCollector<'a> {
    groups: &'a Table,
    names: &'a HashMap<String, String>,
    source: &'a Path,
    checked: Vec<String>,
    selected: Vec<String>,
    active: Vec<String>,
}

impl GroupCollector<'_> {
    fn collect(
        &mut self,
        name: &str,
        requirements: &mut Vec<Requirement>,
        unselected_requirements: &mut Vec<Requirement>,
        selected_group: bool,
        report: &mut Report,
    ) -> Result<bool> {
        if self.active.iter().any(|entry| entry == name) {
            report.line(
                Severity::Error,
                format!(
                    "dependencies: dependency group cycle includes {name:?}"
                ),
            )?;
            return Ok(false);
        }
        if self.checked.iter().any(|entry| entry == name)
            && (!selected_group
                || self.selected.iter().any(|entry| entry == name))
        {
            return Ok(true);
        }
        let Some(original) = self.names.get(name).cloned() else {
            report.line(
                Severity::Error,
                format!("dependencies: group {name:?} is missing; define it in [dependency-groups]"),
            )?;
            return Ok(false);
        };
        let groups = self.groups;
        let Some(group) =
            groups.get(&original).and_then(|group| group.as_array())
        else {
            report.line(
                Severity::Error,
                format!("dependencies: group {original:?} must be an array"),
            )?;
            return Ok(false);
        };
        self.active.push(name.to_owned());
        let mut valid = true;
        for entry in group.iter() {
            if let Some(requirement) = entry.as_str() {
                let target = if selected_group {
                    &mut *requirements
                } else {
                    &mut *unselected_requirements
                };
                target.push(Requirement {
                    text: requirement.to_owned(),
                    source: self.source.to_owned(),
                });
            } else {
                let included = entry
                    .as_inline_table()
                    .filter(|table| table.len() == 1)
                    .and_then(|table| table.get("include-group"))
                    .and_then(|value| value.as_str());
                if let Some(included) = included {
                    let included = normalize_group_name(included);
                    valid &= self.collect(
                        &included,
                        requirements,
                        unselected_requirements,
                        selected_group,
                        report,
                    )?;
                } else {
                    report.line(
                        Severity::Error,
                        format!("dependencies: group {original:?} contains an invalid entry; use a requirement string or an include-group table with no extra keys"),
                    )?;
                    valid = false;
                }
            }
        }
        self.active.pop();
        self.checked.push(name.to_owned());
        if selected_group {
            self.selected.push(name.to_owned());
        }
        Ok(valid)
    }
}

fn uv_env_enabled(name: &str) -> bool {
    env::var(name).is_ok_and(|value| {
        matches!(
            value.trim().to_ascii_lowercase().as_str(),
            "1" | "true" | "t" | "yes" | "y" | "on"
        )
    })
}

struct DeclaredDependencies {
    requirements: Vec<Requirement>,
    selected_count: usize,
}

struct Requirement {
    text: String,
    source: std::path::PathBuf,
}

fn declared_dependencies(
    root: &Path,
    document: &DocumentMut,
    uv_binary: Option<&Path>,
    cancelled: &AtomicBool,
    report: &mut Report,
) -> Result<Option<DeclaredDependencies>> {
    let mut requirements = Vec::new();
    let mut invalid = false;
    let metadata = root.join("pyproject.toml");
    if let Some(dependencies) =
        document.get("project").and_then(|project| project.get("dependencies"))
    {
        if let Some(dependencies) = dependencies.as_array() {
            for dependency in dependencies.iter() {
                if let Some(requirement) = dependency.as_str() {
                    requirements.push(Requirement {
                        text: requirement.to_owned(),
                        source: metadata.clone(),
                    });
                } else {
                    report.line(Severity::Error, "dependencies: each [project].dependencies entry must be a string")?;
                    invalid = true;
                }
            }
        } else {
            report.line(Severity::Error, "dependencies: [project].dependencies must be an array of requirement strings")?;
            invalid = true;
        }
    }
    if document
        .get("project")
        .and_then(|project| project.get("dynamic"))
        .and_then(|dynamic| dynamic.as_array())
        .is_some_and(|dynamic| {
            dynamic.iter().any(|entry| entry.as_str() == Some("dependencies"))
        })
    {
        report.line(Severity::Error, "dependencies: dynamic project dependencies cannot be checked from pyproject.toml; inspect the build backend output and sync the environment")?;
        invalid = true;
    }
    if uv_configuration(root, document, uv_binary, cancelled, report)?.is_none()
    {
        return Ok(None);
    }
    let project_settings = document
        .get("tool")
        .and_then(|tool| tool.get("uv"))
        .map(|_| UvConfiguration {
            document: document.clone(),
            path: metadata.clone(),
            standalone: false,
        });
    let default_setting = project_settings.as_ref().and_then(|config| {
        config.setting("default-groups").map(|value| (config, value))
    });
    let explicit_defaults = default_setting.is_some();
    let has_legacy_dev = project_settings
        .as_ref()
        .is_some_and(|config| config.setting("dev-dependencies").is_some());
    let selected_groups = match default_setting {
        None => Some(vec!["dev".to_owned()]),
        Some((_, value)) if value.as_str() == Some("all") => None,
        Some((config, value)) => match value.as_array() {
            Some(values) => {
                let mut names = Vec::new();
                for entry in values.iter() {
                    if let Some(name) = entry.as_str() {
                        names.push(normalize_group_name(name));
                    } else {
                        report.line(
                            Severity::Error,
                            format!(
                                "dependencies: {} must contain only group names",
                                config.label("default-groups")
                            ),
                        )?;
                        invalid = true;
                    }
                }
                Some(names)
            }
            None => {
                report.line(
                    Severity::Error,
                    format!(
                        "dependencies: {} must be a list or 'all'",
                        config.label("default-groups")
                    ),
                )?;
                invalid = true;
                Some(Vec::new())
            }
        },
    };
    let disabled_groups: Vec<String> = env::var("UV_NO_GROUP")
        .unwrap_or_default()
        .split_whitespace()
        .map(normalize_group_name)
        .chain(uv_env_enabled("UV_NO_DEV").then(|| "dev".to_owned()))
        .collect();
    let no_default_groups = uv_env_enabled("UV_NO_DEFAULT_GROUPS");
    let mut unselected_requirements = Vec::new();
    if let Some(groups) = document.get("dependency-groups") {
        let groups = groups.as_table().cloned().or_else(|| {
            groups.as_inline_table().map(|inline| inline.clone().into_table())
        });
        if let Some(groups) = groups {
            let mut group_names = HashMap::new();
            for (name, _) in groups.iter() {
                if !valid_group_name(name) {
                    report.line(Severity::Error, format!("dependencies: group name {name:?} is invalid; use an ASCII name starting and ending with a letter or digit"))?;
                    invalid = true;
                }
                let normalized = normalize_group_name(name);
                if let Some(previous) =
                    group_names.insert(normalized.clone(), name.to_owned())
                {
                    report.line(Severity::Error, format!("dependencies: duplicate dependency group names {previous:?} and {name:?} normalize to {normalized:?}; rename one group"))?;
                    invalid = true;
                }
            }
            let mut collector = GroupCollector {
                groups: &groups,
                names: &group_names,
                source: &metadata,
                checked: Vec::new(),
                selected: Vec::new(),
                active: Vec::new(),
            };
            let configured_names: Vec<String> = match &selected_groups {
                Some(selected) => selected.clone(),
                None => groups
                    .iter()
                    .map(|(name, _)| normalize_group_name(name))
                    .collect(),
            };
            for name in &configured_names {
                let implicit_dev =
                    name == "dev" && (has_legacy_dev || !explicit_defaults);
                if !group_names.contains_key(name) && !implicit_dev {
                    report.line(Severity::Error, format!("dependencies: group {name:?} is missing; define it in [dependency-groups]"))?;
                    invalid = true;
                }
            }
            for name in configured_names {
                if no_default_groups
                    || disabled_groups.iter().any(|disabled| disabled == &name)
                    || !group_names.contains_key(&name)
                {
                    continue;
                }
                invalid |= !collector.collect(
                    &name,
                    &mut requirements,
                    &mut unselected_requirements,
                    true,
                    report,
                )?;
            }
            for (name, _) in groups.iter() {
                let normalized = normalize_group_name(name);
                if !collector.checked.iter().any(|entry| entry == &normalized) {
                    invalid |= !collector.collect(
                        &normalized,
                        &mut requirements,
                        &mut unselected_requirements,
                        false,
                        report,
                    )?;
                }
                if !collector.selected.iter().any(|entry| entry == &normalized)
                {
                    report.line(Severity::Warn, format!("dependencies: group {name:?} is not checked for installed distributions; sync it explicitly if needed"))?;
                }
            }
        } else {
            report.line(
                Severity::Error,
                "dependencies: [dependency-groups] must be a table",
            )?;
            invalid = true;
        }
    } else if explicit_defaults && let Some(selected) = &selected_groups {
        for name in selected {
            if name == "dev" && has_legacy_dev {
                continue;
            }
            report.line(Severity::Error, format!("dependencies: group {name:?} is missing; define it in [dependency-groups]"))?;
            invalid = true;
        }
    }
    for config in project_settings.iter() {
        let Some(legacy) = config.setting("dev-dependencies") else {
            continue;
        };
        let label = config.label("dev-dependencies");
        let selected = selected_groups
            .as_ref()
            .is_none_or(|selected| selected.iter().any(|name| name == "dev"))
            && !no_default_groups
            && !disabled_groups.iter().any(|name| name == "dev");
        if let Some(legacy) = legacy.as_array() {
            for entry in legacy.iter() {
                if let Some(requirement) = entry.as_str() {
                    let target = if selected {
                        &mut requirements
                    } else {
                        &mut unselected_requirements
                    };
                    target.push(Requirement {
                        text: requirement.to_owned(),
                        source: config.path.clone(),
                    });
                } else {
                    report.line(Severity::Error, format!("dependencies: {label} must contain only requirement strings; correct {}", config.path.display()))?;
                    invalid = true;
                }
            }
        } else {
            report.line(
                Severity::Error,
                format!(
                    "dependencies: {label} must be an array; correct {}",
                    config.path.display()
                ),
            )?;
            invalid = true;
        }
    }
    let selected_count = requirements.len();
    requirements.extend(unselected_requirements);
    Ok((!invalid)
        .then_some(DeclaredDependencies { requirements, selected_count }))
}

const REQUIREMENT_PROBE: &str = r#"
import pathlib
import sys

try:
    from packaging.requirements import Requirement, InvalidRequirement
    from packaging.utils import canonicalize_name
except ImportError:
    try:
        from pip._vendor.packaging.requirements import Requirement, InvalidRequirement
        from pip._vendor.packaging.utils import canonicalize_name
    except ImportError:
        import ensurepip
        bundled = pathlib.Path(ensurepip.__file__).parent / '_bundled'
        wheels = sorted(bundled.glob('pip-*.whl'))
        if not wheels:
            raise RuntimeError('PyPA packaging is unavailable; install packaging or pip in the selected Python environment')
        sys.path.insert(0, str(wheels[-1]))
        from pip._vendor.packaging.requirements import Requirement, InvalidRequirement
        from pip._vendor.packaging.utils import canonicalize_name

for index, source in enumerate(sys.argv[1:]):
    try:
        requirement = Requirement(source)
        if requirement.marker is None or requirement.marker.evaluate({'extra': ''}):
            print(f'A\t{index}\t{canonicalize_name(requirement.name)}')
    except InvalidRequirement:
        print(f'E\t{index}')
"#;

fn active_dependencies(
    python: &Path,
    requirements: &[Requirement],
    cancelled: &AtomicBool,
    report: &mut Report,
) -> Result<Option<Vec<(usize, String)>>> {
    let output = tempfile::NamedTempFile::new().map_err(|error| {
        Error::io(
            "create requirement output",
            Path::new("temporary file"),
            error,
        )
    })?;
    let errors = tempfile::NamedTempFile::new().map_err(|error| {
        Error::io(
            "create requirement error output",
            Path::new("temporary file"),
            error,
        )
    })?;
    let scratch = tempfile::tempdir().map_err(|error| {
        Error::io(
            "create requirement scratch directory",
            Path::new("temporary directory"),
            error,
        )
    })?;
    let mut command = Command::new(python);
    command
        .current_dir(scratch.path())
        .args(["-I", "-B", "-c", REQUIREMENT_PROBE])
        .args(requirements.iter().map(|requirement| &requirement.text))
        .stdin(Stdio::null())
        .stdout(
            output
                .reopen()
                .map_err(|error| Error::io("open", output.path(), error))?,
        )
        .stderr(
            errors
                .reopen()
                .map_err(|error| Error::io("open", errors.path(), error))?,
        );
    let status = match process::run(command, cancelled) {
        Ok(status) => status,
        Err(Error::Cancelled) => return Err(Error::Cancelled),
        Err(error) => {
            report.line(
                Severity::Error,
                format!("dependencies: could not run requirement parser with {}: {error}; repair the selected Python interpreter or run pyfaro sync", python.display()),
            )?;
            return Ok(None);
        }
    };
    if !status.success() {
        report.line(Severity::Error, format!("dependencies: could not parse requirements with {} ({status}): {}; repair the selected Python interpreter or install packaging", python.display(), process::read_diagnostic(errors.path())?))?;
        return Ok(None);
    }
    let output = fs::read_to_string(output.path())
        .map_err(|error| Error::io("read", output.path(), error))?;
    let mut names = Vec::new();
    let mut invalid = false;
    for line in output.lines() {
        let mut fields = line.split('\t');
        let kind = fields.next();
        let index = fields.next().and_then(|value| value.parse::<usize>().ok());
        let Some((index, requirement)) = index.and_then(|index| {
            requirements.get(index).map(|requirement| (index, requirement))
        }) else {
            report.line(
                Severity::Error,
                "dependencies: invalid requirement probe output",
            )?;
            return Ok(None);
        };
        match (kind, fields.next(), fields.next()) {
            (Some("A"), Some(name), None) if !name.is_empty() => {
                names.push((index, name.to_owned()))
            }
            (Some("E"), None, None) => {
                report.line(
                    Severity::Error,
                    format!(
                        "dependencies: invalid requirement {:?}; correct {}",
                        requirement.text,
                        requirement.source.display()
                    ),
                )?;
                invalid = true;
            }
            _ => {
                report.line(
                    Severity::Error,
                    "dependencies: invalid requirement probe output",
                )?;
                return Ok(None);
            }
        }
    }
    if invalid {
        return Ok(None);
    }
    Ok(Some(names))
}

fn inspect_dependencies(
    root: &Path,
    python: Option<&Path>,
    dependencies: &DeclaredDependencies,
    cancelled: &AtomicBool,
    report: &mut Report,
) -> Result<()> {
    if dependencies.requirements.is_empty() {
        return report.line(
            Severity::Ok,
            "dependencies: no runtime dependencies to check",
        );
    }
    let Some(python) = python else {
        return Ok(());
    };
    let environment = tools::project_environment(root);
    let environment_name = if environment == root.join(".venv") {
        ".venv".to_owned()
    } else {
        environment.display().to_string()
    };
    let local = tools::project_python(root);
    let parser_python = if local.is_file() { &local } else { python };
    let declared_count = dependencies.selected_count;
    let Some(dependencies) = active_dependencies(
        parser_python,
        &dependencies.requirements,
        cancelled,
        report,
    )?
    else {
        return Ok(());
    };
    let mut dependencies: Vec<String> = dependencies
        .into_iter()
        .filter(|(index, _)| *index < declared_count)
        .map(|(_, name)| name)
        .collect();
    dependencies.sort_unstable();
    dependencies.dedup();
    if dependencies.is_empty() {
        return report.line(
            Severity::Ok,
            "dependencies: no applicable runtime distributions to check",
        );
    }
    if !local.is_file() {
        return report.line(
            Severity::Error,
            format!("dependencies: project interpreter {} is missing; run pyfaro sync", local.display()),
        );
    }
    let output = tempfile::NamedTempFile::new().map_err(|error| {
        Error::io(
            "create dependency output",
            Path::new("temporary file"),
            error,
        )
    })?;
    let errors = tempfile::NamedTempFile::new().map_err(|error| {
        Error::io(
            "create dependency error output",
            Path::new("temporary file"),
            error,
        )
    })?;
    let mut command = Command::new(&local);
    let scratch = tempfile::tempdir().map_err(|error| {
        Error::io(
            "create dependency scratch directory",
            Path::new("temporary directory"),
            error,
        )
    })?;
    command
        .current_dir(scratch.path())
        .args(["-I", "-B", "-c", "import importlib.metadata as m, sys\nfor name in sys.argv[1:]:\n try: m.version(name)\n except m.PackageNotFoundError: print(name)"])
        .args(&dependencies)
        .stdin(Stdio::null())
        .stdout(output.reopen().map_err(|error| Error::io("open", output.path(), error))?)
        .stderr(errors.reopen().map_err(|error| Error::io("open", errors.path(), error))?);
    let status = match process::run(command, cancelled) {
        Ok(status) => status,
        Err(Error::Cancelled) => return Err(Error::Cancelled),
        Err(error) => return report.line(Severity::Error, format!("dependencies: could not inspect {}: {error}; repair the project environment or run pyfaro sync", local.display())),
    };
    if !status.success() {
        return report.line(Severity::Error, format!("dependencies: interpreter check failed ({status}): {}; repair the project environment or run pyfaro sync", process::read_diagnostic(errors.path())?));
    }
    let missing = fs::read_to_string(output.path())
        .map_err(|error| Error::io("read", output.path(), error))?;
    if missing.is_empty() {
        report.line(
            Severity::Ok,
            format!(
                "dependencies: {} runtime distribution(s) installed in {}",
                dependencies.len(),
                environment_name
            ),
        )
    } else {
        for name in missing.lines() {
            report.line(Severity::Error, format!("dependencies: {name} is missing from {environment_name}; run pyfaro sync"))?;
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::{Report, Requirement, active_dependencies};
    use std::sync::atomic::AtomicBool;

    #[test]
    fn requirement_parser_spawn_failure_is_a_blocking_finding() {
        let sandbox = tempfile::tempdir().unwrap();
        let missing_python = sandbox.path().join("missing-python");
        let mut report = Report { blocking: 0 };
        let result = active_dependencies(
            &missing_python,
            &[Requirement {
                text: "example>=1".to_owned(),
                source: sandbox.path().join("pyproject.toml"),
            }],
            &AtomicBool::new(false),
            &mut report,
        )
        .unwrap();
        assert!(result.is_none());
        assert_eq!(report.blocking, 1);
    }
}
