//! `pyfaro run <task>` and `[tool.pyfaro.tasks]`.
//!
//! The projects here are bare (no dependencies): `uv run` creates the
//! environment on first use, which is all the tasks need.

use std::fs;
use std::path::Path;

use crate::harness::{Outcome, Project, Sandbox};

/// Prints `sys.argv[1:]` as compact ASCII JSON.
const ARGV_SCRIPT: &str = "import json, sys\n\
     print(json.dumps(sys.argv[1:], separators=(',', ':')))\n";

/// Prints the working directory of the process.
const PRINT_CWD: &str = "import os; print(os.getcwd())";

fn toml_string(text: &str) -> String {
    let escaped =
        text.replace('\\', "\\\\").replace('"', "\\\"").replace('\t', "\\t");
    format!("\"{escaped}\"")
}

/// Compact JSON array of strings exactly as Python's
/// `json.dumps(separators=(',', ':'))` prints it (ASCII only).
fn json_args(args: &[&str]) -> String {
    let mut out = String::from("[");
    for (index, arg) in args.iter().enumerate() {
        if index > 0 {
            out.push(',');
        }
        out.push('"');
        for c in arg.chars() {
            match c {
                '"' => out.push_str("\\\""),
                '\\' => out.push_str("\\\\"),
                '\n' => out.push_str("\\n"),
                '\r' => out.push_str("\\r"),
                '\t' => out.push_str("\\t"),
                c if c.is_ascii() && !c.is_ascii_control() => out.push(c),
                c => {
                    let mut units = [0; 2];
                    for unit in c.encode_utf16(&mut units) {
                        out.push_str(&format!("\\u{unit:04x}"));
                    }
                }
            }
        }
        out.push('"');
    }
    out.push(']');
    out
}

/// A project with these exact `pyproject.toml` contents after the
/// `[project]` table, plus `argv.py`.
fn project_with(sandbox: &Sandbox, config: &str) -> Project {
    let root = sandbox.join("project");
    sandbox.write(
        "project/pyproject.toml",
        &format!(
            "[project]\nname = \"tasks-demo\"\nversion = \"0.1.0\"\n\
             requires-python = \">=3.10\"\ndependencies = []\n\n{config}"
        ),
    );
    sandbox.write("project/argv.py", ARGV_SCRIPT);
    Project { root, name: "tasks-demo", package: "tasks_demo" }
}

/// A project whose `[tool.pyfaro.tasks]` table holds `tasks`.
fn project(sandbox: &Sandbox, tasks: &str) -> Project {
    project_with(
        sandbox,
        &format!(
            "[tool.pyfaro]\nschema-version = 1\n\n[tool.pyfaro.tasks]\n{tasks}"
        ),
    )
}

trait Notes {
    /// No "running the program" collision note on stderr.
    fn stderr_empty_of_pyfaro_notes(&self) -> &Self;
}

impl Notes for Outcome {
    fn stderr_empty_of_pyfaro_notes(&self) -> &Self {
        assert!(
            !self.stderr.contains("running the program"),
            "unexpected collision note:\n{}",
            self.stderr
        );
        self
    }
}

fn trimmed_lines(text: &str) -> Vec<&str> {
    text.lines().map(str::trim_end).collect()
}

fn same_directory(printed: &str, expected: &Path) -> bool {
    fs::canonicalize(printed.trim())
        .is_ok_and(|path| path == fs::canonicalize(expected).unwrap())
}

#[test]
fn the_short_form_runs_an_argument_vector() {
    e2e_guard!();
    let sandbox = Sandbox::new();
    let project = project(
        &sandbox,
        "show = [\"python\", \"-c\", \"print('from the short form')\"]\n",
    );
    project
        .pyfaro(["run", "show"])
        .run()
        .success()
        .stdout_text_eq("from the short form\n")
        .stderr_empty_of_pyfaro_notes();
}

#[test]
fn the_table_form_runs_a_command_with_cwd_env_and_description() {
    e2e_guard!();
    let sandbox = Sandbox::new();
    let project = project(
        &sandbox,
        "serve = { command = [\"python\", \"-c\", \"import os; print(os.environ['TASK_ENV'])\"], \
         cwd = \"docs\", env = { TASK_ENV = \"table-form\" }, description = \"Serve\" }\n",
    );
    project.write("docs/.keep", "");
    project
        .pyfaro(["run", "serve"])
        .run()
        .success()
        .stdout_text_eq("table-form\n");
}

