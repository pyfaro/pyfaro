//! `pyfaro python`, `pyfaro tools` and `pyfaro doctor` against real uv, Ruff,
//! ty and pytest.

use std::ffi::OsString;
use std::fs;
use std::path::{Path, PathBuf};

use crate::harness::{
    Cmd, Kind, Outcome, PYTEST, Project, RUFF, Sandbox, TY, UV,
};

const UV_LINE_END: &str = " (uv 0.9.26)";
const DOCTOR_FAILED: &str =
    "follow the actions above and run pyfaro doctor again";

/// Runs a command and drops the build metadata uv, ty and others append to
/// their version (`ty 0.0.85 (d7fc5ef88 2026-10-06)`), which changes by
/// platform and is not under test.
trait RunPlain {
    fn run_plain(self) -> Outcome;
}

impl RunPlain for Cmd {
    fn run_plain(self) -> Outcome {
        let mut out = self.run();
        let plain = out
            .stdout
            .split_inclusive('\n')
            .map(|line| {
                let body = line.trim_end_matches(['\r', '\n']);
                match body.strip_suffix("))").and_then(|head| head.rfind(" ("))
                {
                    Some(start) => {
                        format!("{}){}", &body[..start], &line[body.len()..])
                    }
                    None => line.to_owned(),
                }
            })
            .collect::<String>();
        out.stdout = plain;
        out
    }
}

fn scripts_directory() -> &'static str {
    if cfg!(windows) { "Scripts" } else { "bin" }
}

fn executable(name: &str) -> String {
    format!("{name}{}", std::env::consts::EXE_SUFFIX)
}

/// Canonical path of `<env>/<scripts>/<name>`, as `pyfaro` prints it.
fn tool_path(environment: &Path, name: &str) -> PathBuf {
    let path = environment.join(scripts_directory()).join(executable(name));
    fs::canonicalize(&path)
        .unwrap_or_else(|error| panic!("{}: {error}", path.display()))
}

/// The exact `tools` listing line for an installed tool.
fn tool_line(environment: &Path, name: &str, version: &str) -> String {
    format!(
        "{name}: {} ({name} {version})",
        tool_path(environment, name).display()
    )
}

const MISSING_ADVICE: &str = "install {} in the project environment (.venv by default), activate an environment, or add it to PATH";

fn not_found(name: &str) -> String {
    format!("{name}: not found; {}", MISSING_ADVICE.replace("{}", name))
}

fn lines(text: &str) -> Vec<&str> {
    text.lines().collect()
}

/// A directory holding a copy of the real uv and nothing else, for use as
/// `PATH` when a test needs Ruff, ty or pytest to be absent from it.
fn uv_only_path(sandbox: &Sandbox) -> OsString {
    let directory = sandbox.join("uv-only");
    fs::create_dir_all(&directory).unwrap();
    let uv = which::which("uv").expect("uv on PATH");
    fs::copy(uv, directory.join(executable("uv"))).expect("copy uv");
    directory.into_os_string()
}

/// A directory on `PATH`-style lists: the given directories joined.
fn path_of(directories: &[&Path]) -> OsString {
    std::env::join_paths(directories).expect("join PATH")
}

/// Create a virtual environment at `directory` with one pinned Ruff.
fn ruff_environment(sandbox: &Sandbox, directory: &Path, version: &str) {
    sandbox
        .uv(["venv", "--no-python-downloads", "--quiet"])
        .arg(directory)
        .run_plain()
        .success();
    sandbox
        .uv(["pip", "install", "--quiet", "--python"])
        .arg(directory)
        .arg(format!("ruff=={version}"))
        .run_plain()
        .success();
}

/// Major.minor of the project's own interpreter, e.g. `3.12`.
fn venv_python_minor(project: &Project) -> String {
    let python = project
        .path(".venv")
        .join(scripts_directory())
        .join(executable("python"));
    let output = std::process::Command::new(python)
        .arg("--version")
        .output()
        .expect("run project python");
    let text = String::from_utf8_lossy(&output.stdout).into_owned();
    let version = text.trim().strip_prefix("Python ").unwrap().to_owned();
    version.split('.').take(2).collect::<Vec<_>>().join(".")
}

fn pyfaro_in(sandbox: &Sandbox, project: &Project, args: &[&str]) -> Outcome {
    sandbox.pyfaro(args).arg("--project").arg(&project.root).run_plain()
}

// ---------------------------------------------------------------- python

#[test]
fn python_list_only_installed_hides_downloads() {
    e2e_guard!();
    let sandbox = Sandbox::new();
    let out =
        sandbox.pyfaro(["python", "list", "--only-installed"]).run_plain();
    out.success();
    assert!(!out.stdout.trim().is_empty(), "no interpreter listed");
    assert!(!out.stdout.contains("<download available>"), "{}", out.stdout);
}

#[test]
fn python_list_with_a_request_shows_matching_downloads_only() {
    e2e_guard!();
    let sandbox = Sandbox::new();
    let out = sandbox.pyfaro(["python", "list", "3.12"]).run_plain();
    out.success()
        .stdout_contains("cpython-3.12")
        .stdout_contains("<download available>");
    for line in out.stdout.lines() {
        assert!(line.contains("3.12"), "unfiltered line: {line}");
    }
    assert!(
        !sandbox.join("python").exists(),
        "listing must not install anything"
    );
}

#[test]
fn python_list_all_versions_includes_older_patch_releases() {
    e2e_guard!();
    let sandbox = Sandbox::new();
    let latest = sandbox.pyfaro(["python", "list", "3.12"]).run_plain();
    let all = sandbox
        .pyfaro(["python", "list", "--all-versions", "3.12"])
        .run_plain();
    latest.success();
    all.success();
    assert!(
        all.stdout.lines().count() > latest.stdout.lines().count(),
        "--all-versions added nothing:\n{}",
        all.stdout
    );
    all.stdout_contains("cpython-3.12.0-");
    assert!(!sandbox.join("python").exists());
}

