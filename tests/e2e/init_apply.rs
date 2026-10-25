//! `pyfaro init --apply` on every platform: CRLF and read-only metadata,
//! unusual paths, rollback after a uv failure and cancellation during the uv
//! step.
//!
//! Failures and cancellation need a uv that misbehaves on demand. The
//! portable fixture of `tests/support` does, and installs as `uv` or
//! `uv.exe` like the real thing.

use std::fs;
use std::path::{Path, PathBuf};
use std::thread;
use std::time::Duration;

use crate::harness::{
    Live, SHORT, STARTUP, Sandbox, snapshot, wait_until_process_is_gone,
};
use crate::support;

const PLAIN: &str = "[project]\nname = \"example\"\nversion = \"1.0.0\"\nrequires-python = \">=3.10\"\ndependencies = []\n";

/// The misbehaving uv, in a directory name with a space and a non-ASCII
/// character.
fn fake_uv(sandbox: &Sandbox) -> PathBuf {
    support::install_fixture(&sandbox.join("fake uv ñ"), "uv")
}

fn crlf(text: &str) -> String {
    text.replace('\n', "\r\n")
}

/// Mark `path` read-only or writable again, which is how a sandbox holding a
/// read-only file is cleaned up on Windows.
#[allow(clippy::permissions_set_readonly_false)]
fn set_readonly(path: &Path, readonly: bool) {
    let mut permissions = fs::metadata(path).unwrap().permissions();
    permissions.set_readonly(readonly);
    fs::set_permissions(path, permissions).unwrap();
}

#[test]
fn init_apply_keeps_crlf_line_endings_and_reapplies_byte_for_byte() {
    e2e_guard!();
    let sandbox = Sandbox::new();
    let original = crlf(PLAIN);
    fs::write(sandbox.join("pyproject.toml"), &original).unwrap();
    sandbox.pyfaro(["init", "--apply"]).run().success();
    let applied = fs::read(sandbox.join("pyproject.toml")).unwrap();
    assert_eq!(
        applied,
        format!("{original}\r\n[tool.pyfaro]\r\nschema-version = 1\r\n")
            .into_bytes()
    );
    let first = snapshot(sandbox.path());
    sandbox.pyfaro(["init", "--apply"]).run().success();
    assert_eq!(snapshot(sandbox.path()), first);
    assert_eq!(fs::read(sandbox.join("pyproject.toml")).unwrap(), applied);
}

#[test]
fn init_apply_leaves_a_configured_crlf_project_untouched() {
    e2e_guard!();
    let sandbox = Sandbox::new();
    let original =
        crlf(&format!("{PLAIN}\n[tool.pyfaro]\nschema-version = 1\n"));
    fs::write(sandbox.join("pyproject.toml"), &original).unwrap();
    sandbox.pyfaro(["init", "--apply"]).run().success();
    assert_eq!(
        fs::read(sandbox.join("pyproject.toml")).unwrap(),
        original.as_bytes()
    );
    assert!(sandbox.join("uv.lock").is_file());
}

#[test]
fn init_apply_works_in_paths_with_spaces_and_non_ascii_characters() {
    e2e_guard!();
    let sandbox = Sandbox::new();
    sandbox.write("dossier é/projét 日本/pyproject.toml", PLAIN);
    sandbox
        .pyfaro(["init", "dossier é/projét 日本", "--apply"])
        .run()
        .success()
        .stdout_contains(
            "Initialized dossier é/projét 日本 with Pyfaro and uv.\n",
        );
    let existing = sandbox.join("dossier é/projét 日本");
    assert!(
        fs::read_to_string(existing.join("pyproject.toml"))
            .unwrap()
            .ends_with("\n[tool.pyfaro]\nschema-version = 1\n")
    );
    assert!(existing.join("uv.lock").is_file());
    let before = snapshot(&existing);
    sandbox
        .pyfaro(["init", "dossier é/projét 日本", "--apply"])
        .run()
        .success();
    assert_eq!(snapshot(&existing), before);

    sandbox
        .pyfaro(["init", "dossier é/nouveau projet-é/app", "--apply"])
        .run()
        .success();
    let fresh = sandbox.join("dossier é/nouveau projet-é/app");
    assert!(fresh.join("pyproject.toml").is_file());
    assert!(fresh.join("uv.lock").is_file());
}

