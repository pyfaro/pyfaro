use std::ffi::OsString;
use std::path::PathBuf;

use clap::{Args, Parser, Subcommand};

/// The unified Python development toolchain.
#[derive(Parser)]
#[command(version, about, long_about = None)]
#[command(arg_required_else_help = true)]
#[command(
    after_help = "Experimental alpha: create, init, sync, run, dev, add, remove, python, tools, doctor, lint, fmt, type, test, and check are implemented; other workflow commands are not implemented yet."
)]
pub(crate) struct Cli {
    /// Print tool versions and every command line before it runs
    #[arg(short, long)]
    pub verbose: bool,

    #[command(subcommand)]
    pub command: Command,
}

#[derive(Subcommand)]
pub(crate) enum Command {
    /// Create a packaged Python library, command-line or FastAPI project
    Create(CreateArgs),
    /// Preview or apply uv-backed project initialization
    Init(InitArgs),
    /// Synchronize the project environment through uv
    Sync(SyncArgs),
    /// Add requirements to the project through uv
    Add(DependencyArgs),
    /// Remove requirements from the project through uv
    Remove(DependencyArgs),
    /// Run a named task or a command in the project environment through uv
    Run(RunArgs),
    /// Run the project's development server or process with supervision
    Dev(DevArgs),
    /// Lint project files with Ruff
    Lint(LintArgs),
    /// Format project files with Ruff
    Fmt(FmtArgs),
    /// Type-check project files with ty
    Type(TypeArgs),
    /// Run pytest in the project environment
    Test(TestArgs),
    /// Run format check, lint, and type check; optionally apply safe fixes first
    Check(CheckArgs),
    /// List, install, or pin Python versions through uv
    Python(PythonArgs),
    /// Show resolved paths and versions of external tools
    Tools(ToolsArgs),
    /// Diagnose project configuration, environment, and tool availability
    Doctor(DoctorArgs),
}

impl Command {
    /// Whether `-v`/`--verbose` followed the subcommand. `test` has none: its
    /// arguments, those two included, belong to pytest.
    pub(crate) fn verbose(&self) -> bool {
        match self {
            Self::Create(args) => args.verbose,
            Self::Init(args) => args.verbose.verbose,
            Self::Sync(args) => args.verbose.verbose,
            Self::Add(args) | Self::Remove(args) => args.verbose.verbose,
            Self::Run(args) => args.verbose.verbose,
            Self::Dev(args) => args.verbose.verbose,
            Self::Lint(args) => args.verbose.verbose,
            Self::Fmt(args) => args.verbose.verbose,
            Self::Type(args) => args.verbose.verbose,
            Self::Test(_) => false,
            Self::Check(args) => args.verbose.verbose,
            Self::Python(args) => args.verbose,
            Self::Tools(args) => args.verbose.verbose,
            Self::Doctor(args) => args.verbose.verbose,
        }
    }

    /// The name typed on the command line, as in `pyfaro <name>:` lines.
    pub(crate) fn name(&self) -> &'static str {
        match self {
            Self::Create(_) => "create",
            Self::Init(_) => "init",
            Self::Sync(_) => "sync",
            Self::Add(_) => "add",
            Self::Remove(_) => "remove",
            Self::Run(_) => "run",
            Self::Dev(_) => "dev",
            Self::Lint(_) => "lint",
            Self::Fmt(_) => "fmt",
            Self::Type(_) => "type",
            Self::Test(_) => "test",
            Self::Check(_) => "check",
            Self::Python(_) => "python",
            Self::Tools(_) => "tools",
            Self::Doctor(_) => "doctor",
        }
    }
}

/// `-v`/`--verbose` after the subcommand. Not global: `test` has none, so its
/// arguments, `-v` and `--verbose` included, all reach pytest.
#[derive(Args)]
pub(crate) struct VerboseArgs {
    /// Print tool versions and every command line before it runs
    #[arg(short, long)]
    pub verbose: bool,
}

#[derive(Args)]
pub(crate) struct LintArgs {
    /// Files or directories relative to the current directory (default: the project)
    pub paths: Vec<PathBuf>,

    /// Explicit project directory; otherwise discover the nearest project
    #[arg(long)]
    pub project: Option<PathBuf>,
    #[command(flatten)]
    pub verbose: VerboseArgs,
}

#[derive(Args)]
pub(crate) struct TypeArgs {
    /// Files or directories relative to the current directory (default: the project)
    pub paths: Vec<PathBuf>,

    /// Explicit project directory; otherwise discover the nearest project
    #[arg(long)]
    pub project: Option<PathBuf>,
    #[command(flatten)]
    pub verbose: VerboseArgs,
}

#[derive(Args)]
pub(crate) struct CheckArgs {
    /// Files or directories relative to the current directory (default: the project)
    pub paths: Vec<PathBuf>,

    /// Apply safe Ruff fixes and formatting first, then verify without writing
    #[arg(long)]
    pub fix: bool,

    /// Explicit project directory; otherwise discover the nearest project
    #[arg(long)]
    pub project: Option<PathBuf>,
    #[command(flatten)]
    pub verbose: VerboseArgs,
}

