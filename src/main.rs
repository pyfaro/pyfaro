mod cli;
mod config;
mod create;
mod deps;
mod dev;
mod doctor;
mod error;
mod init;
mod process;
mod project;
mod python;
mod quality;
mod run;
mod sync;
mod tasks;
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
        cli::Command::Dev(args) => dev::run(args),
        cli::Command::Lint(args) => quality::lint(args),
        cli::Command::Fmt(args) => quality::fmt(args),
        cli::Command::Type(args) => quality::type_check(args),
        cli::Command::Test(args) => quality::test(args),
        cli::Command::Check(args) => quality::check(args),
        cli::Command::Python(args) => python::run(args),
        cli::Command::Tools(args) => tools::run(args),
        cli::Command::Doctor(args) => doctor::run(args),
    };
    if let Err(error) = result {
        if matches!(
            error,
            error::Error::CheckFailed | error::Error::Reported { .. }
        ) {
            std::process::exit(error.exit_code());
        }
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
