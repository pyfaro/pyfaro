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
    fs::write(project.join("src"), "wrong kind").unwrap();
    let invalid = doctor(&project, &bin, &uv);
    assert_eq!(invalid.status.code(), Some(1), "{invalid:?}");
    assert!(text(&invalid).contains("schema-version"));
    assert!(text(&invalid).contains("src must be a directory"), "{invalid:?}");

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

    for dependencies in ["'invalid'", "[42]"] {
        fs::write(
            &metadata,
            format!(
                "[project]\nname = 'sample'\ndependencies = {dependencies}\n"
            ),
        )
        .unwrap();
        let invalid_dependencies = doctor(&project, &bin, &uv);
        assert_eq!(
            invalid_dependencies.status.code(),
            Some(1),
            "{invalid_dependencies:?}"
        );
        assert!(text(&invalid_dependencies).contains("ERROR: dependencies:"));
        assert!(!text(&invalid_dependencies).contains("OK: dependencies:"));
    }

    fs::write(
        &metadata,
        "[project]\nname = 'sample'\ndynamic = ['dependencies']\n",
    )
    .unwrap();
    let dynamic = doctor(&project, &bin, &uv);
    assert_eq!(dynamic.status.code(), Some(1), "{dynamic:?}");
    assert!(
        text(&dynamic)
            .contains("dynamic project dependencies cannot be checked")
    );
    assert!(!text(&dynamic).contains("OK: dependencies:"));

    fs::write(&metadata, "[project]\nname = 'sample'\n").unwrap();
    fs::write(project.join(".python-version"), "3.11\n").unwrap();
    let pinned = doctor(&project, &bin, &uv);
    assert_eq!(pinned.status.code(), Some(1), "{pinned:?}");
    assert!(text(&pinned).contains("does not match .python-version"));
    fs::write(project.join(".python-version"), "3.12\n").unwrap();
    let matching = doctor(&project, &bin, &uv);
    assert_eq!(matching.status.code(), Some(0), "{matching:?}");
}

#[test]
fn group_names_are_normalized_for_defaults_and_includes() {
    let sandbox = tempdir().unwrap();
    let project = sandbox.path().join("project");
    let bin = sandbox.path().join("bin");
    fs::create_dir_all(&project).unwrap();
    let uv = install(&bin, "uv");
    install(&bin, "python");
    let metadata = project.join("pyproject.toml");
    fs::write(&metadata, "[project]\nname = 'sample'\n[dependency-groups]\nQA__Test = [{include-group = 'other..test'}]\nOther-Test = []\n[tool.uv]\ndefault-groups = ['qa.test']\n").unwrap();
    let output = doctor(&project, &bin, &uv);
    assert_eq!(output.status.code(), Some(0), "{output:?}");
    let report = text(&output);
    assert!(!report.contains("is not checked"), "{report}");
    assert!(report.contains("no blocking issues"), "{report}");
}

