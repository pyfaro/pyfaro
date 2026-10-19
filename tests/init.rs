use std::fs;
use std::process::Command;

use tempfile::tempdir;

#[cfg(target_os = "linux")]
mod support;

#[test]
fn preview_preserves_the_project_and_does_not_require_uv() {
    let project = tempdir().expect("temporary project");
    let original = "# Keep this comment\n[project]\nname = 'example'\nversion = '1.0.0'\n\n[tool.ruff]\nline-length = 100\n";
    fs::write(project.path().join("pyproject.toml"), original).unwrap();

    let output = Command::new(env!("CARGO_BIN_EXE_pyfaro"))
        .args(["init", "--uv"])
        .arg(project.path().join("missing-uv"))
        .arg(project.path())
        .output()
        .expect("CLI should start");

    assert!(output.status.success(), "{output:?}");
    let stdout = String::from_utf8(output.stdout).unwrap();
    assert!(stdout.contains("[tool.pyfaro]"));
    assert!(stdout.contains("schema-version = 1"));
    assert!(stdout.contains("uv lock"));
    assert!(stdout.contains("--apply"));
    assert_eq!(
        fs::read_to_string(project.path().join("pyproject.toml")).unwrap(),
        original
    );
    assert!(!project.path().join("uv.lock").exists());
}

#[test]
fn inline_tool_preview_shows_the_actual_proposed_metadata() {
    let project = tempdir().unwrap();
    let metadata = project.path().join("pyproject.toml");
    let original = "tool = { ruff = { line-length = 100 } }\n[project]\nname = 'example'\nversion = '1.0.0'\n";
    fs::write(&metadata, original).unwrap();
    let output = Command::new(env!("CARGO_BIN_EXE_pyfaro"))
        .args(["init", "--uv", "missing-uv"])
        .arg(project.path())
        .output()
        .unwrap();
    assert!(output.status.success(), "{output:?}");
    let preview = String::from_utf8(output.stdout).unwrap();
    assert!(preview.contains("ruff = { line-length = 100 }"));
    assert!(preview.contains("pyfaro = { schema-version = 1 }"));
    assert!(!preview.contains("\n[tool.pyfaro]\n"));
    assert_eq!(fs::read_to_string(metadata).unwrap(), original);
}

#[test]
fn inline_pyfaro_preview_shows_the_actual_proposed_metadata() {
    let project = tempdir().unwrap();
    let metadata = project.path().join("pyproject.toml");
    let original = "[project]\nname = 'example'\nversion = '1.0.0'\n\n[tool]\npyfaro = {}\n\n[tool.ruff]\nline-length = 100\n";
    fs::write(&metadata, original).unwrap();
    let output = Command::new(env!("CARGO_BIN_EXE_pyfaro"))
        .args(["init", "--uv", "missing-uv"])
        .arg(project.path())
        .output()
        .unwrap();
    assert!(output.status.success(), "{output:?}");
    let preview = String::from_utf8(output.stdout).unwrap();
    assert!(preview.contains("pyfaro = { schema-version = 1 }"));
    assert!(preview.contains("line-length = 100"));
    assert!(!preview.contains("\n[tool.pyfaro]\n"));
    assert_eq!(fs::read_to_string(metadata).unwrap(), original);
}

#[test]
fn invalid_init_configuration_names_the_project_metadata() {
    let project = tempdir().unwrap();
    let metadata = project.path().join("pyproject.toml");
    fs::write(
        &metadata,
        "[project]\nname='example'\n[tool.pyfaro]\nschema-version=2\n",
    )
    .unwrap();
    let output = Command::new(env!("CARGO_BIN_EXE_pyfaro"))
        .args(["init", "--uv", "missing-uv"])
        .arg(project.path())
        .output()
        .unwrap();
    assert_eq!(output.status.code(), Some(1), "{output:?}");
    let diagnostic = String::from_utf8_lossy(&output.stderr);
    let resolved = fs::canonicalize(&metadata).unwrap();
    assert!(
        diagnostic.contains(&resolved.display().to_string()),
        "{diagnostic}"
    );
    assert!(diagnostic.contains("schema-version"), "{diagnostic}");
}

