use std::fs;
use std::path::Path;
use std::process::{Command, Output};

use tempfile::tempdir;

mod support;

fn pyfaro(args: &[&str], target: &Path) -> Output {
    Command::new(env!("CARGO_BIN_EXE_pyfaro"))
        .arg("create")
        .args(args)
        .args(["--path"])
        .arg(target)
        .output()
        .expect("start pyfaro")
}

fn uv(project: &Path, args: &[&str]) -> Output {
    Command::new("uv")
        .current_dir(project)
        .args(["--no-python-downloads", "--no-progress"])
        .args(args)
        .output()
        .expect("start uv")
}

fn sync(project: &Path) -> Output {
    Command::new(env!("CARGO_BIN_EXE_pyfaro"))
        .arg("sync")
        .arg(project)
        .output()
        .unwrap()
}

fn pyfaro_in(project: &Path, args: &[&str]) -> Output {
    Command::new(env!("CARGO_BIN_EXE_pyfaro"))
        .current_dir(project)
        .args(args)
        .output()
        .unwrap()
}

/// Sync with the pinned quality tools, then check, test, and build. Needs the
/// network, so it only runs when `PYFARO_REAL_QUALITY_TOOLS=1`.
fn sync_check_test_build(project: &Path) -> bool {
    if !support::real_quality_tools() {
        return false;
    }
    assert_pinned_uv();
    assert_success(sync(project));
    assert!(project.join("uv.lock").is_file());
    assert_success(pyfaro_in(project, &["check"]));
    assert_success(pyfaro_in(project, &["test"]));
    assert_success(uv(project, &["build"]));
    true
}

fn assert_success(output: Output) -> String {
    assert!(output.status.success(), "{output:?}");
    // Windows tools may print non-ASCII paths in the console code page.
    String::from_utf8_lossy(&output.stdout).into_owned()
}

fn assert_pinned_uv() {
    let output = Command::new("uv").arg("--version").output().unwrap();
    let version = assert_success(output);
    assert_eq!(version.split_whitespace().nth(1), Some("0.9.26"));
}

#[test]
fn library_syncs_tests_and_builds_with_real_uv() {
    let sandbox = tempdir().unwrap();
    let project = sandbox.path().join("library with spaces é");
    assert_success(pyfaro(
        &["lib", "acme-widget", "--python", "3.10"],
        &project,
    ));
    let metadata = fs::read_to_string(project.join("pyproject.toml")).unwrap();
    assert!(metadata.contains("name = \"acme-widget\""));
    assert!(metadata.contains("requires-python = \">=3.10\""));
    assert!(metadata.contains("[tool.pyfaro]\nschema-version = 1"));
    assert!(project.join("src/acme_widget/__init__.py").is_file());
    assert!(project.join("src/acme_widget/py.typed").is_file());
    assert!(project.join("tests/test_library.py").is_file());
    assert!(project.join("README.md").is_file());
    assert!(!project.join("uv.lock").exists());
    assert!(!project.join(".venv").exists());

    assert!(metadata.contains("pytest==9.1.1"));
    assert!(metadata.contains("ruff==0.16.10"));
    assert!(metadata.contains("ty==0.0.85"));
    assert!(metadata.contains("[tool.pytest.ini_options]"));
    assert!(metadata.contains("[tool.ruff]"));
    let readme = fs::read_to_string(project.join("README.md")).unwrap();
    assert!(readme.contains("pyfaro sync") && readme.contains("uv.lock"));
    assert!(readme.contains("pyfaro check") && readme.contains("pyfaro test"));
    if !sync_check_test_build(&project) {
        return;
    }
    assert!(project.join("dist/acme_widget-0.1.0-py3-none-any.whl").is_file());
    assert!(project.join("dist/acme_widget-0.1.0.tar.gz").is_file());
}

#[test]
fn cli_uses_separate_distribution_package_and_command_names() {
    let sandbox = tempdir().unwrap();
    let project = sandbox.path().join("a cli project");
    assert_success(pyfaro(
        &[
            "cli",
            "acme-tools",
            "--package",
            "acme_console",
            "--command",
            "acme-do",
        ],
        &project,
    ));
    let metadata = fs::read_to_string(project.join("pyproject.toml")).unwrap();
    assert!(metadata.contains("acme-do = \"acme_console.cli:main\""));
    assert!(metadata.contains("module-name = \"acme_console\""));
    assert!(project.join("src/acme_console/cli.py").is_file());
    if !sync_check_test_build(&project) {
        return;
    }
    assert_eq!(
        assert_success(pyfaro_in(
            &project,
            &["run", "--locked", "--", "acme-do"]
        ))
        .trim_end(),
        "Hello from acme-tools!"
    );
    assert!(project.join("dist/acme_tools-0.1.0-py3-none-any.whl").is_file());
}

