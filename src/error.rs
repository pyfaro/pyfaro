use std::fmt;
use std::io;
use std::path::PathBuf;
use std::process::ExitStatus;

#[derive(Debug, thiserror::Error)]
pub(crate) enum Error {
    #[error("could not {action} {path}: {source}")]
    Io {
        action: &'static str,
        path: PathBuf,
        #[source]
        source: io::Error,
    },
    #[error("invalid pyproject.toml: {0}")]
    Toml(#[from] toml_edit::TomlError),
    #[error("invalid {path}: {source}")]
    ConfigToml {
        path: PathBuf,
        #[source]
        source: toml_edit::TomlError,
    },
    #[error("invalid {path}: {source}")]
    Config {
        path: PathBuf,
        #[source]
        source: Box<Error>,
    },
    #[error("{0}")]
    Project(String),
    /// A command line that clap accepted but Pyfaro cannot act on.
    #[error("{0}")]
    Usage(String),
    #[error(
        "doctor found {count} blocking issue(s); follow the actions above and run pyfaro doctor again"
    )]
    DoctorFailed { count: usize },
    #[error("no pyproject.toml found from {}; use --project <PATH>", directory.display())]
    NoProject { directory: PathBuf },
    #[error(
        "could not start executable at {path}: {source}; check installation and executable permissions"
    )]
    ProcessStart {
        path: PathBuf,
        #[source]
        source: io::Error,
    },
    #[error("{}", uv_lookup_message(.path, .source))]
    UvLookup {
        path: PathBuf,
        #[source]
        source: which::Error,
    },
    #[error("uv {operation} failed ({status}); see uv's output above")]
    UvStatus { operation: &'static str, status: ExitStatus },
    #[error("{tool} failed ({status}); see {tool}'s output above")]
    ToolStatus { tool: &'static str, status: ExitStatus },
    #[error(
        "uv --version failed for {path} ({status}): {stderr}; reinstall uv (https://docs.astral.sh/uv/getting-started/installation/) or select another with --uv <path>"
    )]
    UvProbe { path: PathBuf, status: ExitStatus, stderr: String },
    #[error(
        "unsupported uv version: {0}; use uv >=0.9.26 (tested: 0.9.26) by running `uv self update`, reinstalling uv (https://docs.astral.sh/uv/getting-started/installation/) or selecting another with --uv <path>"
    )]
    UvVersion(String),
    #[error("{tool}: {reason}")]
    Tool { tool: &'static str, reason: String },
    #[error(
        "could not locate {tool} at {path}: {source}; check the selected environment"
    )]
    ToolLookup {
        tool: &'static str,
        path: PathBuf,
        #[source]
        source: which::Error,
    },
    #[error(
        "could not inspect {tool} at {path}: {source}; repair or replace the selected installation"
    )]
    ToolProbe {
        tool: &'static str,
        path: PathBuf,
        #[source]
        source: Box<Error>,
    },
    #[error("could not install the cancellation handler: {0}")]
    CancellationSetup(#[from] ctrlc::Error),
    /// `check` already printed its summary; main prints nothing more.
    #[error("check failed")]
    CheckFailed,
    /// The command already printed its diagnostic; main prints nothing more
    /// and exits with `code`.
    #[error("already reported")]
    Reported { code: i32 },
    #[error("operation cancelled")]
    Cancelled,
    #[error("could not write command output: {0}")]
    Output(#[source] io::Error),
    #[error("uv process cleanup failed: {0}")]
    ProcessCleanup(#[source] Failures),
    #[error(
        "{0}; project files were left as-is because process cleanup could not be confirmed; inspect the project before retrying"
    )]
    RecoverySkipped(#[source] Box<Error>),
    #[error("{operation}; {source}")]
    OperationCleanup {
        operation: Box<Error>,
        #[source]
        source: Box<Error>,
    },
    #[error(
        "{operation}; could not restore project files: {source}; inspect pyproject.toml and uv.lock before retrying"
    )]
    Rollback {
        operation: Box<Error>,
        #[source]
        source: Failures,
    },
}

const UV_INSTALL_DOCS: &str =
    "https://docs.astral.sh/uv/getting-started/installation/";

/// The one-line remedy for a uv that cannot be found: where to get it, and
/// how to point Pyfaro at an existing copy.
fn uv_lookup_message(path: &std::path::Path, source: &which::Error) -> String {
    let install = if cfg!(windows) {
        r#"powershell -ExecutionPolicy ByPass -c "irm https://astral.sh/uv/install.ps1 | iex""#
    } else {
        "curl -LsSf https://astral.sh/uv/install.sh | sh"
    };
    // `which` explains only the unusual failures; "cannot find binary path"
    // adds nothing to "could not find".
    let detail = match source {
        which::Error::CannotFindBinaryPath => String::new(),
        other => format!(" ({other})"),
    };
    let install = format!("install uv ({UV_INSTALL_DOCS}, or `{install}`)");
    if path == std::path::Path::new("uv") {
        format!(
            "could not find uv on PATH{detail}; {install} or pass --uv <path>"
        )
    } else {
        format!(
            "could not find uv at {}{detail}; fix --uv <path> or {install}",
            path.display()
        )
    }
}

/// Preserve every recovery failure, including when both files cannot be restored.
#[derive(Debug)]
pub(crate) struct Failures(pub Vec<Error>);

impl fmt::Display for Failures {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        for (index, error) in self.0.iter().enumerate() {
            if index != 0 {
                formatter.write_str("; ")?;
            }
            write!(formatter, "{error}")?;
        }
        Ok(())
    }
}

impl std::error::Error for Failures {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        self.0.first().map(|error| error as &dyn std::error::Error)
    }
}

impl Error {
    pub fn cleanup_incomplete(&self) -> bool {
        match self {
            Self::ProcessCleanup(_) => true,
            Self::OperationCleanup { source, .. } => {
                source.cleanup_incomplete()
            }
            _ => false,
        }
    }
    pub fn io(
        action: &'static str,
        path: &std::path::Path,
        source: io::Error,
    ) -> Self {
        Self::Io { action, path: path.to_owned(), source }
    }

    pub fn config(path: &std::path::Path, source: Error) -> Self {
        Self::Config { path: path.to_owned(), source: Box::new(source) }
    }

    pub fn exit_code(&self) -> i32 {
        match self {
            Self::UvStatus { status, .. }
            | Self::UvProbe { status, .. }
            | Self::ToolStatus { status, .. } => status.code().unwrap_or(1),
            Self::Reported { code } => *code,
            Self::Cancelled => 130,
            Self::Usage(_) => 2,
            _ => 1,
        }
    }
}

pub(crate) type Result<T> = std::result::Result<T, Error>;
