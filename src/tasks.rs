//! Named tasks from `[tool.pyfaro.tasks]`.
//!
//! See `docs/architecture/0002-task-invocation.md` for the rules this module
//! implements. Parsing validates everything that does not depend on the
//! machine; `Task::invocation` handles what does (the cwd on disk, the shell).

use std::ffi::OsString;
use std::fs;
use std::path::{Path, PathBuf};

use toml_edit::{Array, Item};

use crate::error::{Error, Result};

const MAX_NAME_LENGTH: usize = 64;
const TASK_KEYS: [&str; 5] = ["command", "shell", "cwd", "env", "description"];

#[derive(Debug, Default)]
pub(crate) struct Tasks {
    entries: Vec<Task>,
}

#[derive(Debug)]
pub(crate) struct Task {
    name: String,
    action: Action,
    cwd: Option<String>,
    env: Vec<(String, String)>,
    description: Option<String>,
}

#[derive(Debug)]
enum Action {
    Command(Vec<String>),
    Shell(String),
}

/// What `uv run` should execute for a task, and where.
pub(crate) struct Invocation {
    pub cwd: PathBuf,
    pub env: Vec<(OsString, OsString)>,
    /// The argument vector after `uv run ... --`.
    pub command: Vec<OsString>,
}

impl Tasks {
    pub(crate) fn get(&self, name: &str) -> Option<&Task> {
        self.entries.iter().find(|task| task.name == name)
    }

    pub(crate) fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }

    /// One indented line per task, with its description or its command.
    pub(crate) fn listing(&self) -> String {
        let width = self
            .entries
            .iter()
            .map(|task| task.name.chars().count())
            .max()
            .unwrap_or(0);
        self.entries
            .iter()
            .map(|task| format!("  {:<width$}  {}", task.name, task.summary()))
            .collect::<Vec<_>>()
            .join("\n")
    }
}

impl Task {
    fn summary(&self) -> String {
        if let Some(description) = &self.description {
            return description.clone();
        }
        match &self.action {
            Action::Command(argv) => argv
                .iter()
                .map(|argument| display_argument(argument))
                .collect::<Vec<_>>()
                .join(" "),
            Action::Shell(script) => format!("shell: {script}"),
        }
    }

    /// Resolve the working directory and the full argument vector.
    ///
    /// Everything here is decided before uv is launched.
    pub(crate) fn invocation(
        &self,
        root: &Path,
        extra: Vec<OsString>,
    ) -> Result<Invocation> {
        let command = match &self.action {
            Action::Command(argv) => {
                let mut command: Vec<OsString> =
                    argv.iter().map(OsString::from).collect();
                command.extend(extra);
                command
            }
            Action::Shell(script) => {
                if !extra.is_empty() {
                    return Err(Error::Project(format!(
                        "task \"{}\" is a shell task and does not accept extra arguments; \
                         remove them, or define a command task to pass arguments",
                        self.name
                    )));
                }
                shell_command(&self.name, script, cfg!(windows))?
            }
        };
        let cwd = self.resolve_cwd(root)?;
        let env = self
            .env
            .iter()
            .map(|(key, value)| (OsString::from(key), OsString::from(value)))
            .collect();
        Ok(Invocation { cwd, env, command })
    }

    fn resolve_cwd(&self, root: &Path) -> Result<PathBuf> {
        let Some(relative) = &self.cwd else {
            return Ok(for_process(root.to_owned()));
        };
        let candidate = root.join(relative);
        let resolved = match fs::canonicalize(&candidate) {
            Ok(path) if path.is_dir() => path,
            _ => {
                return Err(Error::Project(format!(
                    "task \"{}\": cwd \"{relative}\" is not an existing directory ({})",
                    self.name,
                    candidate.display()
                )));
            }
        };
        if !resolved.starts_with(root) {
            return Err(Error::Project(format!(
                "task \"{}\": cwd \"{relative}\" resolves outside the project root ({})",
                self.name,
                resolved.display()
            )));
        }
        Ok(for_process(resolved))
    }
}