#[test]
fn python_list_offline_reads_the_catalog_and_installs_nothing() {
    e2e_guard!();
    let sandbox = Sandbox::new();
    let out =
        sandbox.pyfaro(["python", "list", "--offline", "3.12"]).run_plain();
    out.success().stdout_contains("cpython-3.12").stderr_empty();
    assert!(!sandbox.join("python").exists());
}

#[test]
fn python_list_only_installed_with_an_unmatched_request_is_empty() {
    e2e_guard!();
    let sandbox = Sandbox::new();
    sandbox
        .pyfaro(["python", "list", "--only-installed", "3.99"])
        .run_plain()
        .success()
        .stdout_empty();
}

#[test]
fn python_list_is_not_filtered_by_requires_python() {
    e2e_guard!();
    let sandbox = Sandbox::new();
    let project = sandbox.project(Kind::Lib);
    let out = project.pyfaro(["python", "list", "3.10"]).run_plain();
    out.success().stdout_contains("cpython-3.10");
}

#[test]
fn python_install_downloads_an_explicit_version_into_the_sandbox() {
    e2e_guard!();
    let sandbox = Sandbox::new();
    let install = sandbox.pyfaro(["python", "install", "3.13"]).run_plain();
    install.success().stderr_contains("Installed Python 3.13");

    let installed = fs::read_dir(sandbox.join("python"))
        .expect("install directory")
        .map(|entry| entry.unwrap().file_name().to_string_lossy().into_owned())
        .collect::<Vec<_>>();
    assert!(
        installed.iter().any(|name| name.starts_with("cpython-3.13")),
        "no CPython 3.13 in {installed:?}"
    );
    let listed = sandbox
        .pyfaro(["python", "list", "--only-installed", "3.13"])
        .run_plain();
    listed.success().stdout_contains("cpython-3.13");
    assert!(!listed.stdout.contains("<download available>"));

    // Installing again is a no-op that still succeeds.
    sandbox
        .pyfaro(["python", "install", "3.13"])
        .run_plain()
        .success()
        .stdout_empty();
}

#[test]
fn python_install_without_targets_reads_python_version() {
    e2e_guard!();
    let sandbox = Sandbox::new();
    let project = sandbox.project(Kind::Lib);
    project.write(".python-version", "3.13\n");
    project
        .pyfaro(["python", "install"])
        .run_plain()
        .success()
        .stderr_contains("Installed Python 3.13");
    assert!(fs::read_dir(sandbox.join("python")).unwrap().any(|entry| {
        entry.unwrap().file_name().to_string_lossy().starts_with("cpython-3.13")
    }));
    assert_eq!(project.read(".python-version"), "3.13\n");
}

#[test]
fn python_install_failure_forwards_uv_status() {
    e2e_guard!();
    let sandbox = Sandbox::new();
    let unknown =
        sandbox.pyfaro(["python", "install", "--offline", "3.99"]).run_plain();
    unknown
        .code(2)
        .stderr_contains("No download found for request")
        .stderr_contains("uv python install failed")
        .stdout_empty();

    let invalid = sandbox.pyfaro(["python", "install", "--", "-x"]).run_plain();
    invalid
        .code(2)
        .stderr_contains("`-x` is not a valid Python download request")
        .stdout_empty();

    // Offline with nothing cached: uv fails to download and exits 1.
    let project = sandbox.project(Kind::Lib);
    project.write(".python-version", "3.11\n");
    let offline =
        project.pyfaro(["python", "install", "--offline"]).run_plain();
    offline
        .code(1)
        .stderr_contains("Failed to install")
        .stderr_contains("uv python install failed");
    assert!(!sandbox.join("python").join("cpython-3.11").exists());
}

#[test]
fn python_pin_writes_python_version() {
    e2e_guard!();
    let sandbox = Sandbox::new();
    let project = sandbox.project(Kind::Lib);
    assert!(!project.path(".python-version").exists());
    project
        .pyfaro(["python", "pin", "3.12"])
        .run_plain()
        .success()
        .stdout_contains("Pinned `.python-version` to `3.12`");
    assert_eq!(project.read(".python-version").trim(), "3.12");
}

#[test]
fn python_pin_without_a_request_displays_the_pin() {
    e2e_guard!();
    let sandbox = Sandbox::new();
    let project = sandbox.project(Kind::Lib);
    project.write(".python-version", "3.12\n");
    let before = project.snapshot();
    project.pyfaro(["python", "pin"]).run_plain().success().stdout_eq("3.12\n");
    assert_eq!(project.snapshot(), before);
}

#[test]
fn python_pin_without_a_pin_file_fails() {
    e2e_guard!();
    let sandbox = Sandbox::new();
    let project = sandbox.project(Kind::Lib);
    project
        .pyfaro(["python", "pin"])
        .run_plain()
        .code(2)
        .stderr_contains("No Python version file found")
        .stderr_contains("uv python pin failed")
        .stdout_empty();
}

#[test]
fn python_pin_rm_removes_the_pin() {
    e2e_guard!();
    let sandbox = Sandbox::new();
    let project = sandbox.project(Kind::Lib);
    project.write(".python-version", "3.12\n");
    project
        .pyfaro(["python", "pin", "--rm"])
        .run_plain()
        .success()
        .stdout_contains("Removed Python version file");
    assert!(!project.path(".python-version").exists());

    project
        .pyfaro(["python", "pin", "--rm"])
        .run_plain()
        .code(2)
        .stderr_contains("No Python version file found");
}