#[test]
#[cfg(target_os = "linux")]
fn applying_an_existing_project_with_real_uv_is_preserving_and_idempotent() {
    let project = tempdir().unwrap();
    let cache = tempdir().unwrap();
    let original = "# User metadata\n[project]\nname = 'existing-example'\nversion = '1.2.3'\nrequires-python = '>=3.10'\ndependencies = []\n\n[tool.ruff]\nline-length = 100 # Keep this\n\n[tool.ty.rules]\nall = 'warn'\n\n[tool.pytest.ini_options]\naddopts = '-q'\n";
    fs::write(project.path().join("pyproject.toml"), original).unwrap();
    fs::write(project.path().join("main.py"), "print('unchanged')\n").unwrap();

    let apply = || {
        Command::new(env!("CARGO_BIN_EXE_pyfaro"))
            .args(["init", "--apply", "--offline"])
            .arg(project.path())
            .env("UV_CACHE_DIR", cache.path())
            .output()
            .unwrap()
    };
    let first = apply();
    assert!(first.status.success(), "{first:?}");
    let config =
        fs::read_to_string(project.path().join("pyproject.toml")).unwrap();
    assert!(config.starts_with(original));
    assert!(config.contains("[tool.pyfaro]\nschema-version = 1"));
    let lock = fs::read(project.path().join("uv.lock")).unwrap();
    assert!(String::from_utf8_lossy(&lock).contains("existing-example"));
    assert_eq!(
        fs::read_to_string(project.path().join("main.py")).unwrap(),
        "print('unchanged')\n"
    );
    assert!(!project.path().join(".venv").exists());

    let second = apply();
    assert!(second.status.success(), "{second:?}");
    assert_eq!(
        fs::read_to_string(project.path().join("pyproject.toml")).unwrap(),
        config
    );
    assert_eq!(fs::read(project.path().join("uv.lock")).unwrap(), lock);
}

#[test]
#[cfg(target_os = "linux")]
fn a_new_project_is_previewed_without_writes_then_initialized_with_real_uv() {
    let sandbox = tempdir().unwrap();
    let cache = tempdir().unwrap();
    let parent = sandbox.path().join("spaces é & punctuation");
    fs::create_dir(&parent).unwrap();
    let project = parent.join("alpha-example");
    let preview = Command::new(env!("CARGO_BIN_EXE_pyfaro"))
        .arg("init")
        .arg(&project)
        .output()
        .unwrap();
    assert!(preview.status.success(), "{preview:?}");
    assert!(String::from_utf8_lossy(&preview.stdout).contains("uv init"));
    assert!(!project.exists());

    let apply = Command::new(env!("CARGO_BIN_EXE_pyfaro"))
        .args(["init", "--apply", "--offline"])
        .arg(&project)
        .env("UV_CACHE_DIR", cache.path())
        .env("UV_PROJECT", sandbox.path().join("wrong-project"))
        .env("UV_WORKING_DIR", sandbox.path().join("wrong-directory"))
        .env("UV_INIT_BUILD_BACKEND", "maturin")
        .env("UV_FROZEN", "1")
        .output()
        .unwrap();
    assert!(apply.status.success(), "{apply:?}");
    let config = fs::read_to_string(project.join("pyproject.toml")).unwrap();
    assert!(config.contains("name = \"alpha-example\""));
    assert!(config.contains("schema-version = 1"));
    assert!(project.join("uv.lock").is_file());
    for unwanted in [
        ".venv",
        ".git",
        ".python-version",
        "README.md",
        "main.py",
        "Cargo.toml",
        "CMakeLists.txt",
    ] {
        assert!(!project.join(unwanted).exists(), "unexpected {unwanted}");
    }
}