/// `sh -c <script>` or `cmd.exe /d /s /c <script>`; the shell is explicit.
fn shell_command(
    name: &str,
    script: &str,
    windows: bool,
) -> Result<Vec<OsString>> {
    if !windows {
        return Ok(["sh", "-c", script].map(OsString::from).to_vec());
    }
    if script.contains('"') {
        return Err(Error::Project(format!(
            "task \"{name}\": a shell script containing a double quote (\") \
             cannot be passed to cmd.exe unchanged on Windows; \
             use a command task, or move the script into a file"
        )));
    }
    Ok(["cmd.exe", "/d", "/s", "/c", script].map(OsString::from).to_vec())
}

/// Drop the `\\?\` prefix of a canonical drive path: `cmd.exe` refuses it as a
/// working directory.
#[cfg(windows)]
fn for_process(path: PathBuf) -> PathBuf {
    let Some(text) = path.to_str() else {
        return path;
    };
    match text.strip_prefix(r"\\?\") {
        Some(rest) if matches!(rest.as_bytes(), [letter, b':', b'\\', ..] if letter.is_ascii_alphabetic()) => {
            PathBuf::from(rest)
        }
        _ => path,
    }
}

#[cfg(not(windows))]
fn for_process(path: PathBuf) -> PathBuf {
    path
}

fn display_argument(argument: &str) -> String {
    if argument.is_empty() || argument.contains(char::is_whitespace) {
        format!("\"{argument}\"")
    } else {
        argument.to_owned()
    }
}

/// Parse and validate the `[tool.pyfaro.tasks]` item.
pub(crate) fn parse(item: &Item) -> Result<Tasks> {
    let table = item.as_table_like().ok_or_else(|| {
        Error::Project("tool.pyfaro.tasks must be a table".into())
    })?;
    let mut entries = Vec::new();
    for (name, item) in table.iter() {
        entries.push(parse_task(name, item)?);
    }
    Ok(Tasks { entries })
}

fn parse_task(name: &str, item: &Item) -> Result<Task> {
    if !valid_name(name) {
        return Err(Error::Project(format!(
            "invalid task name {name:?} in tool.pyfaro.tasks: \
             use at most {MAX_NAME_LENGTH} characters, starting with a letter or digit, \
             then letters, digits, '_', '.' or '-'"
        )));
    }
    let key = task_key(name);
    if let Some(array) = item.as_array() {
        return Ok(Task {
            name: name.to_owned(),
            action: Action::Command(string_list(&key, array)?),
            cwd: None,
            env: Vec::new(),
            description: None,
        });
    }
    let table = item.as_table_like().ok_or_else(|| {
        Error::Project(format!(
            "{key} must be an array of strings or a table with command or shell"
        ))
    })?;
    for (field, _) in table.iter() {
        if !TASK_KEYS.contains(&field) {
            return Err(Error::Project(format!(
                "unknown task setting: {key}.{field} (supported: {})",
                TASK_KEYS.join(", ")
            )));
        }
    }
    let action = match (table.get("command"), table.get("shell")) {
        (Some(command), None) => {
            let array = command.as_array().ok_or_else(|| {
                Error::Project(format!(
                    "{key}.command must be an array of non-empty strings"
                ))
            })?;
            Action::Command(string_list(&format!("{key}.command"), array)?)
        }
        (None, Some(shell)) => {
            let script = shell.as_str().ok_or_else(|| {
                Error::Project(format!("{key}.shell must be a string"))
            })?;
            if script.trim().is_empty() {
                return Err(Error::Project(format!(
                    "{key}.shell must not be empty"
                )));
            }
            reject_nul(&format!("{key}.shell"), script)?;
            Action::Shell(script.to_owned())
        }
        _ => {
            return Err(Error::Project(format!(
                "{key} must set exactly one of command or shell"
            )));
        }
    };
    let cwd = match table.get("cwd") {
        None => None,
        Some(item) => {
            let field = format!("{key}.cwd");
            let value = item.as_str().ok_or_else(|| {
                Error::Project(format!("{field} must be a string"))
            })?;
            check_cwd(&field, value)?;
            Some(value.to_owned())
        }
    };
    let env = match table.get("env") {
        None => Vec::new(),
        Some(item) => parse_env(&format!("{key}.env"), item)?,
    };
    let description = match table.get("description") {
        None => None,
        Some(item) => Some(
            item.as_str()
                .ok_or_else(|| {
                    Error::Project(format!(
                        "{key}.description must be a string"
                    ))
                })?
                .to_owned(),
        ),
    };
    Ok(Task { name: name.to_owned(), action, cwd, env, description })
}