#[test]
fn python_pin_rm_conflicts_with_a_request() {
    e2e_guard!();
    let sandbox = Sandbox::new();
    let project = sandbox.project(Kind::Lib);
    project.write(".python-version", "3.12\n");
    let before = project.snapshot();
    project
        .pyfaro(["python", "pin", "--rm", "3.12"])
        .run_plain()
        .code(2)
        .stderr_contains("the argument '--rm' cannot be used with '[REQUEST]'")
        .stdout_empty();
    assert_eq!(project.snapshot(), before);
}

#[test]
fn python_pin_outside_requires_python_is_rejected_by_uv() {
    e2e_guard!();
    let sandbox = Sandbox::new();
    let project = sandbox.project(Kind::Lib);
    let before = project.snapshot();
    project
        .pyfaro(["python", "pin", "3.10"])
        .run_plain()
        .code(2)
        .stderr_contains(
            "The requested Python version `3.10` is incompatible with the \
             project `requires-python` value of `>=3.12`.",
        )
        .stderr_contains("uv python pin failed")
        .stdout_empty();
    assert_eq!(project.snapshot(), before);
    assert!(!project.path(".python-version").exists());
}

#[test]
fn python_pin_rejects_a_request_that_starts_with_a_dash() {
    e2e_guard!();
    let sandbox = Sandbox::new();
    let project = sandbox.project(Kind::Lib);
    project
        .pyfaro(["python", "pin", "--", "-3"])
        .run_plain()
        .code(2)
        .stderr_contains("Requests for arbitrary names (e.g., `-3`)");
    assert!(!project.path(".python-version").exists());
}

#[test]
fn python_pin_with_project_updates_that_project_from_elsewhere() {
    e2e_guard!();
    let sandbox = Sandbox::new();
    let project = sandbox.project(Kind::Lib);
    pyfaro_in(&sandbox, &project, &["python", "pin", "3.12"])
        .success()
        .stdout_contains("Pinned `.python-version` to `3.12`");
    assert_eq!(project.read(".python-version").trim(), "3.12");
    assert!(!sandbox.join(".python-version").exists());
    pyfaro_in(&sandbox, &project, &["python", "pin"])
        .success()
        .stdout_eq("3.12\n");
    pyfaro_in(&sandbox, &project, &["python", "pin", "--rm"]).success();
    assert!(!project.path(".python-version").exists());
}

#[test]
fn python_pin_respects_the_project_requires_python_when_selected() {
    e2e_guard!();
    let sandbox = Sandbox::new();
    let project = sandbox.project(Kind::Lib);
    pyfaro_in(&sandbox, &project, &["python", "pin", "3.10"])
        .code(2)
        .stderr_contains("incompatible with the project `requires-python`");
    assert!(!project.path(".python-version").exists());
}

#[test]
fn python_pin_works_outside_a_project() {
    e2e_guard!();
    let sandbox = Sandbox::new();
    sandbox.pyfaro(["python", "pin", "3.12"]).run_plain().success();
    assert_eq!(
        fs::read_to_string(sandbox.join(".python-version")).unwrap().trim(),
        "3.12"
    );
}

#[test]
fn python_project_must_be_an_existing_project_directory() {
    e2e_guard!();
    let sandbox = Sandbox::new();
    sandbox
        .pyfaro(["python", "pin", "3.12", "--project", "missing"])
        .run_plain()
        .code(1)
        .stderr_contains("could not resolve project directory missing")
        .stdout_empty();
    assert!(!sandbox.join(".python-version").exists());

    fs::create_dir(sandbox.join("empty")).unwrap();
    sandbox
        .pyfaro(["python", "pin", "3.12", "--project", "empty"])
        .run_plain()
        .code(1)
        .stderr_contains("pyproject.toml");
    assert!(!sandbox.join("empty").join(".python-version").exists());
}

#[test]
fn python_uv_selects_the_executable() {
    e2e_guard!();
    let sandbox = Sandbox::new();
    for args in [["python", "list"], ["python", "install"], ["python", "pin"]] {
        sandbox
            .pyfaro(args)
            .args(["--uv", "missing-uv-binary"])
            .run_plain()
            .code(1)
            .stderr_contains("could not find uv at missing-uv-binary")
            .stdout_empty();
    }

    let copy = sandbox.join(executable("my-uv"));
    fs::copy(which::which("uv").unwrap(), &copy).unwrap();
    sandbox
        .pyfaro(["python", "list", "--only-installed", "--uv"])
        .arg(&copy)
        .run_plain()
        .success();
}

// ----------------------------------------------------------------- tools

#[test]
fn tools_lists_every_tool_with_exact_paths_and_versions() {
    e2e_guard!();
    let sandbox = Sandbox::new();
    let project = sandbox.synced_project(Kind::Lib);
    let venv = project.path(".venv");
    let out = project.pyfaro(["tools"]).run_plain();
    out.success().stderr_empty();
    let lines = lines(&out.stdout);
    assert_eq!(lines.len(), 4, "{}", out.stdout);
    assert!(lines[0].starts_with("uv: "), "{}", lines[0]);
    assert!(lines[0].ends_with(UV_LINE_END), "{}", lines[0]);
    assert_eq!(lines[1], tool_line(&venv, "ruff", RUFF));
    assert_eq!(lines[2], tool_line(&venv, "ty", TY));
    assert_eq!(lines[3], tool_line(&venv, "pytest", PYTEST));
}

