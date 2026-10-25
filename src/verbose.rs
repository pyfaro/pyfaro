//! `--verbose`: print what Pyfaro resolves and runs, to stderr.
//!
//! Verbose is a process-wide setting, enabled once from `main`, so a trace
//! needs no plumbing through the commands. Every child process goes through
//! `process::run*`, which calls [`command`]; the other lines come from the
//! places that resolve a tool. Lines read `pyfaro <command>: <message>`, the
//! format `pyfaro dev` always used. Text is redacted by [`crate::redact`] and
//! environment variable values are never printed.

use std::io::Write;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::sync::Mutex;

use crate::quality::strip_verbatim;
use crate::redact;

struct State {
    command: &'static str,
    root: Option<PathBuf>,
}

static STATE: Mutex<Option<State>> = Mutex::new(None);

fn with_state<T>(use_state: impl FnOnce(&mut State) -> T) -> Option<T> {
    let mut state =
        STATE.lock().unwrap_or_else(|poisoned| poisoned.into_inner());
    state.as_mut().map(use_state)
}

/// Turn verbose output on for the running `command`.
pub(crate) fn enable(command: &'static str) {
    let mut state =
        STATE.lock().unwrap_or_else(|poisoned| poisoned.into_inner());
    *state = Some(State { command, root: None });
}

pub(crate) fn enabled() -> bool {
    with_state(|_| ()).is_some()
}

/// Record the selected project root, so a working directory is only
/// mentioned when it differs.
pub(crate) fn project_root(root: &Path) {
    with_state(|state| state.root = Some(root.to_owned()));
}

/// Print `pyfaro <command>: <message>` when verbose is on.
pub(crate) fn say(message: &str) {
    let Some(command) = with_state(|state| state.command) else {
        return;
    };
    let line = format!("pyfaro {command}: {}", redact::text(message));
    // Diagnostics must not panic if stderr is closed or unavailable.
    let _ = writeln!(std::io::stderr().lock(), "{line}");
}

/// Report a resolved external tool: its version output and its path.
pub(crate) fn tool(version: &str, path: &Path) {
    if enabled() {
        say(&format!("using {version} at {}", path.display()));
    }
}

/// Report a command about to be spawned: the command line, the working
/// directory unless it is the project root, and the names (never the values)
/// of the environment variables Pyfaro sets for it.
pub(crate) fn command(command: &Command) {
    let Some(root) = with_state(|state| state.root.clone()) else {
        return;
    };
    let words = std::iter::once(command.get_program())
        .chain(command.get_args())
        .map(|word| quote_word(&word.to_string_lossy()))
        .collect::<Vec<_>>();
    say(&format!("running {}", words.join(" ")));
    let directory = command
        .get_current_dir()
        .map(Path::to_owned)
        .or_else(|| std::env::current_dir().ok());
    if let Some(directory) = directory
        && !is_root(&directory, root.as_deref())
    {
        say(&format!("working directory {}", directory.display()));
    }
    let names: Vec<_> = command
        .get_envs()
        .filter(|(_, value)| value.is_some())
        .map(|(name, _)| name.to_string_lossy())
        .collect();
    if !names.is_empty() {
        say(&format!("environment variables set: {}", names.join(", ")));
    }
}

fn is_root(directory: &Path, root: Option<&Path>) -> bool {
    let Some(root) = root else {
        return false;
    };
    let resolve = |path: &Path| {
        strip_verbatim(&path.canonicalize().unwrap_or_else(|_| path.to_owned()))
    };
    resolve(directory) == resolve(root)
}

/// Quote a word for display when it is empty or contains spaces or quotes.
fn quote_word(word: &str) -> String {
    if !word.is_empty() && !word.contains([' ', '\t', '"', '\'']) {
        return word.to_owned();
    }
    format!("\"{}\"", word.replace('"', "\\\""))
}

#[cfg(test)]
mod tests {
    use super::quote_word;

    #[test]
    fn displayed_command_lines_quote_ambiguous_words() {
        assert_eq!(quote_word("plain"), "plain");
        assert_eq!(quote_word(""), "\"\"");
        assert_eq!(quote_word("a b"), "\"a b\"");
        assert_eq!(quote_word("say \"hi\""), "\"say \\\"hi\\\"\"");
    }
}
