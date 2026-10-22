//! `pyfaro create` and `pyfaro init`.

use std::fs;
use std::path::{Path, PathBuf};

use crate::harness::{Cmd, PYTEST, RUFF, Sandbox, TY, snapshot};

const NAME_ERROR: &str = "error: project name must contain ASCII letters, digits, dots, underscores, or hyphens, start/end with a letter or digit, and not be a reserved Windows device name or stub-package name\n";
const PYTHON_ERROR: &str =
    "error: Python version must be MAJOR.MINOR, for example 3.12\n";

fn package_error(package: &str) -> String {
    format!(
        "error: invalid Python package name: {package}; use an ASCII identifier that is not a keyword, standard-library module, dunder module, or reserved Windows device name\n"
    )
}

fn command_error(command: &str) -> String {
    format!(
        "error: invalid command name: {command}; use ASCII letters, digits, hyphens, or underscores and avoid reserved Windows device names\n"
    )
}

/// Every file below `root` as `/`-separated relative paths.
fn tree(root: &Path) -> Vec<String> {
    snapshot(root).into_keys().collect()
}

fn entries(dir: &Path) -> usize {
    fs::read_dir(dir).expect("read directory").count()
}

fn read(path: impl AsRef<Path>) -> String {
    fs::read_to_string(path).expect("read file")
}

// ---------------------------------------------------------------- create

#[test]
fn create_lib_defaults_to_a_directory_named_after_the_project() {
    e2e_guard!();
    let sandbox = Sandbox::new();
    let out = sandbox.pyfaro(["create", "lib", "demo-lib"]).run();
    out.success().stderr_empty().stdout_contains("Created library at ");
    assert!(out.stdout.trim_end().ends_with("demo-lib"), "{}", out.stdout);
    let root = sandbox.join("demo-lib");
    assert_eq!(
        tree(&root),
        [
            "README.md",
            "pyproject.toml",
            "src/demo_lib/__init__.py",
            "src/demo_lib/py.typed",
            "tests/test_library.py",
        ]
    );
    assert_eq!(tree(sandbox.path()).len(), 5);
}

#[test]
fn create_lib_writes_the_documented_pyproject() {
    e2e_guard!();
    let sandbox = Sandbox::new();
    sandbox
        .pyfaro(["create", "lib", "acme-widget", "--python", "3.12"])
        .run()
        .success();
    let metadata = read(sandbox.join("acme-widget/pyproject.toml"));
    assert_eq!(
        metadata,
        format!(
            "[project]\nname = \"acme-widget\"\nversion = \"0.1.0\"\nreadme = \"README.md\"\nrequires-python = \">=3.12\"\ndependencies = []\n\n[build-system]\nrequires = [\"uv_build>=0.9.26,<0.10.0\"]\nbuild-backend = \"uv_build\"\n\n[dependency-groups]\ndev = [\"pytest=={PYTEST}\", \"ruff=={RUFF}\", \"ty=={TY}\"]\n\n[tool.pyfaro]\nschema-version = 1\n\n[tool.ruff]\nline-length = 88\nsrc = [\"src\", \"tests\"]\n\n[tool.pytest.ini_options]\ntestpaths = [\"tests\"]\n"
        )
    );
    assert!(!metadata.contains("[tool.ty]"));
    assert!(!metadata.contains("module-name"));
    let readme = read(sandbox.join("acme-widget/README.md"));
    assert!(readme.starts_with(
        "# acme-widget\n\nA Python library created with Pyfaro.\n"
    ));
    assert!(readme.contains("Run `pyfaro sync` first."));
    assert!(readme.contains("`uv.lock`"));
    assert!(readme.contains(
        "```sh\npyfaro sync\npyfaro check\npyfaro test\nuv build\n```"
    ));
    assert!(!readme.contains("pyfaro run"));
}

#[test]
fn create_cli_writes_the_documented_tree_and_scripts() {
    e2e_guard!();
    let sandbox = Sandbox::new();
    let out = sandbox.pyfaro(["create", "cli", "acme-tools"]).run();
    out.success().stderr_empty().stdout_contains("Created CLI at ");
    let root = sandbox.join("acme-tools");
    assert_eq!(
        tree(&root),
        [
            "README.md",
            "pyproject.toml",
            "src/acme_tools/__init__.py",
            "src/acme_tools/cli.py",
            "src/acme_tools/py.typed",
            "tests/test_cli.py",
        ]
    );
    let metadata = read(root.join("pyproject.toml"));
    assert!(metadata.contains("requires-python = \">=3.10\""));
    assert!(metadata.contains(
        "\n[project.scripts]\nacme-tools = \"acme_tools.cli:main\"\n"
    ));
    assert!(!metadata.contains("module-name"));
    let readme = read(root.join("README.md"));
    assert!(readme.contains("A command-line project created with Pyfaro."));
    assert!(readme.contains(
        "Run the installed command with `pyfaro run --locked -- acme-tools`."
    ));
}

#[test]
fn create_path_places_the_project_at_a_relative_or_absolute_target() {
    e2e_guard!();
    let sandbox = Sandbox::new();
    fs::create_dir(sandbox.join("projects")).unwrap();
    sandbox
        .pyfaro(["create", "lib", "demo", "--path", "projects/renamed"])
        .run()
        .success();
    assert!(sandbox.join("projects/renamed/pyproject.toml").is_file());
    assert!(!sandbox.join("demo").exists());
    let absolute = sandbox.join("projects/absolute dir é");
    sandbox
        .pyfaro(["create", "lib", "demo", "--path"])
        .arg(&absolute)
        .run()
        .success();
    assert!(absolute.join("src/demo/__init__.py").is_file());
}

