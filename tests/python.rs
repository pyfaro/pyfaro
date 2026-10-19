mod support;

use std::fs;
use std::process::Command;

use tempfile::tempdir;

fn project(root: &std::path::Path, requires_python: &str) {
    fs::write(
        root.join("pyproject.toml"),
        format!(
            "[project]\nname = 'example'\nversion = '1.0.0'\nrequires-python = '{requires_python}'\n"
        ),
    )
    .unwrap();
}

#[test]
fn install_delegates_requests_without_disabling_explicit_downloads() {
    let sandbox = tempdir().unwrap();
    let project_dir = sandbox.path().join("project with spaces");
    fs::create_dir(&project_dir).unwrap();
    project(&project_dir, ">=3.10");
    let output = Command::new(env!("CARGO_BIN_EXE_pyfaro"))
        .args(["python", "install", "--project"])
        .arg(&project_dir)
        .args(["--uv"])
        .arg(support::fixture_uv())
        .args(["--", "3.12", "pypy3.11"])
        .env("PYFARO_FIXTURE_MODE", "python-install")
        .current_dir(sandbox.path())
        .output()
        .unwrap();
    assert!(output.status.success(), "{output:?}");
    assert!(
        String::from_utf8_lossy(&output.stdout).contains("Python installed")
    );
    let args =
        fs::read_to_string(project_dir.join("uv-arguments.txt")).unwrap();
    assert_eq!(
        args.lines().collect::<Vec<_>>(),
        ["--no-progress", "python", "install", "--", "3.12", "pypy3.11"]
    );
    assert!(!project_dir.join(".python-version").exists());
    assert!(!project_dir.join("uv.lock").exists());
}

#[test]
fn install_without_targets_leaves_version_file_selection_to_uv() {
    let sandbox = tempdir().unwrap();
    project(sandbox.path(), ">=3.10");
    fs::write(sandbox.path().join(".python-version"), "3.11\n").unwrap();
    let output = Command::new(env!("CARGO_BIN_EXE_pyfaro"))
        .args(["python", "install", "--uv"])
        .arg(support::fixture_uv())
        .env("PYFARO_FIXTURE_MODE", "python-install")
        .env("UV_PYTHON", "3.99")
        .current_dir(sandbox.path())
        .output()
        .unwrap();
    assert!(output.status.success(), "{output:?}");
    assert_eq!(
        fs::read_to_string(sandbox.path().join("uv-arguments.txt")).unwrap(),
        "--no-progress\npython\ninstall"
    );
    assert_eq!(
        fs::read_to_string(sandbox.path().join(".python-version")).unwrap(),
        "3.11\n"
    );
}

#[cfg(unix)]
#[test]
fn install_preserves_non_utf8_and_leading_dash_arguments() {
    use std::ffi::OsString;
    use std::os::unix::ffi::OsStringExt;

    let sandbox = tempdir().unwrap();
    let request = OsString::from_vec(vec![b'-', b'3', b'.', b'1', b'2', 0xff]);
    let output = Command::new(env!("CARGO_BIN_EXE_pyfaro"))
        .args(["python", "install", "--uv"])
        .arg(support::fixture_uv())
        .arg("--")
        .arg(&request)
        .env("PYFARO_FIXTURE_MODE", "python-install-os")
        .current_dir(sandbox.path())
        .output()
        .unwrap();
    assert!(output.status.success(), "{output:?}");
    let args = fs::read(sandbox.path().join("uv-arguments.bin")).unwrap();
    assert_eq!(
        args,
        [
            b"--no-progress\0python\0install\0--\0".as_slice(),
            b"-3.12\xff\0".as_slice(),
        ]
        .concat()
    );
}

