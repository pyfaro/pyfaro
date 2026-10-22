mod support;

use std::fs;
use std::path::{Path, PathBuf};
use std::process::{Command, Output};

use tempfile::tempdir;

struct Fixture {
    _sandbox: tempfile::TempDir,
    project: PathBuf,
    record: PathBuf,
}

fn scripts() -> &'static str {
    if cfg!(windows) { "Scripts" } else { "bin" }
}

fn install(directory: &Path, name: &str) {
    fs::create_dir_all(directory).unwrap();
    let binary =
        directory.join(format!("{name}{}", std::env::consts::EXE_SUFFIX));
    fs::copy(support::fixture_uv(), binary).unwrap();
}

fn project(with_ruff: bool) -> Fixture {
    let sandbox = tempdir().unwrap();
    let project = sandbox.path().join("project");
    fs::create_dir_all(project.join("src/nested")).unwrap();
    fs::write(project.join("pyproject.toml"), "[project]\nname = 'example'\n")
        .unwrap();
    fs::write(project.join("src/nested/mod.py"), "").unwrap();
    if with_ruff {
        install(&project.join(".venv").join(scripts()), "ruff");
    }
    let record = sandbox.path().join("record.txt");
    Fixture { _sandbox: sandbox, project, record }
}

fn pyfaro(fixture: &Fixture, cwd: &Path, args: &[&str], exit: &str) -> Output {
    pyfaro_when(fixture, cwd, args, exit, "")
}

/// Like `pyfaro`, but `when` ("<tool> <args>=<code>") overrides the exit code
/// of one exact tool invocation.
fn pyfaro_when(
    fixture: &Fixture,
    cwd: &Path,
    args: &[&str],
    exit: &str,
    when: &str,
) -> Output {
    let empty = fixture.record.parent().unwrap().join("empty");
    fs::create_dir_all(&empty).unwrap();
    Command::new(env!("CARGO_BIN_EXE_pyfaro"))
        .args(args)
        .current_dir(cwd)
        .env("PATH", &empty)
        .env_remove("VIRTUAL_ENV")
        .env_remove("UV_PROJECT_ENVIRONMENT")
        .env("PYFARO_FIXTURE_TOOL_RECORD", &fixture.record)
        .env("PYFARO_FIXTURE_TOOL_EXIT", exit)
        .env("PYFARO_FIXTURE_TOOL_EXIT_WHEN", when)
        .output()
        .unwrap()
}

/// Every recorded invocation, in order, as (cwd, argv).
fn calls(fixture: &Fixture) -> Vec<(PathBuf, Vec<String>)> {
    let text = fs::read_to_string(&fixture.record).unwrap();
    let mut lines = text.lines();
    let mut calls = Vec::new();
    while let Some(cwd) = lines.next() {
        let argv: Vec<String> = lines
            .by_ref()
            .take_while(|line| *line != "--end--")
            .map(String::from)
            .collect();
        calls.push((fs::canonicalize(cwd).unwrap(), argv));
    }
    calls
}

/// Returns the last recorded (cwd, argv).
fn recorded(fixture: &Fixture) -> (PathBuf, Vec<String>) {
    calls(fixture).pop().unwrap()
}

fn same(a: &str, b: &Path) -> bool {
    fs::canonicalize(a).unwrap() == fs::canonicalize(b).unwrap()
}

#[test]
fn lint_without_paths_runs_ruff_check_in_project_root() {
    let fixture = project(true);
    let nested = fixture.project.join("src/nested");
    let output = pyfaro(&fixture, &nested, &["lint"], "0");
    assert!(output.status.success(), "{output:?}");
    let (cwd, args) = recorded(&fixture);
    assert_eq!(cwd, fs::canonicalize(&fixture.project).unwrap());
    assert_eq!(args, ["check"]);
}

#[test]
fn lint_with_paths_forces_exclusions_and_makes_paths_absolute() {
    let fixture = project(true);
    let nested = fixture.project.join("src/nested");
    let output = pyfaro(&fixture, &nested, &["lint", "mod.py", "."], "0");
    assert!(output.status.success(), "{output:?}");
    let (cwd, args) = recorded(&fixture);
    assert_eq!(cwd, fs::canonicalize(&fixture.project).unwrap());
    assert_eq!(args.len(), 4, "{args:?}");
    assert_eq!(args[..2], ["check", "--force-exclude"]);
    assert!(same(&args[2], &nested.join("mod.py")), "{args:?}");
    assert!(same(&args[3], &nested), "{args:?}");
    assert!(Path::new(&args[2]).is_absolute());
}

