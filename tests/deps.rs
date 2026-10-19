use std::fs;
use std::path::Path;
use std::process::{Command, Output};

use tempfile::tempdir;
use toml_edit::DocumentMut;

fn project(root: &Path) {
    fs::create_dir_all(root).unwrap();
    for (name, version) in
        [("core-dep", "1.2.3"), ("dev-dep", "2.0.0"), ("quality-dep", "3.0.0")]
    {
        let source = root.join(name);
        fs::create_dir(&source).unwrap();
        fs::write(
            source.join("pyproject.toml"),
            format!("[project]\nname = '{name}'\nversion = '{version}'\nrequires-python = '>=3.10'\n"),
        )
        .unwrap();
    }
    fs::write(
        root.join("pyproject.toml"),
        "# Keep this comment\n[project]\nname = 'dependency-example'\nversion = '0.1.0'\nrequires-python = '>=3.10'\n\n[tool.uv.sources]\ncore-dep = { path = './core-dep' }\ndev-dep = { path = './dev-dep' }\nquality-dep = { path = './quality-dep' }\n",
    )
    .unwrap();
}

fn run(cwd: &Path, cache: &Path, args: &[&str]) -> Output {
    Command::new(env!("CARGO_BIN_EXE_pyfaro"))
        .args(args)
        .current_dir(cwd)
        .env("UV_CACHE_DIR", cache)
        .output()
        .unwrap()
}

fn assert_success(output: &Output) {
    assert!(output.status.success(), "{output:?}");
}

fn metadata(root: &Path) -> DocumentMut {
    fs::read_to_string(root.join("pyproject.toml")).unwrap().parse().unwrap()
}

#[test]
fn add_and_remove_specifiers_in_default_dev_and_named_groups() {
    let sandbox = tempdir().unwrap();
    let root = sandbox.path().join("project with spaces é");
    project(&root);
    let cache = sandbox.path().join("cache");
    let nested = root.join("nested");
    fs::create_dir(&nested).unwrap();
    let uv = which::which("uv").unwrap();

    let mut add = Command::new(env!("CARGO_BIN_EXE_pyfaro"));
    let first = add
        .args(["add", "--offline", "--uv"])
        .arg(uv)
        .arg("core-dep>=1.0,<2")
        .current_dir(&nested)
        .env("UV_CACHE_DIR", &cache)
        .env("UV_DEV", "1")
        .output()
        .unwrap();
    assert_success(&first);
    assert_success(&run(
        &nested,
        &cache,
        &["add", "--offline", "--dev", "dev-dep==2.0.0"],
    ));
    assert_success(&run(
        &nested,
        &cache,
        &["add", "--offline", "--group", "quality", "quality-dep>=3"],
    ));

    let document = metadata(&root);
    assert!(
        document["project"]["dependencies"]
            .to_string()
            .contains("core-dep>=1.0,<2")
    );
    assert!(
        document["dependency-groups"]["dev"]
            .to_string()
            .contains("dev-dep==2.0.0")
    );
    assert!(
        document["dependency-groups"]["quality"]
            .to_string()
            .contains("quality-dep>=3")
    );
    assert!(
        fs::read_to_string(root.join("pyproject.toml"))
            .unwrap()
            .starts_with("# Keep this comment")
    );
    let lock = fs::read_to_string(root.join("uv.lock")).unwrap();
    for name in ["core-dep", "dev-dep", "quality-dep"] {
        assert!(lock.contains(&format!("name = \"{name}\"")), "{lock}");
    }
    assert!(!root.join(".venv").exists());

    assert_success(&run(
        &nested,
        &cache,
        &["remove", "--offline", "--dev", "dev-dep"],
    ));
    assert_success(&run(
        &nested,
        &cache,
        &["remove", "--offline", "--group", "quality", "quality-dep"],
    ));
    assert_success(&run(&nested, &cache, &["remove", "--offline", "core-dep"]));
    let updated = fs::read_to_string(root.join("pyproject.toml")).unwrap();
    assert!(!updated.contains("core-dep>=1.0,<2"));
    assert!(!updated.contains("dev-dep==2.0.0"));
    assert!(!updated.contains("quality-dep>=3"));
    let lock = fs::read_to_string(root.join("uv.lock")).unwrap();
    assert!(!lock.contains("name = \"core-dep\""));
    assert!(!lock.contains("name = \"dev-dep\""));
    assert!(!lock.contains("name = \"quality-dep\""));
    assert!(!root.join("pyfaro.lock").exists());
}

#[test]
fn failures_keep_uv_diagnostics_status_and_existing_files() {
    let sandbox = tempdir().unwrap();
    let root = sandbox.path().join("project");
    project(&root);
    let cache = sandbox.path().join("cache");
    let before = fs::read(root.join("pyproject.toml")).unwrap();

    let unavailable = run(
        &root,
        &cache,
        &["add", "--offline", "no-such-package-pyfaro-issue-14==1.0"],
    );
    assert_eq!(unavailable.status.code(), Some(1), "{unavailable:?}");
    let stderr = String::from_utf8_lossy(&unavailable.stderr);
    assert!(stderr.contains("No solution found"), "{stderr}");
    assert!(stderr.contains("network was disabled"), "{stderr}");
    assert!(stderr.contains("uv add failed"), "{stderr}");
    assert_eq!(fs::read(root.join("pyproject.toml")).unwrap(), before);
    assert!(!root.join("uv.lock").exists());

    let invalid = run(&root, &cache, &["add", "--offline", "core-dep=>1"]);
    assert!(!invalid.status.success(), "{invalid:?}");
    assert!(String::from_utf8_lossy(&invalid.stderr).contains("error:"));
    assert_eq!(fs::read(root.join("pyproject.toml")).unwrap(), before);

    let literal =
        run(&root, &cache, &["add", "--offline", "core-dep>=1;touch marker"]);
    assert!(!literal.status.success(), "{literal:?}");
    assert!(!root.join("marker").exists());
    assert_eq!(fs::read(root.join("pyproject.toml")).unwrap(), before);

    let missing = run(&root, &cache, &["remove", "--offline", "core-dep"]);
    assert!(!missing.status.success(), "{missing:?}");
    assert!(String::from_utf8_lossy(&missing.stderr).contains("core-dep"));
    assert_eq!(fs::read(root.join("pyproject.toml")).unwrap(), before);
}

#[test]
fn explicit_project_is_exact_and_options_require_packages() {
    let sandbox = tempdir().unwrap();
    let root = sandbox.path().join("root");
    project(&root);
    let child = root.join("child");
    fs::create_dir(&child).unwrap();
    let cache = sandbox.path().join("cache");

    let missing = run(
        &root,
        &cache,
        &["add", "--offline", "--project", "child", "core-dep>=1"],
    );
    assert!(!missing.status.success());
    assert!(
        String::from_utf8_lossy(&missing.stderr)
            .contains("has no pyproject.toml")
    );
    assert!(!root.join("uv.lock").exists());

    let selected = run(
        &child,
        &cache,
        &["add", "--offline", "--project", "..", "core-dep>=1"],
    );
    assert_success(&selected);
    assert!(root.join("uv.lock").exists());

    for command in ["add", "remove"] {
        let empty = run(&root, &cache, &[command, "--offline"]);
        assert_eq!(empty.status.code(), Some(2), "{empty:?}");
    }
    let conflict =
        run(&root, &cache, &["add", "--dev", "--group", "quality", "core-dep"]);
    assert_eq!(conflict.status.code(), Some(2), "{conflict:?}");
}