#[test]
fn real_uv_lists_offline_and_pins_with_project_constraints() {
    let sandbox = tempdir().unwrap();
    let project_dir = sandbox.path().join("project");
    fs::create_dir(&project_dir).unwrap();
    project(&project_dir, ">=3.11,<3.12");
    let list = Command::new(env!("CARGO_BIN_EXE_pyfaro"))
        .args(["python", "list", "--only-installed", "--offline"])
        .arg("3")
        .current_dir(&project_dir)
        .env("UV_CACHE_DIR", sandbox.path().join("cache"))
        .output()
        .unwrap();
    assert!(list.status.success(), "{list:?}");
    assert!(
        !String::from_utf8_lossy(&list.stdout).contains("<download available>")
    );
    let pin = Command::new(env!("CARGO_BIN_EXE_pyfaro"))
        .args(["python", "pin", "3.11", "--offline", "--project"])
        .arg(&project_dir)
        .current_dir(sandbox.path())
        .env("UV_CACHE_DIR", sandbox.path().join("cache"))
        .output()
        .unwrap();
    assert!(pin.status.success(), "{pin:?}");
    assert_eq!(
        fs::read_to_string(project_dir.join(".python-version")).unwrap(),
        "3.11\n"
    );
    let show = Command::new(env!("CARGO_BIN_EXE_pyfaro"))
        .args(["python", "pin", "--offline"])
        .current_dir(&project_dir)
        .env("UV_NO_CONFIG", "1")
        .env("UV_CACHE_DIR", sandbox.path().join("cache"))
        .output()
        .unwrap();
    assert!(show.status.success(), "{show:?}");
    assert!(String::from_utf8_lossy(&show.stdout).contains("3.11"));
    let bad = Command::new(env!("CARGO_BIN_EXE_pyfaro"))
        .args(["python", "pin", "3.10", "--offline"])
        .current_dir(&project_dir)
        .env("UV_NO_PROJECT", "1")
        .env("UV_CACHE_DIR", sandbox.path().join("cache"))
        .output()
        .unwrap();
    assert!(!bad.status.success(), "{bad:?}");
    assert!(String::from_utf8_lossy(&bad.stderr).contains("requires-python"));
    assert_eq!(
        fs::read_to_string(project_dir.join(".python-version")).unwrap(),
        "3.11\n"
    );
    let remove = Command::new(env!("CARGO_BIN_EXE_pyfaro"))
        .args(["python", "pin", "--rm", "--offline"])
        .current_dir(&project_dir)
        .env("UV_CACHE_DIR", sandbox.path().join("cache"))
        .output()
        .unwrap();
    assert!(remove.status.success(), "{remove:?}");
    assert!(!project_dir.join(".python-version").exists());
}

#[test]
fn uv_failures_preserve_status_and_do_not_change_a_pin() {
    let sandbox = tempdir().unwrap();
    project(sandbox.path(), ">=3.10");
    let failure = Command::new(env!("CARGO_BIN_EXE_pyfaro"))
        .args(["python", "install", "--uv"])
        .arg(support::fixture_uv())
        .arg("3.12")
        .env("PYFARO_FIXTURE_MODE", "python-install-fail")
        .current_dir(sandbox.path())
        .output()
        .unwrap();
    assert_eq!(failure.status.code(), Some(37));
    let stderr = String::from_utf8_lossy(&failure.stderr);
    assert!(stderr.contains("Python download failed"), "{stderr}");
    assert!(stderr.contains("uv python install failed"), "{stderr}");
    assert!(!sandbox.path().join(".python-version").exists());

    let missing = Command::new(env!("CARGO_BIN_EXE_pyfaro"))
        .args(["python", "list", "--uv", "missing-uv-binary"])
        .current_dir(sandbox.path())
        .output()
        .unwrap();
    assert_eq!(missing.status.code(), Some(1));
    assert!(String::from_utf8_lossy(&missing.stderr).contains("install uv"));

    let invalid_project = Command::new(env!("CARGO_BIN_EXE_pyfaro"))
        .args(["python", "pin", "3.11", "--project"])
        .arg(sandbox.path().join("missing"))
        .arg("--uv")
        .arg(support::fixture_uv())
        .output()
        .unwrap();
    assert_eq!(invalid_project.status.code(), Some(1));
    assert!(!sandbox.path().join(".python-version").exists());
}
