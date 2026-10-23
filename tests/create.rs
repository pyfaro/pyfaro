use std::fs;
use std::path::Path;
use std::process::{Command, Output};

use tempfile::tempdir;

fn pyfaro(args: &[&str], target: &Path) -> Output {
    Command::new(env!("CARGO_BIN_EXE_pyfaro"))
        .arg("create")
        .args(args)
        .args(["--path"])
        .arg(target)
        .output()
        .expect("start pyfaro")
}

fn assert_success(output: Output) {
    assert!(output.status.success(), "{output:?}");
}

#[test]
fn library_template_has_the_documented_content() {
    let sandbox = tempdir().unwrap();
    let project = sandbox.path().join("library with spaces é");
    assert_success(pyfaro(
        &["lib", "acme-widget", "--python", "3.10"],
        &project,
    ));
    let metadata = fs::read_to_string(project.join("pyproject.toml")).unwrap();
    assert!(metadata.contains("name = \"acme-widget\""));
    assert!(metadata.contains("requires-python = \">=3.10\""));
    assert!(metadata.contains("[tool.pyfaro]\nschema-version = 1"));
    assert!(project.join("src/acme_widget/__init__.py").is_file());
    assert!(project.join("src/acme_widget/py.typed").is_file());
    assert!(project.join("tests/test_library.py").is_file());
    assert!(project.join("README.md").is_file());
    assert!(!project.join("uv.lock").exists());
    assert!(!project.join(".venv").exists());

    assert!(metadata.contains("pytest==9.1.1"));
    assert!(metadata.contains("ruff==0.16.10"));
    assert!(metadata.contains("ty==0.0.85"));
    assert!(metadata.contains("[tool.pytest.ini_options]"));
    assert!(metadata.contains("[tool.ruff]"));
    let readme = fs::read_to_string(project.join("README.md")).unwrap();
    assert!(readme.contains("pyfaro sync") && readme.contains("uv.lock"));
    assert!(readme.contains("pyfaro check") && readme.contains("pyfaro test"));
}

#[test]
fn cli_uses_separate_distribution_package_and_command_names() {
    let sandbox = tempdir().unwrap();
    let project = sandbox.path().join("a cli project");
    assert_success(pyfaro(
        &[
            "cli",
            "acme-tools",
            "--package",
            "acme_console",
            "--command",
            "acme-do",
        ],
        &project,
    ));
    let metadata = fs::read_to_string(project.join("pyproject.toml")).unwrap();
    assert!(metadata.contains("acme-do = \"acme_console.cli:main\""));
    assert!(metadata.contains("module-name = \"acme_console\""));
    assert!(project.join("src/acme_console/cli.py").is_file());
}

#[test]
fn fastapi_template_has_the_documented_content() {
    let sandbox = tempdir().unwrap();
    let project = sandbox.path().join("api with spaces é");
    assert_success(pyfaro(
        &["fastapi", "acme-api", "--package", "backend", "--python", "3.12"],
        &project,
    ));
    let metadata = fs::read_to_string(project.join("pyproject.toml")).unwrap();
    assert!(metadata.contains(
        "dependencies = [\"fastapi>=0.161.1\", \"uvicorn[standard]>=0.54.0\"]"
    ));
    assert!(metadata.contains(
        "dev = [\"pytest==9.1.1\", \"ruff==0.16.10\", \"ty==0.0.85\", \"httpx2==2.16.0; python_full_version >= '3.11'\", \"httpx==0.28.1; python_full_version < '3.11'\"]"
    ));
    assert!(metadata.contains("module-name = \"backend\""));
    assert!(metadata.contains("[tool.pyfaro]\nschema-version = 1"));
    assert!(metadata.contains(
        "[tool.pyfaro]\nschema-version = 1\n\n[tool.pyfaro.dev]\npreset = \"fastapi\"\nentry-point = \"backend.main:app\"\n"
    ));
    let main = fs::read_to_string(project.join("src/backend/main.py")).unwrap();
    assert!(main.contains("app = FastAPI(title=\"acme-api\")"));
    assert!(main.contains("\"Hello from acme-api!\""));
    assert!(main.contains("\"status\": \"ok\""));
    assert!(project.join("src/backend/__init__.py").is_file());
    assert!(project.join("src/backend/py.typed").is_file());
    assert!(project.join("tests/test_main.py").is_file());
    assert!(!project.join("uv.lock").exists());
    assert!(!project.join(".venv").exists());
}