#[test]
fn tools_named_prints_only_that_tool() {
    e2e_guard!();
    let sandbox = Sandbox::new();
    let project = sandbox.synced_project(Kind::Lib);
    let venv = project.path(".venv");
    for (name, version) in [("ruff", RUFF), ("ty", TY), ("pytest", PYTEST)] {
        project
            .pyfaro(["tools", name])
            .run_plain()
            .success()
            .stdout_eq(&format!("{}\n", tool_line(&venv, name, version)))
            .stderr_empty();
    }
    let uv = project.pyfaro(["tools", "uv"]).run_plain();
    uv.success().stderr_empty();
    assert_eq!(uv.stdout.lines().count(), 1);
    assert!(uv.stdout.starts_with("uv: "), "{}", uv.stdout);
    assert!(uv.stdout.trim_end().ends_with(UV_LINE_END), "{}", uv.stdout);
}

#[test]
fn tools_lists_missing_tools_as_unavailable_with_exit_zero() {
    e2e_guard!();
    let sandbox = Sandbox::new();
    let project = sandbox.project(Kind::Lib); // never synced: no .venv
    let out = project
        .pyfaro(["tools"])
        .env("PATH", uv_only_path(&sandbox))
        .run_plain();
    out.success().stderr_empty();
    let lines = lines(&out.stdout);
    assert_eq!(lines.len(), 4, "{}", out.stdout);
    assert!(lines[0].starts_with("uv: "), "{}", lines[0]);
    for (line, name) in lines[1..].iter().zip(["ruff", "ty", "pytest"]) {
        assert_eq!(*line, format!("{name}: unavailable ({})", not_found(name)));
    }
}

#[test]
fn tools_named_missing_tool_is_an_error() {
    e2e_guard!();
    let sandbox = Sandbox::new();
    let project = sandbox.project(Kind::Lib);
    for name in ["ruff", "ty", "pytest"] {
        project
            .pyfaro(["tools", name])
            .env("PATH", uv_only_path(&sandbox))
            .run_plain()
            .code(1)
            .stderr_contains(&format!("error: {}", not_found(name)))
            .stdout_empty();
    }
}

#[test]
fn tools_lists_a_partially_installed_environment() {
    e2e_guard!();
    let sandbox = Sandbox::new();
    let project = sandbox.project(Kind::Lib);
    sandbox
        .uv(["venv", "--no-python-downloads", "--quiet"])
        .arg(project.path(".venv"))
        .run_plain()
        .success();
    project.install_tools(&[("ruff", RUFF)]);
    let out = project
        .pyfaro(["tools"])
        .env("PATH", uv_only_path(&sandbox))
        .run_plain();
    out.success();
    let lines = lines(&out.stdout);
    assert_eq!(lines[1], tool_line(&project.path(".venv"), "ruff", RUFF));
    assert_eq!(lines[2], format!("ty: unavailable ({})", not_found("ty")));
    assert_eq!(
        lines[3],
        format!("pytest: unavailable ({})", not_found("pytest"))
    );
}

#[test]
fn tools_resolves_project_venv_then_virtual_env_then_path() {
    e2e_guard!();
    let sandbox = Sandbox::new();
    let project = sandbox.project(Kind::Lib);
    project.pyfaro(["sync"]).run_plain().success();
    let active = sandbox.join("active-env");
    let on_path = sandbox.join("path-env");
    ruff_environment(&sandbox, &active, "0.16.9");
    ruff_environment(&sandbox, &on_path, "0.16.8");
    let path = path_of(&[
        &on_path.join(scripts_directory()),
        Path::new(&uv_only_path(&sandbox)),
    ]);
    let ruff = |project: &Project| {
        project
            .pyfaro(["tools", "ruff"])
            .env("VIRTUAL_ENV", &active)
            .env("PATH", &path)
            .run_plain()
    };

    // 1. The project environment wins over both.
    ruff(&project).success().stdout_eq(&format!(
        "{}\n",
        tool_line(&project.path(".venv"), "ruff", RUFF)
    ));

    // 2. Without it, VIRTUAL_ENV wins over PATH.
    project
        .uv(["pip", "uninstall", "--quiet", "--python"])
        .arg(project.path(".venv"))
        .arg("ruff")
        .run_plain()
        .success();
    ruff(&project)
        .success()
        .stdout_eq(&format!("{}\n", tool_line(&active, "ruff", "0.16.9")));

    // 3. Without VIRTUAL_ENV, PATH is the last resort.
    project
        .pyfaro(["tools", "ruff"])
        .env_remove("VIRTUAL_ENV")
        .env("PATH", &path)
        .run_plain()
        .success()
        .stdout_eq(&format!("{}\n", tool_line(&on_path, "ruff", "0.16.8")));
}

#[test]
fn tools_without_a_project_uses_virtual_env_then_path() {
    e2e_guard!();
    let sandbox = Sandbox::new();
    let active = sandbox.join("active-env");
    let on_path = sandbox.join("path-env");
    ruff_environment(&sandbox, &active, "0.16.9");
    ruff_environment(&sandbox, &on_path, "0.16.8");
    let path = path_of(&[
        &on_path.join(scripts_directory()),
        Path::new(&uv_only_path(&sandbox)),
    ]);
    sandbox
        .pyfaro(["tools", "ruff"])
        .env("VIRTUAL_ENV", &active)
        .env("PATH", &path)
        .run_plain()
        .success()
        .stdout_eq(&format!("{}\n", tool_line(&active, "ruff", "0.16.9")));
    sandbox
        .pyfaro(["tools", "ruff"])
        .env("PATH", &path)
        .run_plain()
        .success()
        .stdout_eq(&format!("{}\n", tool_line(&on_path, "ruff", "0.16.8")));
}

#[test]
fn tools_honors_a_relative_uv_project_environment() {
    e2e_guard!();
    let sandbox = Sandbox::new();
    let project = sandbox.project(Kind::Lib);
    ruff_environment(&sandbox, &project.path("custom-env"), "0.16.9");
    project
        .pyfaro(["tools", "ruff"])
        .env("UV_PROJECT_ENVIRONMENT", "custom-env")
        .env("PATH", uv_only_path(&sandbox))
        .run_plain()
        .success()
        .stdout_eq(&format!(
            "{}\n",
            tool_line(&project.path("custom-env"), "ruff", "0.16.9")
        ));
}

