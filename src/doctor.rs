use std::ffi::OsString;
use std::fs;
use std::io::Write;
use std::path::Path;
use std::process::{Command, Stdio};
use std::sync::atomic::AtomicBool;

use pep440_rs::{Version, VersionSpecifiers};
use toml_edit::{DocumentMut, Table};

use crate::cli::{DoctorArgs, Tool};
use crate::config::Config;
use crate::error::{Error, Result};
use crate::{process, project, tools};

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
            report.line(
                Severity::Error,
                format!(
                    "project: {error}; use --project <PATH> to select a project"
                ),
            )?;
            None
        }
    };

    let mut dependencies = None;
    let mut requires_python = None;
    if let Some(root) = root.as_deref() {
        match Config::load_document(root) {
            Ok(document) => {
                report.line(
                    Severity::Ok,
                    "pyproject.toml: valid Pyfaro configuration",
                )?;
                inspect_configuration(root, &document, &mut report)?;
                dependencies = declared_dependencies(&document, &mut report)?;
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
                inspect_python_pin(root, &version, &mut report)?;
            }
            Some(path)
        }
        Err(Error::Cancelled) => return Err(Error::Cancelled),
        Err(error) => {
            report.line(Severity::Error, format!("Python: {error}"))?;
            None
        }
    };
    inspect_tool(
        Tool::Uv,
        root.as_deref(),
        args.uv.as_ref(),
        &cancelled,
        &mut report,
        true,
    )?;
    for tool in [Tool::Ruff, Tool::Ty, Tool::Pytest] {
        inspect_tool(
            tool,
            root.as_deref(),
            args.uv.as_ref(),
            &cancelled,
            &mut report,
            false,
        )?;
    }
    if let (Some(root), Some(dependencies)) =
        (root.as_deref(), dependencies.as_deref())
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

fn inspect_tool(
    tool: Tool,
    root: Option<&Path>,
    uv: Option<&OsString>,
    cancelled: &AtomicBool,
    report: &mut Report,
    required: bool,
) -> Result<()> {
    let name = tools::name(tool);
    match tools::inspect(tool, root, uv, cancelled) {
        Ok((path, version)) => report.line(
            Severity::Ok,
            format!("{name}: {} ({version})", path.display()),
        ),
        Err(Error::Cancelled) => Err(Error::Cancelled),
        Err(error) => report.line(
            if required { Severity::Error } else { Severity::Warn },
            format!("{name}: {error}"),
        ),
    }
}