#[test]
fn init_apply_handles_a_read_only_pyproject() {
    e2e_guard!();
    let sandbox = Sandbox::new();
    let metadata = sandbox.write("pyproject.toml", PLAIN);
    set_readonly(&metadata, true);
    let before = snapshot(sandbox.path());
    let out = sandbox.pyfaro(["init", "--apply"]).run();
    if cfg!(windows) {
        // Windows will not rename over a read-only file; nothing changes.
        out.code(1)
            .stdout_contains("Initialization plan for .:\n")
            .stderr_contains("pyproject.toml is read-only; clear its read-only attribute and rerun init");
        assert_eq!(snapshot(sandbox.path()), before);
    } else {
        // Unix replaces the file and keeps its mode.
        out.success();
        assert!(fs::metadata(&metadata).unwrap().permissions().readonly());
        assert!(
            fs::read_to_string(&metadata)
                .unwrap()
                .ends_with("\n[tool.pyfaro]\nschema-version = 1\n")
        );
    }
    set_readonly(&metadata, false);
}

#[cfg(windows)]
#[test]
fn init_apply_refuses_a_read_only_lockfile_before_running_uv() {
    e2e_guard!();
    let sandbox = Sandbox::new();
    sandbox.write("pyproject.toml", PLAIN);
    let lockfile = sandbox.write("uv.lock", "original lock\n");
    set_readonly(&lockfile, true);
    let before = snapshot(sandbox.path());
    sandbox.pyfaro(["init", "--apply"]).run().code(1).stderr_contains(
        "uv.lock is read-only; clear its read-only attribute and rerun init",
    );
    assert_eq!(snapshot(sandbox.path()), before);
    set_readonly(&lockfile, false);
}

#[cfg(windows)]
#[test]
fn init_apply_names_a_file_another_program_holds_open() {
    e2e_guard!();
    use std::fs::OpenOptions;
    use std::os::windows::fs::OpenOptionsExt;

    const FILE_SHARE_READ: u32 = 1;
    let sandbox = Sandbox::new();
    let metadata = sandbox.write("pyproject.toml", PLAIN);
    let before = snapshot(sandbox.path());
    let held = OpenOptions::new()
        .read(true)
        .write(true)
        .share_mode(FILE_SHARE_READ)
        .open(&metadata)
        .unwrap();
    sandbox
        .pyfaro(["init", "--apply"])
        .run()
        .code(1)
        .stderr_contains("cannot replace ")
        .stderr_contains(
            "pyproject.toml: it is read-only or another program has it open",
        )
        .stderr_contains("close that program and rerun init");
    drop(held);
    assert_eq!(snapshot(sandbox.path()), before);
    // Once the other program lets go, the same command succeeds.
    sandbox.pyfaro(["init", "--apply"]).run().success();
    assert!(sandbox.join("uv.lock").is_file());
}

#[test]
fn a_uv_failure_restores_crlf_metadata_and_lockfile_byte_for_byte() {
    e2e_guard!();
    let sandbox = Sandbox::new();
    let uv = fake_uv(&sandbox);
    sandbox.write("project/pyproject.toml", &crlf(PLAIN));
    fs::write(sandbox.join("project/uv.lock"), b"original lock\r\n").unwrap();
    let before = snapshot(sandbox.path());
    sandbox
        .pyfaro(["init", "project", "--apply", "--uv"])
        .arg(&uv)
        .env("PYFARO_FIXTURE_MODE", "fail-lock")
        .run()
        .code(42)
        .stderr_contains("fixture: cannot resolve dependencies")
        .stderr_contains("error: uv lock failed (")
        .stderr_contains("see uv's output above");
    // Equal digests: both files are as they were and no temporary file or
    // incomplete lockfile is left behind.
    assert_eq!(snapshot(sandbox.path()), before);
}

