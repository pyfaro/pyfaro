use std::ffi::OsString;
use std::path::PathBuf;

use clap::{Args, Parser, Subcommand};

/// The unified Python development toolchain.
#[derive(Parser)]
#[command(version, about, long_about = None)]
#[command(arg_required_else_help = true)]
#[command(
    after_help = "Experimental alpha: only init is implemented; other workflow commands are not implemented yet."
)]
pub(crate) struct Cli {
    #[command(subcommand)]
    pub command: Command,
}

#[derive(Subcommand)]
pub(crate) enum Command {
    /// Preview or apply uv-backed project initialization
    Init(InitArgs),
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