fn parse_env(key: &str, item: &Item) -> Result<Vec<(String, String)>> {
    let table = item.as_table_like().ok_or_else(|| {
        Error::Project(format!("{key} must be a table of strings"))
    })?;
    let mut env = Vec::new();
    for (name, value) in table.iter() {
        if name.is_empty() || name.contains('=') || name.contains('\0') {
            return Err(Error::Project(format!(
                "invalid variable name {name:?} in {key}: \
                 names must not be empty or contain '=' or NUL"
            )));
        }
        let value = value.as_str().ok_or_else(|| {
            Error::Project(format!("{key}.{name} must be a string"))
        })?;
        reject_nul(&format!("{key}.{name}"), value)?;
        env.push((name.to_owned(), value.to_owned()));
    }
    Ok(env)
}

fn string_list(key: &str, array: &Array) -> Result<Vec<String>> {
    if array.is_empty() {
        return Err(Error::Project(format!("{key} must not be empty")));
    }
    let mut strings = Vec::new();
    for value in array {
        match value.as_str() {
            Some(text) if !text.is_empty() => {
                reject_nul(key, text)?;
                strings.push(text.to_owned());
            }
            _ => {
                return Err(Error::Project(format!(
                    "{key} must be an array of non-empty strings"
                )));
            }
        }
    }
    Ok(strings)
}

fn reject_nul(key: &str, value: &str) -> Result<()> {
    if value.contains('\0') {
        return Err(Error::Project(format!(
            "{key} must not contain a NUL character"
        )));
    }
    Ok(())
}

/// Lexical check shared by every platform, so one `pyproject.toml` means the
/// same thing everywhere: relative, and never above the project root.
fn check_cwd(key: &str, value: &str) -> Result<()> {
    if value.is_empty() {
        return Err(Error::Project(format!("{key} must not be empty")));
    }
    reject_nul(key, value)?;
    let bytes = value.as_bytes();
    let absolute = value.starts_with(['/', '\\'])
        || matches!(bytes, [letter, b':', ..] if letter.is_ascii_alphabetic())
        || Path::new(value).has_root();
    if absolute {
        return Err(Error::Project(format!(
            "{key} must be relative to the project root, not absolute: {value}"
        )));
    }
    let mut depth = 0_usize;
    for part in value.split(['/', '\\']) {
        match part {
            "" | "." => {}
            ".." => {
                depth = depth.checked_sub(1).ok_or_else(|| {
                    Error::Project(format!(
                        "{key} must stay inside the project root: {value}"
                    ))
                })?;
            }
            _ => depth += 1,
        }
    }
    Ok(())
}

fn valid_name(name: &str) -> bool {
    let mut characters = name.chars();
    name.chars().count() <= MAX_NAME_LENGTH
        && characters.next().is_some_and(|first| first.is_ascii_alphanumeric())
        && characters
            .all(|c| c.is_ascii_alphanumeric() || matches!(c, '_' | '.' | '-'))
}