#[test]
fn workspace_members_are_rejected_before_any_file_or_uv_changes() {
    let workspace = tempdir().unwrap();
    let workspace_config = "[tool.uv.workspace]\nmembers = ['member']\n";
    fs::write(workspace.path().join("pyproject.toml"), workspace_config)
        .unwrap();
    let member = workspace.path().join("member");
    fs::create_dir(&member).unwrap();
    let original = "[project]\nname = 'member'\nversion = '0.1.0'\n";
    fs::write(member.join("pyproject.toml"), original).unwrap();
    for apply in [false, true] {
        let mut command = Command::new(env!("CARGO_BIN_EXE_pyfaro"));
        command.arg("init").arg(&member).arg("--uv").arg("missing-uv");
        if apply {
            command.arg("--apply");
        }
        let output = command.output().unwrap();
        assert_eq!(output.status.code(), Some(1), "{output:?}");
        assert!(
            String::from_utf8_lossy(&output.stderr).contains("workspaces"),
            "{output:?}"
        );
        assert_eq!(
            fs::read_to_string(member.join("pyproject.toml")).unwrap(),
            original
        );
        assert_eq!(
            fs::read_to_string(workspace.path().join("pyproject.toml"))
                .unwrap(),
            workspace_config
        );
        assert!(!workspace.path().join("uv.lock").exists());
    }
}

#[cfg(unix)]
#[test]
fn a_symlinked_parent_cannot_hide_a_workspace_for_a_new_project() {
    let sandbox = tempdir().unwrap();
    let workspace = sandbox.path().join("workspace");
    fs::create_dir(&workspace).unwrap();
    fs::write(
        workspace.join("pyproject.toml"),
        "[tool.uv.workspace]\nmembers = ['members/*']\n",
    )
    .unwrap();
    let members = workspace.join("members");
    fs::create_dir(&members).unwrap();
    let alias = sandbox.path().join("alias");
    std::os::unix::fs::symlink(&members, &alias).unwrap();
    let output = Command::new(env!("CARGO_BIN_EXE_pyfaro"))
        .arg("init")
        .arg(alias.join("new-project"))
        .output()
        .unwrap();
    assert_eq!(output.status.code(), Some(1), "{output:?}");
    assert!(String::from_utf8_lossy(&output.stderr).contains("workspaces"));
    assert!(!members.join("new-project").exists());
}

#[test]
#[cfg(target_os = "linux")]
fn uv_failure_restores_metadata_and_lockfile_and_preserves_its_exit_code() {
    let project = tempdir().unwrap();
    let original =
        "# precious metadata\n[project]\nname = 'example'\nversion = '1.0.0'\n";
    fs::write(project.path().join("pyproject.toml"), original).unwrap();
    fs::write(project.path().join("uv.lock"), "original lock\n").unwrap();
    let output = Command::new(env!("CARGO_BIN_EXE_pyfaro"))
        .args(["init", "--apply", "--uv"])
        .arg(support::fixture_uv())
        .arg(project.path())
        .env("PYFARO_FIXTURE_MODE", "fail-lock")
        .output()
        .unwrap();
    assert_eq!(output.status.code(), Some(42), "{output:?}");
    assert!(
        String::from_utf8_lossy(&output.stderr)
            .contains("fixture: cannot resolve dependencies")
    );
    assert_eq!(
        fs::read_to_string(project.path().join("pyproject.toml")).unwrap(),
        original
    );
    assert_eq!(
        fs::read_to_string(project.path().join("uv.lock")).unwrap(),
        "original lock\n"
    );
}

#[test]
#[cfg(target_os = "linux")]
fn failed_new_initialization_removes_partial_metadata() {
    let sandbox = tempdir().unwrap();
    let project = sandbox.path().join("new-project");
    let output = Command::new(env!("CARGO_BIN_EXE_pyfaro"))
        .args(["init", "--apply", "--uv"])
        .arg(support::fixture_uv())
        .arg(&project)
        .env("PYFARO_FIXTURE_MODE", "fail-init")
        .output()
        .unwrap();
    assert_eq!(output.status.code(), Some(23), "{output:?}");
    assert!(!project.join("pyproject.toml").exists());
    assert!(!project.join("uv.lock").exists());
}