fn inspect_configuration(
    root: &Path,
    document: &DocumentMut,
    report: &mut Report,
) -> Result<()> {
    for (section, files) in [
        ("ruff", &["ruff.toml", ".ruff.toml"][..]),
        ("ty", &["ty.toml"][..]),
        ("pytest", &["pytest.ini"][..]),
        ("uv", &["uv.toml"][..]),
    ] {
        let configured =
            document.get("tool").and_then(|tool| tool.get(section)).is_some();
        let present: Vec<_> = files
            .iter()
            .filter(|file| root.join(file).exists())
            .copied()
            .collect();
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
    Ok(())
}

fn inspect_layout(root: &Path, report: &mut Report) -> Result<()> {
    for directory in ["src", "tests", ".venv"] {
        let path = root.join(directory);
        if path.exists() && !path.is_dir() {
            report.line(
                Severity::Error,
                format!("layout: {} must be a directory; move or remove the conflicting file", path.display()),
            )?;
        }
    }
    let venv = root.join(".venv");
    let interpreter = venv.join(if cfg!(windows) {
        "Scripts/python.exe"
    } else {
        "bin/python"
    });
    if venv.is_dir() && !interpreter.is_file() {
        report.line(
            Severity::Error,
            format!(
                "layout: {} has no Python interpreter; repair it with pyfaro sync",
                venv.display()
            ),
        )?;
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
    reported_version: &str,
    report: &mut Report,
) -> Result<()> {
    let path = root.join(".python-version");
    let request =
        match fs::read_to_string(&path) {
            Ok(request) => request,
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => {
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

fn collect_group(
    name: &str,
    groups: &Table,
    requirements: &mut Vec<String>,
    checked: &mut Vec<String>,
    active: &mut Vec<String>,
    report: &mut Report,
) -> Result<bool> {
    if checked.iter().any(|entry| entry == name) {
        return Ok(true);
    }
    if active.iter().any(|entry| entry == name) {
        report.line(
            Severity::Error,
            format!("dependencies: dependency group cycle includes {name:?}"),
        )?;
        return Ok(false);
    }
    let Some(group) = groups.get(name).and_then(|group| group.as_array())
    else {
        report.line(
            Severity::Error,
            format!("dependencies: group {name:?} is missing or not an array"),
        )?;
        return Ok(false);
    };
    active.push(name.to_owned());
    let mut valid = true;
    for entry in group.iter() {
        if let Some(requirement) = entry.as_str() {
            requirements.push(requirement.to_owned());
        } else if let Some(included) = entry
            .as_inline_table()
            .and_then(|table| table.get("include-group"))
            .and_then(|value| value.as_str())
        {
            valid &= collect_group(
                included,
                groups,
                requirements,
                checked,
                active,
                report,
            )?;
        } else {
            report.line(
                Severity::Error,
                format!(
                    "dependencies: group {name:?} contains an invalid entry"
                ),
            )?;
            valid = false;
        }
    }
    active.pop();
    checked.push(name.to_owned());
    Ok(valid)
}

fn declared_dependencies(
    document: &DocumentMut,
    report: &mut Report,
) -> Result<Option<Vec<String>>> {
    let mut requirements = Vec::new();
    let mut invalid = false;
    if let Some(dependencies) =
        document.get("project").and_then(|project| project.get("dependencies"))
    {
        if let Some(dependencies) = dependencies.as_array() {
            for dependency in dependencies.iter() {
                if let Some(requirement) = dependency.as_str() {
                    requirements.push(requirement.to_owned());
                } else {
                    report.line(Severity::Error, "dependencies: each [project].dependencies entry must be a string")?;
                    invalid = true;
                }
            }
        } else {
            report.line(Severity::Error, "dependencies: [project].dependencies must be an array of requirement strings")?;
            invalid = true;
        }
    } else if document
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
    let default_groups = document
        .get("tool")
        .and_then(|tool| tool.get("uv"))
        .and_then(|uv| uv.get("default-groups"));
    let selected_groups = match default_groups {
        None => Some(vec!["dev".to_owned()]),
        Some(value) if value.as_str() == Some("all") => None,
        Some(value) => {
            if let Some(entries) = value.as_array() {
                let mut names = Vec::new();
                for entry in entries.iter() {
                    if let Some(name) = entry.as_str() {
                        names.push(name.to_owned());
                    } else {
                        report.line(Severity::Error, "dependencies: [tool.uv].default-groups must contain only group names")?;
                        invalid = true;
                    }
                }
                Some(names)
            } else {
                report.line(Severity::Error, "dependencies: [tool.uv].default-groups must be a list or 'all'")?;
                invalid = true;
                Some(Vec::new())
            }
        }
    };
    let mut checked_groups = Vec::new();
    if let Some(groups) = document.get("dependency-groups") {
        if let Some(groups) = groups.as_table() {
            let names: Vec<String> = match &selected_groups {
                Some(selected) => selected
                    .iter()
                    .filter(|name| groups.contains_key(name.as_str()))
                    .cloned()
                    .collect(),
                None => {
                    groups.iter().map(|(name, _)| name.to_owned()).collect()
                }
            };
            for name in names {
                invalid |= !collect_group(
                    &name,
                    groups,
                    &mut requirements,
                    &mut checked_groups,
                    &mut Vec::new(),
                    report,
                )?;
            }
            for (name, _) in groups.iter() {
                if !checked_groups.iter().any(|entry| entry == name) {
                    report.line(Severity::Warn, format!("dependencies: group {name:?} is not checked because pyfaro sync does not select it by default; sync it explicitly if needed"))?;
                }
            }
        } else {
            report.line(
                Severity::Error,
                "dependencies: [dependency-groups] must be a table",
            )?;
            invalid = true;
        }
    }
    if selected_groups
        .as_ref()
        .is_none_or(|selected| selected.iter().any(|name| name == "dev"))
        && let Some(legacy) = document
            .get("tool")
            .and_then(|tool| tool.get("uv"))
            .and_then(|uv| uv.get("dev-dependencies"))
    {
        if let Some(legacy) = legacy.as_array() {
            for entry in legacy.iter() {
                if let Some(requirement) = entry.as_str() {
                    requirements.push(requirement.to_owned());
                } else {
                    report.line(Severity::Error, "dependencies: [tool.uv].dev-dependencies must contain only requirement strings")?;
                    invalid = true;
                }
            }
        } else {
            report.line(
                Severity::Error,
                "dependencies: [tool.uv].dev-dependencies must be an array",
            )?;
            invalid = true;
        }
    }
    Ok((!invalid).then_some(requirements))
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
    requirements: &[String],
    cancelled: &AtomicBool,
    report: &mut Report,
) -> Result<Option<Vec<String>>> {
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
        .args(requirements)
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
    let status = process::run(command, cancelled)?;
    if !status.success() {
        report.line(Severity::Error, format!("dependencies: could not parse requirements with {} ({status}): {}", python.display(), process::read_diagnostic(errors.path())?))?;
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
        let Some(requirement) = index.and_then(|index| requirements.get(index))
        else {
            report.line(
                Severity::Error,
                "dependencies: invalid requirement probe output",
            )?;
            return Ok(None);
        };
        match (kind, fields.next(), fields.next()) {
            (Some("A"), Some(name), None) if !name.is_empty() => {
                names.push(name.to_owned())
            }
            (Some("E"), None, None) => {
                report.line(Severity::Error, format!("dependencies: invalid requirement {requirement:?}; correct pyproject.toml"))?;
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
    names.sort_unstable();
    names.dedup();
    if names.is_empty() && invalid {
        return Ok(None);
    }
    Ok(Some(names))
}

fn inspect_dependencies(
    root: &Path,
    python: Option<&Path>,
    dependencies: &[String],
    cancelled: &AtomicBool,
    report: &mut Report,
) -> Result<()> {
    if dependencies.is_empty() {
        return report.line(
            Severity::Ok,
            "dependencies: no runtime dependencies to check",
        );
    }
    let Some(python) = python else {
        return Ok(());
    };
    let local = root.join(".venv").join(if cfg!(windows) {
        "Scripts/python.exe"
    } else {
        "bin/python"
    });
    let parser_python = if local.is_file() { &local } else { python };
    let Some(dependencies) =
        active_dependencies(parser_python, dependencies, cancelled, report)?
    else {
        return Ok(());
    };
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
                "dependencies: {} runtime distribution(s) installed in .venv",
                dependencies.len()
            ),
        )
    } else {
        for name in missing.lines() {
            report.line(Severity::Error, format!("dependencies: {name} is missing from .venv; run pyfaro sync"))?;
        }
        Ok(())
    }
}
