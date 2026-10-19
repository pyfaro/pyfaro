use std::fs;
use std::process::Command;

use tempfile::tempdir;

#[test]
fn nested_commands_select_the_nearest_project_and_allow_an_override() {
    let sandbox = tempdir().unwrap();
    let outer = sandbox.path().join("outer");
    let inner = outer.join("inner");
    let nested = inner.join("src/nested");
    let other = sandbox.path().join("other");
    fs::create_dir_all(&nested).unwrap();
    fs::create_dir(&other).unwrap();
    for directory in [&outer, &inner, &other] {
        fs::write(
            directory.join("pyproject.toml"),
            "[project]\nname = 'context-example'\nversion = '1.0.0'\nrequires-python = '>=3.10'\ndependencies = []\n",
        )
        .unwrap();
    }
    let cache = sandbox.path().join("uv-cache");
    fs::write(nested.join("marker.txt"), "nested").unwrap();
    let discovered = Command::new(env!("CARGO_BIN_EXE_pyfaro"))
        .args([
            "run",
            "--offline",
            "--",
            "python",
            "-c",
            "import sys; print(sys.prefix); print(open('marker.txt').read())",
        ])
        .current_dir(&nested)
        .env("UV_CACHE_DIR", &cache)
        .output()
        .unwrap();
    assert!(discovered.status.success(), "{discovered:?}");
    let output = String::from_utf8(discovered.stdout).unwrap();
    let mut lines = output.lines();
    let prefix = lines.next().unwrap();
    assert_eq!(
        fs::canonicalize(prefix).unwrap(),
        fs::canonicalize(inner.join(".venv")).unwrap()
    );
    assert_eq!(lines.next(), Some("nested"));
    assert!(!outer.join(".venv").exists());

    let overridden = Command::new(env!("CARGO_BIN_EXE_pyfaro"))
        .args(["run", "--offline", "--project"])
        .arg(&other)
        .args([
            "--",
            "python",
            "-c",
            "import sys; print(sys.prefix); print(open('marker.txt').read())",
        ])
        .current_dir(&nested)
        .env("UV_CACHE_DIR", &cache)
        .output()
        .unwrap();
    assert!(overridden.status.success(), "{overridden:?}");
    let output = String::from_utf8(overridden.stdout).unwrap();
    let mut lines = output.lines();
    let prefix = lines.next().unwrap();
    assert_eq!(
        fs::canonicalize(prefix).unwrap(),
        fs::canonicalize(other.join(".venv")).unwrap()
    );
    assert_eq!(lines.next(), Some("nested"));

    let synced = Command::new(env!("CARGO_BIN_EXE_pyfaro"))
        .args(["sync", "--locked", "--offline"])
        .current_dir(&nested)
        .env("UV_CACHE_DIR", &cache)
        .output()
        .unwrap();
    assert!(synced.status.success(), "{synced:?}");
    assert!(inner.join("uv.lock").exists());

    let explicit = Command::new(env!("CARGO_BIN_EXE_pyfaro"))
        .args(["sync", "--project"])
        .arg(&other)
        .arg("--offline")
        .current_dir(&nested)
        .env("UV_CACHE_DIR", &cache)
        .output()
        .unwrap();
    assert!(explicit.status.success(), "{explicit:?}");
    assert!(other.join("uv.lock").exists());

    let positional = Command::new(env!("CARGO_BIN_EXE_pyfaro"))
        .arg("sync")
        .arg(&nested)
        .arg("--offline")
        .current_dir(sandbox.path())
        .env("UV_CACHE_DIR", &cache)
        .output()
        .unwrap();
    assert!(positional.status.success(), "{positional:?}");
}

#[test]
fn explicit_project_does_not_fall_back_to_a_parent() {
    let sandbox = tempdir().unwrap();
    fs::write(
        sandbox.path().join("pyproject.toml"),
        "[project]\nname = 'parent'\nversion = '1.0.0'\nrequires-python = '>=3.10'\n",
    )
    .unwrap();
    let child = sandbox.path().join("child");
    fs::create_dir(&child).unwrap();
    for subcommand in ["run", "sync"] {
        let mut command = Command::new(env!("CARGO_BIN_EXE_pyfaro"));
        command.args([subcommand, "--project"]).arg(&child);
        if subcommand == "run" {
            command.args(["--", "python", "-c", "print('wrong')"]);
        }
        let output = command.output().unwrap();
        assert!(!output.status.success(), "{output:?}");
        assert!(
            String::from_utf8_lossy(&output.stderr).contains("pyproject.toml"),
            "{output:?}"
        );
    }
    assert!(!sandbox.path().join("uv.lock").exists());
}

#[cfg(unix)]
#[test]
fn symlinked_working_directory_resolves_to_its_physical_project() {
    use std::os::unix::fs::symlink;

    let sandbox = tempdir().unwrap();
    let project = sandbox.path().join("project");
    let nested = project.join("src");
    fs::create_dir_all(&nested).unwrap();
    fs::write(
        project.join("pyproject.toml"),
        "[project]\nname = 'linked'\nversion = '1.0.0'\nrequires-python = '>=3.10'\n",
    )
    .unwrap();
    let link = sandbox.path().join("linked-src");
    symlink(&nested, &link).unwrap();
    let output = Command::new(env!("CARGO_BIN_EXE_pyfaro"))
        .args([
            "run",
            "--offline",
            "--",
            "python",
            "-c",
            "import sys; print(sys.prefix)",
        ])
        .current_dir(&link)
        .env("UV_CACHE_DIR", sandbox.path().join("uv-cache"))
        .output()
        .unwrap();
    assert!(output.status.success(), "{output:?}");
    let prefix = String::from_utf8(output.stdout).unwrap();
    assert_eq!(
        fs::canonicalize(prefix.trim()).unwrap(),
        fs::canonicalize(project.join(".venv")).unwrap()
    );
}
