//! `pyfaro sync`, `add`, `remove` and `run`: the commands that manage and use
//! the project environment through uv.
//!
//! Dependencies are dependency-free local path packages created in the
//! sandbox, so only the generated project's own tools come from the index.

use crate::harness::{Kind, Project, Sandbox, snapshot};

/// Write a minimal local package `name` (importable as `module`) to
/// `<sandbox>/<directory>`.
fn local_package(sandbox: &Sandbox, directory: &str, name: &str, module: &str) {
    sandbox.write(
        format!("{directory}/pyproject.toml"),
        &format!(
            "[project]\nname = \"{name}\"\nversion = \"1.0\"\n\
             requires-python = \">=3.8\"\n\n\
             [build-system]\nrequires = [\"uv_build>=0.9.26,<0.10.0\"]\n\
             build-backend = \"uv_build\"\n"
        ),
    );
    sandbox
        .write(format!("{directory}/src/{module}/__init__.py"), "VALUE = 1\n");
}

/// A project plus `<sandbox>/dep` (`local-dep`) next to it.
fn project_with_local_dep(sandbox: &Sandbox) -> Project {
    local_package(sandbox, "dep", "local-dep", "local_dep");
    sandbox.project(Kind::Lib)
}

/// Declare `local-dep` in the project metadata without touching `uv.lock`,
/// which makes an existing lock stale.
fn make_lock_stale(project: &Project) {
    let metadata = project
        .read("pyproject.toml")
        .replace("dependencies = []", "dependencies = [\"local-dep\"]");
    project.write(
        "pyproject.toml",
        &format!("{metadata}\n[tool.uv.sources]\nlocal-dep = {{ path = \"../dep\" }}\n"),
    );
}

/// `uv pip list` of the project environment.
fn installed(project: &Project) -> String {
    project
        .uv(["pip", "list", "--python"])
        .arg(project.path(".venv"))
        .run()
        .success()
        .stdout
        .clone()
}

fn lock_of(project: &Project) -> String {
    project.read("uv.lock")
}

// ---------------------------------------------------------------- sync

#[test]
fn sync_creates_the_environment_and_the_lock() {
    e2e_guard!();
    let sandbox = Sandbox::new();
    let project = sandbox.project(Kind::Lib);
    assert!(!project.path(".venv").exists());
    project
        .pyfaro(["sync"])
        .run()
        .success()
        .stdout_empty()
        .stderr_contains("Creating virtual environment")
        .stderr_contains("Resolved")
        .stderr_contains("+ pytest==9.1.1")
        .stderr_contains("+ ruff==0.16.10")
        .stderr_contains("+ ty==0.0.85");
    assert!(project.path(".venv/pyvenv.cfg").is_file());
    assert!(lock_of(&project).contains("name = \"acme-widget\""));
    let packages = installed(&project);
    for expected in ["acme-widget", "pytest", "ruff", "ty"] {
        assert!(packages.contains(expected), "{expected} missing:\n{packages}");
    }
}

#[test]
fn sync_is_idempotent_and_leaves_the_lock_alone() {
    e2e_guard!();
    let sandbox = Sandbox::new();
    let project = sandbox.synced_project(Kind::Lib);
    let before = project.snapshot();
    project
        .pyfaro(["sync"])
        .run()
        .success()
        .stderr_contains("Resolved")
        .stderr_contains("Audited");
    assert_eq!(project.snapshot(), before);
}

#[test]
fn sync_path_starts_discovery_from_a_nested_directory() {
    e2e_guard!();
    let sandbox = Sandbox::new();
    let project = sandbox.project(Kind::Lib);
    sandbox.pyfaro(["sync", "project/src/acme_widget"]).run().success();
    assert!(project.path(".venv").is_dir());
    assert!(project.path("uv.lock").is_file());
    assert!(!project.module(".venv").exists());
    assert!(!sandbox.join(".venv").exists());
}

#[test]
fn sync_without_a_path_discovers_the_project_from_the_working_directory() {
    e2e_guard!();
    let sandbox = Sandbox::new();
    let project = sandbox.project(Kind::Lib);
    project
        .pyfaro(["sync"])
        .cwd(project.path("src/acme_widget"))
        .run()
        .success();
    assert!(project.path(".venv").is_dir());
    assert!(project.path("uv.lock").is_file());
    assert!(!project.path("src/acme_widget/.venv").exists());
}

#[test]
fn sync_project_selects_the_exact_directory() {
    e2e_guard!();
    let sandbox = Sandbox::new();
    let project = sandbox.project(Kind::Lib);
    sandbox.pyfaro(["sync", "--project", "project"]).run().success();
    assert!(project.path(".venv").is_dir());
    assert!(project.path("uv.lock").is_file());
}

#[test]
fn sync_project_never_falls_back_to_a_parent_directory() {
    e2e_guard!();
    let sandbox = Sandbox::new();
    let project = sandbox.project(Kind::Lib);
    let before = snapshot(sandbox.path());
    let out = sandbox.pyfaro(["sync", "--project", "project/src"]).run();
    out.code(1)
        .stdout_empty()
        .stderr_contains("error: project directory has no pyproject.toml: ")
        .stderr_contains("src");
    assert_eq!(snapshot(sandbox.path()), before);
    assert!(!project.path(".venv").exists());
}

