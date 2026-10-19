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

fn resolved(path: &Path) -> String {
    fs::canonicalize(path).unwrap().display().to_string()
}

fn cli(project: &Path, path: &Path, active: &Path, args: &[&str]) -> Output {
    Command::new(env!("CARGO_BIN_EXE_pyfaro"))
        .args(args)
        .current_dir(project)
        .env("PATH", path)
        .env("VIRTUAL_ENV", active)
        .output()
        .unwrap()
}

#[test]
fn lists_resolved_paths_and_versions_in_precedence_order() {
    let sandbox = tempdir().unwrap();
    let project = sandbox.path().join("project with spaces");
    let active = sandbox.path().join("active");
    let path = sandbox.path().join("path");
    fs::create_dir(&project).unwrap();
    fs::write(project.join("pyproject.toml"), "[project]\nname = 'example'\n")
        .unwrap();
    let project_ruff = install(&project.join(".venv").join(scripts()), "ruff");
    let active_ruff = install(&active.join(scripts()), "ruff");
    let active_ty = install(&active.join(scripts()), "ty");
    let path_ty = install(&path, "ty");
    let path_pytest = install(&path, "pytest");
    let path_uv = install(&path, "uv");
    let selected_uv = install(&sandbox.path().join("explicit"), "uv");
    let output = Command::new(env!("CARGO_BIN_EXE_pyfaro"))
        .args(["tools", "--project"])
        .arg(&project)
        .arg("--uv")
        .arg(&selected_uv)
        .current_dir(sandbox.path())
        .env("PATH", &path)
        .env("VIRTUAL_ENV", &active)
        .output()
        .unwrap();
    assert!(output.status.success(), "{output:?}");
    let stdout = String::from_utf8(output.stdout).unwrap();
    assert_eq!(
        stdout.lines().collect::<Vec<_>>(),
        [
            format!("uv: {} (uv 0.9.26)", resolved(&selected_uv)),
            format!("ruff: {} (ruff 0.14.0)", resolved(&project_ruff)),
            format!("ty: {} (ty 0.0.1-alpha.24)", resolved(&active_ty)),
            format!("pytest: {} (pytest 8.4.2)", resolved(&path_pytest)),
        ]
    );
    assert!(!stdout.contains(&resolved(&active_ruff)));
    assert!(!stdout.contains(&resolved(&path_ty)));
    assert!(!stdout.contains(&resolved(&path_uv)));
    assert!(output.stderr.is_empty());

    let nested = project.join("src/nested");
    fs::create_dir_all(&nested).unwrap();
    let discovered = cli(&nested, &path, &active, &["tools", "ruff"]);
    assert!(discovered.status.success(), "{discovered:?}");
    assert_eq!(
        String::from_utf8(discovered.stdout).unwrap().trim_end(),
        format!("ruff: {} (ruff 0.14.0)", resolved(&project_ruff))
    );
}

#[test]
fn reports_missing_tools_and_strict_selector_fails_actionably() {
    let sandbox = tempdir().unwrap();
    let project = sandbox.path().join("project");
    let path = sandbox.path().join("empty-path");
    fs::create_dir(&project).unwrap();
    fs::create_dir(&path).unwrap();
    fs::write(project.join("pyproject.toml"), "[project]\nname = 'example'\n")
        .unwrap();
    let output = cli(&project, &path, sandbox.path(), &["tools"]);
    assert!(output.status.success(), "{output:?}");
    let stdout = String::from_utf8(output.stdout).unwrap();
    for name in ["uv", "ruff", "ty", "pytest"] {
        assert!(stdout.contains(&format!("{name}: unavailable")), "{stdout}");
    }
    let selected = cli(&project, &path, sandbox.path(), &["tools", "ruff"]);
    assert_eq!(selected.status.code(), Some(1));
    let stderr = String::from_utf8(selected.stderr).unwrap();
    assert!(stderr.contains("ruff: not found"), "{stderr}");
    assert!(stderr.contains(".venv"), "{stderr}");
    assert!(selected.stdout.is_empty());
    assert!(!project.join(".venv").exists());
    assert!(!project.join("uv.lock").exists());
}

#[test]
fn rejects_unrecognized_versions_and_old_explicit_uv() {
    let sandbox = tempdir().unwrap();
    let path = sandbox.path().join("path");
    let active = sandbox.path().join("active");
    fs::create_dir(&active).unwrap();
    let ruff = install(&path, "ruff");
    let invalid = Command::new(env!("CARGO_BIN_EXE_pyfaro"))
        .args(["tools", "ruff"])
        .current_dir(sandbox.path())
        .env("PATH", &path)
        .env("VIRTUAL_ENV", &active)
        .env("PYFARO_FIXTURE_VERSION", "other 1.2.3")
        .output()
        .unwrap();
    assert_eq!(invalid.status.code(), Some(1), "{invalid:?}");
    let stderr = String::from_utf8(invalid.stderr).unwrap();
    assert!(stderr.contains(&resolved(&ruff)), "{stderr}");
    assert!(stderr.contains("unexpected --version output"), "{stderr}");

    let uv = install(&path, "uv");
    let old = Command::new(env!("CARGO_BIN_EXE_pyfaro"))
        .args(["tools", "uv", "--uv"])
        .arg(&uv)
        .current_dir(sandbox.path())
        .env("PATH", &active)
        .env("PYFARO_FIXTURE_VERSION", "uv 0.9.25")
        .output()
        .unwrap();
    assert_eq!(old.status.code(), Some(1), "{old:?}");
    assert!(String::from_utf8_lossy(&old.stderr).contains("uv >=0.9.26"));
}

