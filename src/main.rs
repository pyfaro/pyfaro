mod cli;
mod error;
mod init;
mod process;

use clap::Parser;
use std::io::Write;

fn main() {
    let cli = cli::Cli::parse();
    let result = match cli.command {
        cli::Command::Init(args) => init::run(args),
    };
    if let Err(error) = result {
        // Diagnostics must not panic if stderr is closed or unavailable.
        let _ = writeln!(std::io::stderr().lock(), "error: {error}");
        std::process::exit(error.exit_code());
    }
}
