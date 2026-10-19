use std::ffi::OsString;
use std::path::PathBuf;

use clap::{Args, Parser, Subcommand};

/// The unified Python development toolchain.
#[derive(Parser)]
#[command(version, about, long_about = None)]
#[command(arg_required_else_help = true)]
#[command(
    after_help = "Experimental alpha: init, sync, run, add, remove, and tools are implemented; other workflow commands are not implemented yet."
)]
pub(crate) struct Cli {
    #[command(subcommand)]
    pub command: Command,
}

#[derive(Subcommand)]
pub(crate) enum Command {
    /// Preview or apply uv-backed project initialization
    Init(InitArgs),
    /// Synchronize the project environment through uv
    Sync(SyncArgs),
    /// Add requirements to the project through uv
    Add(DependencyArgs),
    /// Remove requirements from the project through uv
    Remove(DependencyArgs),
    /// Run a command in the project environment through uv
    Run(RunArgs),
    /// Show resolved paths and versions of external tools
    Tools(ToolsArgs),
}

#[derive(Args)]
pub(crate) struct DependencyArgs {
    /// Requirement specifiers for add; dependency names for remove
    #[arg(required = true, num_args = 1..)]
    pub packages: Vec<OsString>,

    /// Use the development dependency group
    #[arg(long, conflicts_with = "group")]
    pub dev: bool,

    /// Use a named dependency group
    #[arg(long)]
    pub group: Option<String>,

    /// Explicit project directory (otherwise discover from the current directory)
    #[arg(long)]
    pub project: Option<PathBuf>,

    /// uv executable (a path or a name found on PATH)
    #[arg(long, default_value = "uv")]
    pub uv: OsString,

    /// Disable network access in uv
    #[arg(long)]
    pub offline: bool,
}

#[derive(Args)]
pub(crate) struct ToolsArgs {
    /// Check one tool and fail if it is missing or incompatible
    #[arg(value_enum)]
    pub tool: Option<Tool>,

    /// Explicit project directory; otherwise discover the nearest project
    #[arg(long)]
    pub project: Option<PathBuf>,

    /// uv executable (a path or a name found on PATH)
    #[arg(long)]
    pub uv: Option<OsString>,
}

#[derive(Clone, Copy, clap::ValueEnum)]
#[value(rename_all = "lower")]
pub(crate) enum Tool {
    Uv,
    Ruff,
    Ty,
    Pytest,
}

#[derive(Args)]
pub(crate) struct InitArgs {
    /// Project directory (defaults to the current directory)
    #[arg(default_value = ".")]
    pub path: PathBuf,

    /// Apply the displayed plan; without this flag no files are written
    #[arg(long)]
    pub apply: bool,

    /// uv executable (a path or a name found on PATH)
    #[arg(long, default_value = "uv")]
    pub uv: OsString,

    /// Disable network access in uv
    #[arg(long)]
    pub offline: bool,
}

#[derive(Args)]
pub(crate) struct SyncArgs {
    /// Directory to start project discovery from (defaults to the current directory)
    #[arg(conflicts_with = "project")]
    pub path: Option<PathBuf>,

    /// Explicit project directory; overrides discovery
    #[arg(long)]
    pub project: Option<PathBuf>,

    /// Refuse to update an absent or stale uv.lock
    #[arg(long, conflicts_with = "frozen")]
    pub locked: bool,

    /// Use uv.lock without checking it against project metadata
    #[arg(long)]
    pub frozen: bool,

    /// uv executable (a path or a name found on PATH)
    #[arg(long, default_value = "uv")]
    pub uv: OsString,

    /// Disable network access in uv
    #[arg(long)]
    pub offline: bool,
}

#[derive(Args)]
pub(crate) struct RunArgs {
    /// Explicit project directory (otherwise discover from the current directory)
    #[arg(long)]
    pub project: Option<PathBuf>,

    /// Refuse to update an absent or stale uv.lock
    #[arg(long, conflicts_with = "frozen")]
    pub locked: bool,

    /// Use uv.lock without checking it against project metadata
    #[arg(long)]
    pub frozen: bool,

    /// uv executable (a path or a name found on PATH)
    #[arg(long, default_value = "uv")]
    pub uv: OsString,

    /// Disable network access in uv
    #[arg(long)]
    pub offline: bool,

    /// Command and arguments after --
    #[arg(last = true, required = true, num_args = 1..)]
    pub command: Vec<OsString>,
}
