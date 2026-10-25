//! Code-quality commands that run a project tool directly (no shell, no `uv run`).
//!
//! Tested with Ruff 0.16.10 and ty 0.0.85. Other versions are not rejected.
//! These commands never sync: they do not create or modify `.venv` or `uv.lock`.

use std::ffi::OsString;
use std::path::{Path, PathBuf};
use std::process::{Command, ExitStatus, Stdio};
use std::sync::Arc;
use std::sync::atomic::AtomicBool;

use crate::cli::{CheckArgs, FmtArgs, LintArgs, TestArgs, TypeArgs};
use crate::config::Config;
use crate::error::{Error, Result};
use crate::{process, project, redact, tools};

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
    Ok(Context { root: normalize(config.root()), cancelled })
}

/// Windows temp directories often use 8.3 short names (`RUNNER~1`) while ty
/// canonicalizes its project root to the long form, so an exclude pattern
/// never matches a path spelled with the short form. Resolve existing paths
/// to the long form and drop the `\\?\` prefix that `canonicalize` adds.
/// Unchanged elsewhere, and for paths that do not exist.
fn normalize(path: &Path) -> PathBuf {
    if cfg!(windows)
        && let Ok(long) = path.canonicalize()
    {
        return strip_verbatim(&long);
    }
    path.to_owned()
}

/// `\\?\C:\dir` becomes `C:\dir`. UNC and device paths are kept verbatim.
pub(crate) fn strip_verbatim(path: &Path) -> PathBuf {
    let Some(text) = path.to_str() else {
        return path.to_owned();
    };
    match text.strip_prefix(r"\\?\").map(str::as_bytes) {
        Some([drive, b':', b'\\', ..]) if drive.is_ascii_alphabetic() => {
            PathBuf::from(&text[4..])
        }
        _ => path.to_owned(),
    }
}