#[test]
fn project_option_selects_exact_project_from_elsewhere() {
    let fixture = project(true);
    let elsewhere = fixture.record.parent().unwrap().to_path_buf();
    let project = fixture.project.to_str().unwrap().to_owned();
    let output =
        pyfaro(&fixture, &elsewhere, &["lint", "--project", &project], "0");
    assert!(output.status.success(), "{output:?}");
    let (cwd, args) = recorded(&fixture);
    assert_eq!(cwd, fs::canonicalize(&fixture.project).unwrap());
    assert_eq!(args, ["check"]);
}

#[test]
fn ruff_exit_code_is_forwarded_exactly() {
    let fixture = project(true);
    let output = pyfaro(&fixture, &fixture.project, &["lint"], "3");
    assert_eq!(output.status.code(), Some(3), "{output:?}");
    let output = pyfaro(&fixture, &fixture.project, &["fmt", "--check"], "1");
    assert_eq!(output.status.code(), Some(1), "{output:?}");
    let output = pyfaro(&fixture, &fixture.project, &["fmt"], "2");
    assert_eq!(output.status.code(), Some(2), "{output:?}");
}

#[test]
fn fmt_writes_by_default_and_checks_on_request() {
    let fixture = project(true);
    let output = pyfaro(&fixture, &fixture.project, &["fmt"], "0");
    assert!(output.status.success(), "{output:?}");
    assert_eq!(recorded(&fixture).1, ["format"]);

    let output = pyfaro(&fixture, &fixture.project, &["fmt", "--check"], "0");
    assert!(output.status.success(), "{output:?}");
    assert_eq!(recorded(&fixture).1, ["format", "--check"]);

    let output = pyfaro(
        &fixture,
        &fixture.project.join("src"),
        &["fmt", "--check", "nested"],
        "0",
    );
    assert!(output.status.success(), "{output:?}");
    let (_, args) = recorded(&fixture);
    assert_eq!(args[..3], ["format", "--check", "--force-exclude"]);
    assert!(same(&args[3], &fixture.project.join("src/nested")));
}

#[test]
fn missing_ruff_is_actionable_and_does_not_touch_the_environment() {
    let fixture = project(false);
    for args in [&["lint"][..], &["fmt"], &["fmt", "--check"]] {
        let output = pyfaro(&fixture, &fixture.project, args, "0");
        assert_eq!(output.status.code(), Some(1), "{output:?}");
        let stderr = String::from_utf8(output.stderr).unwrap();
        assert!(stderr.contains("ruff"), "{stderr}");
        assert!(stderr.contains("pyfaro add --dev ruff"), "{stderr}");
        assert!(stderr.contains("pyfaro sync"), "{stderr}");
        assert!(output.stdout.is_empty());
    }
    assert!(!fixture.project.join(".venv").exists());
    assert!(!fixture.project.join("uv.lock").exists());
    assert!(!fixture.record.exists());
}

#[test]
fn outside_a_project_fails_with_exit_one() {
    let fixture = project(true);
    let elsewhere = fixture.record.parent().unwrap().to_path_buf();
    let output = pyfaro(&fixture, &elsewhere, &["lint"], "0");
    assert_eq!(output.status.code(), Some(1), "{output:?}");
    assert!(!fixture.record.exists());
}

fn with_ty(fixture: &Fixture) {
    install(&fixture.project.join(".venv").join(scripts()), "ty");
}

#[test]
fn type_passes_the_environment_and_absolute_forced_paths() {
    let fixture = project(false);
    with_ty(&fixture);
    let sub = fixture.project.join("src");
    let output = pyfaro(
        &fixture,
        &sub,
        &["type", "nested", "--project", fixture.project.to_str().unwrap()],
        "0",
    );
    assert_eq!(output.status.code(), Some(0), "{output:?}");
    let (cwd, argv) = recorded(&fixture);
    assert_eq!(cwd, fs::canonicalize(&fixture.project).unwrap());
    assert_eq!(argv[0], "check");
    assert_eq!(argv[1], "--python");
    assert!(same(&argv[2], &fixture.project.join(".venv")), "{argv:?}");
    assert_eq!(argv[3], "--force-exclude");
    assert!(same(&argv[4], &sub.join("nested")), "{argv:?}");
    assert!(Path::new(&argv[4]).is_absolute());
    assert_eq!(argv.len(), 5);
}