#[test]
fn sync_project_conflicts_with_a_positional_path() {
    e2e_guard!();
    let sandbox = Sandbox::new();
    let project = sandbox.project(Kind::Lib);
    let before = project.snapshot();
    project
        .pyfaro(["sync", ".", "--project", "."])
        .run()
        .code(2)
        .stdout_empty()
        .stderr_contains(
            "the argument '[PATH]' cannot be used with '--project <PROJECT>'",
        );
    assert_eq!(project.snapshot(), before);
    assert!(!project.path(".venv").exists());
}

#[test]
fn sync_outside_a_project_reports_where_it_looked() {
    e2e_guard!();
    let sandbox = Sandbox::new();
    let out = sandbox.pyfaro(["sync"]).run();
    out.code(1)
        .stdout_empty()
        .stderr_contains("error: no pyproject.toml found from ")
        .stderr_contains("; use --project <PATH>");
    assert!(snapshot(sandbox.path()).is_empty());
    assert!(!sandbox.join(".venv").exists());
}

#[test]
fn sync_locked_rejects_an_absent_lock_and_forwards_uvs_exit_code() {
    e2e_guard!();
    let sandbox = Sandbox::new();
    let project = sandbox.project(Kind::Lib);
    let before = project.snapshot();
    project
        .pyfaro(["sync", "--locked"])
        .run()
        .code(2)
        .stdout_empty()
        .stderr_contains(
            "Unable to find lockfile at `uv.lock`, but `--locked` was provided",
        )
        .stderr_contains("error: uv sync failed (");
    assert_eq!(project.snapshot(), before);
    assert!(!project.path("uv.lock").exists());
}

#[test]
fn sync_locked_rejects_a_stale_lock_without_changing_it() {
    e2e_guard!();
    let sandbox = Sandbox::new();
    let project = project_with_local_dep(&sandbox);
    project.pyfaro(["sync"]).run().success();
    make_lock_stale(&project);
    let before = project.snapshot();
    let lock = lock_of(&project);
    project
        .pyfaro(["sync", "--locked", "--offline"])
        .run()
        .code(1)
        .stdout_empty()
        .stderr_contains("The lockfile at `uv.lock` needs to be updated, but `--locked` was provided")
        .stderr_contains("error: uv sync failed (");
    assert_eq!(project.snapshot(), before);
    assert_eq!(lock_of(&project), lock);
    assert!(!installed(&project).contains("local-dep"));
}

#[test]
fn sync_locked_accepts_a_fresh_lock() {
    e2e_guard!();
    let sandbox = Sandbox::new();
    let project = sandbox.synced_project(Kind::Lib);
    let before = project.snapshot();
    project
        .pyfaro(["sync", "--locked", "--offline"])
        .run()
        .success()
        .stdout_empty()
        .stderr_contains("Audited");
    assert_eq!(project.snapshot(), before);
}

#[test]
fn sync_frozen_installs_from_the_existing_lock_without_updating_it() {
    e2e_guard!();
    let sandbox = Sandbox::new();
    let project = project_with_local_dep(&sandbox);
    project.uv(["lock"]).run().success();
    make_lock_stale(&project);
    let before = project.snapshot();
    project.pyfaro(["sync", "--frozen"]).run().success();
    assert_eq!(
        project.snapshot(),
        before,
        "frozen sync must not rewrite files"
    );
    let packages = installed(&project);
    assert!(packages.contains("pytest"), "{packages}");
    assert!(
        !packages.contains("local-dep"),
        "a frozen sync skips the freshness check:\n{packages}"
    );
}

#[test]
fn sync_frozen_requires_an_existing_lock() {
    e2e_guard!();
    let sandbox = Sandbox::new();
    let project = sandbox.project(Kind::Lib);
    let before = project.snapshot();
    project
        .pyfaro(["sync", "--frozen"])
        .run()
        .code(2)
        .stderr_contains(
            "Unable to find lockfile at `uv.lock`, but `--frozen` was provided",
        )
        .stderr_contains("error: uv sync failed (");
    assert_eq!(project.snapshot(), before);
}

#[test]
fn sync_locked_conflicts_with_frozen() {
    e2e_guard!();
    let sandbox = Sandbox::new();
    let project = sandbox.project(Kind::Lib);
    let before = project.snapshot();
    project
        .pyfaro(["sync", "--locked", "--frozen"])
        .run()
        .code(2)
        .stdout_empty()
        .stderr_contains(
            "the argument '--locked' cannot be used with '--frozen'",
        );
    assert_eq!(project.snapshot(), before);
    assert!(!project.path(".venv").exists());
}

#[test]
fn sync_offline_succeeds_with_a_warm_cache() {
    e2e_guard!();
    let sandbox = Sandbox::new();
    let project = sandbox.project(Kind::Lib);
    // Warm the shared cache, then repeat offline in a fresh project.
    project.pyfaro(["sync"]).run().success();
    let again = Sandbox::new();
    let second = again.project(Kind::Lib);
    second.pyfaro(["sync", "--offline"]).run().success();
    assert!(second.path(".venv").is_dir());
    assert!(second.path("uv.lock").is_file());
}

