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
    #[error(
        "could not start uv at {path}: {source}; install uv or use --uv <path>"
    )]
    UvStart {
        path: PathBuf,
        #[source]
        source: io::Error,
    },
    #[error(
        "could not find uv at {path}: {source}; install uv or use --uv <path>"
    )]
    UvLookup {
        path: PathBuf,
        #[source]
        source: which::Error,
    },
    #[error("uv {operation} failed ({status}); see uv's output above")]
    UvStatus { operation: &'static str, status: ExitStatus },
    #[error("unsupported uv version: {0}; use uv >=0.9.26 (tested: 0.9.26)")]
    UvVersion(String),
    #[error("could not install the cancellation handler: {0}")]
    CancellationSetup(#[from] ctrlc::Error),
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
            Self::UvStatus { status, .. } => status.code().unwrap_or(1),
            Self::Cancelled => 130,
            _ => 1,
        }
    }
}

pub(crate) type Result<T> = std::result::Result<T, Error>;