/// Dotted key for messages; names with dots need TOML quoting.
fn task_key(name: &str) -> String {
    if name.contains('.') {
        format!("tool.pyfaro.tasks.\"{name}\"")
    } else {
        format!("tool.pyfaro.tasks.{name}")
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use toml_edit::DocumentMut;

    fn parse_source(tasks: &str) -> Result<Tasks> {
        let document: DocumentMut = tasks.parse().unwrap();
        parse(&document["tasks"])
    }

    fn message(tasks: &str) -> String {
        parse_source(tasks).unwrap_err().to_string()
    }

    #[test]
    fn parses_short_and_table_forms_in_definition_order() {
        let tasks = parse_source(
            r#"
[tasks]
lint-all = ["ruff", "check", "."]
serve = { command = ["python", "-m", "http.server"], cwd = "docs/site", env = { A = "1" }, description = "Serve" }
changes = { shell = "git log | head" }
"#,
        )
        .unwrap();
        assert_eq!(
            tasks.listing(),
            "  lint-all  ruff check .\n  serve     Serve\n  changes   shell: git log | head"
        );
        let serve = tasks.get("serve").unwrap();
        assert_eq!(serve.cwd.as_deref(), Some("docs/site"));
        assert_eq!(serve.env, [("A".to_owned(), "1".to_owned())]);
        assert!(tasks.get("missing").is_none());
    }

    #[test]
    fn standard_tables_are_accepted() {
        let tasks = parse_source(
            "[tasks.build]\ncommand = ['uv', 'build']\n[tasks.build.env]\nX = 'y'\n",
        )
        .unwrap();
        assert_eq!(tasks.get("build").unwrap().env.len(), 1);
    }

    #[test]
    fn rejects_invalid_names() {
        let long = "a".repeat(65);
        for name in ["-lead", "_lead", ".lead", "with space", "é", &long] {
            let source = format!("[tasks]\n\"{name}\" = ['x']\n");
            let message = message(&source);
            assert!(message.contains("invalid task name"), "{name}: {message}");
        }
        let longest = "a".repeat(64);
        parse_source(&format!("[tasks]\n{longest} = ['x']\n")).unwrap();
    }

    #[test]
    fn rejects_invalid_values_naming_the_key() {
        for (source, expected) in [
            ("[tasks]\nt = 1\n", "tool.pyfaro.tasks.t must be an array"),
            ("[tasks]\nt = []\n", "tool.pyfaro.tasks.t must not be empty"),
            (
                "[tasks]\nt = ['']\n",
                "tool.pyfaro.tasks.t must be an array of non-empty strings",
            ),
            (
                "[tasks]\nt = [1]\n",
                "tool.pyfaro.tasks.t must be an array of non-empty strings",
            ),
            (
                "[tasks]\nt = {}\n",
                "tool.pyfaro.tasks.t must set exactly one of command or shell",
            ),
            (
                "[tasks]\nt = { command = ['a'], shell = 'b' }\n",
                "tool.pyfaro.tasks.t must set exactly one",
            ),
            (
                "[tasks]\nt = { command = 'a' }\n",
                "tool.pyfaro.tasks.t.command must be an array",
            ),
            (
                "[tasks]\nt = { command = [] }\n",
                "tool.pyfaro.tasks.t.command must not be empty",
            ),
            (
                "[tasks]\nt = { shell = 1 }\n",
                "tool.pyfaro.tasks.t.shell must be a string",
            ),
            (
                "[tasks]\nt = { shell = ' ' }\n",
                "tool.pyfaro.tasks.t.shell must not be empty",
            ),
            (
                "[tasks]\nt = { command = ['a'], extra = 1 }\n",
                "unknown task setting: tool.pyfaro.tasks.t.extra",
            ),
            (
                "[tasks]\nt = { command = ['a'], cwd = 1 }\n",
                "tool.pyfaro.tasks.t.cwd must be a string",
            ),
            (
                "[tasks]\nt = { command = ['a'], description = 1 }\n",
                "tool.pyfaro.tasks.t.description must be a string",
            ),
            (
                "[tasks]\nt = { command = ['a'], env = 1 }\n",
                "tool.pyfaro.tasks.t.env must be a table",
            ),
            (
                "[tasks]\nt = { command = ['a'], env = { A = 1 } }\n",
                "tool.pyfaro.tasks.t.env.A must be a string",
            ),
            (
                "[tasks]\nt = { command = ['a'], env = { 'A=B' = 'x' } }\n",
                "invalid variable name \"A=B\" in tool.pyfaro.tasks.t.env",
            ),
            (
                "[tasks]\nt = { command = ['a'], env = { '' = 'x' } }\n",
                "invalid variable name \"\"",
            ),
            (
                "[tasks]\nt = { command = ['a'], env = { A = \"x\\u0000\" } }\n",
                "tool.pyfaro.tasks.t.env.A must not contain a NUL",
            ),
            (
                "[tasks]\nt = [\"a\\u0000\"]\n",
                "tool.pyfaro.tasks.t must not contain a NUL",
            ),
            (
                "[tasks]\n\"a.b\" = 1\n",
                "tool.pyfaro.tasks.\"a.b\" must be an array",
            ),
        ] {
            let message = message(source);
            assert!(message.contains(expected), "{source}: {message}");
        }
        assert!(
            parse(&"tasks = 1".parse::<DocumentMut>().unwrap()["tasks"])
                .unwrap_err()
                .to_string()
                .contains("tool.pyfaro.tasks must be a table")
        );
    }

    #[test]
    fn cwd_must_be_relative_and_stay_inside_the_project() {
        for cwd in ["/etc", "\\\\server\\share", "\\x", "C:\\x", "C:x", "c:/x"]
        {
            let source =
                format!("[tasks]\nt = {{ command = ['a'], cwd = '{cwd}' }}\n");
            let message = message(&source);
            assert!(message.contains("not absolute"), "{cwd}: {message}");
        }
        for cwd in ["..", "../x", "a/../..", "a\\..\\..\\b", "./../x"] {
            let source =
                format!("[tasks]\nt = {{ command = ['a'], cwd = '{cwd}' }}\n");
            let message = message(&source);
            assert!(
                message.contains("inside the project root"),
                "{cwd}: {message}"
            );
        }
        assert!(
            message("[tasks]\nt = { command = ['a'], cwd = '' }\n")
                .contains("cwd must not be empty")
        );
        for cwd in [".", "a", "a/b", "a/../b", "./a", "a/.."] {
            parse_source(&format!(
                "[tasks]\nt = {{ command = ['a'], cwd = '{cwd}' }}\n"
            ))
            .unwrap();
        }
    }

    #[test]
    fn shell_scripts_pick_the_platform_shell_and_reject_windows_quotes() {
        assert_eq!(
            shell_command("t", "a | b", false).unwrap(),
            ["sh", "-c", "a | b"].map(OsString::from)
        );
        assert_eq!(shell_command("t", "a \"b\"", false).unwrap().len(), 3);
        assert_eq!(
            shell_command("t", "a | b", true).unwrap(),
            ["cmd.exe", "/d", "/s", "/c", "a | b"].map(OsString::from)
        );
        let error = shell_command("t", "echo \"x\"", true).unwrap_err();
        assert!(error.to_string().contains("double quote"), "{error}");
    }

    #[test]
    fn extra_arguments_are_appended_to_commands_and_rejected_for_shells() {
        let tasks = parse_source(
            "[tasks]\nc = ['ruff', 'check']\ns = { shell = 'echo hi' }\n",
        )
        .unwrap();
        let root = tempfile::tempdir().unwrap();
        let root = fs::canonicalize(root.path()).unwrap();
        let invocation = tasks
            .get("c")
            .unwrap()
            .invocation(&root, vec!["--fix".into(), "a b".into()])
            .unwrap();
        assert_eq!(
            invocation.command,
            ["ruff", "check", "--fix", "a b"].map(OsString::from)
        );
        assert_eq!(invocation.cwd, root);
        let error =
            match tasks.get("s").unwrap().invocation(&root, vec!["x".into()]) {
                Ok(_) => panic!("extra arguments were accepted"),
                Err(error) => error,
            };
        assert!(error.to_string().contains("does not accept extra arguments"));
    }

    #[test]
    fn cwd_must_exist_and_stay_inside_after_symlinks() {
        let tasks = parse_source(
            "[tasks]\nok = { command = ['a'], cwd = 'sub' }\nmissing = { command = ['a'], cwd = 'nope' }\nfile = { command = ['a'], cwd = 'file' }\n",
        )
        .unwrap();
        let directory = tempfile::tempdir().unwrap();
        let root = fs::canonicalize(directory.path()).unwrap();
        fs::create_dir(root.join("sub")).unwrap();
        fs::write(root.join("file"), "").unwrap();
        let invocation =
            tasks.get("ok").unwrap().invocation(&root, vec![]).unwrap();
        assert!(invocation.cwd.ends_with("sub"));
        for name in ["missing", "file"] {
            let error = match tasks.get(name).unwrap().invocation(&root, vec![])
            {
                Ok(_) => panic!("{name} was accepted"),
                Err(error) => error,
            };
            assert!(
                error.to_string().contains("is not an existing directory"),
                "{error}"
            );
        }
    }

    #[cfg(unix)]
    #[test]
    fn a_symlink_leaving_the_project_is_rejected() {
        let tasks =
            parse_source("[tasks]\nt = { command = ['a'], cwd = 'link' }\n")
                .unwrap();
        let directory = tempfile::tempdir().unwrap();
        let outside = tempfile::tempdir().unwrap();
        let root = fs::canonicalize(directory.path()).unwrap();
        std::os::unix::fs::symlink(outside.path(), root.join("link")).unwrap();
        let error = match tasks.get("t").unwrap().invocation(&root, vec![]) {
            Ok(_) => panic!("escaping symlink was accepted"),
            Err(error) => error,
        };
        assert!(
            error.to_string().contains("outside the project root"),
            "{error}"
        );
    }
}
