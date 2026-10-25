//! Long help shown after the options. Every indented line that starts with
//! `pyfaro ` is a real invocation: the tests below parse each one with the
//! actual argument definitions so the examples cannot rot.

pub(super) const WORKFLOW: &str = "\
Workflow:
  pyfaro create fastapi acme-api   # then: cd acme-api
  pyfaro sync                      # build the environment
  pyfaro run -- python --version   # run a command in the environment
  pyfaro check --fix               # format, lint and type-check
  pyfaro test                      # run pytest
  pyfaro dev                       # serve the app with auto-reload

Shell completion: pyfaro completions --help
Documentation: https://pyfaro.dev/";

pub(super) const CREATE: &str = "\
Examples:
  pyfaro create lib acme-widget
  pyfaro create cli acme-tools --command acme
  pyfaro create fastapi acme-api --python 3.12";

pub(super) const SYNC: &str = "\
Examples:
  pyfaro sync
  pyfaro sync --locked
  pyfaro sync --project ./my-project --offline";

pub(super) const ADD: &str = "\
Examples:
  pyfaro add httpx
  pyfaro add --dev pytest-cov
  pyfaro add --group docs mkdocs";

pub(super) const RUN: &str = "\
Examples:
  pyfaro run -- python -m pytest
  pyfaro run lint-all                # a task from [tool.pyfaro.tasks]
  pyfaro run lint-all -- --verbose   # extra arguments for the task
  pyfaro run --locked -- python scripts/task.py";

pub(super) const DEV: &str = "\
Examples:
  pyfaro dev
  pyfaro dev --verbose
  pyfaro dev -- --port 9000
  pyfaro dev --project ./my-project --locked";

pub(super) const TEST: &str = "\
Examples:
  pyfaro test
  pyfaro test tests/test_cli.py
  pyfaro test -k parser -x
  pyfaro test --project ./my-project -q";

pub(super) const CHECK: &str = "\
Examples:
  pyfaro check
  pyfaro check --fix
  pyfaro check src tests
  pyfaro check --project ./my-project";

pub(super) const PYTHON: &str = "\
Examples:
  pyfaro python list --only-installed
  pyfaro python install 3.12
  pyfaro python pin 3.12
  pyfaro python pin --rm";

pub(super) const COMPLETIONS: &str = "\
Examples:
  pyfaro completions bash
  pyfaro completions zsh
  pyfaro completions fish
  pyfaro completions powershell

Where to save the script depends on the shell:
https://pyfaro.dev/commands/completions/";

#[cfg(test)]
mod tests {
    use clap::{CommandFactory, Parser};

    use crate::cli::Cli;

    /// Every `pyfaro ...` line in the help of `command` and its subcommands.
    fn examples(command: &clap::Command, found: &mut Vec<String>) {
        if let Some(help) = command.get_after_help() {
            for line in help.to_string().lines() {
                let line = line.trim();
                if line.starts_with("pyfaro ") {
                    let line = line.split(" #").next().unwrap_or(line);
                    found.push(line.trim().to_owned());
                }
            }
        }
        for subcommand in command.get_subcommands() {
            examples(subcommand, found);
        }
    }

    #[test]
    fn every_documented_example_parses() {
        let mut found = Vec::new();
        examples(&Cli::command(), &mut found);
        assert!(found.len() >= 30, "examples went missing: {found:?}");
        for example in found {
            let words = example.split_whitespace();
            if let Err(error) = Cli::try_parse_from(words) {
                panic!("documented example `{example}` is invalid: {error}");
            }
        }
    }

    #[test]
    fn the_key_commands_document_examples() {
        let root = Cli::command();
        for name in [
            "create",
            "sync",
            "add",
            "run",
            "dev",
            "test",
            "check",
            "python",
            "completions",
        ] {
            let command = root
                .find_subcommand(name)
                .unwrap_or_else(|| panic!("missing command {name}"));
            let mut found = Vec::new();
            examples(command, &mut found);
            assert!(!found.is_empty(), "{name} has no examples");
            assert!(
                found.iter().all(|line| line.contains(name)),
                "{name} examples must use {name}: {found:?}"
            );
        }
    }

    #[test]
    fn top_level_help_links_to_the_website() {
        let help = Cli::command().get_after_help().unwrap().to_string();
        assert!(help.contains("https://pyfaro.dev/"));
    }
}