#[test]
fn create_path_requires_an_existing_parent_and_leaves_nothing_behind() {
    e2e_guard!();
    let sandbox = Sandbox::new();
    let out = sandbox
        .pyfaro(["create", "lib", "demo", "--path", "missing/child"])
        .run();
    out.code(1).stdout_empty().stderr_contains(
        "error: target parent must be an existing directory: ",
    );
    assert!(out.stderr.trim_end().ends_with("missing"), "{}", out.stderr);
    assert_eq!(entries(sandbox.path()), 0);
}

#[test]
fn create_rejects_an_existing_directory_without_touching_it() {
    e2e_guard!();
    let sandbox = Sandbox::new();
    sandbox.write("existing/keep.txt", "keep");
    let before = snapshot(sandbox.path());
    sandbox
        .pyfaro(["create", "lib", "demo", "--path", "existing"])
        .run()
        .code(1)
        .stdout_empty()
        .stderr_contains("error: could not create project directory ");
    assert_eq!(snapshot(sandbox.path()), before);
    assert_eq!(read(sandbox.join("existing/keep.txt")), "keep");
}

#[test]
fn create_rejects_an_existing_file_target() {
    e2e_guard!();
    let sandbox = Sandbox::new();
    sandbox.write("occupied", "file");
    sandbox
        .pyfaro(["create", "cli", "demo", "--path", "occupied"])
        .run()
        .code(1)
        .stderr_contains("error: could not create project directory ");
    assert_eq!(read(sandbox.join("occupied")), "file");
    assert_eq!(entries(sandbox.path()), 1);
}

#[cfg(unix)]
#[test]
fn create_rejects_a_symlink_target_without_following_it() {
    e2e_guard!();
    let sandbox = Sandbox::new();
    fs::create_dir(sandbox.join("real")).unwrap();
    std::os::unix::fs::symlink(sandbox.join("real"), sandbox.join("link"))
        .unwrap();
    sandbox
        .pyfaro(["create", "lib", "demo", "--path", "link"])
        .run()
        .code(1)
        .stderr_contains("error: could not create project directory ");
    assert_eq!(entries(&sandbox.join("real")), 0);
}

#[test]
fn create_python_sets_the_minimum_version() {
    e2e_guard!();
    let sandbox = Sandbox::new();
    for version in ["3.8", "3.10", "3.12", "3.13", "3.99"] {
        let target = format!("py{version}");
        sandbox
            .pyfaro(["create", "lib", "demo", "--python", version, "--path"])
            .arg(&target)
            .run()
            .success();
        assert!(
            read(sandbox.join(&target).join("pyproject.toml"))
                .contains(&format!("requires-python = \">={version}\"\n")),
            "{version}"
        );
    }
}

#[test]
fn create_rejects_invalid_python_versions_and_creates_nothing() {
    e2e_guard!();
    let sandbox = Sandbox::new();
    for version in [
        "3", "3.", "3.7", "3.08", "3.100", "3.x", "3.12.1", "4.0", "2.7",
        "312", "", "3.-1", "3.1e1",
    ] {
        sandbox
            .pyfaro(["create", "lib", "demo", "--python", version])
            .run()
            .code(1)
            .stdout_empty()
            .stderr_contains(PYTHON_ERROR);
        assert_eq!(entries(sandbox.path()), 0, "--python {version:?}");
    }
}

#[test]
fn create_cli_package_and_command_names_are_independent() {
    e2e_guard!();
    let sandbox = Sandbox::new();
    sandbox
        .pyfaro([
            "create",
            "cli",
            "acme-tools",
            "--package",
            "acme_console",
            "--command",
            "acme-do",
        ])
        .run()
        .success();
    let root = sandbox.join("acme-tools");
    assert_eq!(
        tree(&root),
        [
            "README.md",
            "pyproject.toml",
            "src/acme_console/__init__.py",
            "src/acme_console/cli.py",
            "src/acme_console/py.typed",
            "tests/test_cli.py",
        ]
    );
    let metadata = read(root.join("pyproject.toml"));
    assert!(metadata.contains("name = \"acme-tools\""));
    assert!(
        metadata.contains(
            "[project.scripts]\nacme-do = \"acme_console.cli:main\"\n"
        )
    );
    assert!(
        metadata.contains(
            "[tool.uv.build-backend]\nmodule-name = \"acme_console\"\n"
        )
    );
    assert!(
        read(root.join("README.md")).contains("pyfaro run --locked -- acme-do")
    );
    assert!(
        read(root.join("tests/test_cli.py"))
            .contains("from acme_console.cli import main")
    );
    assert!(
        read(root.join("tests/test_cli.py")).contains("Hello from acme-tools!")
    );

    // Only the package differs, then only the command differs.
    sandbox
        .pyfaro(["create", "cli", "only-pkg", "--package", "other_pkg"])
        .run()
        .success();
    let metadata = read(sandbox.join("only-pkg/pyproject.toml"));
    assert!(metadata.contains("only-pkg = \"other_pkg.cli:main\""));
    assert!(metadata.contains("module-name = \"other_pkg\""));
    sandbox
        .pyfaro(["create", "cli", "only-cmd", "--command", "run_it"])
        .run()
        .success();
    let metadata = read(sandbox.join("only-cmd/pyproject.toml"));
    assert!(metadata.contains("run_it = \"only_cmd.cli:main\""));
    assert!(!metadata.contains("module-name"));
}

#[test]
fn create_cli_explicit_package_equal_to_the_default_adds_no_module_name() {
    e2e_guard!();
    let sandbox = Sandbox::new();
    sandbox
        .pyfaro(["create", "cli", "acme-tools", "--package", "acme_tools"])
        .run()
        .success();
    assert!(
        !read(sandbox.join("acme-tools/pyproject.toml"))
            .contains("module-name")
    );
}