#[test]
#[cfg(target_os = "linux")]
fn absent_or_incompatible_uv_fails_without_changing_the_project() {
    let project = tempdir().unwrap();
    let original = "[project]\nname = 'example'\nversion = '1.0.0'\n";
    fs::write(project.path().join("pyproject.toml"), original).unwrap();
    let missing = Command::new(env!("CARGO_BIN_EXE_pyfaro"))
        .args(["init", "--apply", "--uv"])
        .arg(project.path().join("missing-uv"))
        .arg(project.path())
        .output()
        .unwrap();
    assert_eq!(missing.status.code(), Some(1));
    assert!(String::from_utf8_lossy(&missing.stderr).contains("install uv"));
    for version in ["uv 0.9.25", "uv 0.9.26-beta.1", "not uv"] {
        let output = Command::new(env!("CARGO_BIN_EXE_pyfaro"))
            .args(["init", "--apply", "--uv"])
            .arg(support::fixture_uv())
            .arg(project.path())
            .env("PYFARO_FIXTURE_VERSION", version)
            .output()
            .unwrap();
        assert_eq!(output.status.code(), Some(1), "{output:?}");
        assert!(
            String::from_utf8_lossy(&output.stderr)
                .contains("unsupported uv version")
        );
    }
    assert_eq!(
        fs::read_to_string(project.path().join("pyproject.toml")).unwrap(),
        original
    );
    assert!(!project.path().join("uv.lock").exists());
}

#[test]
#[cfg(target_os = "linux")]
fn an_exited_uv_leader_does_not_leave_a_running_descendant() {
    use std::thread;
    use std::time::Duration;
    let project = tempdir().unwrap();
    let original = "[project]\nname = 'example'\nversion = '1.0.0'\n";
    fs::write(project.path().join("pyproject.toml"), original).unwrap();
    let output = Command::new(env!("CARGO_BIN_EXE_pyfaro"))
        .args(["init", "--apply", "--uv"])
        .arg(support::fixture_uv())
        .arg(project.path())
        .env("PYFARO_FIXTURE_MODE", "exit-with-child")
        .stdout(std::process::Stdio::null())
        .stderr(std::process::Stdio::null())
        .status()
        .unwrap();
    assert_eq!(output.code(), Some(42));
    let descendant: i32 =
        fs::read_to_string(project.path().join("uv-descendant.pid"))
            .unwrap()
            .parse()
            .unwrap();
    assert_eq!(
        nix::sys::signal::kill(nix::unistd::Pid::from_raw(descendant), None),
        Err(nix::errno::Errno::ESRCH)
    );
    let heartbeat = project.path().join("heartbeat");
    let stopped = fs::read(&heartbeat).unwrap();
    thread::sleep(Duration::from_millis(150));
    assert_eq!(fs::read(&heartbeat).unwrap(), stopped);
    assert_eq!(
        fs::read_to_string(project.path().join("pyproject.toml")).unwrap(),
        original
    );
}