#[test]
fn sync_offline_refuses_to_reach_the_network() {
    e2e_guard!();
    let sandbox = Sandbox::new();
    let project = sandbox.project(Kind::Lib);
    let metadata = project.read("pyproject.toml").replace(
        "dependencies = []",
        "dependencies = [\"pyfaro-e2e-package-that-does-not-exist\"]",
    );
    project.write("pyproject.toml", &metadata);
    let out = project.pyfaro(["sync", "--offline"]).run();
    out.code(1)
        .stdout_empty()
        .stderr_contains("pyfaro-e2e-package-that-does-not-exist")
        .stderr_contains("the network was disabled")
        .stderr_contains("error: uv sync failed (");
    assert!(!project.path("uv.lock").exists());
}

#[test]
fn sync_reports_a_missing_uv_with_the_remedy() {
    e2e_guard!();
    let sandbox = Sandbox::new();
    let project = sandbox.project(Kind::Lib);
    let before = project.snapshot();
    project
        .pyfaro(["sync", "--uv", "pyfaro-e2e-no-such-uv"])
        .run()
        .code(1)
        .stdout_empty()
        .stderr_contains("error: could not find uv at pyfaro-e2e-no-such-uv")
        .stderr_contains("install uv or use --uv <path>");
    assert_eq!(project.snapshot(), before);
}

#[test]
fn sync_rejects_an_executable_that_is_not_a_supported_uv() {
    e2e_guard!();
    let sandbox = Sandbox::new();
    let project = sandbox.project(Kind::Lib);
    let before = project.snapshot();
    // pyfaro itself answers `--version` with something that is not uv.
    project
        .pyfaro(["sync", "--uv", env!("CARGO_BIN_EXE_pyfaro")])
        .run()
        .code(1)
        .stdout_empty()
        .stderr_contains("error: unsupported uv version: pyfaro ")
        .stderr_contains("use uv >=0.9.26 (tested: 0.9.26)");
    assert_eq!(project.snapshot(), before);
    assert!(!project.path(".venv").exists());
}

#[cfg(unix)]
#[test]
fn sync_rejects_a_uv_that_is_too_old() {
    use std::fs;
    use std::os::unix::fs::PermissionsExt;

    e2e_guard!();
    let sandbox = Sandbox::new();
    let project = sandbox.project(Kind::Lib);
    let old_uv = sandbox
        .write("old-uv", "#!/bin/sh\necho 'uv 0.9.25 (0000000 2025-01-01)'\n");
    fs::set_permissions(&old_uv, fs::Permissions::from_mode(0o755)).unwrap();
    let before = project.snapshot();
    project
        .pyfaro(["sync", "--uv"])
        .arg(&old_uv)
        .run()
        .code(1)
        .stdout_empty()
        .stderr_contains("error: unsupported uv version: uv 0.9.25")
        .stderr_contains("use uv >=0.9.26 (tested: 0.9.26)");
    assert_eq!(project.snapshot(), before);
}

#[test]
fn sync_reports_an_invalid_tool_pyfaro_config_with_the_file_path() {
    e2e_guard!();
    let sandbox = Sandbox::new();
    let project = sandbox.project(Kind::Lib);
    let metadata = project.read("pyproject.toml").replace(
        "schema-version = 1\n",
        "schema-version = 1\nbogus-setting = 1\n",
    );
    project.write("pyproject.toml", &metadata);
    let before = project.snapshot();
    let out = project.pyfaro(["sync"]).run();
    out.code(1)
        .stdout_empty()
        .stderr_contains("error: invalid ")
        .stderr_contains(
            "pyproject.toml: unknown [tool.pyfaro] setting: bogus-setting",
        );
    assert_eq!(project.snapshot(), before);
    assert!(!project.path(".venv").exists());
}

#[test]
fn sync_reports_malformed_project_metadata_with_the_file_path() {
    e2e_guard!();
    let sandbox = Sandbox::new();
    let project = sandbox.project(Kind::Lib);
    project.write("pyproject.toml", "[project\n");
    project
        .pyfaro(["sync"])
        .run()
        .code(1)
        .stdout_empty()
        .stderr_contains("error: invalid ")
        .stderr_contains("pyproject.toml: TOML parse error");
    assert!(!project.path(".venv").exists());
}

// ------------------------------------------------------- add and remove

#[test]
fn add_records_a_runtime_dependency_in_metadata_and_lock_without_syncing() {
    e2e_guard!();
    let sandbox = Sandbox::new();
    let project = project_with_local_dep(&sandbox);
    project
        .pyfaro(["add", "../dep"])
        .run()
        .success()
        .stdout_empty()
        .stderr_contains("Resolved");
    let metadata = project.read("pyproject.toml");
    assert!(
        metadata.contains("dependencies = [\n    \"local-dep\",\n]"),
        "{metadata}"
    );
    assert!(metadata.contains("[tool.uv.sources]"), "{metadata}");
    assert!(metadata.contains("local-dep = { path = "), "{metadata}");
    assert!(lock_of(&project).contains("name = \"local-dep\""));
    assert!(!project.path(".venv").exists(), "add must not sync");
}

#[test]
fn add_does_not_install_into_an_existing_environment() {
    e2e_guard!();
    let sandbox = Sandbox::new();
    let project = project_with_local_dep(&sandbox);
    project.pyfaro(["sync"]).run().success();
    project.pyfaro(["add", "../dep"]).run().success();
    assert!(lock_of(&project).contains("name = \"local-dep\""));
    assert!(!installed(&project).contains("local-dep"));
    project.pyfaro(["sync"]).run().success();
    assert!(installed(&project).contains("local-dep"));
}