#[test]
fn create_cli_dotted_name_gets_a_hyphenated_default_command() {
    e2e_guard!();
    let sandbox = Sandbox::new();
    sandbox.pyfaro(["create", "cli", "acme.tools"]).run().success();
    let root = sandbox.join("acme.tools");
    let metadata = read(root.join("pyproject.toml"));
    assert!(metadata.contains("name = \"acme.tools\""));
    assert!(metadata.contains("acme-tools = \"acme_tools.cli:main\""));
    assert!(root.join("src/acme_tools/cli.py").is_file());
}

#[test]
fn create_numeric_name_gets_a_leading_underscore_package() {
    e2e_guard!();
    let sandbox = Sandbox::new();
    sandbox.pyfaro(["create", "lib", "1widget"]).run().success();
    let root = sandbox.join("1widget");
    assert!(root.join("src/_1widget/__init__.py").is_file());
    assert!(
        read(root.join("pyproject.toml"))
            .contains("[tool.uv.build-backend]\nmodule-name = \"_1widget\"\n")
    );
    assert!(
        read(root.join("tests/test_library.py"))
            .contains("from _1widget import hello")
    );
}

#[test]
fn create_repeated_separators_collapse_into_one_underscore() {
    e2e_guard!();
    let sandbox = Sandbox::new();
    sandbox.pyfaro(["create", "lib", "Foo-_.Bar--baz"]).run().success();
    let root = sandbox.join("Foo-_.Bar--baz");
    assert!(root.join("src/foo_bar_baz/__init__.py").is_file());
    let metadata = read(root.join("pyproject.toml"));
    assert!(metadata.contains("name = \"Foo-_.Bar--baz\""));
    assert!(!metadata.contains("module-name"));
}

#[test]
fn create_rejects_invalid_distribution_names_and_creates_nothing() {
    e2e_guard!();
    let sandbox = Sandbox::new();
    for name in [
        "../escape",
        ".hidden",
        "trailing-",
        "_lead",
        "has space",
        "bad/name",
        "pâquet",
        "foo-stubs",
        "Foo_Stubs",
        "foo.stubs",
        "con",
        "CON",
        "con.demo",
        "NUL.txt",
        "aux",
        "prn",
        "COM1",
        "lpt9",
        "",
    ] {
        sandbox
            .pyfaro(["create", "lib", name])
            .run()
            .code(1)
            .stdout_empty()
            .stderr_eq(NAME_ERROR);
        sandbox
            .pyfaro(["create", "cli", name])
            .run()
            .code(1)
            .stderr_eq(NAME_ERROR);
        assert_eq!(entries(sandbox.path()), 0, "{name:?}");
    }
}

#[test]
fn create_rejects_invalid_package_names_and_creates_nothing() {
    e2e_guard!();
    let sandbox = Sandbox::new();
    for package in [
        "for",
        "match",
        "type",
        "__pycache__",
        "__main__",
        "__hello__",
        "unittest",
        "json",
        "con",
        "COM1",
        "nul",
        "1pkg",
        "my-pkg",
        "pâquet",
        "has space",
        "",
    ] {
        sandbox
            .pyfaro(["create", "cli", "hello", "--package", package])
            .run()
            .code(1)
            .stdout_empty()
            .stderr_eq(&package_error(package));
        assert_eq!(entries(sandbox.path()), 0, "--package {package:?}");
    }
}

#[test]
fn create_rejects_a_default_package_that_collides_with_the_standard_library() {
    e2e_guard!();
    let sandbox = Sandbox::new();
    for name in ["unittest", "cgi", "Json"] {
        let package = name.to_ascii_lowercase();
        sandbox
            .pyfaro(["create", "lib", name])
            .run()
            .code(1)
            .stderr_eq(&package_error(&package));
        sandbox
            .pyfaro(["create", "cli", name])
            .run()
            .code(1)
            .stderr_eq(&package_error(&package));
        assert_eq!(entries(sandbox.path()), 0);
    }
    // An explicit package gets past the colliding default.
    sandbox
        .pyfaro(["create", "cli", "unittest", "--package", "unit_cli"])
        .run()
        .success();
}

#[test]
fn create_rejects_invalid_command_names_and_creates_nothing() {
    e2e_guard!();
    let sandbox = Sandbox::new();
    for command in [
        "bad/name", "bad name", "cmd.exe", "_lead", "pâquet", "con", "AUX",
        "COM1", "lpt1", "",
    ] {
        sandbox
            .pyfaro(["create", "cli", "hello", "--command", command])
            .run()
            .code(1)
            .stdout_empty()
            .stderr_eq(&command_error(command));
        assert_eq!(entries(sandbox.path()), 0, "--command {command:?}");
    }
}

#[test]
fn create_usage_errors_exit_2_and_create_nothing() {
    e2e_guard!();
    let sandbox = Sandbox::new();
    sandbox.pyfaro(["create"]).run().code(2).stderr_contains("Usage: pyfaro");
    sandbox
        .pyfaro(["create", "lib"])
        .run()
        .code(2)
        .stderr_contains("required arguments were not provided");
    sandbox
        .pyfaro(["create", "lib", "demo", "--command", "demo"])
        .run()
        .code(2)
        .stderr_contains("unexpected argument '--command'");
    sandbox
        .pyfaro(["create", "lib", "demo", "--package", "demo"])
        .run()
        .code(2)
        .stderr_contains("unexpected argument '--package'");
    sandbox
        .pyfaro(["create", "app", "demo"])
        .run()
        .code(2)
        .stderr_contains("unrecognized subcommand 'app'");
    assert_eq!(entries(sandbox.path()), 0);
}