#[test]
fn type_without_paths_and_environment_omits_python() {
    let fixture = project(false);
    // ty lives on PATH-less discovery only through the environment, so use a
    // VIRTUAL_ENV that holds ty while the project environment does not exist.
    let other = fixture.record.parent().unwrap().join("other");
    install(&other.join(scripts()), "ty");
    let empty = fixture.record.parent().unwrap().join("empty");
    fs::create_dir_all(&empty).unwrap();
    let output = Command::new(env!("CARGO_BIN_EXE_pyfaro"))
        .args(["type"])
        .current_dir(&fixture.project)
        .env("PATH", &empty)
        .env("VIRTUAL_ENV", &other)
        .env_remove("UV_PROJECT_ENVIRONMENT")
        .env("PYFARO_FIXTURE_TOOL_RECORD", &fixture.record)
        .env("PYFARO_FIXTURE_TOOL_EXIT", "0")
        .output()
        .unwrap();
    assert_eq!(output.status.code(), Some(0), "{output:?}");
    let (_, argv) = recorded(&fixture);
    assert_eq!(argv, ["check"]);
    assert!(!fixture.project.join(".venv").exists());
}

#[test]
fn type_forwards_the_exit_code() {
    let fixture = project(false);
    with_ty(&fixture);
    let output = pyfaro(&fixture, &fixture.project, &["type"], "2");
    assert_eq!(output.status.code(), Some(2), "{output:?}");
}

#[test]
fn missing_ty_is_actionable() {
    let fixture = project(true);
    let output = pyfaro(&fixture, &fixture.project, &["type"], "0");
    assert_eq!(output.status.code(), Some(1), "{output:?}");
    let stderr = String::from_utf8(output.stderr).unwrap();
    assert!(stderr.contains("pyfaro add --dev ty"), "{stderr}");
    assert!(!fixture.record.exists());
}

fn checked_project() -> Fixture {
    let fixture = project(true);
    with_ty(&fixture);
    fixture
}

fn argvs(fixture: &Fixture) -> Vec<Vec<String>> {
    calls(fixture).into_iter().map(|(_, argv)| argv).collect()
}

/// Assert a ty argv: `check --python <venv> [rest...]`.
fn assert_ty(fixture: &Fixture, argv: &[String], rest: &[&str]) {
    assert_eq!(argv[0], "check", "{argv:?}");
    assert_eq!(argv[1], "--python", "{argv:?}");
    assert!(same(&argv[2], &fixture.project.join(".venv")), "{argv:?}");
    assert_eq!(&argv[3..], rest, "{argv:?}");
}

fn tree(root: &Path) -> Vec<(PathBuf, Vec<u8>)> {
    let mut out = Vec::new();
    let mut stack = vec![root.to_path_buf()];
    while let Some(dir) = stack.pop() {
        for entry in fs::read_dir(dir).unwrap() {
            let path = entry.unwrap().path();
            if path.is_dir() {
                stack.push(path);
            } else if !path.starts_with(root.join(".venv")) {
                out.push((path.clone(), fs::read(path).unwrap()));
            }
        }
    }
    out.sort();
    out
}

#[test]
fn check_runs_all_steps_in_order_and_never_writes() {
    let fixture = checked_project();
    let before = tree(&fixture.project);
    let output = pyfaro(&fixture, &fixture.project, &["check"], "0");
    assert_eq!(output.status.code(), Some(0), "{output:?}");
    let all = argvs(&fixture);
    assert_eq!(all.len(), 3, "{all:?}");
    assert_eq!(all[0], ["format", "--check"]);
    assert_eq!(all[1], ["check"]);
    assert_ty(&fixture, &all[2], &[]);
    assert!(all.iter().flatten().all(|arg| arg != "--fix"));
    assert_eq!(tree(&fixture.project), before);
    let stderr = String::from_utf8(output.stderr).unwrap();
    for line in ["format: ok", "lint: ok", "type: ok", "check: ok"] {
        assert!(stderr.lines().any(|l| l == line), "{stderr}");
    }
    assert!(output.stdout.is_empty());
}

#[test]
fn check_runs_every_step_even_when_the_first_fails() {
    let fixture = checked_project();
    let output = pyfaro_when(
        &fixture,
        &fixture.project,
        &["check"],
        "0",
        "ruff format --check=1",
    );
    assert_eq!(output.status.code(), Some(1), "{output:?}");
    let all = argvs(&fixture);
    assert_eq!(all.len(), 3, "{all:?}");
    assert_eq!(all[0], ["format", "--check"]);
    assert_eq!(all[1], ["check"]);
    assert_ty(&fixture, &all[2], &[]);
    let stderr = String::from_utf8(output.stderr).unwrap();
    assert!(stderr.lines().any(|l| l == "format: failed (exit 1)"), "{stderr}");
    assert!(stderr.lines().any(|l| l == "lint: ok"), "{stderr}");
    assert!(stderr.lines().any(|l| l == "check: failed (format)"), "{stderr}");
}

