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
            Ok((path, version)) => {
                let report =
                    format!("{}: {} ({version})", name(tool), path.display());
                writeln!(output, "{}", single_line(&report))
                    .map_err(Error::Output)?
            }
            Err(Error::Cancelled) => return Err(Error::Cancelled),
            Err(error) if args.tool.is_some() => return Err(error),
            Err(error) => {
                let report = format!("{}: unavailable ({error})", name(tool));
                writeln!(output, "{}", single_line(&report))
                    .map_err(Error::Output)?
            }
        }
    }
    Ok(())
}

pub(crate) fn single_line(value: &str) -> String {
    let mut escaped = String::new();
    for character in value.chars() {
        match character {
            '\n' => escaped.push_str("\\n"),
            '\r' => escaped.push_str("\\r"),
            '\t' => escaped.push_str("\\t"),
            character if character.is_control() => {
                escaped.push_str(&format!("\\u{{{:x}}}", character as u32));
            }
            character => escaped.push(character),
        }
    }
    escaped
}

pub(crate) fn name(tool: Tool) -> &'static str {
    match tool {
        Tool::Uv => "uv",
        Tool::Ruff => "ruff",
        Tool::Ty => "ty",
        Tool::Pytest => "pytest",
    }
}

pub(crate) fn inspect(
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
    let binary = resolve(tool_name, root)?;
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

pub(crate) fn inspect_python(
    root: Option<&Path>,
    cancelled: &AtomicBool,
) -> Result<(PathBuf, String)> {
    let binary = match resolve("python", root) {
        Ok(binary) => binary,
        Err(Error::Tool { .. }) if !cfg!(windows) => {
            resolve("python3", root).map_err(|error| match error {
                Error::Tool { .. } => Error::Tool {
                    tool: "python",
                    reason: "not found; install Python or make it available in the project environment (.venv by default), active environment, or PATH".into(),
                },
                error => error,
            })?
        }
        Err(error) => return Err(error),
    };
    let version =
        probe_version(&binary, "python", cancelled).map_err(|error| {
            if matches!(error, Error::Cancelled) {
                error
            } else {
                Error::ToolProbe {
                    tool: "python",
                    path: binary.clone(),
                    source: Box::new(error),
                }
            }
        })?;
    Ok((binary, version))
}

fn resolve(tool: &'static str, root: Option<&Path>) -> Result<PathBuf> {
    let mut directories = Vec::new();
    if let Some(root) = root {
        directories.push(project_environment(root).join(scripts_directory()));
    }
    if let Some(active) =
        env::var_os("VIRTUAL_ENV").filter(|value| !value.is_empty())
    {
        directories.push(PathBuf::from(active).join(scripts_directory()));
    }
    for directory in directories {
        let names = if tool == "python" && !cfg!(windows) {
            &["python", "python3"][..]
        } else {
            &[tool][..]
        };
        for name in names {
            let candidate = directory.join(name);
            match which::which(&candidate) {
                Ok(binary) => return native_binary(tool, binary),
                Err(which::Error::CannotFindBinaryPath) => {}
                Err(source) => {
                    return Err(lookup_failure(tool, candidate, source));
                }
            }
        }
    }
    which::which(tool)
        .map_err(|source| lookup_failure(tool, PathBuf::from(tool), source))
        .and_then(|binary| native_binary(tool, binary))
}

pub(crate) fn project_environment(root: &Path) -> PathBuf {
    match env::var_os("UV_PROJECT_ENVIRONMENT")
        .filter(|value| !value.is_empty())
    {
        Some(value) => {
            let path = PathBuf::from(value);
            if path.is_absolute() { path } else { root.join(path) }
        }
        None => root.join(".venv"),
    }
}

pub(crate) fn project_python(root: &Path) -> PathBuf {
    let scripts = project_environment(root).join(scripts_directory());
    let python =
        scripts.join(if cfg!(windows) { "python.exe" } else { "python" });
    if !cfg!(windows) && !python.is_file() {
        let python3 = scripts.join("python3");
        if python3.is_file() {
            return python3;
        }
    }
    python
}

fn lookup_failure(
    tool: &'static str,
    path: PathBuf,
    source: which::Error,
) -> Error {
    match source {
        which::Error::CannotFindBinaryPath => Error::Tool {
            tool,
            reason: format!(
                "not found; install {tool} in the project environment (.venv by default), activate an environment, or add it to PATH"
            ),
        },
        source => Error::ToolLookup { tool, path, source },
    }
}

fn scripts_directory() -> &'static str {
    if cfg!(windows) { "Scripts" } else { "bin" }
}

fn native_binary(tool: &'static str, binary: PathBuf) -> Result<PathBuf> {
    let binary =
        fs::canonicalize(&binary).map_err(|source| Error::ToolProbe {
            tool,
            path: binary.clone(),
            source: Box::new(Error::io("resolve", &binary, source)),
        })?;
    #[cfg(windows)]
    if binary.extension().is_some_and(|extension| {
        extension.eq_ignore_ascii_case("cmd")
            || extension.eq_ignore_ascii_case("bat")
    }) {
        return Err(Error::Tool {
            tool,
            reason: format!(
                "{} is a batch wrapper; use a native executable",
                binary.display()
            ),
        });
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
    let label = if tool == "python" { "Python" } else { tool };
    let number = version
        .strip_prefix(label)
        .and_then(|rest| rest.strip_prefix(' '))
        .and_then(|rest| rest.split_whitespace().next());
    let valid = number.is_some_and(|number| {
        if tool == "pytest" || tool == "python" {
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn non_missing_lookup_failure_keeps_its_source() {
        let error = lookup_failure(
            "ruff",
            PathBuf::from("ruff"),
            which::Error::CannotCanonicalize,
        );
        assert!(matches!(
            error,
            Error::ToolLookup { source: which::Error::CannotCanonicalize, .. }
        ));
    }
}