#[test]
fn extra_arguments_after_the_separator_are_appended_in_order() {
    e2e_guard!();
    let sandbox = Sandbox::new();
    let project =
        project(&sandbox, "echo = [\"python\", \"argv.py\", \"first\"]\n");
    project
        .pyfaro(["run", "echo", "--", "second", "--flag", "-n", "--", "x y"])
        .run()
        .success()
        .stdout_text_eq(&format!(
            "{}\n",
            json_args(&["first", "second", "--flag", "-n", "--", "x y"])
        ));
    // Without the separator, nothing is appended.
    project
        .pyfaro(["run", "echo"])
        .run()
        .success()
        .stdout_text_eq("[\"first\"]\n");
}

#[test]
fn the_default_cwd_is_the_project_root_even_from_a_subdirectory() {
    e2e_guard!();
    let sandbox = Sandbox::new();
    let project = project(
        &sandbox,
        &format!("where = [\"python\", \"-c\", {}]\n", toml_string(PRINT_CWD)),
    );
    project.write("src/deep/.keep", "");
    let outcome =
        project.pyfaro(["run", "where"]).cwd(project.path("src/deep")).run();
    outcome.success();
    assert!(
        same_directory(&outcome.stdout, &project.root),
        "cwd was {:?}",
        outcome.stdout
    );
    // The program form keeps the caller's directory, unchanged.
    let program = project
        .pyfaro(["run", "--", "python", "-c", PRINT_CWD])
        .cwd(project.path("src/deep"))
        .run();
    program.success();
    assert!(same_directory(&program.stdout, &project.path("src/deep")));
}

#[test]
fn an_explicit_cwd_is_resolved_against_the_project_root() {
    e2e_guard!();
    let sandbox = Sandbox::new();
    let project = project(
        &sandbox,
        &format!(
            "where = {{ command = [\"python\", \"-c\", {}], cwd = \"docs/site\" }}\n",
            toml_string(PRINT_CWD)
        ),
    );
    project.write("docs/site/.keep", "");
    for caller in [project.root.clone(), project.path("docs")] {
        let outcome = project.pyfaro(["run", "where"]).cwd(caller).run();
        outcome.success();
        assert!(
            same_directory(&outcome.stdout, &project.path("docs/site")),
            "cwd was {:?}",
            outcome.stdout
        );
    }
}

#[test]
fn an_absolute_or_escaping_cwd_is_rejected_before_launch() {
    e2e_guard!();
    let sandbox = Sandbox::new();
    let outside = sandbox.join("outside");
    fs::create_dir_all(&outside).unwrap();
    let absolute = outside.to_string_lossy().into_owned();
    for (cwd, expected) in [
        (absolute.as_str(), "must be relative to the project root"),
        ("/etc", "must be relative to the project root"),
        ("C:\\Windows", "must be relative to the project root"),
        ("..", "must stay inside the project root"),
        ("../outside", "must stay inside the project root"),
        ("docs/../../outside", "must stay inside the project root"),
        ("", "must not be empty"),
    ] {
        let project = project(
            &sandbox,
            &format!(
                "t = {{ command = [\"python\", \"-c\", \"print('ran')\"], cwd = {} }}\n",
                toml_string(cwd)
            ),
        );
        project
            .pyfaro(["run", "t"])
            .run()
            .code(1)
            .stdout_empty()
            .stderr_contains("pyproject.toml")
            .stderr_contains("tool.pyfaro.tasks.t.cwd")
            .stderr_contains(expected);
        assert!(!project.path(".venv").exists(), "uv ran for cwd {cwd:?}");
    }
}

