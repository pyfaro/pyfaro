use clap::Parser;

/// The unified Python development toolchain.
#[derive(Parser)]
#[command(version, about, long_about = None)]
#[command(arg_required_else_help = true)]
#[command(
    after_help = "Early development: workflow commands are not implemented yet."
)]
pub(crate) struct Cli {}
