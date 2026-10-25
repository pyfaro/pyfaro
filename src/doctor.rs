use std::io::Write;

use crate::cli::{DoctorArgs, Tool};
use crate::config::Config;
use crate::error::{Error, Result};
use crate::{process, project, redact, tools};

use self::configuration::{
    inspect_configuration, inspect_project_metadata,
    uses_pytest_9_configuration,
};
use self::dependencies::{declared_dependencies, inspect_dependencies};
use self::python::{inspect_python_pin, inspect_python_requirement};
use self::toolchain::{
    check_pytest_9_configuration, failure_detail, inspect_tool,
};
use self::workspace::{inspect_layout, inspect_workspace};

mod configuration;
mod dependencies;
mod python;
mod toolchain;
mod uv_config;
mod workspace;

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
            tools::single_line(&redact::text(message.as_ref()))
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
            check_pytest_9_configuration(&version, &mut report)?;
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