#[test]
fn a_missing_cwd_is_rejected_at_launch() {
    e2e_guard!();
    let sandbox = Sandbox::new();
    let project = project(
        &sandbox,
        "t = { command = [\"python\", \"-c\", \"print('ran')\"], cwd = \"nope\" }\n\
         f = { command = [\"python\", \"-c\", \"print('ran')\"], cwd = \"argv.py\" }\n",
    );
    for name in ["t", "f"] {
        project
            .pyfaro(["run", name])
            .run()
            .code(1)
            .stdout_empty()
            .stderr_contains(&format!("task \"{name}\""))
            .stderr_contains("is not an existing directory");
    }
}

#[cfg(unix)]
#[test]
fn a_cwd_symlink_leaving_the_project_is_rejected() {
    e2e_guard!();
    let sandbox = Sandbox::new();
    let project = project(
        &sandbox,
        "t = { command = [\"python\", \"-c\", \"print('ran')\"], cwd = \"link\" }\n",
    );
    let outside = sandbox.join("outside");
    fs::create_dir_all(&outside).unwrap();
    std::os::unix::fs::symlink(&outside, project.path("link")).unwrap();
    project
        .pyfaro(["run", "t"])
        .run()
        .code(1)
        .stdout_empty()
        .stderr_contains("resolves outside the project root");
}

#[test]
fn env_is_applied_and_wins_over_the_inherited_environment() {
    e2e_guard!();
    let sandbox = Sandbox::new();
    let script = "import os; print(os.environ.get('PYFARO_TASK_A')); print(os.environ.get('PYFARO_TASK_B'))";
    let project = project(
        &sandbox,
        &format!(
            "t = {{ command = [\"python\", \"-c\", {}], env = {{ PYFARO_TASK_A = \"from-task\" }} }}\n",
            toml_string(script)
        ),
    );
    project
        .pyfaro(["run", "t"])
        .env("PYFARO_TASK_A", "inherited")
        .env("PYFARO_TASK_B", "inherited-too")
        .run()
        .success()
        .stdout_text_eq("from-task\ninherited-too\n");
    // Without the task value the inherited one stays.
    project
        .pyfaro(["run", "--", "python", "-c", script])
        .env("PYFARO_TASK_A", "inherited")
        .run()
        .success()
        .stdout_text_eq("inherited\nNone\n");
}

#[test]
fn every_validation_error_names_the_key_and_the_file() {
    e2e_guard!();
    let sandbox = Sandbox::new();
    let long_name = "a".repeat(65);
    for (tasks, expected) in [
        (
            "t = 1\n",
            "tool.pyfaro.tasks.t must be an array of strings or a table",
        ),
        ("t = []\n", "tool.pyfaro.tasks.t must not be empty"),
        (
            "t = [\"\"]\n",
            "tool.pyfaro.tasks.t must be an array of non-empty strings",
        ),
        (
            "t = [1]\n",
            "tool.pyfaro.tasks.t must be an array of non-empty strings",
        ),
        (
            "t = {}\n",
            "tool.pyfaro.tasks.t must set exactly one of command or shell",
        ),
        (
            "t = { command = [\"a\"], shell = \"b\" }\n",
            "tool.pyfaro.tasks.t must set exactly one of command or shell",
        ),
        (
            "t = { description = \"only\" }\n",
            "tool.pyfaro.tasks.t must set exactly one of command or shell",
        ),
        (
            "t = { command = \"a\" }\n",
            "tool.pyfaro.tasks.t.command must be an array",
        ),
        (
            "t = { command = [] }\n",
            "tool.pyfaro.tasks.t.command must not be empty",
        ),
        ("t = { shell = 1 }\n", "tool.pyfaro.tasks.t.shell must be a string"),
        (
            "t = { shell = \"\" }\n",
            "tool.pyfaro.tasks.t.shell must not be empty",
        ),
        (
            "t = { command = [\"a\"], cwd = 1 }\n",
            "tool.pyfaro.tasks.t.cwd must be a string",
        ),
        (
            "t = { command = [\"a\"], description = 1 }\n",
            "tool.pyfaro.tasks.t.description must be a string",
        ),
        (
            "t = { command = [\"a\"], env = [\"A=1\"] }\n",
            "tool.pyfaro.tasks.t.env must be a table of strings",
        ),
        (
            "t = { command = [\"a\"], env = { A = 1 } }\n",
            "tool.pyfaro.tasks.t.env.A must be a string",
        ),
        (
            "t = { command = [\"a\"], env = { \"A=B\" = \"x\" } }\n",
            "invalid variable name \"A=B\" in tool.pyfaro.tasks.t.env",
        ),
        (
            "t = { command = [\"a\"], env = { \"\" = \"x\" } }\n",
            "invalid variable name \"\" in tool.pyfaro.tasks.t.env",
        ),
        (
            "t = { command = [\"a\"], env = { A = \"x\\u0000y\" } }\n",
            "tool.pyfaro.tasks.t.env.A must not contain a NUL",
        ),
        (
            "t = { command = [\"a\"], colour = \"red\" }\n",
            "unknown task setting: tool.pyfaro.tasks.t.colour",
        ),
        ("\"-lead\" = [\"a\"]\n", "invalid task name \"-lead\""),
        ("\"has space\" = [\"a\"]\n", "invalid task name \"has space\""),
        (&format!("{long_name} = [\"a\"]\n"), "invalid task name"),
        ("\"a.b\" = 1\n", "tool.pyfaro.tasks.\"a.b\" must be an array"),
    ] {
        let project = project(&sandbox, tasks);
        project
            .pyfaro(["run", "--uv", "no-such-uv", "--", "python", "-c", "pass"])
            .run()
            .code(1)
            .stdout_empty()
            .stderr_contains("pyproject.toml")
            .stderr_contains(expected);
    }
    // The tasks table itself, and the other commands that load the config.
    let project = project_with(&sandbox, "[tool.pyfaro]\ntasks = 1\n");
    project
        .pyfaro(["run", "x"])
        .run()
        .code(1)
        .stderr_contains("tool.pyfaro.tasks must be a table");
    project
        .pyfaro(["sync", "--uv", "no-such-uv"])
        .run()
        .code(1)
        .stderr_contains("tool.pyfaro.tasks must be a table");
}

