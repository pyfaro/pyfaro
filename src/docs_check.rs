//! Tests that keep the reference documentation in step with the code.
//!
//! `website/docs/reference/cli.md` is checked against the Clap definitions:
//! the `pyfaro --help` block must equal the real output, and every command
//! and subcommand needs a `## `pyfaro <path>`` (or `###`) heading with its
//! exact usage line, one table row per argument and flag with the help text
//! Clap prints, no row for anything that no longer exists, and an
//! `Exit codes` line. Every `toml` block in `docs/` and `website/docs/` that
//! mentions `tool.pyfaro` must be accepted by Pyfaro's own configuration
//! validation.
//!
//! To update after a CLI change: edit the doc comment or flag, then run
//! `cargo test docs_check` and fix what the failure messages list. A table
//! row is `` | `<key>` | <help text> | ``; the cell may add backticks and a
//! trailing remark such as ``(default `3.10`)``. A TOML example that is
//! meant to be rejected starts with `# invalid: <part of the error>`.

use std::fs;
use std::path::{Path, PathBuf};

use clap::{Arg, Command, CommandFactory};

use crate::cli::Cli;
use crate::config::Config;

fn repo_file(relative: &str) -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join(relative)
}

/// The text of a documentation file with `\n` line endings. `None` when the
/// website sources are absent, as in an unpacked crate.
fn read(relative: &str) -> Option<String> {
    if !repo_file("website").is_dir() {
        eprintln!("skipped: the documentation sources are not available");
        return None;
    }
    let text = fs::read_to_string(repo_file(relative))
        .unwrap_or_else(|error| panic!("cannot read {relative}: {error}"));
    Some(text.replace("\r\n", "\n"))
}

const CLI_REFERENCE: &str = "website/docs/reference/cli.md";

// ---------------------------------------------------------------------------
// reference/cli.md

#[test]
fn the_top_level_help_block_is_the_real_help() {
    let Some(page) = read(CLI_REFERENCE) else { return };
    let start = page
        .find("```console\n$ pyfaro --help\n")
        .expect("cli.md needs a console block starting with `$ pyfaro --help`");
    let block = &page[start + "```console\n$ pyfaro --help\n".len()..];
    let block = &block[..block.find("\n```").expect("unterminated block")];
    let real = Cli::command().render_help().to_string();
    assert_eq!(
        block.trim_end(),
        real.trim_end(),
        "the `pyfaro --help` block of {CLI_REFERENCE} is out of date; \
         paste the output of `cargo run -- --help`"
    );
}

/// `(path, command)` for every command below the root, e.g. `create lib`.
fn commands(root: &Command) -> Vec<(String, &Command)> {
    fn walk<'a>(
        prefix: &str,
        command: &'a Command,
        found: &mut Vec<(String, &'a Command)>,
    ) {
        for sub in command.get_subcommands() {
            if sub.get_name() == "help" {
                continue;
            }
            let path = if prefix.is_empty() {
                sub.get_name().to_owned()
            } else {
                format!("{prefix} {}", sub.get_name())
            };
            found.push((path.clone(), sub));
            walk(&path, sub, found);
        }
    }
    let mut found = Vec::new();
    walk("", root, &mut found);
    found
}

/// The lines of the section whose heading is `` `pyfaro <path>` ``, up to the
/// next heading of any level.
fn section<'a>(page: &'a str, path: &str) -> Option<Vec<&'a str>> {
    let heading = format!("`pyfaro {path}`");
    let mut lines = page.lines().skip_while(|line| {
        !(line.starts_with('#')
            && line.trim_start_matches('#').trim() == heading)
    });
    lines.next()?;
    Some(lines.take_while(|line| !line.starts_with('#')).collect())
}