#[test]
fn invalid_inputs_do_not_create_a_project() {
    let sandbox = tempdir().unwrap();
    for (args, expected) in [
        (vec!["lib", "../escape"], "project name"),
        (vec!["lib", "con.demo"], "project name"),
        (vec!["lib", "foo-stubs"], "project name"),
        (vec!["lib", "hello", "--python", "3.12.1"], "Python version"),
        (vec!["cli", "hello", "--package", "for"], "package name"),
        (vec!["cli", "hello", "--package", "__pycache__"], "package name"),
        (vec!["cli", "hello", "--package", "unittest"], "package name"),
        (vec!["cli", "hello", "--package", "__main__"], "package name"),
        (vec!["cli", "hello", "--package", "__hello__"], "package name"),
        (vec!["cli", "hello", "--package", "con"], "package name"),
        (vec!["fastapi", "hello", "--package", "for"], "package name"),
        (vec!["fastapi", "hello", "--python", "3.7"], "Python version"),
        (vec!["fastapi", "unittest"], "package name"),
        (vec!["fastapi", "foo-stubs"], "project name"),
        (vec!["lib", "unittest"], "package name"),
        (vec!["lib", "cgi"], "package name"),
        (vec!["cli", "hello", "--command", "bad/name"], "command name"),
        (vec!["cli", "hello", "--command", "COM1"], "command name"),
    ] {
        let target = sandbox.path().join("new-project");
        let output = pyfaro(&args, &target);
        assert_eq!(output.status.code(), Some(1), "{output:?}");
        assert!(String::from_utf8_lossy(&output.stderr).contains(expected));
        assert!(!target.exists());
    }
}

#[test]
fn existing_target_and_missing_parent_are_rejected_without_changes() {
    let sandbox = tempdir().unwrap();
    let project = sandbox.path().join("existing");
    fs::create_dir(&project).unwrap();
    fs::write(project.join("keep.txt"), "keep").unwrap();
    let output = pyfaro(&["lib", "example"], &project);
    assert_eq!(output.status.code(), Some(1), "{output:?}");
    assert_eq!(fs::read_to_string(project.join("keep.txt")).unwrap(), "keep");
    assert!(!project.join("pyproject.toml").exists());

    let missing = sandbox.path().join("missing/child");
    let output = pyfaro(&["lib", "example"], &missing);
    assert_eq!(output.status.code(), Some(1), "{output:?}");
    assert!(!missing.parent().unwrap().exists());
}

#[test]
fn default_target_is_the_named_directory() {
    let sandbox = tempdir().unwrap();
    let output = Command::new(env!("CARGO_BIN_EXE_pyfaro"))
        .current_dir(sandbox.path())
        .args(["create", "lib", "demo-lib"])
        .output()
        .unwrap();
    assert_success(output);
    assert!(sandbox.path().join("demo-lib/src/demo_lib/__init__.py").is_file());
}

#[test]
fn numeric_distribution_gets_an_importable_package() {
    let sandbox = tempdir().unwrap();
    let project = sandbox.path().join("numeric");
    assert_success(pyfaro(&["lib", "1widget"], &project));
    let metadata = fs::read_to_string(project.join("pyproject.toml")).unwrap();
    assert!(metadata.contains("module-name = \"_1widget\""));
    assert!(project.join("src/_1widget/__init__.py").is_file());
}

#[test]
fn repeated_separators_normalize_to_a_buildable_package() {
    let sandbox = tempdir().unwrap();
    let project = sandbox.path().join("normalized");
    assert_success(pyfaro(&["lib", "foo--bar"], &project));
    assert!(project.join("src/foo_bar/__init__.py").is_file());
}

#[test]
fn dotted_distribution_gets_a_valid_default_command() {
    let sandbox = tempdir().unwrap();
    let project = sandbox.path().join("dotted");
    assert_success(pyfaro(&["cli", "acme.tools"], &project));
    let metadata = fs::read_to_string(project.join("pyproject.toml")).unwrap();
    assert!(metadata.contains("acme-tools = \"acme_tools.cli:main\""));
}