#[test]
fn tools_honors_an_absolute_uv_project_environment() {
    e2e_guard!();
    let sandbox = Sandbox::new();
    let project = sandbox.synced_project(Kind::Lib);
    let elsewhere = sandbox.join("elsewhere-env");
    ruff_environment(&sandbox, &elsewhere, "0.16.9");
    project
        .pyfaro(["tools", "ruff"])
        .env("UV_PROJECT_ENVIRONMENT", &elsewhere)
        .run_plain()
        .success()
        .stdout_eq(&format!("{}\n", tool_line(&elsewhere, "ruff", "0.16.9")));
    // The variable replaces `.venv`: ty is not installed in that environment.
    project
        .pyfaro(["tools", "ty"])
        .env("UV_PROJECT_ENVIRONMENT", &elsewhere)
        .env("PATH", uv_only_path(&sandbox))
        .run_plain()
        .code(1)
        .stderr_contains(&format!("error: {}", not_found("ty")));
}

#[test]
fn tools_project_selects_the_project_from_elsewhere() {
    e2e_guard!();
    let sandbox = Sandbox::new();
    let project = sandbox.synced_project(Kind::Lib);
    let venv = project.path(".venv");
    let path = uv_only_path(&sandbox);

    // From the sandbox root there is no project and no tool on PATH.
    sandbox
        .pyfaro(["tools", "pytest"])
        .env("PATH", &path)
        .run_plain()
        .code(1)
        .stderr_contains(&format!("error: {}", not_found("pytest")));

    sandbox
        .pyfaro(["tools", "pytest", "--project"])
        .arg(&project.root)
        .env("PATH", &path)
        .run_plain()
        .success()
        .stdout_eq(&format!("{}\n", tool_line(&venv, "pytest", PYTEST)));
    sandbox
        .pyfaro(["tools", "--project"])
        .arg(&project.root)
        .env("PATH", &path)
        .run_plain()
        .success()
        .stdout_contains(&tool_line(&venv, "ruff", RUFF));
}

#[test]
fn tools_discovers_the_nearest_project_from_a_subdirectory() {
    e2e_guard!();
    let sandbox = Sandbox::new();
    let project = sandbox.synced_project(Kind::Lib);
    let venv = project.path(".venv");
    Cmd::pyfaro(&project.path("src"), ["tools", "ty"])
        .env("PATH", uv_only_path(&sandbox))
        .run_plain()
        .success()
        .stdout_eq(&format!("{}\n", tool_line(&venv, "ty", TY)));
}

#[test]
fn tools_project_without_pyproject_is_an_error() {
    e2e_guard!();
    let sandbox = Sandbox::new();
    fs::create_dir(sandbox.join("empty")).unwrap();
    sandbox
        .pyfaro(["tools", "--project", "empty"])
        .run_plain()
        .code(1)
        .stderr_contains("pyproject.toml")
        .stdout_empty();
}

#[test]
fn tools_uv_selects_the_uv_executable() {
    e2e_guard!();
    let sandbox = Sandbox::new();
    let copy = sandbox.join(executable("my-uv"));
    fs::copy(which::which("uv").unwrap(), &copy).unwrap();

    let named = sandbox.pyfaro(["tools", "uv", "--uv"]).arg(&copy).run_plain();
    named.success().stderr_empty();
    assert!(named.stdout.starts_with("uv: "), "{}", named.stdout);
    assert!(
        named.stdout.trim_end().ends_with(&format!(
            "my-uv{}{UV_LINE_END}",
            std::env::consts::EXE_SUFFIX
        )),
        "{}",
        named.stdout
    );

    let listing = sandbox.pyfaro(["tools", "--uv"]).arg(&copy).run_plain();
    listing.success();
    assert!(
        listing.stdout.lines().next().unwrap().contains("my-uv"),
        "{}",
        listing.stdout
    );

    sandbox
        .pyfaro(["tools", "uv", "--uv", "missing-uv-binary"])
        .run_plain()
        .code(1)
        .stderr_contains("could not inspect uv at missing-uv-binary")
        .stdout_empty();

    // In the listing a bad uv is reported, not fatal.
    let listing =
        sandbox.pyfaro(["tools", "--uv", "missing-uv-binary"]).run_plain();
    listing.success();
    assert!(
        listing.stdout.starts_with(
            "uv: unavailable (could not inspect uv at missing-uv-binary"
        ),
        "{}",
        listing.stdout
    );
}

#[test]
fn tools_output_stays_on_one_line_per_tool() {
    e2e_guard!();
    let sandbox = Sandbox::new();
    let hostile = "bad\nuv\tpath";

    let strict = sandbox.pyfaro(["tools", "uv", "--uv", hostile]).run_plain();
    strict.code(1).stdout_empty();
    assert_eq!(strict.stderr.lines().count(), 1, "{}", strict.stderr);
    strict.stderr_contains("could not inspect uv at bad\\nuv\\tpath");

    let listing = sandbox.pyfaro(["tools", "--uv", hostile]).run_plain();
    listing.success();
    assert_eq!(listing.stdout.lines().count(), 4, "{}", listing.stdout);
    listing.stdout_contains(
        "uv: unavailable (could not inspect uv at bad\\nuv\\tpath",
    );
}

#[test]
fn tools_does_not_write_to_the_project() {
    e2e_guard!();
    let sandbox = Sandbox::new();
    let project = sandbox.synced_project(Kind::Lib);
    let before = project.snapshot();
    project.pyfaro(["tools"]).run_plain().success();
    assert_eq!(project.snapshot(), before);

    let unsynced = Sandbox::new();
    let project = unsynced.project(Kind::Cli);
    let before = project.snapshot();
    project.pyfaro(["tools"]).run_plain().success();
    assert_eq!(project.snapshot(), before);
    assert!(!project.path(".venv").exists());
    assert!(!project.path("uv.lock").exists());
}

