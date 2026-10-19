mod support;

use std::fs;
use std::path::{Path, PathBuf};
use std::process::{Command, Output};

use tempfile::tempdir;

fn install(directory: &Path, name: &str) -> PathBuf {
    fs::create_dir_all(directory).unwrap();
    let binary =
        directory.join(format!("{name}{}", std::env::consts::EXE_SUFFIX));
    fs::copy(support::fixture_uv(), &binary).unwrap();
    binary
}

fn doctor(project: &Path, path: &Path, uv: &Path) -> Output {
    Command::new(env!("CARGO_BIN_EXE_pyfaro"))
        .arg("doctor")
        .arg("--project")
        .arg(project)
        .arg("--uv")
        .arg(uv)
        .env("PATH", path)
        .env_remove("VIRTUAL_ENV")
        .output()
        .unwrap()
}

fn text(output: &Output) -> String {
    String::from_utf8(output.stdout.clone()).unwrap()
}

#[test]
fn healthy_fixture_reports_all_versions_without_writing_project() {
    let sandbox = tempdir().unwrap();
    let project = sandbox.path().join("project");
    let bin = sandbox.path().join("bin");
    fs::create_dir(&project).unwrap();
    let metadata = "[project]\nname = 'sample'\ndependencies = []\n";
    fs::write(project.join("pyproject.toml"), metadata).unwrap();
    let uv = install(&bin, "uv");
    for name in ["python", "ruff", "ty", "pytest"] {
        install(&bin, name);
    }
    let output = doctor(&project, &bin, &uv);
    assert_eq!(output.status.code(), Some(0), "{output:?}");
    let report = text(&output);
    for version in [
        "Python 3.12.1",
        "uv 0.9.26",
        "ruff 0.14.0",
        "ty 0.0.1-alpha.24",
        "pytest 8.4.2",
    ] {
        assert!(report.contains(version), "{report}");
    }
    assert!(report.contains("no blocking issues"), "{report}");
    assert!(output.stderr.is_empty(), "{output:?}");
    assert_eq!(
        fs::read_to_string(project.join("pyproject.toml")).unwrap(),
        metadata
    );
    assert!(!project.join(".venv").exists());
    assert!(!project.join("uv.lock").exists());
}

#[test]
fn missing_optional_tools_warn_but_missing_uv_or_python_block() {
    let sandbox = tempdir().unwrap();
    let project = sandbox.path().join("project");
    let bin = sandbox.path().join("bin");
    fs::create_dir_all(&project).unwrap();
    fs::write(project.join("pyproject.toml"), "[project]\nname = 'sample'\n")
        .unwrap();
    let python = install(&bin, "python");
    let uv = install(&bin, "uv");
    let optional = doctor(&project, &bin, &uv);
    assert_eq!(optional.status.code(), Some(0), "{optional:?}");
    let report = text(&optional);
    for name in ["ruff", "ty", "pytest"] {
        assert!(report.contains(&format!("WARN: {name}:")), "{report}");
    }
    fs::remove_file(&uv).unwrap();
    let no_uv = doctor(&project, &bin, &uv);
    assert_eq!(no_uv.status.code(), Some(1), "{no_uv:?}");
    assert!(text(&no_uv).contains("ERROR: uv:"));
    fs::remove_file(python).unwrap();
    let no_python = doctor(&project, &bin, &uv);
    assert_eq!(no_python.status.code(), Some(1), "{no_python:?}");
    assert!(text(&no_python).contains("ERROR: Python:"));
    assert!(
        String::from_utf8_lossy(&no_python.stderr).contains("blocking issue")
    );
}

#[test]
fn missing_project_is_a_blocking_finding() {
    let sandbox = tempdir().unwrap();
    let bin = sandbox.path().join("bin");
    let uv = install(&bin, "uv");
    install(&bin, "python");
    let output = Command::new(env!("CARGO_BIN_EXE_pyfaro"))
        .arg("doctor")
        .current_dir(sandbox.path())
        .arg("--uv")
        .arg(&uv)
        .env("PATH", &bin)
        .env_remove("VIRTUAL_ENV")
        .output()
        .unwrap();
    assert_eq!(output.status.code(), Some(1), "{output:?}");
    assert!(text(&output).contains("ERROR: project: no pyproject.toml found"));
    assert!(text(&output).contains("Python 3.12.1"));
}

