use std::fs;
use std::path::Path;
use std::process::{Command, Output};

use tempfile::tempdir;

fn pyfaro(args: &[&str], target: &Path) -> Output {
    Command::new(env!("CARGO_BIN_EXE_pyfaro"))
        .arg("create")
        .args(args)
        .args(["--path"])
        .arg(target)
        .output()
        .expect("start pyfaro")
}

fn uv(project: &Path, cache: &Path, args: &[&str]) -> Output {
    Command::new("uv")
        .current_dir(project)
        .env("UV_CACHE_DIR", cache)
        .args(["--no-python-downloads", "--offline", "--no-progress"])
        .args(args)
        .output()
        .expect("start uv")
}

fn sync(project: &Path, cache: &Path) -> Output {
    Command::new(env!("CARGO_BIN_EXE_pyfaro"))
        .args(["sync", "--offline"])
        .arg(project)
        .env("UV_CACHE_DIR", cache)
        .output()
        .unwrap()
}

fn run_project(project: &Path, cache: &Path, command: &[&str]) -> Output {
    Command::new(env!("CARGO_BIN_EXE_pyfaro"))
        .current_dir(project)
        .env("UV_CACHE_DIR", cache)
        .args(["run", "--locked", "--offline", "--"])
        .args(command)
        .output()
        .unwrap()
}

fn assert_success(output: Output) -> String {
    assert!(output.status.success(), "{output:?}");
    String::from_utf8(output.stdout).unwrap()
}

fn assert_pinned_uv() {
    let output = Command::new("uv").arg("--version").output().unwrap();
    assert_eq!(assert_success(output).trim(), "uv 0.9.26");
}

#[test]
fn library_syncs_tests_and_builds_with_real_uv() {
    assert_pinned_uv();
    let sandbox = tempdir().unwrap();
    let project = sandbox.path().join("library with spaces é");
    let cache = sandbox.path().join("uv-cache");
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

    assert_success(sync(&project, &cache));
    assert_success(run_project(
        &project,
        &cache,
        &["python", "-m", "unittest", "discover", "-s", "tests"],
    ));
    assert_success(uv(&project, &cache, &["build"]));
    assert!(project.join("dist/acme_widget-0.1.0-py3-none-any.whl").is_file());
    assert!(project.join("dist/acme_widget-0.1.0.tar.gz").is_file());
}

#[test]
fn cli_uses_separate_distribution_package_and_command_names() {
    assert_pinned_uv();
    let sandbox = tempdir().unwrap();
    let project = sandbox.path().join("a cli project");
    let cache = sandbox.path().join("uv-cache");
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
    assert_success(sync(&project, &cache));
    assert_success(run_project(
        &project,
        &cache,
        &["python", "-m", "unittest", "discover", "-s", "tests"],
    ));
    assert_eq!(
        assert_success(run_project(&project, &cache, &["acme-do"])),
        "Hello from acme-tools!\n"
    );
    assert_success(uv(&project, &cache, &["build"]));
    assert!(project.join("dist/acme_tools-0.1.0-py3-none-any.whl").is_file());
}

#[test]
fn invalid_inputs_do_not_create_a_project() {
    let sandbox = tempdir().unwrap();
    for (args, expected) in [
        (vec!["lib", "../escape"], "project name"),
        (vec!["lib", "hello", "--python", "3.12.1"], "Python version"),
        (vec!["cli", "hello", "--package", "for"], "package name"),
        (vec!["cli", "hello", "--command", "bad/name"], "command name"),
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