// ---------------------------------------------------------------- doctor

fn assert_blocking(out: &Outcome, finding: &str, count: usize) {
    out.code(1).stdout_contains(finding).stderr_contains(&format!(
        "error: doctor found {count} blocking issue(s); {DOCTOR_FAILED}"
    ));
    assert!(
        out.stdout.lines().any(|line| line.starts_with("ERROR: ")),
        "{}",
        out.stdout
    );
}

#[test]
fn doctor_reports_a_healthy_project() {
    e2e_guard!();
    let sandbox = Sandbox::new();
    let project = sandbox.synced_project(Kind::Lib);
    let venv = project.path(".venv");
    let before = project.snapshot();
    let out = project.pyfaro(["doctor"]).run_plain();
    out.success().stderr_empty();
    let lines = lines(&out.stdout);
    assert_eq!(lines.len(), 9, "{}", out.stdout);
    assert!(lines[0].starts_with("OK: project: "), "{}", lines[0]);
    assert!(lines[0].ends_with("project"), "{}", lines[0]);
    assert_eq!(lines[1], "OK: pyproject.toml: valid Pyfaro configuration");
    assert!(lines[2].starts_with("OK: Python: "), "{}", lines[2]);
    assert!(lines[2].contains("(Python 3."), "{}", lines[2]);
    assert!(lines[3].starts_with("OK: uv: "), "{}", lines[3]);
    assert!(lines[3].contains(&format!("(uv {UV})")), "{}", lines[3]);
    assert_eq!(lines[4], format!("OK: {}", tool_line(&venv, "ruff", RUFF)));
    assert_eq!(lines[5], format!("OK: {}", tool_line(&venv, "ty", TY)));
    assert_eq!(lines[6], format!("OK: {}", tool_line(&venv, "pytest", PYTEST)));
    assert_eq!(
        lines[7],
        "OK: dependencies: 3 runtime distribution(s) installed in .venv"
    );
    assert_eq!(lines[8], "OK: doctor: no blocking issues");
    assert_eq!(project.snapshot(), before, "doctor must not write");
}

#[test]
fn doctor_accepts_a_matching_python_version_pin() {
    e2e_guard!();
    let sandbox = Sandbox::new();
    let project = sandbox.synced_project(Kind::Lib);
    project.write(
        ".python-version",
        &format!("{}\n", venv_python_minor(&project)),
    );
    let out = project.pyfaro(["doctor"]).run_plain();
    out.success()
        .stderr_empty()
        .stdout_contains("OK: doctor: no blocking issues");
    assert!(!out.stdout.contains("WARN") && !out.stdout.contains("ERROR"));
}

#[test]
fn doctor_works_on_a_cli_project() {
    e2e_guard!();
    let sandbox = Sandbox::new();
    let project = sandbox.synced_project(Kind::Cli);
    project
        .pyfaro(["doctor"])
        .run_plain()
        .success()
        .stdout_contains("OK: doctor: no blocking issues")
        .stderr_empty();
}

#[test]
fn doctor_blocks_on_an_invalid_pyfaro_configuration() {
    e2e_guard!();
    let sandbox = Sandbox::new();
    let project = sandbox.synced_project(Kind::Lib);
    let pyproject = project.read("pyproject.toml");
    project.write(
        "pyproject.toml",
        &pyproject.replace("schema-version = 1", "schema-version = 9"),
    );
    let before = project.snapshot();
    let out = project.pyfaro(["doctor"]).run_plain();
    assert_blocking(&out, "ERROR: configuration: invalid ", 1);
    out.stdout_contains("tool.pyfaro.schema-version must be the integer 1; correct pyproject.toml");
    assert_eq!(project.snapshot(), before);
}

#[test]
fn doctor_blocks_on_malformed_pyproject_toml() {
    e2e_guard!();
    let sandbox = Sandbox::new();
    let project = sandbox.synced_project(Kind::Lib);
    project.write("pyproject.toml", "[project\n");
    let out = project.pyfaro(["doctor"]).run_plain();
    assert_blocking(&out, "ERROR: configuration: invalid ", 1);
    out.stdout_contains("TOML parse error at line 1")
        .stdout_contains("correct pyproject.toml");
    // The TOML error is escaped onto one line.
    assert!(
        out.stdout
            .lines()
            .any(|line| line.starts_with("ERROR: configuration:")
                && line.contains("\\n")),
        "{}",
        out.stdout
    );
}

#[test]
fn doctor_blocks_when_the_environment_is_missing() {
    e2e_guard!();
    let sandbox = Sandbox::new();
    let project = sandbox.project(Kind::Lib); // not synced
    let before = project.snapshot();
    let out = project.pyfaro(["doctor"]).run_plain();
    assert_blocking(&out, "ERROR: dependencies: project interpreter ", 1);
    out.stdout_contains("is missing; run pyfaro sync");
    assert_eq!(project.snapshot(), before);
    assert!(!project.path(".venv").exists(), "doctor must not create .venv");
}

#[test]
fn doctor_blocks_on_missing_runtime_distributions() {
    e2e_guard!();
    let sandbox = Sandbox::new();
    let project = sandbox.synced_project(Kind::Lib);
    let pyproject = project.read("pyproject.toml");
    project.write(
        "pyproject.toml",
        &pyproject.replace("dependencies = []", "dependencies = [\"idna\"]"),
    );
    let before = project.snapshot();
    let out = project.pyfaro(["doctor"]).run_plain();
    assert_blocking(
        &out,
        "ERROR: dependencies: idna is missing from .venv; run pyfaro sync",
        1,
    );
    assert_eq!(project.snapshot(), before);
}

