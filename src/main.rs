mod build;
mod cli;
mod completions;
mod config;
mod create;
mod deps;
mod dev;
#[cfg(test)]
mod docs_check;
mod doctor;
mod error;
mod init;
mod process;
mod project;
mod python;
mod quality;
mod redact;
mod run;
mod sync;
mod tasks;
mod tools;
mod uv;
mod verbose;

#[cfg(test)]
#[path = "../tests/support/mod.rs"]
mod test_support;

use clap::Parser;
use std::io::Write;

/// Parse the command line like `Cli::parse`, but never echo a credential:
/// Clap quotes the offending argument in its usage errors.
fn parse() -> cli::Cli {
    let error = match cli::Cli::try_parse() {
        Ok(cli) => return cli,
        Err(error) => error,
    };
    let message = error.to_string();
    let redacted = redact::text(&message);
    if redacted == message {
        error.exit();
    }
    let _ = write!(std::io::stderr().lock(), "{redacted}");
    std::process::exit(error.exit_code());
}

fn main() {
    let cli = parse();
    if cli.verbose || cli.command.verbose() {
        verbose::enable(cli.command.name());
    }
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
        cli::Command::Build(args) => build::run(args),
        cli::Command::Tools(args) => tools::run(args),
        cli::Command::Doctor(args) => doctor::run(args),
        cli::Command::Completions(args) => completions::run(args),
    };
    if let Err(error) = result {
        if matches!(
            error,
            error::Error::CheckFailed | error::Error::Reported { .. }
        ) {
            std::process::exit(error.exit_code());
        }
        let diagnostic = redact::text(&format!("error: {error}"));
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
