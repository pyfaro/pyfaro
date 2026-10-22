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

fn project(with_python: bool) -> Fixture {
    let sandbox = tempdir().unwrap();
    let project = sandbox.path().join("project");
    fs::create_dir_all(project.join("tests/nested")).unwrap();
    fs::write(project.join("pyproject.toml"), "[project]\nname = 'example'\n")
        .unwrap();
    if with_python {
        let directory = project.join(".venv").join(scripts());
        fs::create_dir_all(&directory).unwrap();
        let binary =
            directory.join(format!("python{}", std::env::consts::EXE_SUFFIX));
        fs::copy(support::fixture_uv(), binary).unwrap();
    }
    let record = sandbox.path().join("record.txt");
    Fixture { _sandbox: sandbox, project, record }
}

fn pyfaro(
    fixture: &Fixture,
    cwd: &Path,
    args: &[&str],
    exit: &str,
    no_pytest: bool,
) -> Output {
    let empty = fixture.record.parent().unwrap().join("empty");
    fs::create_dir_all(&empty).unwrap();
    let mut command = Command::new(env!("CARGO_BIN_EXE_pyfaro"));
    command
        .arg("test")
        .args(args)
        .current_dir(cwd)
        .env("PATH", &empty)
        .env_remove("VIRTUAL_ENV")
        .env_remove("UV_PROJECT_ENVIRONMENT")
        .env_remove("PYFARO_FIXTURE_NO_PYTEST")
        .env("PYFARO_FIXTURE_TOOL_RECORD", &fixture.record)
        .env("PYFARO_FIXTURE_TOOL_EXIT", exit);
    if no_pytest {
        command.env("PYFARO_FIXTURE_NO_PYTEST", "1");
    }
    command.output().unwrap()
}

/// The most recent invocation; the fixture appends one block per run.
fn recorded(fixture: &Fixture) -> (PathBuf, Vec<String>) {
    let text = fs::read_to_string(&fixture.record).unwrap();
    let block = text.trim_end().trim_end_matches("--end--");
    let block = block.rsplit("--end--\n").next().unwrap();
    let mut lines = block.lines();
    let cwd = PathBuf::from(lines.next().unwrap());
    (fs::canonicalize(cwd).unwrap(), lines.map(String::from).collect())
}

#[test]
fn forwards_arguments_verbatim_with_project_root_cwd() {
    let fixture = project(true);
    let nested = fixture.project.join("tests/nested");
    let output = pyfaro(
        &fixture,
        &nested,
        &["-k", "a and not b", "-m", "slow", "tests/t.py::test_x[1]", "-x"],
        "0",
        false,
    );
    assert!(output.status.success(), "{output:?}");
    let (cwd, args) = recorded(&fixture);
    assert_eq!(cwd, fs::canonicalize(&fixture.project).unwrap());
    assert_eq!(
        args,
        [
            "-m",
            "pytest",
            "-k",
            "a and not b",
            "-m",
            "slow",
            "tests/t.py::test_x[1]",
            "-x"
        ]
    );
}

#[test]
fn separator_and_no_arguments_work() {
    let fixture = project(true);
    let output = pyfaro(&fixture, &fixture.project, &["--", "-x"], "0", false);
    assert!(output.status.success(), "{output:?}");
    assert_eq!(recorded(&fixture).1, ["-m", "pytest", "-x"]);
    let output = pyfaro(&fixture, &fixture.project, &[], "0", false);
    assert!(output.status.success(), "{output:?}");
    assert_eq!(recorded(&fixture).1, ["-m", "pytest"]);
}

#[test]
fn project_option_precedes_forwarded_arguments() {
    let fixture = project(true);
    let elsewhere = fixture.record.parent().unwrap().to_path_buf();
    let project = fixture.project.to_str().unwrap().to_owned();
    let output = pyfaro(
        &fixture,
        &elsewhere,
        &["--project", &project, "-k", "foo", "tests/"],
        "0",
        false,
    );
    assert!(output.status.success(), "{output:?}");
    let (cwd, args) = recorded(&fixture);
    assert_eq!(cwd, fs::canonicalize(&fixture.project).unwrap());
    assert_eq!(args, ["-m", "pytest", "-k", "foo", "tests/"]);
}

#[test]
fn pytest_exit_codes_are_forwarded_exactly() {
    let fixture = project(true);
    for code in [1, 2, 3, 4, 5] {
        let output = pyfaro(
            &fixture,
            &fixture.project,
            &["-x"],
            &code.to_string(),
            false,
        );
        assert_eq!(output.status.code(), Some(code), "{output:?}");
    }
}

#[test]
fn missing_environment_asks_for_sync() {
    let fixture = project(false);
    let output = pyfaro(&fixture, &fixture.project, &[], "5", false);
    assert_eq!(output.status.code(), Some(1), "{output:?}");
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(stderr.contains("pyfaro sync"), "{stderr}");
    assert!(!fixture.record.exists());
    assert!(!fixture.project.join("uv.lock").exists());
}

#[test]
fn missing_pytest_is_reported_by_pyfaro() {
    let fixture = project(true);
    let output = pyfaro(&fixture, &fixture.project, &[], "5", true);
    assert_eq!(output.status.code(), Some(1), "{output:?}");
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(stderr.contains("pyfaro add --dev pytest"), "{stderr}");
    assert!(stderr.contains("pyfaro sync"), "{stderr}");
    assert!(!fixture.record.exists());
}