#[test]
fn add_dev_writes_to_the_dev_group() {
    e2e_guard!();
    let sandbox = Sandbox::new();
    let project = project_with_local_dep(&sandbox);
    project.pyfaro(["add", "--dev", "../dep"]).run().success();
    let metadata = project.read("pyproject.toml");
    assert!(metadata.contains("dependencies = []"), "{metadata}");
    let dev = metadata
        .split("dev = [")
        .nth(1)
        .and_then(|rest| rest.split(']').next())
        .expect("dev group");
    assert!(dev.contains("\"local-dep\""), "{metadata}");
    assert!(dev.contains("\"pytest==9.1.1\""), "{metadata}");
    assert!(lock_of(&project).contains("name = \"local-dep\""));
    assert!(!project.path(".venv").exists());
}

#[test]
fn add_group_creates_a_named_dependency_group() {
    e2e_guard!();
    let sandbox = Sandbox::new();
    let project = project_with_local_dep(&sandbox);
    project.pyfaro(["add", "--group", "extras", "../dep"]).run().success();
    let metadata = project.read("pyproject.toml");
    assert!(metadata.contains("dependencies = []"), "{metadata}");
    assert!(
        metadata.contains("extras = [\n    \"local-dep\",\n]"),
        "{metadata}"
    );
    assert!(lock_of(&project).contains("name = \"local-dep\""));
    assert!(!project.path(".venv").exists());
}

#[test]
fn add_dev_conflicts_with_group() {
    e2e_guard!();
    let sandbox = Sandbox::new();
    let project = project_with_local_dep(&sandbox);
    let before = project.snapshot();
    project
        .pyfaro(["add", "--dev", "--group", "extras", "../dep"])
        .run()
        .code(2)
        .stdout_empty()
        .stderr_contains(
            "the argument '--dev' cannot be used with '--group <GROUP>'",
        );
    assert_eq!(project.snapshot(), before);
}

#[test]
fn remove_dev_conflicts_with_group() {
    e2e_guard!();
    let sandbox = Sandbox::new();
    let project = project_with_local_dep(&sandbox);
    let before = project.snapshot();
    project
        .pyfaro(["remove", "--dev", "--group", "extras", "local-dep"])
        .run()
        .code(2)
        .stdout_empty()
        .stderr_contains(
            "the argument '--dev' cannot be used with '--group <GROUP>'",
        );
    assert_eq!(project.snapshot(), before);
}

#[test]
fn add_accepts_several_packages_at_once() {
    e2e_guard!();
    let sandbox = Sandbox::new();
    let project = project_with_local_dep(&sandbox);
    local_package(&sandbox, "other", "other-dep", "other_dep");
    project.pyfaro(["add", "../dep", "../other"]).run().success();
    let metadata = project.read("pyproject.toml");
    assert!(metadata.contains("\"local-dep\""), "{metadata}");
    assert!(metadata.contains("\"other-dep\""), "{metadata}");
    let lock = lock_of(&project);
    assert!(lock.contains("name = \"local-dep\""));
    assert!(lock.contains("name = \"other-dep\""));
    assert!(!project.path(".venv").exists());
}

#[test]
fn add_requires_at_least_one_package() {
    e2e_guard!();
    let sandbox = Sandbox::new();
    let project = sandbox.project(Kind::Lib);
    let before = project.snapshot();
    project
        .pyfaro(["add"])
        .run()
        .code(2)
        .stdout_empty()
        .stderr_contains("required arguments were not provided")
        .stderr_contains("<PACKAGES>...");
    assert_eq!(project.snapshot(), before);
}

#[test]
fn add_forwards_uvs_failure_for_an_unavailable_package_offline() {
    e2e_guard!();
    let sandbox = Sandbox::new();
    let project = sandbox.project(Kind::Lib);
    let before = project.snapshot();
    project
        .pyfaro(["add", "--offline", "pyfaro-e2e-package-that-does-not-exist"])
        .run()
        .code(1)
        .stdout_empty()
        .stderr_contains("pyfaro-e2e-package-that-does-not-exist")
        .stderr_contains("error: uv add failed (");
    assert_eq!(project.snapshot(), before);
}

#[test]
fn add_with_project_selects_the_exact_directory() {
    e2e_guard!();
    let sandbox = Sandbox::new();
    let project = project_with_local_dep(&sandbox);
    // uv runs in the project root, so relative requirement paths resolve
    // from there, not from the caller's directory.
    sandbox.pyfaro(["add", "--project", "project", "../dep"]).run().success();
    assert!(project.read("pyproject.toml").contains("\"local-dep\""));
    assert!(lock_of(&project).contains("name = \"local-dep\""));
    assert!(!project.path(".venv").exists());
}

#[test]
fn add_with_project_requires_a_pyproject_in_that_directory() {
    e2e_guard!();
    let sandbox = Sandbox::new();
    let project = project_with_local_dep(&sandbox);
    let before = snapshot(sandbox.path());
    sandbox
        .pyfaro(["add", "--project", "project/src", "dep"])
        .run()
        .code(1)
        .stdout_empty()
        .stderr_contains("error: project directory has no pyproject.toml: ");
    assert_eq!(snapshot(sandbox.path()), before);
    assert!(!project.path("uv.lock").exists());
}