#[test]
fn invalid_configuration_conflicts_and_layout_are_actionable() {
    let sandbox = tempdir().unwrap();
    let project = sandbox.path().join("project");
    let bin = sandbox.path().join("bin");
    fs::create_dir_all(&project).unwrap();
    let uv = install(&bin, "uv");
    install(&bin, "python");
    let metadata = project.join("pyproject.toml");
    fs::write(
        &metadata,
        "[project]\nname = 'sample'\n[tool.pyfaro]\nschema-version = 2\n",
    )
    .unwrap();
    let invalid = doctor(&project, &bin, &uv);
    assert_eq!(invalid.status.code(), Some(1), "{invalid:?}");
    assert!(text(&invalid).contains("schema-version"));

    fs::write(
        &metadata,
        "[project]\nname = 'sample'\n[tool.ruff]\nline-length = 88\n",
    )
    .unwrap();
    fs::write(project.join("ruff.toml"), "line-length = 90\n").unwrap();
    fs::write(project.join("src"), "wrong kind").unwrap();
    let problems = doctor(&project, &bin, &uv);
    assert_eq!(problems.status.code(), Some(1), "{problems:?}");
    let report = text(&problems);
    assert!(report.contains("[tool.ruff] and ruff.toml"), "{report}");
    assert!(report.contains("src must be a directory"), "{report}");
    assert!(report.contains("keep one source of configuration"), "{report}");

    fs::remove_file(project.join("ruff.toml")).unwrap();
    fs::remove_file(project.join("src")).unwrap();
    fs::write(
        &metadata,
        "[project]\nname = 'sample'\nrequires-python = '>=3.13'\n",
    )
    .unwrap();
    let incompatible = doctor(&project, &bin, &uv);
    assert_eq!(incompatible.status.code(), Some(1), "{incompatible:?}");
    assert!(text(&incompatible).contains("does not satisfy requires-python"));
}

#[test]
fn real_offline_uv_environment_checks_missing_runtime_distributions() {
    let sandbox = tempdir().unwrap();
    let project = sandbox.path().join("project");
    fs::create_dir(&project).unwrap();
    let metadata = project.join("pyproject.toml");
    fs::write(&metadata, "[project]\nname = 'sample'\nversion = '1.0.0'\nrequires-python = '>=3.10'\ndependencies = []\n").unwrap();
    let sync = Command::new(env!("CARGO_BIN_EXE_pyfaro"))
        .arg("sync")
        .arg("--offline")
        .arg("--project")
        .arg(&project)
        .env("UV_CACHE_DIR", sandbox.path().join("cache"))
        .output()
        .unwrap();
    assert!(sync.status.success(), "{sync:?}");
    let uv = PathBuf::from("uv");
    let path = std::env::var_os("PATH").unwrap();
    let healthy = Command::new(env!("CARGO_BIN_EXE_pyfaro"))
        .arg("doctor")
        .arg("--project")
        .arg(&project)
        .env("PATH", &path)
        .env_remove("VIRTUAL_ENV")
        .output()
        .unwrap();
    assert_eq!(healthy.status.code(), Some(0), "{healthy:?}");
    fs::write(&metadata, "[project]\nname = 'sample'\nversion = '1.0.0'\nrequires-python = '>=3.10'\ndependencies = ['missing-doctor-fixture-package==1.0']\n").unwrap();
    let before = fs::read(project.join("uv.lock")).unwrap();
    let missing = Command::new(env!("CARGO_BIN_EXE_pyfaro"))
        .arg("doctor")
        .arg("--project")
        .arg(&project)
        .arg("--uv")
        .arg(&uv)
        .env("PATH", &path)
        .env_remove("VIRTUAL_ENV")
        .output()
        .unwrap();
    assert_eq!(missing.status.code(), Some(1), "{missing:?}");
    assert!(
        text(&missing)
            .contains("missing-doctor-fixture-package is missing from .venv")
    );
    assert_eq!(fs::read(project.join("uv.lock")).unwrap(), before);
    assert!(!project.join("pyfaro.lock").exists());
}