#[test]
fn an_unknown_task_lists_the_tasks_and_points_to_the_program_form() {
    e2e_guard!();
    let sandbox = Sandbox::new();
    let project = project(
        &sandbox,
        "lint-all = [\"ruff\", \"check\", \".\"]\n\
         docs = { command = [\"mkdocs\", \"serve\"], description = \"Serve the docs\" }\n\
         changes = { shell = \"git log\" }\n",
    );
    let outcome = project.pyfaro(["run", "nope"]).run();
    outcome
        .code(1)
        .stdout_empty()
        .stderr_contains("error: no task named \"nope\" in ")
        .stderr_contains("pyproject.toml")
        .stderr_contains("lint-all  ruff check .")
        .stderr_contains("docs      Serve the docs")
        .stderr_contains("changes   shell: git log")
        .stderr_contains("to run a program, use pyfaro run -- nope");
    assert!(!project.path(".venv").exists(), "uv ran for an unknown task");

    let bare = project_with(&sandbox, "");
    bare.pyfaro(["run", "nope"])
        .run()
        .code(1)
        .stderr_contains("no tasks are defined under [tool.pyfaro.tasks]")
        .stderr_contains("to run a program, use pyfaro run -- nope");
}

#[test]
fn a_name_outside_a_project_is_a_usage_error_with_the_program_hint() {
    e2e_guard!();
    let sandbox = Sandbox::new();
    sandbox
        .pyfaro(["run", "python"])
        .run()
        .code(2)
        .stderr_contains("no pyproject.toml found from")
        .stderr_contains("to run a program, use pyfaro run -- <program>");
}

#[test]
fn without_a_task_or_a_program_it_is_a_usage_error_listing_the_tasks() {
    e2e_guard!();
    let sandbox = Sandbox::new();
    let project = project(
        &sandbox,
        "lint-all = [\"ruff\", \"check\", \".\"]\nserve = { shell = \"echo hi\", description = \"Say hi\" }\n",
    );
    for args in [vec!["run"], vec!["run", "--"], vec!["run", "--offline"]] {
        project
            .pyfaro(&args)
            .run()
            .code(2)
            .stdout_empty()
            .stderr_contains("a task name, or a program after --, is required")
            .stderr_contains(
                "Usage: pyfaro run [OPTIONS] <TASK> [-- <ARGS>...]",
            )
            .stderr_contains("pyfaro run [OPTIONS] -- <PROGRAM> [ARGS]...")
            .stderr_contains("Tasks defined in ")
            .stderr_contains("lint-all  ruff check .")
            .stderr_contains("serve     Say hi");
    }
    // Without tasks, or outside any project, there is still a usage error.
    project_with(&sandbox, "")
        .pyfaro(["run"])
        .run()
        .code(2)
        .stderr_contains("is required")
        .stderr_contains("Usage: pyfaro run");
    let elsewhere = Sandbox::new();
    elsewhere.pyfaro(["run"]).run().code(2).stderr_contains("is required");
}