/// How a table row names an argument: `<NAME>`, `[PATH]`, `--uv <UV>` or
/// `-v`, `--verbose`.
fn row_key(arg: &Arg) -> String {
    if arg.is_positional() {
        return format!("`{arg}`");
    }
    let long = arg.get_long().expect("every option has a long name");
    let mut key = String::new();
    if let Some(short) = arg.get_short() {
        key.push_str(&format!("`-{short}`, "));
    }
    key.push_str(&format!("`--{long}"));
    if arg.get_action().takes_values() {
        let names = arg.get_value_names().expect("an option with a value");
        key.push_str(&format!(" <{}>", names[0]));
    }
    key.push('`');
    key
}

/// `(key, description)` of every `` | `key` | description | `` row.
fn rows<'a>(section: &[&'a str]) -> Vec<(String, &'a str)> {
    section
        .iter()
        .filter_map(|line| {
            let line = line.strip_prefix("| ")?.strip_suffix(" |")?;
            let (key, description) = line.split_once(" | ")?;
            // `-v`, `--verbose` contains a separator of its own.
            Some((key.to_owned(), description))
        })
        .filter(|(key, _)| key.starts_with('`'))
        .collect()
}

#[test]
fn every_command_is_documented_in_the_cli_reference() {
    let Some(page) = read(CLI_REFERENCE) else { return };
    let mut root = Cli::command();
    root.build();
    let mut problems = Vec::new();
    for (path, command) in commands(&root) {
        let Some(section) = section(&page, &path) else {
            problems.push(format!("`pyfaro {path}` has no heading"));
            continue;
        };
        let text = section.join("\n");
        let usage = command.clone().render_usage().to_string();
        if !text.contains(&usage) {
            problems.push(format!("`pyfaro {path}`: missing `{usage}`"));
        }
        let leaf = command.get_subcommands().next().is_none();
        if leaf && !text.contains("Exit codes") {
            problems.push(format!("`pyfaro {path}`: no `Exit codes` line"));
        }
        let documented = rows(&section);
        let mut known = Vec::new();
        for arg in command.get_arguments() {
            if matches!(arg.get_id().as_str(), "help" | "version") {
                continue;
            }
            let key = row_key(arg);
            known.push(key.clone());
            // A leaf shows the flags it inherits from its parent (such as
            // `python --project`) in the parent's table only, but may repeat
            // them.
            let inherited = leaf && arg.is_global_set();
            match documented.iter().find(|(documented, _)| *documented == key) {
                None if inherited => {}
                None => {
                    problems.push(format!("`pyfaro {path}`: no row for {key}"))
                }
                Some((_, description)) => {
                    let help = arg.get_help().map(|h| h.to_string());
                    let help = help.unwrap_or_default();
                    if !description.replace('`', "").starts_with(&help) {
                        problems.push(format!(
                            "`pyfaro {path}`: the row for {key} should \
                             start with: {help}"
                        ));
                    }
                }
            }
        }
        for (key, _) in &documented {
            if !known.contains(key) {
                problems.push(format!(
                    "`pyfaro {path}`: {key} is documented but does not exist"
                ));
            }
        }
    }
    assert!(
        problems.is_empty(),
        "{CLI_REFERENCE} is out of step with the CLI (see the comment at the \
         top of src/docs_check.rs):\n{}",
        problems.join("\n")
    );
}

#[test]
fn the_cli_reference_explains_the_global_options() {
    let Some(page) = read(CLI_REFERENCE) else { return };
    assert!(
        page.lines().any(|line| line == "## Global options"),
        "cli.md needs a `## Global options` section"
    );
    for needle in ["`-v`, `--verbose`", "`-h`, `--help`", "`-V`, `--version`"] {
        assert!(page.contains(needle), "cli.md does not mention {needle}");
    }
    assert!(
        page.contains("pyfaro -v test"),
        "cli.md must explain that `test` forwards -v to pytest"
    );
}

// ---------------------------------------------------------------------------
// TOML examples