#[test]
fn check_failing_type_step_is_marked_and_exits_one() {
    let fixture = checked_project();
    let output =
        pyfaro_when(&fixture, &fixture.project, &["check"], "0", "ty check=2");
    assert_eq!(output.status.code(), Some(1), "{output:?}");
    let stderr = String::from_utf8(output.stderr).unwrap();
    assert!(stderr.lines().any(|l| l == "format: ok"), "{stderr}");
    assert!(stderr.lines().any(|l| l == "type: failed (exit 2)"), "{stderr}");
    assert!(stderr.lines().any(|l| l == "check: failed (type)"), "{stderr}");
}

#[test]
fn check_missing_ty_is_unavailable_but_ruff_still_runs() {
    let fixture = project(true);
    let output = pyfaro(&fixture, &fixture.project, &["check"], "0");
    assert_eq!(output.status.code(), Some(1), "{output:?}");
    let all = argvs(&fixture);
    assert_eq!(all, [vec!["format", "--check"], vec!["check"]]);
    let stderr = String::from_utf8(output.stderr).unwrap();
    assert!(stderr.lines().any(|l| l == "format: ok"), "{stderr}");
    assert!(stderr.lines().any(|l| l == "lint: ok"), "{stderr}");
    assert!(stderr.contains("type: unavailable (ty not found"), "{stderr}");
    assert!(stderr.lines().any(|l| l == "check: failed (type)"), "{stderr}");
}

#[test]
fn check_forwards_paths_to_every_step() {
    let fixture = checked_project();
    let sub = fixture.project.join("src");
    let output = pyfaro(&fixture, &sub, &["check", "nested"], "0");
    assert_eq!(output.status.code(), Some(0), "{output:?}");
    let all = argvs(&fixture);
    assert_eq!(all.len(), 3);
    let target = sub.join("nested");
    assert_eq!(all[0][..2], ["format", "--check"]);
    assert_eq!(all[0][2], "--force-exclude");
    assert!(same(&all[0][3], &target));
    assert_eq!(all[1][..2], ["check", "--force-exclude"]);
    assert!(same(&all[1][2], &target));
    assert_eq!(all[2][..1], ["check"]);
    assert_eq!(all[2][3], "--force-exclude");
    assert!(same(&all[2][4], &target));
}

#[test]
fn check_fix_formats_fixes_then_reverifies_without_writing() {
    let fixture = checked_project();
    let output = pyfaro(&fixture, &fixture.project, &["check", "--fix"], "0");
    assert_eq!(output.status.code(), Some(0), "{output:?}");
    let all = argvs(&fixture);
    assert_eq!(all.len(), 5, "{all:?}");
    assert_eq!(all[0], ["format"]);
    assert_eq!(all[1], ["check", "--fix"]);
    assert_eq!(all[2], ["format", "--check"]);
    assert_eq!(all[3], ["check"]);
    assert_ty(&fixture, &all[4], &[]);
    assert!(!all[4].iter().any(|arg| arg == "--fix"));
    let stderr = String::from_utf8(output.stderr).unwrap();
    assert!(stderr.contains("type has no automatic fixes"), "{stderr}");
    assert!(stderr.lines().any(|l| l == "check: ok"), "{stderr}");
}

#[test]
fn check_fix_failure_does_not_skip_verification_and_reports_remaining() {
    let fixture = checked_project();
    let output = pyfaro_when(
        &fixture,
        &fixture.project,
        &["check", "--fix"],
        "0",
        "ruff check --fix=2",
    );
    assert_eq!(
        output.status.code(),
        Some(0),
        "verification passes: {output:?}"
    );
    let all = argvs(&fixture);
    assert_eq!(all.len(), 5, "{all:?}");
    let stderr = String::from_utf8(output.stderr).unwrap();
    assert!(stderr.contains("fix lint: failed (exit 2)"), "{stderr}");

    let output = pyfaro_when(
        &fixture,
        &fixture.project,
        &["check", "--fix"],
        "0",
        "ruff format --check=1",
    );
    assert_eq!(output.status.code(), Some(1), "{output:?}");
    let stderr = String::from_utf8(output.stderr).unwrap();
    assert!(stderr.contains("check: failed (format)"), "{stderr}");
}

#[test]
fn check_without_a_project_fails_before_running_anything() {
    let fixture = checked_project();
    let elsewhere = fixture.record.parent().unwrap().to_path_buf();
    let output = pyfaro(&fixture, &elsewhere, &["check"], "0");
    assert_eq!(output.status.code(), Some(1), "{output:?}");
    assert!(!fixture.record.exists());
}