#[test]
fn add_discovers_the_project_from_a_nested_directory() {
    e2e_guard!();
    let sandbox = Sandbox::new();
    let project = project_with_local_dep(&sandbox);
    project
        // Relative to the project root, where uv runs.
        .pyfaro(["add", "../dep"])
        .cwd(project.path("src/acme_widget"))
        .run()
        .success();
    assert!(project.read("pyproject.toml").contains("\"local-dep\""));
    assert!(project.path("uv.lock").is_file());
    assert!(!project.path("src/acme_widget/pyproject.toml").exists());
    assert!(!project.path("src/acme_widget/uv.lock").exists());
}

#[test]
fn add_outside_a_project_reports_where_it_looked() {
    e2e_guard!();
    let sandbox = Sandbox::new();
    sandbox
        .pyfaro(["add", "anything"])
        .run()
        .code(1)
        .stdout_empty()
        .stderr_contains("error: no pyproject.toml found from ")
        .stderr_contains("; use --project <PATH>");
    assert!(snapshot(sandbox.path()).is_empty());
}

#[test]
fn add_offline_resolves_a_local_package_without_the_network() {
    e2e_guard!();
    let sandbox = Sandbox::new();
    let project = project_with_local_dep(&sandbox);
    project.pyfaro(["add", "--offline", "../dep"]).run().success();
    assert!(lock_of(&project).contains("name = \"local-dep\""));
}

#[test]
fn add_reports_a_missing_uv() {
    e2e_guard!();
    let sandbox = Sandbox::new();
    let project = project_with_local_dep(&sandbox);
    let before = project.snapshot();
    project
        .pyfaro(["add", "--uv", "pyfaro-e2e-no-such-uv", "../dep"])
        .run()
        .code(1)
        .stderr_contains("error: could not find uv at pyfaro-e2e-no-such-uv");
    assert_eq!(project.snapshot(), before);
}

#[test]
fn remove_deletes_a_runtime_dependency_from_metadata_and_lock() {
    e2e_guard!();
    let sandbox = Sandbox::new();
    let project = project_with_local_dep(&sandbox);
    project.pyfaro(["add", "../dep"]).run().success();
    project.pyfaro(["remove", "local-dep"]).run().success().stdout_empty();
    let metadata = project.read("pyproject.toml");
    assert!(metadata.contains("dependencies = []"), "{metadata}");
    assert!(!metadata.contains("local-dep"), "{metadata}");
    assert!(!lock_of(&project).contains("local-dep"));
    assert!(!project.path(".venv").exists(), "remove must not sync");
}

#[test]
fn remove_dev_deletes_from_the_dev_group() {
    e2e_guard!();
    let sandbox = Sandbox::new();
    let project = project_with_local_dep(&sandbox);
    project.pyfaro(["add", "--dev", "../dep"]).run().success();
    project.pyfaro(["remove", "--dev", "local-dep"]).run().success();
    let metadata = project.read("pyproject.toml");
    assert!(!metadata.contains("local-dep"), "{metadata}");
    assert!(metadata.contains("\"pytest==9.1.1\""), "{metadata}");
    assert!(!lock_of(&project).contains("local-dep"));
    assert!(!project.path(".venv").exists());
}

#[test]
fn remove_group_deletes_from_the_named_group() {
    e2e_guard!();
    let sandbox = Sandbox::new();
    let project = project_with_local_dep(&sandbox);
    project.pyfaro(["add", "--group", "extras", "../dep"]).run().success();
    project
        .pyfaro(["remove", "--group", "extras", "local-dep"])
        .run()
        .success();
    assert!(!project.read("pyproject.toml").contains("local-dep"));
    assert!(!lock_of(&project).contains("local-dep"));
    assert!(!project.path(".venv").exists());
}

#[test]
fn remove_accepts_several_packages_at_once() {
    e2e_guard!();
    let sandbox = Sandbox::new();
    let project = project_with_local_dep(&sandbox);
    local_package(&sandbox, "other", "other-dep", "other_dep");
    project.pyfaro(["add", "../dep", "../other"]).run().success();
    project.pyfaro(["remove", "local-dep", "other-dep"]).run().success();
    let metadata = project.read("pyproject.toml");
    assert!(!metadata.contains("local-dep"), "{metadata}");
    assert!(!metadata.contains("other-dep"), "{metadata}");
}

#[test]
fn remove_does_not_uninstall_from_an_existing_environment() {
    e2e_guard!();
    let sandbox = Sandbox::new();
    let project = project_with_local_dep(&sandbox);
    project.pyfaro(["add", "../dep"]).run().success();
    project.pyfaro(["sync"]).run().success();
    assert!(installed(&project).contains("local-dep"));
    project.pyfaro(["remove", "local-dep"]).run().success();
    assert!(installed(&project).contains("local-dep"));
    project.pyfaro(["sync"]).run().success();
    assert!(!installed(&project).contains("local-dep"));
}

#[test]
fn remove_a_missing_dependency_forwards_uvs_failure() {
    e2e_guard!();
    let sandbox = Sandbox::new();
    let project = sandbox.project(Kind::Lib);
    let before = project.snapshot();
    project
        .pyfaro(["remove", "not-a-dependency"])
        .run()
        .code(2)
        .stdout_empty()
        .stderr_contains(
            "The dependency `not-a-dependency` could not be found in `project.dependencies`",
        )
        .stderr_contains("error: uv remove failed (");
    assert_eq!(project.snapshot(), before);
}