/// Every `toml` fenced block of `text`, de-indented.
fn toml_blocks(text: &str) -> Vec<String> {
    let mut blocks = Vec::new();
    let mut current: Option<(usize, Vec<&str>)> = None;
    for line in text.lines() {
        let trimmed = line.trim_start();
        match &mut current {
            None if trimmed.starts_with("```toml") => {
                current = Some((line.len() - trimmed.len(), Vec::new()));
            }
            Some((indent, lines)) => {
                if trimmed.starts_with("```") {
                    blocks.push(lines.join("\n"));
                    current = None;
                } else {
                    lines.push(line.get(*indent..).unwrap_or(trimmed));
                }
            }
            None => {}
        }
    }
    blocks
}

fn markdown_files(directory: &Path, found: &mut Vec<PathBuf>) {
    let Ok(entries) = fs::read_dir(directory) else { return };
    for entry in entries.flatten() {
        let path = entry.path();
        if path.is_dir() {
            if path.file_name().is_some_and(|name| name != "site") {
                markdown_files(&path, found);
            }
        } else if path.extension().is_some_and(|ext| ext == "md") {
            found.push(path);
        }
    }
}

#[test]
fn every_pyfaro_toml_example_passes_the_configuration_validation() {
    if read(CLI_REFERENCE).is_none() {
        return;
    }
    let mut pages = vec![repo_file("README.md")];
    markdown_files(&repo_file("docs"), &mut pages);
    markdown_files(&repo_file("website/docs"), &mut pages);
    let (mut checked, mut problems) = (0, Vec::new());
    for page in pages {
        let text = fs::read_to_string(&page).unwrap().replace("\r\n", "\n");
        for block in toml_blocks(&text) {
            if !block.contains("tool.pyfaro") {
                continue;
            }
            checked += 1;
            let name = page.strip_prefix(repo_file("")).unwrap().display();
            if let Err(problem) = validate_example(&block) {
                problems.push(format!("{name}:\n{block}\n=> {problem}"));
            }
        }
    }
    assert!(checked >= 10, "only {checked} examples found");
    assert!(
        problems.is_empty(),
        "invalid [tool.pyfaro] examples (mark an intentionally invalid one \
         with `# invalid: <part of the error>`):\n\n{}",
        problems.join("\n\n")
    );
}

/// Write `block` as a `pyproject.toml` (with a `[project]` table when the
/// block has none) and load it like every command does.
fn validate_example(block: &str) -> Result<(), String> {
    let expected = block.lines().next().and_then(|line| {
        line.strip_prefix("# invalid:").map(|rest| rest.trim().to_owned())
    });
    let directory = tempfile::tempdir().unwrap();
    let header = if block.contains("[project]") {
        ""
    } else {
        "[project]\nname = \"demo\"\nversion = \"0.1.0\"\n\n"
    };
    // A block that starts below `[project]` (`dependencies = ...`) continues
    // the table the header opens.
    fs::write(
        directory.path().join("pyproject.toml"),
        format!("{header}{block}\n"),
    )
    .unwrap();
    let loaded = Config::load(directory.path());
    match (loaded, expected) {
        (Ok(_), None) => Ok(()),
        (Err(error), None) => Err(error.to_string()),
        (Ok(_), Some(_)) => Err("accepted, but marked invalid".into()),
        (Err(error), Some(part)) if error.to_string().contains(&part) => Ok(()),
        (Err(error), Some(part)) => {
            Err(format!("rejected with `{error}`, expected `{part}`"))
        }
    }
}

#[test]
fn the_example_checker_accepts_and_rejects() {
    assert!(validate_example("[tool.pyfaro]\nschema-version = 1").is_ok());
    assert!(
        validate_example("[tool.pyfaro]\nschema-version = 2")
            .unwrap_err()
            .contains("schema-version")
    );
    assert!(
        validate_example(
            "# invalid: schema-version\n[tool.pyfaro]\nschema-version = 2"
        )
        .is_ok()
    );
    assert!(validate_example("# invalid: x\n[tool.pyfaro]").is_err());
    assert_eq!(
        toml_blocks("1. a\n\n   ```toml\n   [a]\n   b = 1\n   ```\n"),
        ["[a]\nb = 1"]
    );
}