#[test]
fn create_never_invokes_uv() {
    e2e_guard!();
    let sandbox = Sandbox::new();
    let empty_path = sandbox.join("empty path");
    fs::create_dir(&empty_path).unwrap();
    for (kind, name) in [("lib", "no-uv-lib"), ("cli", "no-uv-cli")] {
        sandbox
            .pyfaro(["create", kind, name, "--path"])
            .arg(sandbox.join(name))
            .env("PATH", &empty_path)
            .env("UV", "/nonexistent/uv")
            .run()
            .success();
        let root = sandbox.join(name);
        assert!(!root.join(".venv").exists());
        assert!(!root.join("uv.lock").exists());
        assert!(!root.join("dist").exists());
    }
}

/// Generate a project in the sandbox and return its root.
fn generate(sandbox: &Sandbox, args: &[&str], dir: &str) -> PathBuf {
    let root = sandbox.join(dir);
    sandbox
        .pyfaro(["create"])
        .args(args)
        .args(["--python", "3.12", "--path"])
        .arg(&root)
        .run()
        .success();
    root
}

/// `pyfaro sync`, `check`, `test`, and `uv build`; returns the built files.
fn develop(root: &Path) -> Vec<String> {
    Cmd::pyfaro(root, ["sync"]).run().success();
    assert!(root.join("uv.lock").is_file());
    assert!(root.join(".venv").is_dir());
    let lock = read(root.join("uv.lock"));
    for tool in [
        format!("name = \"pytest\"\nversion = \"{PYTEST}\""),
        format!("name = \"ruff\"\nversion = \"{RUFF}\""),
        format!("name = \"ty\"\nversion = \"{TY}\""),
    ] {
        assert!(lock.contains(&tool), "{tool} missing from uv.lock");
    }
    Cmd::pyfaro(root, ["check"]).run().success();
    Cmd::pyfaro(root, ["test"]).run().success().stdout_contains("1 passed");
    Cmd::uv(root, ["build"]).run().success();
    let mut built: Vec<String> = fs::read_dir(root.join("dist"))
        .unwrap()
        .map(|entry| entry.unwrap().file_name().to_string_lossy().into_owned())
        .filter(|name| !name.starts_with('.'))
        .collect();
    built.sort();
    built
}

#[test]
fn created_lib_syncs_checks_tests_and_builds() {
    e2e_guard!();
    let sandbox = Sandbox::new();
    let root = generate(&sandbox, &["lib", "acme-widget"], "library é");
    assert_eq!(
        develop(&root),
        ["acme_widget-0.1.0-py3-none-any.whl", "acme_widget-0.1.0.tar.gz"]
    );
}

#[test]
fn created_cli_runs_its_installed_command_through_pyfaro_run() {
    e2e_guard!();
    let sandbox = Sandbox::new();
    let root = generate(&sandbox, &["cli", "acme-tools"], "a cli project");
    assert_eq!(
        develop(&root),
        ["acme_tools-0.1.0-py3-none-any.whl", "acme_tools-0.1.0.tar.gz"]
    );
    assert_runs(&root, "acme-tools", "Hello from acme-tools!");
}

#[test]
fn created_cli_with_distinct_names_builds_and_runs_the_selected_command() {
    e2e_guard!();
    let sandbox = Sandbox::new();
    let root = generate(
        &sandbox,
        &[
            "cli",
            "acme-tools",
            "--package",
            "acme_console",
            "--command",
            "acme-do",
        ],
        "distinct",
    );
    assert_eq!(
        develop(&root),
        ["acme_tools-0.1.0-py3-none-any.whl", "acme_tools-0.1.0.tar.gz"]
    );
    assert_runs(&root, "acme-do", "Hello from acme-tools!");
    Cmd::pyfaro(&root, ["run", "--locked", "--", "acme-tools"]).run().code(2);
}

#[test]
fn created_dotted_cli_runs_its_default_command() {
    e2e_guard!();
    let sandbox = Sandbox::new();
    let root = generate(&sandbox, &["cli", "acme.tools"], "dotted");
    develop(&root);
    assert_runs(&root, "acme-tools", "Hello from acme.tools!");
}

#[test]
fn created_numeric_lib_syncs_checks_tests_and_builds() {
    e2e_guard!();
    let sandbox = Sandbox::new();
    let root = generate(&sandbox, &["lib", "1widget"], "numeric");
    assert_eq!(
        develop(&root),
        ["1widget-0.1.0-py3-none-any.whl", "1widget-0.1.0.tar.gz"]
    );
}

#[test]
fn created_lib_with_repeated_separators_syncs_checks_tests_and_builds() {
    e2e_guard!();
    let sandbox = Sandbox::new();
    let root = generate(&sandbox, &["lib", "foo--bar"], "normalized");
    assert_eq!(
        develop(&root),
        ["foo_bar-0.1.0-py3-none-any.whl", "foo_bar-0.1.0.tar.gz"]
    );
}

// ------------------------------------------------------------------ init

const PREVIEW_NEW: &str = "Initialization plan for fresh:\n  Run uv init --bare (no Git, README, or Python pin files).\n  Ensure the following configuration in pyproject.toml:\n\n[tool.pyfaro]\nschema-version = 1\n\n  Run uv lock --no-build to create or update uv.lock.\nPreview only: rerun with --apply to initialize this project.\n";
const PREVIEW_EXISTING: &str = "Initialization plan for .:\n  Ensure the following configuration in pyproject.toml:\n\n[tool.pyfaro]\nschema-version = 1\n\n  Run uv lock --no-build to create or update uv.lock.\nPreview only: rerun with --apply to initialize this project.\n";
const PREVIEW_CONFIGURED: &str = "Initialization plan for .:\n  [tool.pyfaro] is already configured.\n  Run uv lock --no-build to create or update uv.lock.\nPreview only: rerun with --apply to initialize this project.\n";
const PLAIN: &str = "[project]\nname = \"example\"\nversion = \"1.0.0\"\nrequires-python = \">=3.10\"\ndependencies = []\n";
const UNSUPPORTED_APPLY: &str = "error: init --apply is supported on Linux only in this alpha; preview is available on all platforms\n";