#[test]
#[cfg(target_os = "linux")]
fn real_uv_locks_existing_local_dependencies_without_rewriting_them() {
    let sandbox = tempdir().unwrap();
    let cache = tempdir().unwrap();
    let project = sandbox.path().join("app");
    let dependency = sandbox.path().join("helper");
    fs::create_dir(&project).unwrap();
    fs::create_dir(&dependency).unwrap();
    fs::write(dependency.join("pyproject.toml"), "[project]\nname = 'local-helper'\nversion = '0.2.0'\nrequires-python = '>=3.10'\ndependencies = []\n").unwrap();
    fs::write(project.join("pyproject.toml"), "[project]\nname = 'example'\nversion = '1.0.0'\nrequires-python = '>=3.10'\ndependencies = []\n").unwrap();
    let initial_lock = Command::new("uv")
        .args(["--offline", "--no-python-downloads", "lock", "--no-build"])
        .current_dir(&project)
        .env("UV_CACHE_DIR", cache.path())
        .env_remove("UV_FROZEN")
        .env_remove("UV_PROJECT")
        .env_remove("UV_WORKING_DIR")
        .output()
        .unwrap();
    assert!(initial_lock.status.success(), "{initial_lock:?}");
    assert!(
        !fs::read_to_string(project.join("uv.lock"))
            .unwrap()
            .contains("local-helper")
    );
    let original = "[project]\nname = 'example'\nversion = '1.0.0'\nrequires-python = '>=3.10'\ndependencies = ['local-helper==0.2.0']\n\n[tool.uv.sources]\nlocal-helper = { path = '../helper' }\n";
    fs::write(project.join("pyproject.toml"), original).unwrap();
    let output = Command::new(env!("CARGO_BIN_EXE_pyfaro"))
        .args(["init", "--apply", "--offline"])
        .arg(&project)
        .env("UV_FROZEN", "1")
        .env("UV_CACHE_DIR", cache.path())
        .output()
        .unwrap();
    assert!(output.status.success(), "{output:?}");
    assert!(
        fs::read_to_string(project.join("pyproject.toml"))
            .unwrap()
            .starts_with(original)
    );
    let lock = fs::read_to_string(project.join("uv.lock")).unwrap();
    assert!(lock.contains("local-helper"));
    assert!(lock.contains("0.2.0"));
    assert!(!project.join(".venv").exists());
    assert!(!dependency.join(".venv").exists());
}

#[test]
#[cfg(target_os = "linux")]
fn a_relative_path_entry_is_resolved_once_before_changing_directories() {
    let sandbox = tempdir().unwrap();
    let tools = sandbox.path().join("relative-tools");
    let project = sandbox.path().join("project");
    fs::create_dir(&tools).unwrap();
    fs::create_dir(&project).unwrap();
    fs::copy(
        support::fixture_uv(),
        tools.join(format!("uv{}", std::env::consts::EXE_SUFFIX)),
    )
    .unwrap();
    let original = "[project]\nname = 'example'\nversion = '1.0.0'\n";
    fs::write(project.join("pyproject.toml"), original).unwrap();
    let output = Command::new(env!("CARGO_BIN_EXE_pyfaro"))
        .args(["init", "--apply", "project"])
        .current_dir(sandbox.path())
        .env("PATH", "relative-tools")
        .env("PYFARO_FIXTURE_MODE", "fail-lock")
        .output()
        .unwrap();
    assert_eq!(output.status.code(), Some(42), "{output:?}");
    assert_eq!(
        fs::read_to_string(project.join("pyproject.toml")).unwrap(),
        original
    );
}

#[test]
#[cfg(target_os = "linux")]
fn both_failed_file_restorations_are_reported() {
    let project = tempdir().unwrap();
    fs::write(
        project.path().join("pyproject.toml"),
        "[project]\nname = 'example'\nversion = '1.0.0'\n",
    )
    .unwrap();
    fs::write(project.path().join("uv.lock"), "original\n").unwrap();
    let output = Command::new(env!("CARGO_BIN_EXE_pyfaro"))
        .args(["init", "--apply", "--uv"])
        .arg(support::fixture_uv())
        .arg(project.path())
        .env("PYFARO_FIXTURE_MODE", "fail-restore")
        .output()
        .unwrap();
    assert_eq!(output.status.code(), Some(1), "{output:?}");
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(stderr.contains("could not restore project files"));
    assert!(stderr.contains("pyproject.toml"));
    assert!(stderr.contains("uv.lock"));
    assert!(stderr.contains("42"));
}

