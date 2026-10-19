mod cli;
mod config;
mod create;
mod deps;
mod error;
mod init;
mod process;
mod project;
mod run;
mod sync;
mod tools;
mod uv;

#[cfg(test)]
#[path = "../tests/support/mod.rs"]
mod test_support;

use clap::Parser;
use std::io::Write;

fn main() {
    let cli = cli::Cli::parse();
    let tools_command = matches!(cli.command, cli::Command::Tools(_));
    let result = match cli.command {
        cli::Command::Create(args) => create::run(args),
        cli::Command::Init(args) => init::run(args),
        cli::Command::Sync(args) => sync::run(args),
        cli::Command::Add(args) => deps::run(deps::Action::Add, args),
        cli::Command::Remove(args) => deps::run(deps::Action::Remove, args),
        cli::Command::Run(args) => run::run(args),
        cli::Command::Tools(args) => tools::run(args),
    };
    if let Err(error) = result {
        let diagnostic = format!("error: {error}");
        let diagnostic = if tools_command {
            tools::single_line(&diagnostic)
        } else {
            diagnostic
        };
        // Diagnostics must not panic if stderr is closed or unavailable.
        let _ = writeln!(std::io::stderr().lock(), "{diagnostic}");
        std::process::exit(error.exit_code());
    }
}