/// A `PATH` that cannot find uv, to prove a path never needs it.
fn without_uv(sandbox: &Sandbox) -> PathBuf {
    let empty = sandbox.join("no uv here");
    fs::create_dir_all(&empty).unwrap();
    empty
}

#[test]
fn init_preview_of_a_new_project_writes_nothing_and_needs_no_uv() {
    e2e_guard!();
    let sandbox = Sandbox::new();
    let empty = without_uv(&sandbox);
    let before = snapshot(sandbox.path());
    sandbox
        .pyfaro(["init", "fresh"])
        .env("PATH", &empty)
        .run()
        .success()
        .stdout_eq(PREVIEW_NEW)
        .stderr_empty();
    assert_eq!(snapshot(sandbox.path()), before);
    assert!(!sandbox.join("fresh").exists());
}

#[test]
fn init_preview_of_an_existing_project_writes_nothing() {
    e2e_guard!();
    let sandbox = Sandbox::new();
    let empty = without_uv(&sandbox);
    sandbox.write("pyproject.toml", PLAIN);
    let before = snapshot(sandbox.path());
    // The path defaults to the current directory.
    sandbox
        .pyfaro(["init"])
        .env("PATH", &empty)
        .run()
        .success()
        .stdout_eq(PREVIEW_EXISTING)
        .stderr_empty();
    sandbox
        .pyfaro(["init", "--uv", "definitely-missing-uv", "--offline", "."])
        .run()
        .success()
        .stdout_eq(PREVIEW_EXISTING);
    assert_eq!(snapshot(sandbox.path()), before);
}

#[test]
fn init_preview_reports_an_already_configured_project() {
    e2e_guard!();
    let sandbox = Sandbox::new();
    sandbox.write(
        "pyproject.toml",
        &format!("{PLAIN}\n[tool.pyfaro]\nschema-version = 1\n"),
    );
    let before = snapshot(sandbox.path());
    sandbox.pyfaro(["init"]).run().success().stdout_eq(PREVIEW_CONFIGURED);
    assert_eq!(snapshot(sandbox.path()), before);
}

#[test]
fn init_preview_of_inline_tables_shows_the_proposed_pyproject() {
    e2e_guard!();
    let sandbox = Sandbox::new();
    for (original, expected) in [
        (
            format!(
                "{PLAIN}\n[tool]\npyfaro = {{}}\n\n[tool.ruff]\nline-length = 100\n"
            ),
            "pyfaro = { schema-version = 1 }",
        ),
        (
            format!("tool = {{ ruff = {{ line-length = 100 }} }}\n\n{PLAIN}"),
            "schema-version = 1",
        ),
    ] {
        sandbox.write("pyproject.toml", &original);
        let before = snapshot(sandbox.path());
        let out = sandbox.pyfaro(["init"]).run();
        out.success()
            .stdout_contains("  Proposed pyproject.toml:\n")
            .stdout_contains(expected)
            .stdout_contains("line-length = 100")
            .stderr_empty();
        assert!(!out.stdout.contains("\n[tool.pyfaro]\n"), "{}", out.stdout);
        assert_eq!(snapshot(sandbox.path()), before);
        assert_eq!(read(sandbox.join("pyproject.toml")), original);
    }
}

#[test]
fn init_rejects_invalid_configuration_before_writing() {
    e2e_guard!();
    let sandbox = Sandbox::new();
    let cases = [
        (
            "[project]\nname = \"x\"\n\n[tool.pyfaro]\nschema-version = 2\n",
            "tool.pyfaro.schema-version must be the integer 1",
        ),
        (
            "[project]\nname = \"x\"\n\n[tool.pyfaro]\nschema-version = \"1\"\n",
            "tool.pyfaro.schema-version must be the integer 1",
        ),
        (
            "[project]\nname = \"x\"\n\n[tool.pyfaro]\ncolour = \"red\"\n",
            "unknown [tool.pyfaro] setting: colour",
        ),
        (
            "[project]\nname = \"x\"\n\n[tool]\npyfaro = 3\n",
            "[tool.pyfaro] must be a table",
        ),
        (
            "[tool.poetry]\nname = \"x\"\n",
            "expected a standard [project] table in pyproject.toml",
        ),
        ("[project\nname = ", "invalid "),
    ];
    for (index, (source, message)) in cases.iter().enumerate() {
        let dir = format!("case{index}");
        sandbox.write(format!("{dir}/pyproject.toml"), source);
        let before = snapshot(&sandbox.join(&dir));
        for apply in [false, true] {
            let mut command = sandbox.pyfaro(["init", &dir]);
            if apply {
                command = command.arg("--apply");
            }
            command
                .run()
                .code(1)
                .stdout_empty()
                .stderr_contains("error: invalid ")
                .stderr_contains(message);
            assert_eq!(snapshot(&sandbox.join(&dir)), before, "{source:?}");
        }
    }
}

#[test]
fn init_rejects_a_lockfile_without_metadata() {
    e2e_guard!();
    let sandbox = Sandbox::new();
    sandbox.write("orphan/uv.lock", "version = 1\n");
    let before = snapshot(sandbox.path());
    for apply in [false, true] {
        let mut command = sandbox.pyfaro(["init", "orphan"]);
        if apply {
            command = command.arg("--apply");
        }
        command.run().code(1).stdout_empty().stderr_eq(
            "error: uv.lock exists without pyproject.toml; restore project metadata before running init\n");
    }
    assert_eq!(snapshot(sandbox.path()), before);
}