#[cfg(target_os = "linux")]
#[test]
fn metadata_symlinks_are_rejected_and_regular_file_permissions_are_preserved() {
    use std::os::unix::fs::{PermissionsExt, symlink};
    let project = tempdir().unwrap();
    let cache = tempdir().unwrap();
    let source = project.path().join("source.toml");
    let path = project.path().join("pyproject.toml");
    let original = "[project]\nname = 'example'\nversion = '1.0.0'\nrequires-python = '>=3.10'\n";
    fs::write(&source, original).unwrap();
    symlink(&source, &path).unwrap();
    let preview = Command::new(env!("CARGO_BIN_EXE_pyfaro"))
        .arg("init")
        .arg(project.path())
        .output()
        .unwrap();
    assert_eq!(preview.status.code(), Some(1), "{preview:?}");
    assert_eq!(fs::read_to_string(&source).unwrap(), original);
    fs::remove_file(&path).unwrap();
    fs::rename(&source, &path).unwrap();
    fs::set_permissions(&path, fs::Permissions::from_mode(0o640)).unwrap();
    let output = Command::new(env!("CARGO_BIN_EXE_pyfaro"))
        .args(["init", "--apply", "--offline"])
        .arg(project.path())
        .env("UV_CACHE_DIR", cache.path())
        .output()
        .unwrap();
    assert!(output.status.success(), "{output:?}");
    assert_eq!(
        fs::metadata(&path).unwrap().permissions().mode() & 0o777,
        0o640
    );
}

#[cfg(target_os = "linux")]
#[test]
fn ctrl_c_stops_uv_and_its_descendants_and_restores_the_project() {
    use nix::sys::signal::{Signal, killpg};
    use nix::unistd::Pid;
    use std::os::unix::process::CommandExt;
    use std::process::Stdio;
    use std::thread;
    use std::time::{Duration, Instant};

    let project = tempdir().unwrap();
    let original = "[project]\nname = 'example'\nversion = '1.0.0'\n";
    fs::write(project.path().join("pyproject.toml"), original).unwrap();
    let mut child = Command::new(env!("CARGO_BIN_EXE_pyfaro"))
        .args(["init", "--apply", "--uv"])
        .arg(support::fixture_uv())
        .arg(project.path())
        .env("PYFARO_FIXTURE_MODE", "wait")
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .process_group(0)
        .spawn()
        .unwrap();
    let deadline = Instant::now() + Duration::from_secs(5);
    while !project.path().join("heartbeat").exists()
        && Instant::now() < deadline
    {
        thread::sleep(Duration::from_millis(20));
    }
    if !project.path().join("heartbeat").exists() {
        let _ = child.kill();
        let _ = child.wait();
        panic!("uv fixture did not start in time");
    }
    killpg(Pid::from_raw(child.id() as i32), Signal::SIGINT).unwrap();
    let status = loop {
        if let Some(status) = child.try_wait().unwrap() {
            break status;
        }
        if Instant::now() > deadline {
            let _ = child.kill();
            let _ = child.wait();
            panic!("Pyfaro did not finish cancellation in time");
        }
        thread::sleep(Duration::from_millis(20));
    };
    assert_eq!(status.code(), Some(130));
    let heartbeat = fs::read(project.path().join("heartbeat")).unwrap();
    thread::sleep(Duration::from_millis(150));
    assert_eq!(fs::read(project.path().join("heartbeat")).unwrap(), heartbeat);
    assert_eq!(
        fs::read_to_string(project.path().join("pyproject.toml")).unwrap(),
        original
    );
    assert!(!project.path().join("uv.lock").exists());
}

#[cfg(not(target_os = "linux"))]
#[test]
fn apply_is_explicitly_unavailable_outside_linux_in_this_alpha() {
    let project = tempdir().unwrap();
    let original = "[project]\nname = 'example'\nversion = '1.0.0'\n";
    fs::write(project.path().join("pyproject.toml"), original).unwrap();
    let output = Command::new(env!("CARGO_BIN_EXE_pyfaro"))
        .args(["init", "--apply", "--uv", "missing-uv"])
        .arg(project.path())
        .output()
        .unwrap();
    assert_eq!(output.status.code(), Some(1), "{output:?}");
    assert!(String::from_utf8_lossy(&output.stderr).contains("Linux only"));
    assert_eq!(
        fs::read_to_string(project.path().join("pyproject.toml")).unwrap(),
        original
    );
    assert!(!project.path().join("uv.lock").exists());
}
