//! Code-quality commands that run a project tool directly (no shell, no `uv run`).
//!
//! Tested with Ruff 0.16.10 and ty 0.0.85. Other versions are not rejected.
//! These commands never sync: they do not create or modify `.venv` or `uv.lock`.

use std::ffi::OsString;
use std::path::{Path, PathBuf};
use std::process::{Command, ExitStatus, Stdio};
use std::sync::Arc;
use std::sync::atomic::AtomicBool;

use crate::cli::{FmtArgs, LintArgs, TestArgs, TypeArgs};
use crate::config::Config;
use crate::error::{Error, Result};
use crate::{process, project, tools};

/// A selected project and the cancellation flag shared by its steps.
pub(crate) struct Context {
    pub root: PathBuf,
    pub cancelled: Arc<AtomicBool>,
}

/// Select the project (`--project` exactly, otherwise discovery from the
/// current directory) and validate its configuration.
pub(crate) fn context(project: Option<&Path>) -> Result<Context> {
    let root = project::select(project)?;
    let config = Config::load(&root)?;
    let cancelled = process::cancellation()?;
    Ok(Context { root: config.root().to_owned(), cancelled })
}

/// Make positional paths absolute against the user's current directory, so
/// they stay valid when the tool runs with the project root as its cwd.
pub(crate) fn absolute_paths(paths: &[PathBuf]) -> Result<Vec<OsString>> {
    paths
        .iter()
        .map(|path| {
            std::path::absolute(path)
                .map(PathBuf::into_os_string)
                .map_err(|source| Error::io("resolve path", path, source))
        })
        .collect()
}

/// Run `tool` (resolved with the tools.rs precedence) with cwd = project root
/// and return its exit status. Stdio is inherited; interactive execution keeps
/// stdin usable (for example `ruff format -`) and handles cancellation.
/// A non-zero status is not an error here, so callers can run several steps.
pub(crate) fn run_tool(
    context: &Context,
    tool: &'static str,
    args: &[OsString],
) -> Result<ExitStatus> {
    let binary = tools::resolve(tool, Some(&context.root)).map_err(|error| {
        match error {
            // Keep other reasons, such as a rejected Windows batch wrapper.
            Error::Tool { tool, reason } if reason.starts_with("not found") => Error::Tool {
                tool,
                reason: format!(
                    "not found in the project environment, VIRTUAL_ENV, or PATH; run `pyfaro add --dev {tool}` then `pyfaro sync`"
                ),
            },
            error => error,
        }
    })?;
    let mut command = Command::new(binary);
    command.args(args).current_dir(&context.root);
    process::run_interactive(command, &context.cancelled)
}

/// Convert a tool's non-success status into [`Error::ToolStatus`].
pub(crate) fn require_success(
    tool: &'static str,
    status: ExitStatus,
) -> Result<()> {
    if status.success() {
        Ok(())
    } else {
        Err(Error::ToolStatus { tool, status })
    }
}

/// Arguments for `ruff check` or `ruff format`: subcommand, extra flags, then
/// `--force-exclude` and absolute paths only when paths were given.
pub(crate) fn ruff_args(
    subcommand: &str,
    flags: &[&str],
    paths: &[PathBuf],
) -> Result<Vec<OsString>> {
    let mut args = vec![OsString::from(subcommand)];
    args.extend(flags.iter().map(OsString::from));
    if !paths.is_empty() {
        args.push(OsString::from("--force-exclude"));
        args.extend(absolute_paths(paths)?);
    }
    Ok(args)
}

pub(crate) fn lint(args: LintArgs) -> Result<()> {
    let context = context(args.project.as_deref())?;
    let arguments = ruff_args("check", &[], &args.paths)?;
    require_success("ruff", run_tool(&context, "ruff", &arguments)?)
}

pub(crate) fn fmt(args: FmtArgs) -> Result<()> {
    let context = context(args.project.as_deref())?;
    let flags: &[&str] = if args.check { &["--check"] } else { &[] };
    let arguments = ruff_args("format", flags, &args.paths)?;
    require_success("ruff", run_tool(&context, "ruff", &arguments)?)
}

/// Arguments for `ty check`: `--python <environment>` only when that
/// directory exists, then `--force-exclude` and absolute paths only when
/// paths were given.
pub(crate) fn ty_args(
    environment: &Path,
    paths: &[PathBuf],
) -> Result<Vec<OsString>> {
    let mut args = vec![OsString::from("check")];
    if environment.is_dir() {
        args.push(OsString::from("--python"));
        args.push(environment.as_os_str().to_owned());
    }
    if !paths.is_empty() {
        args.push(OsString::from("--force-exclude"));
        args.extend(absolute_paths(paths)?);
    }
    Ok(args)
}

pub(crate) fn type_check(args: TypeArgs) -> Result<()> {
    let context = context(args.project.as_deref())?;
    let environment = tools::project_environment(&context.root);
    let arguments = ty_args(&environment, &args.paths)?;
    require_success("ty", run_tool(&context, "ty", &arguments)?)
}

/// Run pytest through the project environment's interpreter, so it imports
/// that environment's packages. Pyfaro's own failures (no interpreter, pytest
/// not importable) are reported as errors, never as pytest statuses.
pub(crate) fn test(args: TestArgs) -> Result<()> {
    let context = context(args.project.as_deref())?;
    let python = tools::project_python(&context.root);
    if !python.is_file() {
        return Err(Error::Tool {
            tool: "pytest",
            reason: format!(
                "project environment interpreter not found at {}; run `pyfaro sync` first",
                python.display()
            ),
        });
    }
    let mut probe = Command::new(&python);
    probe
        .args([
            "-c",
            "import importlib.util, sys; sys.exit(importlib.util.find_spec('pytest') is None)",
        ])
        .current_dir(&context.root)
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null());
    if !process::run(probe, &context.cancelled)?.success() {
        return Err(Error::Tool {
            tool: "pytest",
            reason: format!(
                "not installed in the project environment ({}); run `pyfaro add --dev pytest` then `pyfaro sync`",
                python.display()
            ),
        });
    }
    let mut command = Command::new(&python);
    command
        .args(["-m", "pytest"])
        .args(&args.pytest_args)
        .current_dir(&context.root);
    let status = process::run_interactive(command, &context.cancelled)?;
    require_success("pytest", status)
}
