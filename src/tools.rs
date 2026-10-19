use std::env;
use std::ffi::OsString;
use std::fs;
use std::io::Write;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::sync::atomic::AtomicBool;

use semver::Version;

use crate::cli::{Tool, ToolsArgs};
use crate::error::{Error, Result};
use crate::{process, project, uv};

const ALL: [Tool; 4] = [Tool::Uv, Tool::Ruff, Tool::Ty, Tool::Pytest];

pub(crate) fn run(args: ToolsArgs) -> Result<()> {
    let root = match args.project.as_deref() {
        Some(path) => Some(project::select(Some(path))?),
        None => match project::discover(Path::new(".")) {
            Ok(root) => Some(root),
            Err(Error::NoProject { .. }) => None,
            Err(error) => return Err(error),
        },
    };
    let cancelled = process::cancellation()?;
    let mut output = std::io::stdout().lock();
    for tool in args
        .tool
        .into_iter()
        .chain(ALL.into_iter().filter(|_| args.tool.is_none()))
    {
        let result =
            inspect(tool, root.as_deref(), args.uv.as_ref(), &cancelled);
        match result {
            Ok((path, version)) => writeln!(
                output,
                "{}: {} ({version})",
                name(tool),
                path.display()
            )
            .map_err(Error::Output)?,
            Err(Error::Cancelled) => return Err(Error::Cancelled),
            Err(error) if args.tool.is_some() => return Err(error),
            Err(error) => {
                writeln!(output, "{}: unavailable ({error})", name(tool))
                    .map_err(Error::Output)?
            }
        }
    }
    Ok(())
}

fn name(tool: Tool) -> &'static str {
    match tool {
        Tool::Uv => "uv",
        Tool::Ruff => "ruff",
        Tool::Ty => "ty",
        Tool::Pytest => "pytest",
    }
}

fn inspect(
    tool: Tool,
    root: Option<&Path>,
    explicit_uv: Option<&OsString>,
    cancelled: &AtomicBool,
) -> Result<(PathBuf, String)> {
    if matches!(tool, Tool::Uv) {
        let selected =
            explicit_uv.cloned().unwrap_or_else(|| OsString::from("uv"));
        return uv::Uv::inspect(selected.clone(), cancelled).map_err(|error| {
            if matches!(error, Error::Cancelled) {
                error
            } else {
                Error::ToolProbe {
                    tool: "uv",
                    path: PathBuf::from(selected),
                    source: Box::new(error),
                }
            }
        });
    }
    let tool_name = name(tool);
    let binary = resolve(tool_name, root)
        .map_err(|reason| Error::Tool { tool: tool_name, reason })?;
    let version =
        probe_version(&binary, tool_name, cancelled).map_err(|error| {
            if matches!(error, Error::Cancelled) {
                error
            } else {
                Error::ToolProbe {
                    tool: tool_name,
                    path: binary.clone(),
                    source: Box::new(error),
                }
            }
        })?;
    Ok((binary, version))
}

fn resolve(
    tool: &'static str,
    root: Option<&Path>,
) -> std::result::Result<PathBuf, String> {
    let mut directories = Vec::new();
    if let Some(root) = root {
        directories.push(root.join(".venv").join(scripts_directory()));
    }
    if let Some(active) =
        env::var_os("VIRTUAL_ENV").filter(|value| !value.is_empty())
    {
        directories.push(PathBuf::from(active).join(scripts_directory()));
    }
    for directory in directories {
        if let Ok(binary) = which::which(directory.join(tool)) {
            return native_binary(binary);
        }
    }
    which::which(tool)
        .map_err(|_| format!("not found; install {tool} in the project .venv, activate an environment, or add it to PATH"))
        .and_then(native_binary)
}

fn scripts_directory() -> &'static str {
    if cfg!(windows) { "Scripts" } else { "bin" }
}

fn native_binary(binary: PathBuf) -> std::result::Result<PathBuf, String> {
    let binary = fs::canonicalize(&binary).map_err(|error| {
        format!("could not resolve {}: {error}", binary.display())
    })?;
    #[cfg(windows)]
    if binary.extension().is_some_and(|extension| {
        extension.eq_ignore_ascii_case("cmd")
            || extension.eq_ignore_ascii_case("bat")
    }) {
        return Err(format!(
            "{} is a batch wrapper; use a native executable",
            binary.display()
        ));
    }
    Ok(binary)
}

fn probe_version(
    binary: &Path,
    tool: &str,
    cancelled: &AtomicBool,
) -> Result<String> {
    let output = tempfile::NamedTempFile::new().map_err(|error| {
        Error::io(
            "create tool version output",
            Path::new("temporary file"),
            error,
        )
    })?;
    let file = output
        .reopen()
        .map_err(|error| Error::io("open", output.path(), error))?;
    let stderr = tempfile::NamedTempFile::new().map_err(|error| {
        Error::io(
            "create tool error output",
            Path::new("temporary file"),
            error,
        )
    })?;
    let error_file = stderr
        .reopen()
        .map_err(|error| Error::io("open", stderr.path(), error))?;
    let mut command = Command::new(binary);
    let version_argument = if tool == "ty" { "version" } else { "--version" };
    command
        .arg(version_argument)
        .stdin(Stdio::null())
        .stdout(file)
        .stderr(error_file);
    let status = process::run(command, cancelled)?;
    if !status.success() {
        let detail = process::read_diagnostic(stderr.path())?;
        return Err(Error::Project(format!(
            "{version_argument} exited with {status}: {}",
            detail
        )));
    }
    let output = fs::read_to_string(output.path())
        .map_err(|error| Error::io("read", output.path(), error))?;
    let version = output.lines().next().unwrap_or_default().trim();
    let number = version
        .strip_prefix(tool)
        .and_then(|rest| rest.strip_prefix(' '))
        .and_then(|rest| rest.split_whitespace().next());
    let valid = number.is_some_and(|number| {
        if tool == "pytest" {
            number.parse::<pep440_rs::Version>().is_ok()
        } else {
            Version::parse(number).is_ok()
        }
    });
    if !valid {
        return Err(Error::Project(format!(
            "unexpected {version_argument} output: {version:?}"
        )));
    }
    Ok(version.to_owned())
}