#[test]
fn init_rejects_workspace_members_in_preview_and_apply() {
    e2e_guard!();
    let sandbox = Sandbox::new();
    sandbox.write(
        "ws/pyproject.toml",
        "[tool.uv.workspace]\nmembers = ['member']\n",
    );
    sandbox.write("ws/member/pyproject.toml", PLAIN);
    let before = snapshot(sandbox.path());
    for apply in [false, true] {
        let mut command = sandbox.pyfaro(["init", "ws/member"]);
        if apply {
            command = command.arg("--apply");
        }
        let out = command.run();
        out.code(1)
            .stdout_empty()
            .stderr_contains(
                "error: uv workspaces are not supported by this alpha (found ",
            )
            .stderr_contains("); initialize a standalone project\n");
    }
    // A new project below a workspace root is rejected too.
    sandbox
        .pyfaro(["init", "ws/brand-new"])
        .run()
        .code(1)
        .stderr_contains("uv workspaces are not supported by this alpha");
    assert_eq!(snapshot(sandbox.path()), before);
}

#[test]
fn init_rejects_a_workspace_root_itself() {
    e2e_guard!();
    let sandbox = Sandbox::new();
    sandbox.write(
        "pyproject.toml",
        &format!("{PLAIN}\n[tool.uv.workspace]\nmembers = ['member']\n"),
    );
    let before = snapshot(sandbox.path());
    sandbox
        .pyfaro(["init"])
        .run()
        .code(1)
        .stderr_contains("uv workspaces are not supported by this alpha");
    assert_eq!(snapshot(sandbox.path()), before);
}

#[cfg(unix)]
#[test]
fn init_rejects_a_metadata_symlink() {
    e2e_guard!();
    let sandbox = Sandbox::new();
    sandbox.write("target.toml", PLAIN);
    fs::create_dir(sandbox.join("linked")).unwrap();
    std::os::unix::fs::symlink(
        sandbox.join("target.toml"),
        sandbox.join("linked/pyproject.toml"),
    )
    .unwrap();
    sandbox
        .pyfaro(["init", "linked"])
        .run()
        .code(1)
        .stderr_contains("must be a regular file, not a symlink or directory");
    assert_eq!(read(sandbox.join("target.toml")), PLAIN);
}

#[test]
fn init_usage_errors_exit_2() {
    e2e_guard!();
    let sandbox = Sandbox::new();
    sandbox
        .pyfaro(["init", "one", "two"])
        .run()
        .code(2)
        .stderr_contains("unexpected argument 'two'");
    sandbox
        .pyfaro(["init", "--frobnicate"])
        .run()
        .code(2)
        .stderr_contains("unexpected argument '--frobnicate'");
    assert_eq!(entries(sandbox.path()), 0);
}

// `--apply` is only supported on Linux; elsewhere it is rejected before uv is
// looked up or any file is written.

#[test]
fn init_apply_is_rejected_off_linux_for_new_and_existing_projects() {
    e2e_guard!();
    if cfg!(target_os = "linux") {
        return;
    }
    let sandbox = Sandbox::new();
    sandbox.write("existing/pyproject.toml", PLAIN);
    let before = snapshot(sandbox.path());
    // A missing uv proves the platform check runs first.
    let out = sandbox
        .pyfaro(["init", "fresh", "--apply", "--uv", "missing-uv"])
        .run();
    out.code(1)
        .stderr_eq(UNSUPPORTED_APPLY)
        .stdout_contains("Initialization plan for fresh:\n");
    let out = sandbox.pyfaro(["init", "existing", "--apply"]).run();
    out.code(1).stderr_eq(UNSUPPORTED_APPLY);
    assert_eq!(snapshot(sandbox.path()), before);
    assert!(!sandbox.join("fresh").exists());
}

#[test]
fn init_apply_creates_a_new_project_with_a_lockfile() {
    e2e_guard!();
    if !cfg!(target_os = "linux") {
        return;
    }
    let sandbox = Sandbox::new();
    sandbox
        .pyfaro(["init", "fresh", "--apply"])
        .run()
        .success()
        .stdout_contains("Initialization plan for fresh:\n")
        .stdout_contains("Initialized fresh with Pyfaro and uv.\n");
    let root = sandbox.join("fresh");
    assert_eq!(tree(&root), ["pyproject.toml", "uv.lock"]);
    let metadata = read(root.join("pyproject.toml"));
    assert!(metadata.contains("[project]\nname = \"fresh\""), "{metadata}");
    assert!(metadata.ends_with("\n[tool.pyfaro]\nschema-version = 1\n"));
    assert!(!root.join(".venv").exists());
    assert!(!root.join(".git").exists());
}

#[test]
fn init_apply_preserves_existing_content_and_adds_only_the_pyfaro_section() {
    e2e_guard!();
    if !cfg!(target_os = "linux") {
        return;
    }
    let sandbox = Sandbox::new();
    let original = format!(
        "# keep this comment\n{PLAIN}\n[build-system]\nrequires = [\"uv_build>=0.9.26,<0.10.0\"]\nbuild-backend = \"uv_build\"\n\n[dependency-groups]\ndev = [\"pytest=={PYTEST}\"]  # pinned\n\n[tool.ruff]\nline-length = 100  # wide\nsrc = [\"src\"]\n\n[tool.pytest.ini_options]\ntestpaths = [\"tests\"]\n\n[tool.ty.environment]\npython-version = \"3.12\"\n"
    );
    sandbox.write("keep/pyproject.toml", &original);
    sandbox.write("keep/src/example/__init__.py", "");
    sandbox
        .pyfaro(["init", "keep", "--apply"])
        .run()
        .success()
        .stdout_contains("Initialized keep with Pyfaro and uv.\n");
    let root = sandbox.join("keep");
    assert_eq!(
        read(root.join("pyproject.toml")),
        format!("{original}\n[tool.pyfaro]\nschema-version = 1\n")
    );
    assert!(root.join("uv.lock").is_file());
    assert_eq!(
        tree(&root),
        ["pyproject.toml", "src/example/__init__.py", "uv.lock"]
    );
}