#[test]
fn the_program_form_notes_a_task_with_the_same_name_and_still_runs_the_program()
{
    e2e_guard!();
    let sandbox = Sandbox::new();
    let project = project(
        &sandbox,
        "python = [\"python\", \"-c\", \"print('the task')\"]\n",
    );
    project
        .pyfaro(["run", "--", "python", "-c", "print('the program')"])
        .run()
        .success()
        .stdout_text_eq("the program\n")
        .stderr_contains(
            "pyfaro: running the program \"python\"; a task with the same name exists, run it with pyfaro run python",
        );
    project
        .pyfaro(["run", "python"])
        .run()
        .success()
        .stdout_text_eq("the task\n")
        .stderr_empty_of_pyfaro_notes();
    // No note when no task has the program's name.
    let other = project_with(&sandbox, "");
    other
        .pyfaro(["run", "--", "python", "-c", "print('plain')"])
        .run()
        .success()
        .stdout_text_eq("plain\n")
        .stderr_empty_of_pyfaro_notes();
}

#[test]
fn the_exit_code_of_a_task_is_forwarded_unchanged() {
    e2e_guard!();
    let sandbox = Sandbox::new();
    let project = project(
        &sandbox,
        "fail = [\"python\", \"-c\", \"import sys; print('failing'); sys.exit(7)\"]\n\
         shell-fail = { shell = \"exit 5\" }\n",
    );
    project
        .pyfaro(["run", "fail"])
        .run()
        .code(7)
        .stdout_text_eq("failing\n")
        .stderr_contains("error: uv run failed (");
    project.pyfaro(["run", "shell-fail"]).run().code(5);
}

#[test]
fn a_missing_program_in_a_task_is_uvs_failure() {
    e2e_guard!();
    let sandbox = Sandbox::new();
    let project = project(&sandbox, "t = [\"no-such-program-pyfaro\"]\n");
    project
        .pyfaro(["run", "t"])
        .run()
        .code(2)
        .stderr_contains("no-such-program-pyfaro")
        .stderr_contains("error: uv run failed (");
}

#[test]
fn locked_frozen_and_project_apply_to_tasks() {
    e2e_guard!();
    let sandbox = Sandbox::new();
    let project =
        project(&sandbox, "t = [\"python\", \"-c\", \"print('ran')\"]\n");
    // No lock yet: the strict modes refuse before the task runs.
    project
        .pyfaro(["run", "--locked", "t"])
        .run()
        .code(2)
        .stdout_empty()
        .stderr_contains(
            "Unable to find lockfile at `uv.lock`, but `--locked` was provided",
        );
    assert!(!project.path("uv.lock").exists());
    project
        .pyfaro(["run", "--frozen", "t"])
        .run()
        .code(2)
        .stdout_empty()
        .stderr_contains(
            "Unable to find lockfile at `uv.lock`, but `--frozen` was provided",
        );
    project
        .pyfaro(["run", "--locked", "--frozen", "t"])
        .run()
        .code(2)
        .stderr_contains("cannot be used with");
    // The first plain run creates the lock; then both modes pass.
    project.pyfaro(["run", "t"]).run().success().stdout_text_eq("ran\n");
    let before = project.snapshot();
    project
        .pyfaro(["run", "--locked", "t"])
        .run()
        .success()
        .stdout_text_eq("ran\n");
    project
        .pyfaro(["run", "--frozen", "t"])
        .run()
        .success()
        .stdout_text_eq("ran\n");
    assert_eq!(project.snapshot(), before);
    // --project from outside the project, and from a sibling directory.
    sandbox
        .pyfaro(["run", "--project"])
        .arg(&project.root)
        .arg("t")
        .run()
        .success()
        .stdout_text_eq("ran\n");
    sandbox
        .pyfaro(["run", "t"])
        .run()
        .code(2)
        .stderr_contains("no pyproject.toml found from");
}