#[test]
fn invalid_inputs_do_not_create_a_project() {
    let sandbox = tempdir().unwrap();
    for (args, expected) in [
        (vec!["lib", "../escape"], "project name"),
        (vec!["lib", "con.demo"], "project name"),
        (vec!["lib", "foo-stubs"], "project name"),
        (vec!["lib", "hello", "--python", "3.12.1"], "Python version"),
        (vec!["cli", "hello", "--package", "for"], "package name"),
        (vec!["cli", "hello", "--package", "__pycache__"], "package name"),
        (vec!["cli", "hello", "--package", "unittest"], "package name"),
        (vec!["cli", "hello", "--package", "__main__"], "package name"),
        (vec!["cli", "hello", "--package", "__hello__"], "package name"),
        (vec!["cli", "hello", "--package", "con"], "package name"),
        (vec!["lib", "unittest"], "package name"),
        (vec!["lib", "cgi"], "package name"),
        (vec!["cli", "hello", "--command", "bad/name"], "command name"),
        (vec!["cli", "hello", "--command", "COM1"], "command name"),
    ] {
        let target = sandbox.path().join("new-project");
        let output = pyfaro(&args, &target);
        assert_eq!(output.status.code(), Some(1), "{output:?}");
        assert!(String::from_utf8_lossy(&output.stderr).contains(expected));
        assert!(!target.exists());
    }
}

#[test]
fn existing_target_and_missing_parent_are_rejected_without_changes() {
    let sandbox = tempdir().unwrap();
    let project = sandbox.path().join("existing");
    fs::create_dir(&project).unwrap();
    fs::write(project.join("keep.txt"), "keep").unwrap();
    let output = pyfaro(&["lib", "example"], &project);
    assert_eq!(output.status.code(), Some(1), "{output:?}");
    assert_eq!(fs::read_to_string(project.join("keep.txt")).unwrap(), "keep");
    assert!(!project.join("pyproject.toml").exists());

    let missing = sandbox.path().join("missing/child");
    let output = pyfaro(&["lib", "example"], &missing);
    assert_eq!(output.status.code(), Some(1), "{output:?}");
    assert!(!missing.parent().unwrap().exists());
}

#[test]
fn default_target_is_the_named_directory() {
    let sandbox = tempdir().unwrap();
    let output = Command::new(env!("CARGO_BIN_EXE_pyfaro"))
        .current_dir(sandbox.path())
        .args(["create", "lib", "demo-lib"])
        .output()
        .unwrap();
    assert_success(output);
    assert!(sandbox.path().join("demo-lib/src/demo_lib/__init__.py").is_file());
}

#[test]
fn numeric_distribution_gets_an_importable_package() {
    let sandbox = tempdir().unwrap();
    let project = sandbox.path().join("numeric");
    assert_success(pyfaro(&["lib", "1widget"], &project));
    let metadata = fs::read_to_string(project.join("pyproject.toml")).unwrap();
    assert!(metadata.contains("module-name = \"_1widget\""));
    assert!(project.join("src/_1widget/__init__.py").is_file());
    sync_check_test_build(&project);
}

#[test]
fn repeated_separators_normalize_to_a_buildable_package() {
    let sandbox = tempdir().unwrap();
    let project = sandbox.path().join("normalized");
    assert_success(pyfaro(&["lib", "foo--bar"], &project));
    assert!(project.join("src/foo_bar/__init__.py").is_file());
    sync_check_test_build(&project);
}

#[test]
fn dotted_distribution_gets_a_valid_default_command() {
    let sandbox = tempdir().unwrap();
    let project = sandbox.path().join("dotted");
    assert_success(pyfaro(&["cli", "acme.tools"], &project));
    let metadata = fs::read_to_string(project.join("pyproject.toml")).unwrap();
    assert!(metadata.contains("acme-tools = \"acme_tools.cli:main\""));
    if !sync_check_test_build(&project) {
        return;
    }
    assert_eq!(
        assert_success(pyfaro_in(
            &project,
            &["run", "--locked", "--", "acme-tools"]
        ))
        .trim_end(),
        "Hello from acme.tools!"
    );
}