#[test]
fn doctor_blocks_on_missing_default_group_distributions() {
    e2e_guard!();
    let sandbox = Sandbox::new();
    let project = sandbox.synced_project(Kind::Lib);
    project
        .uv(["pip", "uninstall", "--quiet", "--python"])
        .arg(project.path(".venv"))
        .arg("pytest")
        .run_plain()
        .success();
    let out = project
        .pyfaro(["doctor"])
        .env("PATH", uv_only_path(&sandbox))
        .run_plain();
    assert_blocking(
        &out,
        "ERROR: dependencies: pytest is missing from .venv; run pyfaro sync",
        1,
    );
    // The same finding appears as a warning about the missing executable.
    out.stdout_contains(&format!("WARN: {}\n", not_found("pytest")));
    assert!(!out.stdout.contains("pytest: pytest:"), "{}", out.stdout);
}

#[test]
fn doctor_ignores_default_groups_switched_off_by_uv_environment() {
    e2e_guard!();
    let sandbox = Sandbox::new();
    let project = sandbox.synced_project(Kind::Lib);
    project
        .uv(["pip", "uninstall", "--quiet", "--python"])
        .arg(project.path(".venv"))
        .arg("pytest")
        .run_plain()
        .success();
    let out = project
        .pyfaro(["doctor"])
        .env("UV_NO_DEV", "1")
        .env("PATH", uv_only_path(&sandbox))
        .run_plain();
    out.success().stdout_contains("OK: doctor: no blocking issues");
}

#[test]
fn doctor_blocks_on_layout_conflicts() {
    e2e_guard!();
    let sandbox = Sandbox::new();
    let project = sandbox.synced_project(Kind::Lib);

    // A file where `src` must be a directory.
    fs::rename(project.path("src"), sandbox.join("src.moved")).unwrap();
    project.write("src", "not a directory\n");
    let out = project.pyfaro(["doctor"]).run_plain();
    assert_blocking(&out, "ERROR: layout: ", 1);
    out.stdout_contains(
        "src must be a directory; move or remove the conflicting file",
    );
    fs::remove_file(project.path("src")).unwrap();
    fs::rename(sandbox.join("src.moved"), project.path("src")).unwrap();

    // A file where `tests` must be a directory.
    fs::rename(project.path("tests"), sandbox.join("tests.moved")).unwrap();
    project.write("tests", "not a directory\n");
    let out = project.pyfaro(["doctor"]).run_plain();
    assert_blocking(&out, "ERROR: layout: ", 1);
    out.stdout_contains(
        "must be a directory; move or remove the conflicting file",
    );
    fs::remove_file(project.path("tests")).unwrap();
    fs::rename(sandbox.join("tests.moved"), project.path("tests")).unwrap();

    // A file where the environment must be a directory.
    fs::rename(project.path(".venv"), sandbox.join("venv.moved")).unwrap();
    project.write(".venv", "not a directory\n");
    let out = project.pyfaro(["doctor"]).run_plain();
    out.code(1).stdout_contains("ERROR: layout: ").stdout_contains(
        ".venv must be a directory; move or remove the conflicting file",
    );
}

#[test]
fn doctor_blocks_on_an_environment_without_an_interpreter() {
    e2e_guard!();
    let sandbox = Sandbox::new();
    let project = sandbox.project(Kind::Lib);
    fs::create_dir_all(project.path(".venv").join(scripts_directory()))
        .unwrap();
    let out = project.pyfaro(["doctor"]).run_plain();
    out.code(1)
        .stdout_contains(
            "has no Python interpreter; repair it with pyfaro sync",
        )
        .stderr_contains(DOCTOR_FAILED);
}

#[test]
fn doctor_blocks_on_conflicting_tool_configuration() {
    e2e_guard!();
    let sandbox = Sandbox::new();
    let project = sandbox.synced_project(Kind::Lib);

    project.write("ruff.toml", "line-length = 100\n");
    assert_blocking(
        &project.pyfaro(["doctor"]).run_plain(),
        "ERROR: configuration: [tool.ruff] and ruff.toml both configure ruff; keep one source of configuration",
        1,
    );
    fs::remove_file(project.path("ruff.toml")).unwrap();

    project.write("pytest.ini", "[pytest]\n");
    assert_blocking(
        &project.pyfaro(["doctor"]).run_plain(),
        "ERROR: configuration: [tool.pytest] and pytest.ini both configure pytest; keep one source of configuration",
        1,
    );
    fs::remove_file(project.path("pytest.ini")).unwrap();

    let pyproject = project.read("pyproject.toml");
    project.write("pyproject.toml", &format!("{pyproject}\n[tool.ty.rules]\n"));
    project.write("ty.toml", "[rules]\n");
    assert_blocking(
        &project.pyfaro(["doctor"]).run_plain(),
        "ERROR: configuration: [tool.ty] and ty.toml both configure ty; keep one source of configuration",
        1,
    );
}

#[test]
fn doctor_blocks_on_a_python_pin_the_interpreter_does_not_match() {
    e2e_guard!();
    let sandbox = Sandbox::new();
    let project = sandbox.synced_project(Kind::Lib);
    project.write(".python-version", "3.9\n");
    let before = project.snapshot();
    let out = project.pyfaro(["doctor"]).run_plain();
    assert_blocking(&out, "ERROR: Python: Python 3.", 1);
    out.stdout_contains(
        "does not match .python-version \"3.9\"; select a matching interpreter or update the pin",
    );
    assert_eq!(project.snapshot(), before);
}