#[test]
fn a_failed_new_initialization_leaves_no_project_files() {
    e2e_guard!();
    let sandbox = Sandbox::new();
    let uv = fake_uv(&sandbox);
    sandbox
        .pyfaro(["init", "fresh", "--apply", "--uv"])
        .arg(&uv)
        .env("PYFARO_FIXTURE_MODE", "fail-init")
        .run()
        .code(23)
        .stderr_contains("error: uv init failed (");
    assert!(!sandbox.join("fresh/pyproject.toml").exists());
    assert!(!sandbox.join("fresh/uv.lock").exists());
}

#[test]
fn ctrl_c_during_the_uv_step_stops_the_process_tree_and_restores_the_project() {
    e2e_guard!();
    let sandbox = Sandbox::new();
    let uv = fake_uv(&sandbox);
    let original = crlf(PLAIN);
    let root = sandbox.join("project");
    fs::create_dir(&root).unwrap();
    fs::write(root.join("pyproject.toml"), &original).unwrap();
    fs::write(root.join("uv.lock"), b"original lock\r\n").unwrap();
    let mut live = sandbox
        .pyfaro(["init", "project", "--apply", "--uv"])
        .arg(&uv)
        .env("PYFARO_FIXTURE_MODE", "wait")
        .spawn();
    // The fixture's descendant is running, so uv, the leader, is too.
    let heartbeat = root.join("heartbeat");
    live.wait_until(STARTUP, "the uv fixture to start", |_| {
        heartbeat.exists() && root.join("uv-ready").exists()
    });
    // Pyfaro has already rewritten the metadata, so restoring it is real work.
    assert!(
        fs::read_to_string(root.join("pyproject.toml"))
            .unwrap()
            .contains("[tool.pyfaro]"),
        "the configuration was not written before uv ran"
    );
    let descendant = descendant_pid(&mut live, &root);
    live.interrupt();
    let out = live.finish(SHORT);
    out.code(130);
    assert!(!out.stderr.contains("uv lock failed"), "{}", out.stderr);
    wait_until_process_is_gone(descendant, SHORT);
    let stopped = fs::read(&heartbeat).unwrap();
    thread::sleep(Duration::from_millis(300));
    assert_eq!(fs::read(&heartbeat).unwrap(), stopped, "uv kept running");
    assert_eq!(
        fs::read(root.join("pyproject.toml")).unwrap(),
        original.as_bytes()
    );
    assert_eq!(fs::read(root.join("uv.lock")).unwrap(), b"original lock\r\n");
    let mut names: Vec<_> = fs::read_dir(&root)
        .unwrap()
        .map(|entry| entry.unwrap().file_name().to_string_lossy().into_owned())
        .collect();
    names.sort();
    assert_eq!(
        names,
        [
            "heartbeat",
            "pyproject.toml",
            "uv-descendant.pid",
            "uv-ready",
            "uv.lock"
        ],
        "a temporary file was left behind"
    );
}

/// The pid the uv fixture's descendant recorded, once the file is complete.
fn descendant_pid(live: &mut Live, root: &Path) -> u32 {
    let mut pid = None;
    live.wait_until(SHORT, "the descendant's pid", |_| {
        pid = fs::read_to_string(root.join("uv-descendant.pid"))
            .ok()
            .and_then(|text| text.trim().parse().ok());
        pid.is_some()
    });
    pid.unwrap()
}

#[test]
fn ctrl_c_during_a_new_initialization_removes_the_partial_project() {
    e2e_guard!();
    let sandbox = Sandbox::new();
    let uv = fake_uv(&sandbox);
    let root = sandbox.join("fresh");
    let mut live = sandbox
        .pyfaro(["init", "fresh", "--apply", "--uv"])
        .arg(&uv)
        .env("PYFARO_FIXTURE_MODE", "wait-init")
        .spawn();
    live.wait_until(STARTUP, "the uv fixture to start", |_| {
        root.join("uv-ready").exists()
    });
    let descendant = descendant_pid(&mut live, &root);
    live.interrupt();
    live.finish(SHORT).code(130);
    wait_until_process_is_gone(descendant, SHORT);
    assert!(!root.join("pyproject.toml").exists());
    assert!(!root.join("uv.lock").exists());
}
