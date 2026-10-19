use std::ffi::OsString;
use std::fs;
use std::io::Write;
use std::path::Path;
use std::process::{Command, Stdio};
use std::sync::atomic::AtomicBool;

use pep440_rs::{Version, VersionSpecifiers};
use toml_edit::DocumentMut;

use crate::cli::{DoctorArgs, Tool};
use crate::config::Config;
use crate::error::{Error, Result};
use crate::{process, project, tools};

struct Report {
    blocking: usize,
}

impl Report {
    fn line(&mut self, level: &str, message: impl AsRef<str>) -> Result<()> {
        if level == "ERROR" {
            self.blocking += 1;
        }
        writeln!(
            std::io::stdout().lock(),
            "{level}: {}",
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
            report.line("OK", format!("project: {}", root.display()))?;
            Some(root)
        }
        Err(error) => {
            report.line(
                "ERROR",
                format!(
                    "project: {error}; use --project <PATH> to select a project"
                ),
            )?;
            None
        }
    };

    let mut dependencies = Vec::new();
    let mut requires_python = None;
    if let Some(root) = root.as_deref() {
        match Config::load(root) {
            Ok(_) => {
                report
                    .line("OK", "pyproject.toml: valid Pyfaro configuration")?;
                let metadata = root.join("pyproject.toml");
                let source =
                    fs::read_to_string(&metadata).map_err(|error| {
                        Error::io("read project metadata", &metadata, error)
                    })?;
                let document: DocumentMut =
                    source.parse().map_err(|source| Error::ConfigToml {
                        path: metadata,
                        source,
                    })?;
                inspect_configuration(root, &document, &mut report)?;
                dependencies = declared_dependencies(&document, &mut report)?;
                requires_python = document
                    .get("project")
                    .and_then(|project| project.get("requires-python"))
                    .map(|value| value.as_str().map(str::to_owned));
                inspect_layout(root, &mut report)?;
            }
            Err(error) => report.line(
                "ERROR",
                format!("configuration: {error}; correct pyproject.toml"),
            )?,
        }
    }

    let python = match tools::inspect_python(root.as_deref(), &cancelled) {
        Ok((path, version)) => {
            report.line(
                "OK",
                format!("Python: {} ({version})", path.display()),
            )?;
            if let Some(requirement) = requires_python.as_ref() {
                inspect_python_requirement(
                    requirement.as_deref(),
                    &version,
                    &mut report,
                )?;
            }
            Some(path)
        }
        Err(Error::Cancelled) => return Err(Error::Cancelled),
        Err(error) => {
            report.line("ERROR", format!("Python: {error}"))?;
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
    if let Some(root) = root.as_deref() {
        inspect_dependencies(
            root,
            python.as_deref(),
            &dependencies,
            &cancelled,
            &mut report,
        )?;
    }
    if report.blocking > 0 {
        Err(Error::DoctorFailed { count: report.blocking })
    } else {
        report.line("OK", "doctor: no blocking issues")
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
        Ok((path, version)) => {
            report.line("OK", format!("{name}: {} ({version})", path.display()))
        }
        Err(Error::Cancelled) => Err(Error::Cancelled),
        Err(error) => report.line(
            if required { "ERROR" } else { "WARN" },
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
                "ERROR",
                format!(
                    "configuration: [tool.{section}] and {} both configure {section}; keep one source of configuration",
                    present.join(", ")
                ),
            )?;
        } else if present.len() > 1 {
            report.line(
                "ERROR",
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
                "ERROR",
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
            "ERROR",
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
            "ERROR",
            "configuration: [project].requires-python must be a version specifier string",
        );
    };
    let Ok(specifiers) = requirement.parse::<VersionSpecifiers>() else {
        return report.line(
            "ERROR",
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
            "ERROR",
            format!("Python: {reported_version} does not satisfy requires-python {requirement:?}; select a compatible interpreter"),
        );
    }
    Ok(())
}

fn declared_dependencies(
    document: &DocumentMut,
    report: &mut Report,
) -> Result<Vec<String>> {
    let Some(dependencies) =
        document.get("project").and_then(|project| project.get("dependencies"))
    else {
        return Ok(Vec::new());
    };
    let Some(dependencies) = dependencies.as_array() else {
        report.line("ERROR", "dependencies: [project].dependencies must be an array of requirement strings")?;
        return Ok(Vec::new());
    };
    let mut names = Vec::new();
    for dependency in dependencies.iter() {
        let Some(requirement) = dependency.as_str() else {
            report.line("ERROR", "dependencies: each [project].dependencies entry must be a string")?;
            continue;
        };
        if requirement.contains(';') {
            report.line("WARN", format!("dependencies: conditional requirement {requirement:?} was not checked; verify its marker in the target environment"))?;
            continue;
        }
        let name: String = requirement
            .chars()
            .take_while(|character| {
                character.is_ascii_alphanumeric()
                    || matches!(character, '-' | '_' | '.')
            })
            .collect();
        if name.is_empty()
            || !name
                .chars()
                .next()
                .is_some_and(|character| character.is_ascii_alphanumeric())
        {
            report.line("ERROR", format!("dependencies: invalid requirement {requirement:?}; correct pyproject.toml"))?;
        } else {
            names.push(name);
        }
    }
    names.sort_unstable();
    names.dedup();
    Ok(names)
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
            "OK",
            "dependencies: no unconditional runtime dependencies to check",
        );
    }
    let local = root.join(".venv").join(if cfg!(windows) {
        "Scripts/python.exe"
    } else {
        "bin/python"
    });
    if !local.is_file() {
        return report.line(
            "ERROR",
            format!("dependencies: project interpreter {} is missing; run pyfaro sync", local.display()),
        );
    }
    if python.is_none() {
        return Ok(());
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
    command
        .current_dir(root)
        .args(["-I", "-B", "-c", "import importlib.metadata as m, sys\nfor name in sys.argv[1:]:\n try: m.version(name)\n except m.PackageNotFoundError: print(name)"])
        .args(dependencies)
        .stdin(Stdio::null())
        .stdout(output.reopen().map_err(|error| Error::io("open", output.path(), error))?)
        .stderr(errors.reopen().map_err(|error| Error::io("open", errors.path(), error))?);
    let status = match process::run(command, cancelled) {
        Ok(status) => status,
        Err(Error::Cancelled) => return Err(Error::Cancelled),
        Err(error) => return report.line("ERROR", format!("dependencies: could not inspect {}: {error}; repair the project environment or run pyfaro sync", local.display())),
    };
    if !status.success() {
        return report.line("ERROR", format!("dependencies: interpreter check failed ({status}): {}; repair the project environment or run pyfaro sync", process::read_diagnostic(errors.path())?));
    }
    let missing = fs::read_to_string(output.path())
        .map_err(|error| Error::io("read", output.path(), error))?;
    if missing.is_empty() {
        report.line(
            "OK",
            format!(
                "dependencies: {} runtime distribution(s) installed in .venv",
                dependencies.len()
            ),
        )
    } else {
        for name in missing.lines() {
            report.line("ERROR", format!("dependencies: {name} is missing from .venv; run pyfaro sync"))?;
        }
        Ok(())
    }
}