#[test]
fn command_tasks_pass_every_argument_literally() {
    e2e_guard!();
    let sandbox = Sandbox::new();
    let matrix = [
        "a b",
        "it's",
        "say \"hi\"",
        "$HOME",
        "${HOME}",
        "%PATH%",
        "*",
        "?",
        "~",
        "&",
        "|",
        ";",
        "a&b|c;d",
        ">",
        "<",
        "(x)",
        "^caret",
        "!bang!",
        "héllo wörld ✓",
        "--flag",
        "-n",
        "--",
        " ",
        "tab\there",
        "C:\\path with space\\",
    ];
    // The matrix inside the configuration itself.
    let literal =
        matrix.iter().map(|a| toml_string(a)).collect::<Vec<_>>().join(", ");
    let project = project(
        &sandbox,
        &format!(
            "config = [\"python\", \"argv.py\", {literal}]\nplain = [\"python\", \"argv.py\"]\n"
        ),
    );
    // Files in the cwd must not be globbed into `*`.
    project.write("globbed-file.txt", "");
    project
        .pyfaro(["run", "config"])
        .run()
        .success()
        .stdout_text_eq(&format!("{}\n", json_args(&matrix)));
    // The same matrix, plus an empty string, as extra arguments.
    let mut extra: Vec<&str> = matrix.to_vec();
    extra.push("");
    project
        .pyfaro(["run", "plain", "--"])
        .args(&extra)
        .run()
        .success()
        .stdout_text_eq(&format!("{}\n", json_args(&extra)));
}

#[test]
fn an_empty_string_in_a_command_array_is_rejected() {
    e2e_guard!();
    let sandbox = Sandbox::new();
    let project = project(&sandbox, "t = [\"python\", \"\"]\n");
    project.pyfaro(["run", "t"]).run().code(1).stderr_contains(
        "tool.pyfaro.tasks.t must be an array of non-empty strings",
    );
}

/// A script that proves a real shell parsed it: a pipe and `&&`.
fn pipe_and_and_script() -> &'static str {
    if cfg!(windows) {
        "echo hello | findstr hel && echo done"
    } else {
        "printf '%s\\n' hello | tr a-z A-Z && echo done"
    }
}

#[test]
fn shell_tasks_really_use_the_shell() {
    e2e_guard!();
    let sandbox = Sandbox::new();
    let project = project(
        &sandbox,
        &format!(
            "pipeline = {{ shell = {} }}\n",
            toml_string(pipe_and_and_script())
        ),
    );
    let outcome = project.pyfaro(["run", "pipeline"]).run();
    outcome.success();
    let expected: &[&str] =
        if cfg!(windows) { &["hello", "done"] } else { &["HELLO", "done"] };
    assert_eq!(trimmed_lines(&outcome.stdout), expected, "{}", outcome.stdout);
}

#[test]
fn shell_tasks_expand_variables_and_honor_cwd_and_env() {
    e2e_guard!();
    let sandbox = Sandbox::new();
    let (variable, directory) = if cfg!(windows) {
        ("echo %PYFARO_SHELL_VAR%", "cd")
    } else {
        ("echo $PYFARO_SHELL_VAR", "pwd")
    };
    let project = project(
        &sandbox,
        &format!(
            "expand = {{ shell = {}, env = {{ PYFARO_SHELL_VAR = \"expanded\" }} }}\n\
             where = {{ shell = {}, cwd = \"docs\" }}\n",
            toml_string(variable),
            toml_string(directory)
        ),
    );
    project.write("docs/.keep", "");
    project
        .pyfaro(["run", "expand"])
        .run()
        .success()
        .stdout_text_eq("expanded\n");
    let outcome = project.pyfaro(["run", "where"]).run();
    outcome.success();
    assert!(
        same_directory(&outcome.stdout, &project.path("docs")),
        "cwd was {:?}",
        outcome.stdout
    );
}