#[test]
fn init_apply_keeps_an_existing_compatible_pyfaro_section_byte_for_byte() {
    e2e_guard!();
    if !cfg!(target_os = "linux") {
        return;
    }
    let sandbox = Sandbox::new();
    let original = format!("{PLAIN}\n[tool.pyfaro]\nschema-version = 1\n");
    sandbox.write("pyproject.toml", &original);
    sandbox.pyfaro(["init", "--apply"]).run().success();
    assert_eq!(read(sandbox.join("pyproject.toml")), original);
    assert!(sandbox.join("uv.lock").is_file());
}

#[test]
fn init_apply_is_idempotent() {
    e2e_guard!();
    if !cfg!(target_os = "linux") {
        return;
    }
    let sandbox = Sandbox::new();
    sandbox.write("pyproject.toml", PLAIN);
    sandbox.pyfaro(["init", "--apply"]).run().success();
    let first = snapshot(sandbox.path());
    assert_eq!(first.keys().collect::<Vec<_>>(), ["pyproject.toml", "uv.lock"]);
    sandbox.pyfaro(["init", "--apply"]).run().success();
    assert_eq!(snapshot(sandbox.path()), first);
    sandbox.pyfaro(["init"]).run().success().stdout_eq(PREVIEW_CONFIGURED);
    assert_eq!(snapshot(sandbox.path()), first);
    sandbox.pyfaro(["init", "--apply", "--offline"]).run().success();
    assert_eq!(snapshot(sandbox.path()), first);
}

#[test]
fn init_apply_offline_initializes_a_dependency_free_project() {
    e2e_guard!();
    if !cfg!(target_os = "linux") {
        return;
    }
    let sandbox = Sandbox::new();
    sandbox
        .pyfaro(["init", "fresh", "--apply", "--offline"])
        .run()
        .success()
        .stdout_contains("Initialized fresh with Pyfaro and uv.\n");
    assert_eq!(tree(&sandbox.join("fresh")), ["pyproject.toml", "uv.lock"]);
    sandbox.write("existing/pyproject.toml", PLAIN);
    sandbox
        .pyfaro(["init", "existing", "--offline", "--apply"])
        .run()
        .success();
    assert_eq!(tree(&sandbox.join("existing")), ["pyproject.toml", "uv.lock"]);
}

#[test]
fn init_apply_offline_with_an_unavailable_dependency_fails_and_restores() {
    e2e_guard!();
    if !cfg!(target_os = "linux") {
        return;
    }
    let sandbox = Sandbox::new();
    // A name that cannot be in any local cache, with no network allowed.
    let original = "[project]\nname = \"example\"\nversion = \"1.0.0\"\nrequires-python = \">=3.10\"\ndependencies = [\"pyfaro-e2e-no-such-package-0xdead\"]\n";
    sandbox.write("pyproject.toml", original);
    sandbox
        .pyfaro(["init", "--apply", "--offline"])
        .run()
        .code(1)
        .stderr_contains("error: uv lock failed (");
    assert_eq!(read(sandbox.join("pyproject.toml")), original);
    assert!(!sandbox.join("uv.lock").exists());
}

#[cfg(target_os = "linux")]
mod linux_uv {
    use super::*;
    use std::os::unix::fs::PermissionsExt;

    /// An executable `/bin/sh` script standing in for uv.
    fn script(sandbox: &Sandbox, name: &str, body: &str) -> PathBuf {
        let uv = std::env::split_paths(&std::env::var_os("PATH").unwrap())
            .map(|dir| dir.join("uv"))
            .find(|candidate| candidate.is_file())
            .expect("uv on PATH");
        let body = body.replace("@UV@", &uv.to_string_lossy());
        let path = sandbox.write(name, &format!("#!/bin/sh\n{body}\n"));
        fs::set_permissions(&path, fs::Permissions::from_mode(0o755)).unwrap();
        path
    }

    fn project(sandbox: &Sandbox) {
        sandbox.write("project/pyproject.toml", PLAIN);
    }

    #[test]
    fn init_uv_selects_the_executable_used_for_every_operation() {
        e2e_guard!();
        let sandbox = Sandbox::new();
        let log = sandbox.join("uv calls.log");
        let wrapper = script(
            &sandbox,
            "wrapped uv",
            &format!(
                "echo \"$@\" >> '{}'\nexec \"@UV@\" \"$@\"",
                log.display()
            ),
        );
        sandbox
            .pyfaro(["init", "fresh", "--apply", "--uv"])
            .arg(&wrapper)
            .run()
            .success();
        assert_eq!(tree(&sandbox.join("fresh")), ["pyproject.toml", "uv.lock"]);
        let calls = read(&log);
        let calls: Vec<&str> = calls.lines().collect();
        let global = "--no-python-downloads --no-progress";
        assert_eq!(calls.len(), 3, "{calls:?}");
        assert_eq!(calls[0], "--version");
        assert!(
            calls[1].starts_with(&format!(
                "{global} init --bare --vcs none --no-workspace"
            )),
            "{calls:?}"
        );
        assert_eq!(calls[2], format!("{global} lock --no-build"));
    }