#[test]
fn remove_a_dependency_missing_from_the_dev_group_forwards_uvs_failure() {
    e2e_guard!();
    let sandbox = Sandbox::new();
    let project = sandbox.project(Kind::Lib);
    let before = project.snapshot();
    project
        .pyfaro(["remove", "--dev", "not-a-dependency"])
        .run()
        .code(2)
        .stderr_contains("The dependency `not-a-dependency` could not be found in `tool.uv.dev-dependencies` or `tool.uv.dependency-groups.dev`")
        .stderr_contains("error: uv remove failed (");
    assert_eq!(project.snapshot(), before);
}

#[test]
fn remove_with_project_and_from_a_nested_directory() {
    e2e_guard!();
    let sandbox = Sandbox::new();
    let project = project_with_local_dep(&sandbox);
    project.pyfaro(["add", "../dep"]).run().success();
    sandbox
        .pyfaro(["remove", "--project", "project", "--offline", "local-dep"])
        .run()
        .success();
    assert!(!project.read("pyproject.toml").contains("local-dep"));

    project.pyfaro(["add", "../dep"]).run().success();
    project
        .pyfaro(["remove", "--offline", "local-dep"])
        .cwd(project.path("tests"))
        .run()
        .success();
    assert!(!project.read("pyproject.toml").contains("local-dep"));
    assert!(!project.path("tests/uv.lock").exists());
}

#[test]
fn remove_requires_at_least_one_package() {
    e2e_guard!();
    let sandbox = Sandbox::new();
    let project = sandbox.project(Kind::Lib);
    project
        .pyfaro(["remove"])
        .run()
        .code(2)
        .stdout_empty()
        .stderr_contains("<PACKAGES>...");
}

#[test]
fn remove_outside_a_project_reports_where_it_looked() {
    e2e_guard!();
    let sandbox = Sandbox::new();
    sandbox
        .pyfaro(["remove", "anything"])
        .run()
        .code(1)
        .stderr_contains("error: no pyproject.toml found from ");
    assert!(snapshot(sandbox.path()).is_empty());
}

// ----------------------------------------------------------------- run

/// Python one-liner printing `sys.argv[1:]`.
const PRINT_ARGV: &str = "import sys; print(sys.argv[1:])";

#[test]
fn run_passes_arguments_through_verbatim() {
    e2e_guard!();
    let sandbox = Sandbox::new();
    let project = sandbox.synced_project(Kind::Lib);
    project
        .pyfaro(["run", "--offline", "--", "python", "-c", PRINT_ARGV])
        .args(["a b", "--flag", "-n", "--", "-x", "", "café"])
        .env("PYTHONIOENCODING", "utf-8")
        .run()
        .success()
        .stdout_text_eq("['a b', '--flag', '-n', '--', '-x', '', 'café']\n");
}

#[test]
fn run_does_not_interpret_hyphenated_arguments_as_pyfaro_options() {
    e2e_guard!();
    let sandbox = Sandbox::new();
    let project = sandbox.synced_project(Kind::Lib);
    project
        .pyfaro([
            "run",
            "--",
            "python",
            "-c",
            PRINT_ARGV,
            "--locked",
            "--project",
            "x",
            "--offline",
        ])
        .run()
        .success()
        .stdout_text_eq("['--locked', '--project', 'x', '--offline']\n");
}

#[test]
fn run_forwards_the_exit_code_of_the_program() {
    e2e_guard!();
    let sandbox = Sandbox::new();
    let project = sandbox.synced_project(Kind::Lib);
    project
        .pyfaro(["run", "--", "python", "-c", "import sys; sys.exit(7)"])
        .run()
        .code(7)
        .stdout_empty()
        .stderr_contains("error: uv run failed (");
}

#[test]
fn run_streams_stdout_and_stderr_of_the_program() {
    e2e_guard!();
    let sandbox = Sandbox::new();
    let project = sandbox.synced_project(Kind::Lib);
    project
        .pyfaro([
            "run",
            "--",
            "python",
            "-c",
            "import sys; print('to-out'); print('to-err', file=sys.stderr)",
        ])
        .run()
        .success()
        .stdout_text_eq("to-out\n")
        .stderr_contains("to-err");
}

#[test]
fn run_executes_a_py_script_relative_to_the_working_directory() {
    e2e_guard!();
    let sandbox = Sandbox::new();
    let project = sandbox.synced_project(Kind::Lib);
    project.write(
        "scripts/probe.py",
        "import os, sys\n\
         print(os.path.basename(os.getcwd()))\n\
         print('.venv' in sys.prefix)\n\
         print(sys.argv[1:])\n",
    );
    project
        .pyfaro(["run", "--", "probe.py", "--verbose", "x y"])
        .cwd(project.path("scripts"))
        .run()
        .success()
        .stdout_text_eq("scripts\nTrue\n['--verbose', 'x y']\n");
}

#[test]
fn run_py_script_imports_the_project_package() {
    e2e_guard!();
    let sandbox = Sandbox::new();
    let project = sandbox.synced_project(Kind::Lib);
    project.write(
        "check_import.py",
        "import acme_widget\nprint(acme_widget.__name__)\n",
    );
    project
        .pyfaro(["run", "--", "check_import.py"])
        .run()
        .success()
        .stdout_text_eq("acme_widget\n");
}

