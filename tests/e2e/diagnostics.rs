//! `--verbose`, secret redaction and the exit-code contract of
//! docs/exit-codes.md, with the real built binary and real uv, Ruff, ty and
//! pytest.

use std::path::PathBuf;

use crate::harness::{Cmd, Kind, Outcome, PYTEST, RUFF, Sandbox, TY, UV};

/// Passwords and tokens that must never be printed.
const PASSWORD: &str = "s3cr3t-pa55w0rd";
const TOKEN: &str = "t0ken-0nly-v4lue";

/// A virtual project (no dependencies, no build system) with these extra
/// `pyproject.toml` tables.
fn virtual_project(sandbox: &Sandbox, extra: &str) -> PathBuf {
    sandbox.write(
        "project/pyproject.toml",
        &format!(
            "[project]\nname = \"diag\"\nversion = \"0.1.0\"\n\
             requires-python = \">=3.10\"\ndependencies = []\n\n\
             [tool.pyfaro]\nschema-version = 1\n\n{extra}\n"
        ),
    );
    sandbox.join("project")
}

/// The lines Pyfaro itself printed for `command`, in order.
fn own_lines<'a>(out: &'a Outcome, command: &str) -> Vec<&'a str> {
    let prefix = format!("pyfaro {command}: ");
    out.stderr.lines().filter(|line| line.starts_with(&prefix)).collect()
}

fn position(lines: &[&str], predicate: impl Fn(&str) -> bool) -> usize {
    lines
        .iter()
        .position(|line| predicate(line))
        .unwrap_or_else(|| panic!("no matching line in {lines:#?}"))
}

/// Everything a run printed.
fn printed(out: &Outcome) -> String {
    format!("{}{}", out.stdout, out.stderr)
}

fn assert_no_secret(out: &Outcome, secrets: &[&str]) {
    let text = printed(out);
    for secret in secrets {
        assert!(!text.contains(secret), "{secret:?} leaked:\n{text}");
    }
}

// ------------------------------------------------------------------ verbose

#[test]
fn verbose_before_or_after_the_subcommand_traces_uv() {
    e2e_guard!();
    let sandbox = Sandbox::new();
    let root = virtual_project(&sandbox, "");
    for args in [["-v", "sync"], ["sync", "-v"], ["sync", "--verbose"]] {
        let out = Cmd::pyfaro(&root, args).run();
        out.success();
        let lines = own_lines(&out, "sync");
        let probe =
            position(&lines, |l| l.contains("uv") && l.ends_with(" --version"));
        let using = position(&lines, |l| {
            l.starts_with(&format!("pyfaro sync: using uv {UV} "))
                && l.contains(" at ")
        });
        let sync = position(&lines, |l| {
            l.contains("--no-python-downloads --no-progress sync")
        });
        assert!(probe < using && using < sync, "{lines:#?}");
        assert!(
            lines.iter().all(|line| !line.contains("working directory")),
            "the cwd is the project root: {lines:#?}"
        );
    }
    // Without the flag Pyfaro prints nothing of its own.
    let quiet = Cmd::pyfaro(&root, ["sync"]).run();
    quiet.success();
    assert!(own_lines(&quiet, "sync").is_empty(), "{}", quiet.stderr);
    assert!(!quiet.stderr.contains("running"), "{}", quiet.stderr);
}