    #[test]
    fn init_offline_passes_offline_to_uv_and_apply_ignores_uv_overrides() {
        e2e_guard!();
        let sandbox = Sandbox::new();
        let log = sandbox.join("env.log");
        let wrapper = script(
            &sandbox,
            "logging-uv",
            &format!(
                "echo \"$@ | frozen=$UV_FROZEN project=$UV_PROJECT\" >> '{}'\nexec \"@UV@\" \"$@\"",
                log.display()
            ),
        );
        project(&sandbox);
        sandbox
            .pyfaro(["init", "project", "--apply", "--offline", "--uv"])
            .arg(&wrapper)
            .env("UV_FROZEN", "1")
            .env("UV_PROJECT", sandbox.join("elsewhere"))
            .run()
            .success();
        assert!(sandbox.join("project/uv.lock").is_file());
        let calls = read(&log);
        let lock = calls.lines().last().unwrap();
        assert_eq!(
            lock,
            "--no-python-downloads --no-progress --offline lock --no-build | frozen= project="
        );
    }

    #[test]
    fn init_apply_with_a_missing_uv_fails_without_changing_the_project() {
        e2e_guard!();
        let sandbox = Sandbox::new();
        project(&sandbox);
        let before = snapshot(sandbox.path());
        let missing = sandbox.join("missing-uv");
        sandbox
            .pyfaro(["init", "project", "--apply", "--uv"])
            .arg(&missing)
            .run()
            .code(1)
            .stderr_contains("error: could not find uv at ")
            .stderr_contains("missing-uv")
            .stderr_contains("; install uv or use --uv <path>\n");
        sandbox
            .pyfaro(["init", "fresh", "--apply", "--uv", "no-such-uv"])
            .run()
            .code(1)
            .stderr_contains("error: could not find uv at no-such-uv")
            .stderr_contains("install uv or use --uv <path>");
        assert_eq!(snapshot(sandbox.path()), before);
        assert!(!sandbox.join("fresh").exists());
    }

    #[test]
    fn init_apply_with_an_unsupported_uv_fails_without_changing_the_project() {
        e2e_guard!();
        let sandbox = Sandbox::new();
        project(&sandbox);
        let before = snapshot(sandbox.path());
        for version in ["uv 0.9.25", "uv 0.9.26-beta.1", "not uv", "uv 0.8.0"] {
            let old = script(&sandbox, "old-uv", &format!("echo '{version}'"));
            sandbox
                .pyfaro(["init", "project", "--apply", "--uv"])
                .arg(&old)
                .run()
                .code(1)
                .stdout_contains("Initialization plan for project:")
                .stderr_eq(
                    &format!(
                        "error: unsupported uv version: {version}; use uv >=0.9.26 (tested: 0.9.26)\n"
                    ));
            let before = {
                let mut copy = before.clone();
                copy.insert(
                    "old-uv".into(),
                    snapshot(sandbox.path())["old-uv"].clone(),
                );
                copy
            };
            assert_eq!(snapshot(sandbox.path()), before, "{version}");
        }
    }

    #[test]
    fn init_apply_with_a_newer_stable_uv_is_accepted() {
        e2e_guard!();
        let sandbox = Sandbox::new();
        project(&sandbox);
        let wrapper = script(
            &sandbox,
            "newer-uv",
            "if [ \"$1\" = --version ]; then echo 'uv 0.9.99 (fake)'; exit 0; fi\nexec \"@UV@\" \"$@\"",
        );
        sandbox
            .pyfaro(["init", "project", "--apply", "--uv"])
            .arg(&wrapper)
            .run()
            .success();
        assert!(sandbox.join("project/uv.lock").is_file());
    }

    #[test]
    fn init_apply_forwards_a_failing_uv_version_probe_status() {
        e2e_guard!();
        let sandbox = Sandbox::new();
        project(&sandbox);
        let broken = script(
            &sandbox,
            "broken-uv",
            "echo 'interpreter is broken' >&2\nexit 7",
        );
        sandbox
            .pyfaro(["init", "project", "--apply", "--uv"])
            .arg(&broken)
            .run()
            .code(7)
            .stderr_contains("error: uv --version failed for ")
            .stderr_contains("interpreter is broken");
        assert!(!sandbox.join("project/uv.lock").exists());
    }

    #[test]
    fn init_apply_forwards_the_uv_exit_status_and_restores_files() {
        e2e_guard!();
        let sandbox = Sandbox::new();
        project(&sandbox);
        sandbox.write("project/uv.lock", "original lock\n");
        let before = snapshot(sandbox.path());
        let failing = script(
            &sandbox,
            "failing-uv",
            "if [ \"$1\" = --version ]; then echo 'uv 0.9.26'; exit 0; fi\necho 'resolution exploded' >&2\nexit 42",
        );
        let before = {
            let mut copy = before;
            copy.insert(
                "failing-uv".into(),
                snapshot(sandbox.path())["failing-uv"].clone(),
            );
            copy
        };
        sandbox
            .pyfaro(["init", "project", "--apply", "--uv"])
            .arg(&failing)
            .run()
            .code(42)
            .stderr_contains("resolution exploded")
            .stderr_contains("error: uv lock failed (")
            .stderr_contains("see uv's output above");
        assert_eq!(snapshot(sandbox.path()), before);
        assert_eq!(read(sandbox.join("project/uv.lock")), "original lock\n");
        assert_eq!(read(sandbox.join("project/pyproject.toml")), PLAIN);
    }
}

/// Run the installed command through `pyfaro run`; Python prints CRLF on Windows.
fn assert_runs(root: &Path, command: &str, greeting: &str) {
    let out = Cmd::pyfaro(root, ["run", "--locked", "--", command]).run();
    out.success().stderr_empty();
    assert_eq!(out.stdout.replace("\r\n", "\n"), format!("{greeting}\n"));
}