#[test]
fn run_py_script_runs_through_the_project_python_not_a_shebang() {
    e2e_guard!();
    let sandbox = Sandbox::new();
    let project = sandbox.synced_project(Kind::Lib);
    // A script without an executable bit still runs.
    project.write("plain.py", "print('ran')\n");
    project
        .pyfaro(["run", "--", "plain.py"])
        .run()
        .success()
        .stdout_text_eq("ran\n");
}

#[test]
fn run_locked_rejects_a_stale_lock_without_running_the_program() {
    e2e_guard!();
    let sandbox = Sandbox::new();
    let project = project_with_local_dep(&sandbox);
    project.pyfaro(["sync"]).run().success();
    make_lock_stale(&project);
    let before = project.snapshot();
    project
        .pyfaro(["run", "--locked", "--offline", "--", "python", "-c", "print('ran')"])
        .run()
        .code(2)
        .stdout_empty()
        .stderr_contains("The lockfile at `uv.lock` needs to be updated, but `--locked` was provided")
        .stderr_contains("error: uv run failed (");
    assert_eq!(project.snapshot(), before);
}

#[test]
fn run_locked_rejects_an_absent_lock() {
    e2e_guard!();
    let sandbox = Sandbox::new();
    let project = sandbox.project(Kind::Lib);
    let before = project.snapshot();
    project
        .pyfaro(["run", "--locked", "--", "python", "-c", "print('ran')"])
        .run()
        .code(2)
        .stdout_empty()
        .stderr_contains(
            "Unable to find lockfile at `uv.lock`, but `--locked` was provided",
        );
    assert_eq!(project.snapshot(), before);
}

#[test]
fn run_locked_accepts_a_fresh_lock() {
    e2e_guard!();
    let sandbox = Sandbox::new();
    let project = sandbox.synced_project(Kind::Lib);
    let before = project.snapshot();
    project
        .pyfaro([
            "run",
            "--locked",
            "--offline",
            "--",
            "python",
            "-c",
            "print('ran')",
        ])
        .run()
        .success()
        .stdout_text_eq("ran\n");
    assert_eq!(project.snapshot(), before);
}

#[test]
fn run_frozen_uses_the_existing_lock_without_updating_it() {
    e2e_guard!();
    let sandbox = Sandbox::new();
    let project = project_with_local_dep(&sandbox);
    project.pyfaro(["sync"]).run().success();
    make_lock_stale(&project);
    let before = project.snapshot();
    project
        .pyfaro(["run", "--frozen", "--", "python", "-c", "print('ran')"])
        .run()
        .success()
        .stdout_text_eq("ran\n");
    assert_eq!(project.snapshot(), before);
    assert!(!installed(&project).contains("local-dep"));
}

#[test]
fn run_frozen_requires_an_existing_lock() {
    e2e_guard!();
    let sandbox = Sandbox::new();
    let project = sandbox.project(Kind::Lib);
    project
        .pyfaro(["run", "--frozen", "--", "python", "-c", "print('ran')"])
        .run()
        .code(2)
        .stdout_empty()
        .stderr_contains(
            "Unable to find lockfile at `uv.lock`, but `--frozen` was provided",
        );
    assert!(!project.path("uv.lock").exists());
}

#[test]
fn run_locked_conflicts_with_frozen() {
    e2e_guard!();
    let sandbox = Sandbox::new();
    let project = sandbox.project(Kind::Lib);
    let before = project.snapshot();
    project
        .pyfaro(["run", "--locked", "--frozen", "--", "python", "-c", "pass"])
        .run()
        .code(2)
        .stdout_empty()
        .stderr_contains(
            "the argument '--locked' cannot be used with '--frozen'",
        );
    assert_eq!(project.snapshot(), before);
    assert!(!project.path(".venv").exists());
}

#[test]
fn run_syncs_the_environment_by_default() {
    e2e_guard!();
    let sandbox = Sandbox::new();
    let project = sandbox.project(Kind::Lib);
    project
        .pyfaro(["run", "--", "python", "-c", "print('ran')"])
        .run()
        .success()
        .stdout_text_eq("ran\n")
        .stderr_contains("Creating virtual environment");
    assert!(project.path(".venv").is_dir());
    assert!(project.path("uv.lock").is_file());
}

#[test]
fn run_from_a_nested_directory_uses_the_project_environment_and_keeps_the_cwd()
{
    e2e_guard!();
    let sandbox = Sandbox::new();
    let project = sandbox.synced_project(Kind::Lib);
    let nested = project.path("src/acme_widget");
    project
        .pyfaro([
            "run",
            "--",
            "python",
            "-c",
            "import os, sys; print(os.path.basename(os.getcwd())); print('.venv' in sys.prefix)",
        ])
        .cwd(&nested)
        .run()
        .success()
        .stdout_text_eq("acme_widget\nTrue\n");
    assert!(!nested.join(".venv").exists());
    assert!(!nested.join("uv.lock").exists());
}