#[test]
fn verbose_run_traces_programs_and_names_task_variables_only() {
    e2e_guard!();
    let sandbox = Sandbox::new();
    let root = virtual_project(
        &sandbox,
        "[tool.pyfaro.tasks]\n\
         probe = { command = [\"python\", \"-c\", \"import os; print(len(os.environ['DIAG_TOKEN']))\"], env = { DIAG_TOKEN = \"v4lue-of-the-t0ken\", DIAG_MODE = \"m0de-v4lue\" } }\n",
    );

    // A program: the command line follows `--`, with the project root as
    // working directory (so no working-directory line).
    let out =
        Cmd::pyfaro(&root, ["-v", "run", "--", "python", "-c", "print('hi')"])
            .run();
    out.success()
        .stdout_eq(&format!("hi{}", if cfg!(windows) { "\r\n" } else { "\n" }));
    let lines = own_lines(&out, "run");
    assert!(
        lines.iter().any(|l| l.starts_with("pyfaro run: using uv ")),
        "{lines:#?}"
    );
    assert!(
        lines.iter().any(|l| l.contains(" run --project ")
            && l.ends_with(" -- python -c \"print('hi')\"")),
        "{lines:#?}"
    );
    assert!(lines.iter().all(|l| !l.contains("working directory")));

    // A program runs in the current directory: say so when it is not the root.
    let sub = root.join("sub");
    std::fs::create_dir(&sub).unwrap();
    let out =
        Cmd::pyfaro(&sub, ["run", "-v", "--", "python", "-c", "pass"]).run();
    out.success();
    let lines = own_lines(&out, "run");
    assert!(
        lines.iter().any(|l| l.starts_with("pyfaro run: working directory ")
            && l.ends_with("sub")),
        "{lines:#?}"
    );

    // A task with `env`: the names are shown, the values never.
    let out = Cmd::pyfaro(&root, ["--verbose", "run", "probe"]).run();
    out.success().stdout_text_eq("18\n");
    let lines = own_lines(&out, "run");
    let variables = lines
        .iter()
        .find(|l| l.contains("environment variables set"))
        .unwrap_or_else(|| panic!("{lines:#?}"));
    assert!(
        variables.contains("DIAG_TOKEN") && variables.contains("DIAG_MODE"),
        "{variables}"
    );
    assert_no_secret(&out, &["v4lue-of-the-t0ken", "m0de-v4lue"]);
    // The task's command line is traced as well.
    assert!(
        lines.iter().any(|l| l.contains("import os; print(len(os.environ")),
        "{lines:#?}"
    );

    // Without the flag the output has no Pyfaro line at all.
    let out = Cmd::pyfaro(&root, ["run", "probe"]).run();
    out.success().stdout_text_eq("18\n").stderr_empty();
}

#[test]
fn verbose_quality_commands_report_each_tool_and_command() {
    e2e_guard!();
    let sandbox = Sandbox::new();
    let project = sandbox.synced_project(Kind::Lib);

    for (args, command, tool, version) in [
        (vec!["lint", "-v"], "lint", "ruff", RUFF),
        (vec!["fmt", "--check", "--verbose"], "fmt", "ruff", RUFF),
        (vec!["-v", "type"], "type", "ty", TY),
    ] {
        let out = project.pyfaro(&args).run();
        out.success();
        let lines = own_lines(&out, command);
        let using = position(&lines, |l| {
            l.starts_with(&format!("pyfaro {command}: using {tool} {version}"))
        });
        // The tool's own command line is the last thing traced.
        let last = lines.len() - 1;
        assert!(using < last, "{lines:#?}");
        assert!(
            lines[last].contains(": running ")
                && lines[last].contains(tool)
                && !lines[last].ends_with("version"),
            "{lines:#?}"
        );
        assert!(
            lines.iter().all(|l| !l.contains("working directory")),
            "{lines:#?}"
        );
    }
    // Without the flag the successful output is exactly the tool's own.
    project.pyfaro(["lint"]).run().success().stderr_empty();

    // `check` resolves each tool once and traces its three steps.
    let out = project.pyfaro(["check", "-v"]).run();
    out.success();
    let lines = own_lines(&out, "check");
    let using: Vec<_> =
        lines.iter().filter(|l| l.contains(": using ")).collect();
    assert_eq!(using.len(), 2, "{lines:#?}");
    assert!(
        using[0].contains(&format!("ruff {RUFF}"))
            && using[1].contains(&format!("ty {TY}"))
    );
    let steps: Vec<_> = lines
        .iter()
        .filter(|l| {
            l.contains(": running ")
                && !l.ends_with(" --version")
                && !l.ends_with(" version")
        })
        .collect();
    assert_eq!(steps.len(), 3, "{lines:#?}");
    assert!(steps[0].ends_with(" format --check"), "{}", steps[0]);
    assert!(
        steps[1].contains("ruff") && steps[1].ends_with(" check"),
        "{}",
        steps[1]
    );
    assert!(steps[2].contains(" check --python "), "{}", steps[2]);
    assert!(out.stderr.contains("check: ok"), "{}", out.stderr);
}