#[test]
fn shell_tasks_reject_extra_arguments() {
    e2e_guard!();
    let sandbox = Sandbox::new();
    let project = project(&sandbox, "s = { shell = \"echo hi\" }\n");
    project
        .pyfaro(["run", "s", "--", "; echo injected"])
        .run()
        .code(1)
        .stdout_empty()
        .stderr_contains(
            "task \"s\" is a shell task and does not accept extra arguments",
        );
    assert!(!project.path(".venv").exists(), "uv ran for a rejected task");
    // An empty list of extras is fine.
    project.pyfaro(["run", "s", "--"]).run().success().stdout_text_eq("hi\n");
}

#[cfg(unix)]
#[test]
fn unix_shell_scripts_may_contain_quotes() {
    e2e_guard!();
    let sandbox = Sandbox::new();
    let project =
        project(&sandbox, "q = { shell = \"echo \\\"a  b\\\" 'c  d'\" }\n");
    project.pyfaro(["run", "q"]).run().success().stdout_text_eq("a  b c  d\n");
}

#[cfg(windows)]
#[test]
fn windows_shell_scripts_with_a_double_quote_are_rejected_before_launch() {
    e2e_guard!();
    let sandbox = Sandbox::new();
    let project = project(
        &sandbox,
        "q = { shell = 'echo \"hi\"' }\nok = { shell = 'echo fine' }\n",
    );
    project
        .pyfaro(["run", "q"])
        .run()
        .code(1)
        .stdout_empty()
        .stderr_contains("task \"q\"")
        .stderr_contains("double quote");
    assert!(!project.path(".venv").exists(), "uv ran for a rejected script");
    // Other tasks are not affected by the rejection.
    project.pyfaro(["run", "ok"]).run().success();
}

#[cfg(any(target_os = "linux", target_os = "macos"))]
#[test]
fn ctrl_c_stops_a_task_and_leaves_no_process() {
    use nix::errno::Errno;
    use nix::sys::signal::{Signal, kill, killpg};
    use nix::unistd::Pid;
    use std::os::unix::process::CommandExt;
    use std::process::Stdio;
    use std::thread;
    use std::time::{Duration, Instant};

    e2e_guard!();
    let sandbox = Sandbox::new();
    let project = project(
        &sandbox,
        "slow = [\"python\", \"slow.py\"]\n\
         slow-shell = { shell = \"python slow.py\" }\n",
    );
    project.write(
        "slow.py",
        "import os, pathlib, time\n\
         pathlib.Path('ready.pid').write_text(str(os.getpid()))\n\
         time.sleep(120)\n",
    );
    for task in ["slow", "slow-shell"] {
        let _ = fs::remove_file(project.path("ready.pid"));
        let mut command = project.pyfaro(["run", task]).into_command();
        let mut child = command
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .process_group(0)
            .spawn()
            .expect("start pyfaro");
        let deadline = Instant::now() + Duration::from_secs(60);
        let pid = loop {
            if let Ok(text) = fs::read_to_string(project.path("ready.pid"))
                && let Ok(pid) = text.trim().parse::<i32>()
            {
                break pid;
            }
            if Instant::now() > deadline {
                let _ = child.kill();
                let _ = child.wait();
                panic!("the task did not start in time");
            }
            thread::sleep(Duration::from_millis(50));
        };
        killpg(Pid::from_raw(child.id() as i32), Signal::SIGINT).unwrap();
        let deadline = Instant::now() + Duration::from_secs(20);
        let status = loop {
            if let Some(status) = child.try_wait().unwrap() {
                break status;
            }
            if Instant::now() > deadline {
                let _ = child.kill();
                let _ = child.wait();
                panic!("pyfaro did not finish cancelling in time");
            }
            thread::sleep(Duration::from_millis(50));
        };
        assert_eq!(status.code(), Some(130), "task {task}");
        let deadline = Instant::now() + Duration::from_secs(10);
        while kill(Pid::from_raw(pid), None) != Err(Errno::ESRCH) {
            assert!(
                Instant::now() < deadline,
                "task {task}: process {pid} survived the cancellation"
            );
            thread::sleep(Duration::from_millis(50));
        }
    }
}