#[test]
fn run_project_selects_the_exact_directory_and_keeps_the_cwd() {
    e2e_guard!();
    let sandbox = Sandbox::new();
    let project = sandbox.synced_project(Kind::Lib);
    let sandbox_name =
        sandbox.path().file_name().unwrap().to_string_lossy().into_owned();
    sandbox
        .pyfaro(["run", "--project", "project", "--", "python", "-c"])
        .arg("import os, sys; print(os.path.basename(os.getcwd())); print('.venv' in sys.prefix)")
        .env("PYTHONIOENCODING", "utf-8")
        .run()
        .success()
        .stdout_text_eq(&format!("{sandbox_name}\nTrue\n"));
    assert!(!sandbox.join(".venv").exists());
    assert!(project.path(".venv").is_dir());
}

#[test]
fn run_project_never_falls_back_to_a_parent_directory() {
    e2e_guard!();
    let sandbox = Sandbox::new();
    let project = sandbox.project(Kind::Lib);
    let before = snapshot(sandbox.path());
    sandbox
        .pyfaro([
            "run",
            "--project",
            "project/src",
            "--",
            "python",
            "-c",
            "pass",
        ])
        .run()
        .code(1)
        .stdout_empty()
        .stderr_contains("error: project directory has no pyproject.toml: ");
    assert_eq!(snapshot(sandbox.path()), before);
    assert!(!project.path(".venv").exists());
}

#[test]
fn run_outside_a_project_reports_where_it_looked() {
    e2e_guard!();
    let sandbox = Sandbox::new();
    sandbox
        .pyfaro(["run", "--", "python", "-c", "pass"])
        .run()
        .code(1)
        .stdout_empty()
        .stderr_contains("error: no pyproject.toml found from ")
        .stderr_contains("; use --project <PATH>");
    assert!(snapshot(sandbox.path()).is_empty());
}

#[test]
fn run_forwards_standard_input() {
    e2e_guard!();
    let sandbox = Sandbox::new();
    let project = sandbox.synced_project(Kind::Lib);
    project.write(
        "echo.py",
        "import sys\nsys.stdout.write(sys.stdin.read().upper())\n",
    );
    project
        .pyfaro(["run", "--", "echo.py"])
        .stdin("piped input\nsecond line\n")
        .run()
        .success()
        .stdout_text_eq("PIPED INPUT\nSECOND LINE\n");
}

#[test]
fn run_forwards_standard_input_to_a_program_on_the_path() {
    e2e_guard!();
    let sandbox = Sandbox::new();
    let project = sandbox.synced_project(Kind::Lib);
    project
        .pyfaro([
            "run",
            "--",
            "python",
            "-c",
            "import sys; print(len(sys.stdin.read()))",
        ])
        .stdin("12345")
        .run()
        .success()
        .stdout_text_eq("5\n");
}

#[test]
fn run_forwards_uvs_error_for_a_missing_program() {
    e2e_guard!();
    let sandbox = Sandbox::new();
    let project = sandbox.synced_project(Kind::Lib);
    let before = project.snapshot();
    project
        .pyfaro(["run", "--", "pyfaro-e2e-no-such-program"])
        .run()
        .code(2)
        .stdout_empty()
        .stderr_contains("Failed to spawn: `pyfaro-e2e-no-such-program`")
        .stderr_contains("error: uv run failed (");
    assert_eq!(project.snapshot(), before);
}

#[test]
fn run_requires_a_command() {
    e2e_guard!();
    let sandbox = Sandbox::new();
    let project = sandbox.project(Kind::Lib);
    let before = project.snapshot();
    project
        .pyfaro(["run"])
        .run()
        .code(2)
        .stdout_empty()
        .stderr_contains("required arguments were not provided");
    assert_eq!(project.snapshot(), before);
    assert!(!project.path(".venv").exists());
}

#[test]
fn run_offline_works_for_a_synced_project() {
    e2e_guard!();
    let sandbox = Sandbox::new();
    let project = sandbox.synced_project(Kind::Lib);
    project
        .pyfaro(["run", "--offline", "--", "python", "-c", "print('ran')"])
        .run()
        .success()
        .stdout_text_eq("ran\n");
}

#[test]
fn run_reports_a_missing_uv() {
    e2e_guard!();
    let sandbox = Sandbox::new();
    let project = sandbox.project(Kind::Lib);
    let before = project.snapshot();
    project
        .pyfaro([
            "run",
            "--uv",
            "pyfaro-e2e-no-such-uv",
            "--",
            "python",
            "-c",
            "pass",
        ])
        .run()
        .code(1)
        .stdout_empty()
        .stderr_contains("error: could not find uv at pyfaro-e2e-no-such-uv")
        .stderr_contains("install uv or use --uv <path>");
    assert_eq!(project.snapshot(), before);
}

#[test]
fn run_reports_an_invalid_tool_pyfaro_config_with_the_file_path() {
    e2e_guard!();
    let sandbox = Sandbox::new();
    let project = sandbox.project(Kind::Lib);
    let metadata = project
        .read("pyproject.toml")
        .replace("schema-version = 1\n", "schema-version = 2\n");
    project.write("pyproject.toml", &metadata);
    let before = project.snapshot();
    project
        .pyfaro(["run", "--", "python", "-c", "pass"])
        .run()
        .code(1)
        .stdout_empty()
        .stderr_contains("error: invalid ")
        .stderr_contains(
            "pyproject.toml: tool.pyfaro.schema-version must be the integer 1",
        );
    assert_eq!(project.snapshot(), before);
}