#[derive(Args)]
pub(crate) struct FmtArgs {
    /// Files or directories relative to the current directory (default: the project)
    pub paths: Vec<PathBuf>,

    /// Report files that would change without modifying them
    #[arg(long)]
    pub check: bool,

    /// Explicit project directory; otherwise discover the nearest project
    #[arg(long)]
    pub project: Option<PathBuf>,
    #[command(flatten)]
    pub verbose: VerboseArgs,
}

#[derive(Args)]
pub(crate) struct TestArgs {
    /// Explicit project directory; otherwise discover the nearest project
    #[arg(long)]
    pub project: Option<PathBuf>,

    /// Arguments forwarded verbatim to pytest (paths are relative to the project root)
    #[arg(trailing_var_arg = true, allow_hyphen_values = true, num_args = 0..)]
    pub pytest_args: Vec<OsString>,
}

#[derive(Args)]
pub(crate) struct DoctorArgs {
    /// Explicit project directory; otherwise discover the nearest project
    #[arg(long)]
    pub project: Option<PathBuf>,

    /// uv executable (a path or a name found on PATH)
    #[arg(long)]
    pub uv: Option<OsString>,
    #[command(flatten)]
    pub verbose: VerboseArgs,
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
    #[command(flatten)]
    pub verbose: VerboseArgs,
}

#[derive(Args)]
pub(crate) struct CreateArgs {
    #[command(subcommand)]
    pub kind: CreateKind,
    /// Print tool versions and every command line before it runs
    #[arg(short, long, global = true)]
    pub verbose: bool,
}

#[derive(Subcommand)]
pub(crate) enum CreateKind {
    /// Create a src-layout library
    Lib(CreateProjectArgs),
    /// Create an installable command-line application
    Cli(CreateCliArgs),
    /// Create a FastAPI application with tests
    Fastapi(CreateFastApiArgs),
}

#[derive(Args)]
pub(crate) struct CreateProjectArgs {
    /// Distribution name (also the default target directory name)
    pub name: String,

    /// Target directory; its parent must already exist
    #[arg(long)]
    pub path: Option<PathBuf>,

    /// Minimum Python version as MAJOR.MINOR
    #[arg(long, default_value = "3.10")]
    pub python: String,
}

#[derive(Args)]
pub(crate) struct CreateCliArgs {
    #[command(flatten)]
    pub project: CreateProjectArgs,

    /// Import package name (defaults to the normalized distribution name)
    #[arg(long)]
    pub package: Option<String>,

    /// Installed command name (defaults to the distribution name with dots replaced by hyphens)
    #[arg(long)]
    pub command: Option<String>,
}

#[derive(Args)]
pub(crate) struct CreateFastApiArgs {
    #[command(flatten)]
    pub project: CreateProjectArgs,

    /// Import package name (defaults to the normalized distribution name)
    #[arg(long)]
    pub package: Option<String>,
}

#[derive(Args)]
pub(crate) struct PythonArgs {
    #[command(subcommand)]
    pub command: PythonCommand,

    /// Exact project directory; otherwise use the current directory
    #[arg(long, global = true)]
    pub project: Option<PathBuf>,

    /// uv executable (a path or a name found on PATH)
    #[arg(long, global = true, default_value = "uv")]
    pub uv: OsString,

    /// Disable network access in uv
    #[arg(long, global = true)]
    pub offline: bool,
    /// Print tool versions and every command line before it runs
    #[arg(short, long, global = true)]
    pub verbose: bool,
}

#[derive(Subcommand)]
pub(crate) enum PythonCommand {
    /// List installed and available Python versions
    List {
        /// Python request to filter by
        request: Option<OsString>,
        /// Show installed versions only
        #[arg(long)]
        only_installed: bool,
        /// Include old patch versions
        #[arg(long)]
        all_versions: bool,
    },
    /// Explicitly download and install Python versions
    Install {
        /// Python version requests; without targets uv uses .python-version
        targets: Vec<OsString>,
    },
    /// Write or display the local .python-version pin
    Pin {
        /// Python version request; omit to display the current pin
        request: Option<OsString>,
        /// Remove the local pin
        #[arg(long, conflicts_with = "request")]
        rm: bool,
    },
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
    #[command(flatten)]
    pub verbose: VerboseArgs,
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
    #[command(flatten)]
    pub verbose: VerboseArgs,
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
    #[command(flatten)]
    pub verbose: VerboseArgs,
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

    /// Task defined in [tool.pyfaro.tasks]; omit it to run a program after --
    pub task: Option<OsString>,

    /// Program and arguments after --, or extra arguments for the task
    #[arg(last = true, num_args = 0.., value_name = "ARGS")]
    pub command: Vec<OsString>,
    #[command(flatten)]
    pub verbose: VerboseArgs,
}

#[derive(Args)]
pub(crate) struct DevArgs {
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

    /// Extra arguments after --, appended after the configured arguments
    #[arg(last = true, num_args = 0..)]
    pub extra: Vec<OsString>,
    #[command(flatten)]
    pub verbose: VerboseArgs,
}