/// Make positional paths absolute against the user's current directory, so
/// they stay valid when the tool runs with the project root as its cwd.
pub(crate) fn absolute_paths(paths: &[PathBuf]) -> Result<Vec<OsString>> {
    paths
        .iter()
        .map(|path| {
            std::path::absolute(path)
                .map(|absolute| normalize(&absolute).into_os_string())
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

/// One tool invocation shared by `lint`, `fmt`, `type` and `check`.
#[derive(Clone, Copy)]
enum Step {
    /// `ruff format`, optionally `--check`.
    Format { check: bool },
    /// `ruff check`, optionally `--fix` (safe fixes only).
    Lint { fix: bool },
    /// `ty check`.
    Type,
}

impl Step {
    fn tool(self) -> &'static str {
        match self {
            Self::Format { .. } | Self::Lint { .. } => "ruff",
            Self::Type => "ty",
        }
    }

    /// Full argument list. Paths are absolute and, when given, preceded by
    /// `--force-exclude`; `ty` gets `--python <environment>` only when that
    /// directory exists.
    fn args(
        self,
        context: &Context,
        paths: &[PathBuf],
    ) -> Result<Vec<OsString>> {
        let mut args = Vec::new();
        match self {
            Self::Format { check } => {
                args.push(OsString::from("format"));
                if check {
                    args.push(OsString::from("--check"));
                }
            }
            Self::Lint { fix } => {
                args.push(OsString::from("check"));
                if fix {
                    args.push(OsString::from("--fix"));
                }
            }
            Self::Type => {
                args.push(OsString::from("check"));
                let environment = tools::project_environment(&context.root);
                if environment.is_dir() {
                    args.push(OsString::from("--python"));
                    args.push(environment.into_os_string());
                }
            }
        }
        if !paths.is_empty() {
            args.push(OsString::from("--force-exclude"));
            args.extend(absolute_paths(paths)?);
        }
        Ok(args)
    }

    fn run(self, context: &Context, paths: &[PathBuf]) -> Result<ExitStatus> {
        run_tool(context, self.tool(), &self.args(context, paths)?)
    }
}

fn single(step: Step, project: Option<&Path>, paths: &[PathBuf]) -> Result<()> {
    let context = context(project)?;
    tools::announce(&[step.tool()], Some(&context.root), &context.cancelled);
    require_success(step.tool(), step.run(&context, paths)?)
}

pub(crate) fn lint(args: LintArgs) -> Result<()> {
    single(Step::Lint { fix: false }, args.project.as_deref(), &args.paths)
}

pub(crate) fn fmt(args: FmtArgs) -> Result<()> {
    single(
        Step::Format { check: args.check },
        args.project.as_deref(),
        &args.paths,
    )
}

pub(crate) fn type_check(args: TypeArgs) -> Result<()> {
    single(Step::Type, args.project.as_deref(), &args.paths)
}

/// What happened when a step ran.
enum Outcome {
    Passed,
    Failed(ExitStatus),
    /// The tool could not be started (for example it is not installed).
    Unavailable(String),
}

impl Outcome {
    fn passed(&self) -> bool {
        matches!(self, Self::Passed)
    }

    fn describe(&self) -> String {
        match self {
            Self::Passed => "ok".to_owned(),
            Self::Failed(status) => match status.code() {
                Some(code) => format!("failed (exit {code})"),
                None => "failed (terminated by signal)".to_owned(),
            },
            Self::Unavailable(reason) => format!("unavailable ({reason})"),
        }
    }
}

/// Run one step. A missing or unusable tool is an outcome, not an error, so
/// later steps still run; cancellation and other errors abort.
fn attempt(
    step: Step,
    context: &Context,
    paths: &[PathBuf],
) -> Result<Outcome> {
    match step.run(context, paths) {
        Ok(status) if status.success() => Ok(Outcome::Passed),
        Ok(status) => Ok(Outcome::Failed(status)),
        Err(Error::Tool { tool, reason }) => {
            Ok(Outcome::Unavailable(format!("{tool} {reason}")))
        }
        Err(error) => Err(error),
    }
}

const VERIFY: [(&str, Step); 3] = [
    ("format", Step::Format { check: true }),
    ("lint", Step::Lint { fix: false }),
    ("type", Step::Type),
];

/// Print to stderr; never panic when it is closed.
fn say(text: &str) {
    use std::io::Write;
    let _ = writeln!(std::io::stderr().lock(), "{}", redact::text(text));
}

/// Run format check, lint and type check in order, then print the summary.
/// Returns whether every step passed.
fn verify(context: &Context, paths: &[PathBuf]) -> Result<bool> {
    let mut results = Vec::new();
    for (name, step) in VERIFY {
        results.push((name, attempt(step, context, paths)?));
    }
    say("");
    say("== pyfaro check summary ==");
    for (name, outcome) in &results {
        say(&format!("{name}: {}", outcome.describe()));
    }
    let failed: Vec<&str> = results
        .iter()
        .filter(|(_, outcome)| !outcome.passed())
        .map(|(name, _)| *name)
        .collect();
    if failed.is_empty() {
        say("check: ok");
    } else {
        say(&format!("check: failed ({})", failed.join(", ")));
    }
    Ok(failed.is_empty())
}

/// Apply safe automatic fixes. Failures are reported but never stop `check`.
fn apply_fixes(context: &Context, paths: &[PathBuf]) -> Result<()> {
    say(
        "fix: format and lint can be fixed automatically; type has no automatic fixes",
    );
    for (name, step) in [
        ("lint", Step::Lint { fix: true }),
        ("format", Step::Format { check: false }),
    ] {
        let outcome = attempt(step, context, paths)?;
        say(&format!("fix {name}: {}", outcome.describe()));
    }
    Ok(())
}

pub(crate) fn check(args: CheckArgs) -> Result<()> {
    let context = context(args.project.as_deref())?;
    tools::announce(&["ruff", "ty"], Some(&context.root), &context.cancelled);
    if args.fix {
        apply_fixes(&context, &args.paths)?;
        say("fix: rechecking without writing");
    }
    if verify(&context, &args.paths)? {
        Ok(())
    } else {
        Err(Error::CheckFailed)
    }
}

/// Run pytest through the project environment's interpreter, so it imports
/// that environment's packages. Pyfaro's own failures (no interpreter, pytest
/// not importable) are reported as errors, never as pytest statuses.
pub(crate) fn test(args: TestArgs) -> Result<()> {
    let context = context(args.project.as_deref())?;
    tools::announce(&["pytest"], Some(&context.root), &context.cancelled);
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn strip_verbatim_drops_the_prefix_of_drive_paths_only() {
        let strip = |text: &str| strip_verbatim(Path::new(text));
        assert_eq!(
            strip(r"\\?\C:\Users\runneradmin"),
            Path::new(r"C:\Users\runneradmin")
        );
        assert_eq!(
            strip(r"\\?\UNC\host\share"),
            Path::new(r"\\?\UNC\host\share")
        );
        assert_eq!(strip(r"C:\Users"), Path::new(r"C:\Users"));
        assert_eq!(strip("/tmp/x"), Path::new("/tmp/x"));
    }

    #[test]
    fn normalize_keeps_missing_paths_as_given() {
        let missing = Path::new("definitely/not/here.py");
        assert_eq!(normalize(missing), missing);
    }
}