#[test]
fn invalid_and_unselected_groups_are_blocking_without_distribution_checks() {
    let sandbox = tempdir().unwrap();
    let project = sandbox.path().join("project");
    let bin = sandbox.path().join("bin");
    fs::create_dir_all(&project).unwrap();
    let uv = install(&bin, "uv");
    install(&bin, "python");
    let metadata = project.join("pyproject.toml");
    let cases = [
        (
            "[tool.uv]\ndefault-groups = ['missing']\n",
            "group \"missing\" is missing",
        ),
        (
            "[dependency-groups]\nqa = []\n[tool.uv]\ndefault-groups = ['missing']\n",
            "group \"missing\" is missing",
        ),
        (
            "[dependency-groups]\ndev = []\nQA_Test = []\nqa-test = []\n",
            "duplicate dependency group names",
        ),
        (
            "[dependency-groups]\ndev = []\n'bad_' = []\n",
            "group name \"bad_\" is invalid",
        ),
        (
            "[dependency-groups]\ndev = []\nother = [42]\n",
            "group \"other\" contains an invalid entry",
        ),
        (
            "[dependency-groups]\ndev = []\nother = [{include-group = 'cycle'}]\ncycle = [{include-group = 'OTHER'}]\n",
            "dependency group cycle",
        ),
        (
            "[dependency-groups]\ndev = []\nother = [{include-group = 'missing'}]\n",
            "group \"missing\" is missing",
        ),
        (
            "[dependency-groups]\ndev = []\nother = [{include-group = 'dev', extra = true}]\n",
            "group \"other\" contains an invalid entry",
        ),
    ];
    for (groups, expected) in cases {
        fs::write(&metadata, format!("[project]\nname = 'sample'\n{groups}"))
            .unwrap();
        let output = doctor(&project, &bin, &uv);
        let report = text(&output);
        assert_eq!(output.status.code(), Some(1), "{output:?}");
        assert!(report.contains(expected), "{report}");
        assert!(!report.contains("OK: dependencies:"), "{report}");
    }
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
    let interpreter = project.join(".venv").join(if cfg!(windows) {
        "Scripts/python.exe"
    } else {
        "bin/python"
    });
    let version_output =
        Command::new(&interpreter).arg("--version").output().unwrap();
    assert!(version_output.status.success(), "{version_output:?}");
    let version = String::from_utf8(version_output.stdout).unwrap();
    let major_minor = version
        .trim()
        .strip_prefix("Python ")
        .unwrap()
        .split('.')
        .take(2)
        .collect::<Vec<_>>()
        .join(".");
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

    fs::write(&metadata, format!("[project]\nname = 'sample'\nversion = '1.0.0'\nrequires-python = '>=3.10'\ndependencies = ['missing-doctor-fixture-package==1.0; python_version == \"{major_minor}\"', 'ignored-doctor-fixture-package; python_version < \"0\"']\n")).unwrap();
    let marked = Command::new(env!("CARGO_BIN_EXE_pyfaro"))
        .arg("doctor")
        .arg("--project")
        .arg(&project)
        .env("PATH", &path)
        .env_remove("VIRTUAL_ENV")
        .output()
        .unwrap();
    assert_eq!(marked.status.code(), Some(1), "{marked:?}");
    let report = text(&marked);
    assert!(
        report.contains("missing-doctor-fixture-package is missing from .venv"),
        "{report}"
    );
    assert!(
        !report.contains("ignored-doctor-fixture-package is missing"),
        "{report}"
    );
    assert_eq!(fs::read(project.join("uv.lock")).unwrap(), before);

    fs::write(&metadata, "[project]\nname = 'sample'\nversion = '1.0.0'\ndependencies = ['missing-doctor-valid-package==1.0', 'broken =>1']\n").unwrap();
    let malformed = Command::new(env!("CARGO_BIN_EXE_pyfaro"))
        .arg("doctor")
        .arg("--project")
        .arg(&project)
        .env("PATH", &path)
        .env_remove("VIRTUAL_ENV")
        .output()
        .unwrap();
    assert_eq!(malformed.status.code(), Some(1), "{malformed:?}");
    assert!(text(&malformed).contains("invalid requirement"), "{malformed:?}");
    assert!(!text(&malformed).contains("OK: dependencies:"));

    fs::write(&metadata, "[project]\nname = 'sample'\nversion = '1.0.0'\n[dependency-groups]\ndev = []\nother = ['broken =>1']\n").unwrap();
    let malformed_unselected = Command::new(env!("CARGO_BIN_EXE_pyfaro"))
        .arg("doctor")
        .arg("--project")
        .arg(&project)
        .env("PATH", &path)
        .env_remove("VIRTUAL_ENV")
        .output()
        .unwrap();
    assert_eq!(
        malformed_unselected.status.code(),
        Some(1),
        "{malformed_unselected:?}"
    );
    assert!(text(&malformed_unselected).contains("invalid requirement"));
    assert!(!text(&malformed_unselected).contains("OK: dependencies:"));

    fs::write(&metadata, "[project]\nname = 'sample'\nversion = '1.0.0'\n[dependency-groups]\ndev = ['missing-doctor-dev-package==1.0']\nquality = ['missing-doctor-quality-package==1.0']\n").unwrap();
    let grouped = Command::new(env!("CARGO_BIN_EXE_pyfaro"))
        .arg("doctor")
        .arg("--project")
        .arg(&project)
        .env("PATH", &path)
        .env_remove("VIRTUAL_ENV")
        .output()
        .unwrap();
    assert_eq!(grouped.status.code(), Some(1), "{grouped:?}");
    let report = text(&grouped);
    assert!(
        report.contains("missing-doctor-dev-package is missing from .venv"),
        "{report}"
    );
    assert!(report.contains("group \"quality\" is not checked"), "{report}");
    assert!(
        !report.contains("missing-doctor-quality-package is missing"),
        "{report}"
    );

    fs::write(&metadata, "[project]\nname = 'sample'\nversion = '1.0.0'\n[dependency-groups]\ndev = ['missing-doctor-dev-package==1.0']\nquality = ['missing-doctor-quality-package==1.0']\n[tool.uv]\ndefault-groups = ['quality']\n").unwrap();
    let selected = Command::new(env!("CARGO_BIN_EXE_pyfaro"))
        .arg("doctor")
        .arg("--project")
        .arg(&project)
        .env("PATH", &path)
        .env_remove("VIRTUAL_ENV")
        .output()
        .unwrap();
    assert_eq!(selected.status.code(), Some(1), "{selected:?}");
    let report = text(&selected);
    assert!(
        report.contains("missing-doctor-quality-package is missing from .venv"),
        "{report}"
    );
    assert!(
        !report.contains("missing-doctor-dev-package is missing"),
        "{report}"
    );

    fs::write(&metadata, "[project]\nname = 'sample'\nversion = '1.0.0'\n[dependency-groups]\ndev = [{include-group = 'quality'}]\nquality = ['missing-doctor-quality-package==1.0']\n").unwrap();
    let included = Command::new(env!("CARGO_BIN_EXE_pyfaro"))
        .arg("doctor")
        .arg("--project")
        .arg(&project)
        .env("PATH", &path)
        .env_remove("VIRTUAL_ENV")
        .output()
        .unwrap();
    assert_eq!(included.status.code(), Some(1), "{included:?}");
    let report = text(&included);
    assert!(
        report.contains("missing-doctor-quality-package is missing from .venv"),
        "{report}"
    );
    assert!(!report.contains("group \"quality\" is not checked"), "{report}");

    fs::write(&metadata, "[project]\nname = 'sample'\nversion = '1.0.0'\n[dependency-groups]\nQA__Test = [{include-group = 'other..test'}]\nOther-Test = ['missing-doctor-normalized-package==1.0']\n[tool.uv]\ndefault-groups = ['qa.test']\n").unwrap();
    let normalized = Command::new(env!("CARGO_BIN_EXE_pyfaro"))
        .arg("doctor")
        .arg("--project")
        .arg(&project)
        .env("PATH", &path)
        .env_remove("VIRTUAL_ENV")
        .output()
        .unwrap();
    assert_eq!(normalized.status.code(), Some(1), "{normalized:?}");
    let report = text(&normalized);
    assert!(
        report.contains(
            "missing-doctor-normalized-package is missing from .venv"
        ),
        "{report}"
    );
    assert!(!report.contains("is not checked"), "{report}");

    fs::write(&metadata, "dependency-groups = { dev = ['missing-doctor-inline-package==1.0'] }\n[project]\nname = 'sample'\nversion = '1.0.0'\n").unwrap();
    let inline = Command::new(env!("CARGO_BIN_EXE_pyfaro"))
        .arg("doctor")
        .arg("--project")
        .arg(&project)
        .env("PATH", &path)
        .env_remove("VIRTUAL_ENV")
        .output()
        .unwrap();
    assert_eq!(inline.status.code(), Some(1), "{inline:?}");
    assert!(
        text(&inline)
            .contains("missing-doctor-inline-package is missing from .venv"),
        "{inline:?}"
    );

    fs::write(&metadata, "[project]\nname = 'sample'\nversion = '1.0.0'\n[dependency-groups]\ndev = ['missing-doctor-dev-package==1.0']\nquality = ['missing-doctor-uvtoml-package==1.0']\n").unwrap();
    fs::write(project.join("uv.toml"), "default-groups = ['quality']\n")
        .unwrap();
    let configured = Command::new(env!("CARGO_BIN_EXE_pyfaro"))
        .arg("doctor")
        .arg("--project")
        .arg(&project)
        .env("PATH", &path)
        .env_remove("VIRTUAL_ENV")
        .output()
        .unwrap();
    assert_eq!(configured.status.code(), Some(1), "{configured:?}");
    let report = text(&configured);
    assert!(
        report.contains("missing-doctor-uvtoml-package is missing from .venv"),
        "{report}"
    );
    assert!(
        !report.contains("missing-doctor-dev-package is missing"),
        "{report}"
    );
}