#[test]
fn verbose_test_is_pyfaros_only_before_the_subcommand() {
    e2e_guard!();
    let sandbox = Sandbox::new();
    let project = sandbox.synced_project(Kind::Lib);

    // `pyfaro -v test`: Pyfaro's trace, pytest stays quiet.
    let out = project.pyfaro(["-v", "test"]).run();
    out.success();
    let lines = own_lines(&out, "test");
    assert!(
        lines.iter().any(|l| l
            .starts_with(&format!("pyfaro test: using pytest {PYTEST} at "))),
        "{lines:#?}"
    );
    assert!(lines.iter().any(|l| l.ends_with(" -m pytest")), "{lines:#?}");
    assert!(!out.stdout.contains("PASSED"), "{}", out.stdout);

    // After `test` the flags belong to pytest: verbose pytest, no trace.
    for flag in ["-v", "--verbose"] {
        let out = project.pyfaro(["test", flag]).run();
        out.success().stdout_contains("PASSED");
        assert!(own_lines(&out, "test").is_empty(), "{flag}: {}", out.stderr);
        assert!(!out.stderr.contains("running"), "{flag}: {}", out.stderr);
    }
    // `test --help` has no Pyfaro verbose option to document.
    let help = project.pyfaro(["test", "--help"]).run();
    help.success();
    assert!(!help.stdout.contains("--verbose"), "{}", help.stdout);
}

// ---------------------------------------------------------------- redaction

#[test]
fn credentials_in_arguments_are_masked_in_the_trace() {
    e2e_guard!();
    let sandbox = Sandbox::new();
    let root = virtual_project(&sandbox, "");
    let with_password =
        format!("--index-url=https://deploy:{PASSWORD}@example.invalid/simple");
    let with_token = format!("https://{TOKEN}@example.invalid/files/x.whl");
    let out = Cmd::pyfaro(
        &root,
        ["-v", "run", "--offline", "--", "python", "-c", "pass"],
    )
    .args([&with_password, &with_token])
    .run();
    out.success();
    assert_no_secret(&out, &[PASSWORD, TOKEN]);
    out.stderr_contains(
        "--index-url=https://deploy:****@example.invalid/simple",
    );
    out.stderr_contains(" https://****@example.invalid/files/x.whl");
}

#[test]
fn a_real_uv_invocation_with_a_credentialed_url_never_shows_the_password() {
    e2e_guard!();
    let sandbox = Sandbox::new();
    let project = sandbox.project(Kind::Lib);
    let requirement = format!(
        "demo @ https://deploy:{PASSWORD}@example.invalid/demo-1.0-py3-none-any.whl"
    );
    let out =
        project.pyfaro(["-v", "add", "--offline", "--", &requirement]).run();
    // uv cannot fetch the wheel offline: its failure is forwarded.
    out.code(1).stderr_contains("error: uv add failed");
    assert_no_secret(&out, &[PASSWORD]);
    out.stderr_contains(
        " https://deploy:****@example.invalid/demo-1.0-py3-none-any.whl",
    );
}