#[test]
fn project_environment_wins_even_when_its_path_contains_a_path_separator() {
    let sandbox = tempdir().unwrap();
    let folder = if cfg!(windows) {
        "project;with-separator"
    } else {
        "project:with-separator"
    };
    let project = sandbox.path().join(folder);
    let path = sandbox.path().join("global");
    fs::create_dir(&project).unwrap();
    fs::write(project.join("pyproject.toml"), "[project]\nname = 'example'\n")
        .unwrap();
    let local = install(&project.join(".venv").join(scripts()), "ruff");
    install(&path, "ruff");
    let output = cli(&project, &path, sandbox.path(), &["tools", "ruff"]);
    assert!(output.status.success(), "{output:?}");
    assert_eq!(
        String::from_utf8(output.stdout).unwrap().trim_end(),
        format!("ruff: {} (ruff 0.14.0)", resolved(&local))
    );
}

#[test]
fn failed_version_probe_reports_its_stderr_and_selected_path() {
    let sandbox = tempdir().unwrap();
    let path = sandbox.path().join("path");
    let ruff = install(&path, "ruff");
    let output = Command::new(env!("CARGO_BIN_EXE_pyfaro"))
        .args(["tools", "ruff"])
        .current_dir(sandbox.path())
        .env("PATH", &path)
        .env("PYFARO_FIXTURE_VERSION_FAIL", "1")
        .output()
        .unwrap();
    assert_eq!(output.status.code(), Some(1), "{output:?}");
    let diagnostic = String::from_utf8_lossy(&output.stderr);
    assert!(diagnostic.contains(&resolved(&ruff)), "{diagnostic}");
    assert!(
        diagnostic.contains("fixture interpreter is broken"),
        "{diagnostic}"
    );
}

#[test]
fn accepts_pytest_prerelease_versions_without_semver_rules() {
    let sandbox = tempdir().unwrap();
    let path = sandbox.path().join("path");
    let pytest = install(&path, "pytest");
    let output = Command::new(env!("CARGO_BIN_EXE_pyfaro"))
        .args(["tools", "pytest"])
        .current_dir(sandbox.path())
        .env("PATH", &path)
        .env("PYFARO_FIXTURE_VERSION", "pytest 9.0.0rc1")
        .output()
        .unwrap();
    assert!(output.status.success(), "{output:?}");
    assert_eq!(
        String::from_utf8(output.stdout).unwrap().trim_end(),
        format!("pytest: {} (pytest 9.0.0rc1)", resolved(&pytest))
    );
}

#[test]
fn uv_probe_failure_is_actionable_without_forwarding_the_probe_status() {
    let sandbox = tempdir().unwrap();
    let uv = install(&sandbox.path().join("bin"), "uv");
    let output = Command::new(env!("CARGO_BIN_EXE_pyfaro"))
        .args(["tools", "uv", "--uv"])
        .arg(&uv)
        .current_dir(sandbox.path())
        .env("PYFARO_FIXTURE_VERSION_FAIL", "1")
        .output()
        .unwrap();
    assert_eq!(output.status.code(), Some(1), "{output:?}");
    let diagnostic = String::from_utf8_lossy(&output.stderr);
    assert!(diagnostic.contains(&resolved(&uv)), "{diagnostic}");
    assert!(
        diagnostic.contains("fixture interpreter is broken"),
        "{diagnostic}"
    );
}

#[test]
fn version_reports_remain_one_line_when_tools_print_extra_output() {
    let sandbox = tempdir().unwrap();
    let path = sandbox.path().join("bin");
    let uv = install(&path, "uv");
    let ruff = install(&path, "ruff");
    for (tool, binary, version) in
        [("uv", &uv, "uv 0.9.26"), ("ruff", &ruff, "ruff 0.14.0")]
    {
        let output = Command::new(env!("CARGO_BIN_EXE_pyfaro"))
            .args(["tools", tool])
            .current_dir(sandbox.path())
            .env("PATH", &path)
            .env("PYFARO_FIXTURE_VERSION_EXTRA", "1")
            .output()
            .unwrap();
        assert!(output.status.success(), "{output:?}");
        assert_eq!(
            String::from_utf8(output.stdout).unwrap(),
            format!("{tool}: {} ({version})\n", resolved(binary))
        );
    }
}

#[test]
fn explicit_project_requires_its_own_metadata() {
    let sandbox = tempdir().unwrap();
    fs::write(
        sandbox.path().join("pyproject.toml"),
        "[project]\nname = 'outer'\n",
    )
    .unwrap();
    let child = sandbox.path().join("child");
    fs::create_dir(&child).unwrap();
    let output = Command::new(env!("CARGO_BIN_EXE_pyfaro"))
        .args(["tools", "ruff", "--project"])
        .arg(&child)
        .output()
        .unwrap();
    assert_eq!(output.status.code(), Some(1));
    assert!(
        String::from_utf8_lossy(&output.stderr)
            .contains("has no pyproject.toml")
    );
}

fn scripts() -> &'static str {
    if cfg!(windows) { "Scripts" } else { "bin" }
}
