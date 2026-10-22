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
        .output()
        .unwrap()
}

/// Returns the recorded (cwd, argv).
fn recorded(fixture: &Fixture) -> (PathBuf, Vec<String>) {
    let text = fs::read_to_string(&fixture.record).unwrap();
    let mut lines = text.lines();
    let cwd = PathBuf::from(lines.next().unwrap());
    (fs::canonicalize(cwd).unwrap(), lines.map(String::from).collect())
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