#[test]
fn doctor_blocks_when_python_is_outside_requires_python() {
    e2e_guard!();
    let sandbox = Sandbox::new();
    let project = sandbox.synced_project(Kind::Lib);
    let pyproject = project.read("pyproject.toml");
    project.write(
        "pyproject.toml",
        &pyproject.replace(
            "requires-python = \">=3.12\"",
            "requires-python = \">=3.99\"",
        ),
    );
    let out = project.pyfaro(["doctor"]).run_plain();
    assert_blocking(&out, "ERROR: Python: Python 3.", 1);
    out.stdout_contains("does not satisfy requires-python \">=3.99\"; select a compatible interpreter");
}

#[test]
fn doctor_warns_about_a_pin_it_cannot_compare_without_failing() {
    e2e_guard!();
    let sandbox = Sandbox::new();
    let project = sandbox.synced_project(Kind::Lib);
    project.write(".python-version", "foo\n");
    project
        .pyfaro(["doctor"])
        .run_plain()
        .success()
        .stdout_contains(
            "WARN: Python: cannot compare .python-version request \"foo\" with the selected interpreter; verify the pin",
        )
        .stdout_contains("OK: doctor: no blocking issues")
        .stderr_empty();
}

#[test]
fn doctor_warnings_for_missing_optional_tools_do_not_fail() {
    e2e_guard!();
    let sandbox = Sandbox::new();
    let project = sandbox.synced_project(Kind::Lib);
    // Remove only the executables: the distributions stay installed.
    for name in ["ruff", "ty", "pytest"] {
        fs::remove_file(
            project
                .path(".venv")
                .join(scripts_directory())
                .join(executable(name)),
        )
        .unwrap();
    }
    let before = project.snapshot();
    let out = project
        .pyfaro(["doctor"])
        .env("PATH", uv_only_path(&sandbox))
        .run_plain();
    out.success().stderr_empty();
    for name in ["ruff", "ty", "pytest"] {
        out.stdout_contains(&format!("WARN: {}\n", not_found(name)));
        assert!(!out.stdout.contains(&format!("{name}: {name}:")));
    }
    out.stdout_contains("OK: doctor: no blocking issues");
    assert!(!out.stdout.contains("ERROR"), "{}", out.stdout);
    assert_eq!(project.snapshot(), before);
}

#[test]
fn doctor_project_selects_the_project_from_elsewhere() {
    e2e_guard!();
    let sandbox = Sandbox::new();
    let project = sandbox.synced_project(Kind::Lib);
    let before = project.snapshot();
    let out =
        sandbox.pyfaro(["doctor", "--project"]).arg(&project.root).run_plain();
    out.success()
        .stderr_empty()
        .stdout_contains("OK: doctor: no blocking issues")
        .stdout_contains(&format!(
            "OK: {}",
            tool_line(&project.path(".venv"), "ruff", RUFF)
        ));
    assert_eq!(project.snapshot(), before);

    // From the sandbox root alone there is no project to diagnose.
    let alone = sandbox.pyfaro(["doctor"]).run_plain();
    alone
        .code(1)
        .stdout_contains("ERROR: project: no pyproject.toml found from ")
        .stdout_contains("; use --project <PATH>\n")
        .stderr_contains(DOCTOR_FAILED);
    assert!(
        !alone.stdout.contains("<PATH>; use --project"),
        "{}",
        alone.stdout
    );
    assert!(!alone.stdout.contains("to select a project"), "{}", alone.stdout);

    // From inside the project the nearest project is found.
    Cmd::pyfaro(&project.path("tests"), ["doctor"])
        .run_plain()
        .success()
        .stdout_contains("OK: doctor: no blocking issues");
}

#[test]
fn doctor_uv_selects_the_uv_executable() {
    e2e_guard!();
    let sandbox = Sandbox::new();
    let project = sandbox.synced_project(Kind::Lib);
    let copy = sandbox.join(executable("my-uv"));
    fs::copy(which::which("uv").unwrap(), &copy).unwrap();

    let out = project.pyfaro(["doctor", "--uv"]).arg(&copy).run_plain();
    out.success();
    assert!(
        out.stdout
            .lines()
            .any(|line| line.starts_with("OK: uv: ") && line.contains("my-uv")),
        "{}",
        out.stdout
    );

    let before = project.snapshot();
    let out =
        project.pyfaro(["doctor", "--uv", "missing-uv-binary"]).run_plain();
    assert_blocking(
        &out,
        "ERROR: uv: could not inspect uv at missing-uv-binary: could not find uv at missing-uv-binary",
        1,
    );
    assert_eq!(project.snapshot(), before);
}

#[test]
fn doctor_honors_uv_project_environment() {
    e2e_guard!();
    let sandbox = Sandbox::new();
    let project = sandbox.project(Kind::Lib);
    project
        .pyfaro(["sync"])
        .env("UV_PROJECT_ENVIRONMENT", "custom-env")
        .run_plain()
        .success();
    assert!(!project.path(".venv").exists());

    // Without the variable doctor looks in `.venv`, which does not exist.
    project
        .pyfaro(["doctor"])
        .run_plain()
        .code(1)
        .stdout_contains("is missing; run pyfaro sync");

    // A relative value is resolved from the project root.
    let out = project
        .pyfaro(["doctor"])
        .env("UV_PROJECT_ENVIRONMENT", "custom-env")
        .run_plain();
    out.success()
        .stdout_contains("OK: doctor: no blocking issues")
        .stdout_contains(&format!(
            "OK: {}",
            tool_line(&project.path("custom-env"), "pytest", PYTEST)
        ))
        .stdout_contains("runtime distribution(s) installed in ");
    assert!(
        out.stdout.lines().any(|line| line.starts_with("OK: dependencies: ")
            && line.ends_with("custom-env")),
        "{}",
        out.stdout
    );
}