#[test]
fn error_messages_mask_credentials_in_arguments() {
    e2e_guard!();
    let sandbox = Sandbox::new();
    let root = virtual_project(&sandbox, "");

    // Pyfaro's own error echoes the selected uv.
    let out = Cmd::pyfaro(
        &root,
        ["sync", "--uv", &format!("https://{TOKEN}@example.invalid/bin/uv")],
    )
    .run();
    out.code(1).stdout_empty().stderr_contains(
        "error: could not find uv at https://****@example.invalid/bin/uv",
    );
    assert_no_secret(&out, &[TOKEN]);

    // A task name is echoed by the "no task named" error.
    let out = Cmd::pyfaro(
        &root,
        ["run", &format!("https://u:{PASSWORD}@h.invalid/x")],
    )
    .run();
    out.code(1).stdout_empty().stderr_contains("https://u:****@h.invalid/x");
    assert_no_secret(&out, &[PASSWORD]);

    // Clap quotes the offending argument of a usage error.
    let out = Cmd::pyfaro(
        &root,
        [
            "sync".to_owned(),
            ".".to_owned(),
            format!("https://u:{PASSWORD}@h.invalid/x"),
        ],
    )
    .run();
    out.code(2)
        .stdout_empty()
        .stderr_contains("error: unexpected argument")
        .stderr_contains("https://u:****@h.invalid/x");
    assert_no_secret(&out, &[PASSWORD]);
}

#[test]
fn doctor_masks_credentials_echoed_from_the_configuration() {
    e2e_guard!();
    let sandbox = Sandbox::new();
    sandbox.write(
        "project/pyproject.toml",
        &format!(
            "[project]\nname = \"diag\"\nversion = \"0.1.0\"\n\
             requires-python = \">=3.10\"\n\
             dependencies = [\"pkg @ https://deploy:{PASSWORD}@example.invalid/pkg.whl ; python_version >>> '3'\"]\n\n\
             [tool.pyfaro]\nschema-version = 1\n"
        ),
    );
    let root = sandbox.join("project");
    for args in [vec!["doctor"], vec!["-v", "doctor"]] {
        let out = Cmd::pyfaro(&root, args).run();
        out.code(1)
            .stdout_contains("ERROR: dependencies: invalid requirement")
            .stdout_contains("https://deploy:****@example.invalid/pkg.whl");
        assert_no_secret(&out, &[PASSWORD]);
    }
}

// ---------------------------------------------------------------- exit codes

/// An executable shell script that, unless it is asked for its version, kills
/// itself, so the parent sees a status without a numeric code.
#[cfg(unix)]
fn killed_tool(path: &std::path::Path, version: &str) {
    use std::os::unix::fs::PermissionsExt;
    std::fs::create_dir_all(path.parent().unwrap()).unwrap();
    std::fs::write(
        path,
        format!("#!/bin/sh\nif [ \"$1\" = \"--version\" ]; then echo '{version}'; exit 0; fi\nkill -9 $$\n"),
    )
    .unwrap();
    std::fs::set_permissions(path, std::fs::Permissions::from_mode(0o755))
        .unwrap();
}

#[cfg(unix)]
#[test]
fn a_tool_that_ends_without_a_numeric_status_exits_1() {
    e2e_guard!();
    let sandbox = Sandbox::new();

    // uv, forwarded by `sync`.
    let root = virtual_project(&sandbox, "");
    let uv = sandbox.join("bin/uv");
    killed_tool(&uv, &format!("uv {UV}"));
    Cmd::pyfaro(&root, ["sync", "--uv"])
        .arg(&uv)
        .run()
        .code(1)
        .stdout_empty()
        .stderr_contains("error: uv sync failed (signal: 9");

    // Ruff, forwarded by `lint`, and the same failure inside `check`.
    let other = Sandbox::new();
    let project = other.project(Kind::Lib);
    killed_tool(&project.path(".venv/bin/ruff"), &format!("ruff {RUFF}"));
    project
        .pyfaro(["lint"])
        .run()
        .code(1)
        .stdout_empty()
        .stderr_contains("error: ruff failed (signal: 9");
    let out = project.pyfaro(["check"]).run();
    out.code(1)
        .stderr_contains("format: failed (terminated by signal)")
        .stderr_contains("lint: failed (terminated by signal)")
        .stderr_contains("check: failed (");
}
