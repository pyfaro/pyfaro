use std::io::Write;

use clap::CommandFactory;
use clap_complete::{Shell, generate};

use crate::cli::{Cli, CompletionShell, CompletionsArgs};
use crate::error::{Error, Result};

pub(crate) fn run(args: CompletionsArgs) -> Result<()> {
    let script = script(args.shell);
    std::io::stdout().lock().write_all(&script).map_err(Error::Output)
}

/// The static completion script for `shell`, generated from the CLI definition.
fn script(shell: CompletionShell) -> Vec<u8> {
    let shell = match shell {
        CompletionShell::Bash => Shell::Bash,
        CompletionShell::Zsh => Shell::Zsh,
        CompletionShell::Fish => Shell::Fish,
        CompletionShell::Powershell => Shell::PowerShell,
    };
    let mut script = Vec::new();
    let mut command = Cli::command();
    generate(shell, &mut command, "pyfaro", &mut script);
    script
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn every_script_names_the_subcommands() {
        for shell in [
            CompletionShell::Bash,
            CompletionShell::Zsh,
            CompletionShell::Fish,
            CompletionShell::Powershell,
        ] {
            let script = String::from_utf8(script(shell)).unwrap();
            for command in ["create", "sync", "run", "completions"] {
                assert!(script.contains(command), "{command} is missing");
            }
        }
    }
}
