use std::fs;
use std::process::Command;

use tempfile::tempdir;

#[test]
fn sync_uses_uv_lockfile_and_reproducible_modes() {
    let sandbox = tempdir().unwrap();
    let project = sandbox.path().join("project with spaces é");
    let cache = sandbox.path().join("uv-cache");
    fs::create_dir(&project).unwrap();
    let metadata = project.join("pyproject.toml");
    fs::write(&metadata, project_metadata("1.0.0")).unwrap();

    let sync = |mode: Option<&str>| {
        let mut command = Command::new(env!("CARGO_BIN_EXE_pyfaro"));
        command.arg("sync").arg(&project).arg("--offline");
        if let Some(mode) = mode {
            command.arg(mode);
        }
        command.env("UV_CACHE_DIR", &cache).output().unwrap()
    };

    let first = sync(None);
    assert!(first.status.success(), "{first:?}");
    let lock = fs::read(project.join("uv.lock")).unwrap();
    assert!(project.join(".venv").is_dir());
    assert!(!project.join("pyfaro.lock").exists());

    fs::write(&metadata, project_metadata("2.0.0")).unwrap();
    let locked = sync(Some("--locked"));
    assert!(!locked.status.success(), "{locked:?}");
    assert_eq!(fs::read(project.join("uv.lock")).unwrap(), lock);

    let frozen = sync(Some("--frozen"));
    assert!(frozen.status.success(), "{frozen:?}");
    assert_eq!(fs::read(project.join("uv.lock")).unwrap(), lock);

    let refreshed = sync(None);
    assert!(refreshed.status.success(), "{refreshed:?}");
    assert_ne!(fs::read(project.join("uv.lock")).unwrap(), lock);
    assert!(!project.join("pyfaro.lock").exists());
}

#[test]
fn locked_sync_rejects_missing_lockfile() {
    let project = tempdir().unwrap();
    fs::write(project.path().join("pyproject.toml"), project_metadata("1.0.0"))
        .unwrap();
    let output = Command::new(env!("CARGO_BIN_EXE_pyfaro"))
        .args(["sync", "--locked", "--offline"])
        .arg(project.path())
        .env("UV_CACHE_DIR", project.path().join("uv-cache"))
        .output()
        .unwrap();
    assert!(!output.status.success(), "{output:?}");
    assert!(!project.path().join("uv.lock").exists());
}

fn project_metadata(version: &str) -> String {
    format!(
        "[project]\nname = 'sync-example'\nversion = '{version}'\nrequires-python = '>=3.10'\ndependencies = []\n"
    )
}
