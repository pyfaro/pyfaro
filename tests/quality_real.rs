//! Quality commands against generated library and CLI projects with the real,
//! pinned Ruff, ty and pytest. These tests need network access to sync the
//! tools, so they only run when `PYFARO_REAL_QUALITY_TOOLS=1` is set.

use std::collections::BTreeMap;
use std::fs;
use std::hash::{DefaultHasher, Hash, Hasher};
use std::path::{Path, PathBuf};
use std::process::{Command, Output};

use tempfile::{TempDir, tempdir};

mod support;

struct Project {
    _sandbox: TempDir,
    root: PathBuf,
    package: &'static str,
}

impl Project {
    fn module(&self, file: &str) -> PathBuf {
        self.root.join("src").join(self.package).join(file)
    }

    fn write(&self, path: &Path, contents: &str) {
        fs::write(path, contents).unwrap();
    }

    fn pyfaro(&self, args: &[&str]) -> Output {
        Command::new(env!("CARGO_BIN_EXE_pyfaro"))
            .current_dir(&self.root)
            .env_remove("VIRTUAL_ENV")
            .env_remove("UV_PROJECT_ENVIRONMENT")
            .args(args)
            .output()
            .unwrap()
    }

    fn assert_code(&self, args: &[&str], expected: i32) {
        let output = self.pyfaro(args);
        assert_eq!(
            output.status.code(),
            Some(expected),
            "pyfaro {args:?}\nstdout:\n{}\nstderr:\n{}",
            String::from_utf8_lossy(&output.stdout),
            String::from_utf8_lossy(&output.stderr),
        );
    }
}

/// Generate a project with `pyfaro create`, then `pyfaro sync` it.
fn generate(kind: &str) -> Project {
    let (name, package) = match kind {
        "lib" => ("acme-widget", "acme_widget"),
        _ => ("acme-tools", "acme_tools"),
    };
    let sandbox = tempdir().unwrap();
    let root = sandbox.path().join("project");
    let created = Command::new(env!("CARGO_BIN_EXE_pyfaro"))
        .args(["create", kind, name, "--path"])
        .arg(&root)
        .output()
        .unwrap();
    assert!(created.status.success(), "{created:?}");
    let project = Project { _sandbox: sandbox, root, package };
    let synced = Command::new(env!("CARGO_BIN_EXE_pyfaro"))
        .arg("sync")
        .arg(&project.root)
        .env_remove("VIRTUAL_ENV")
        .env_remove("UV_PROJECT_ENVIRONMENT")
        .output()
        .unwrap();
    assert!(synced.status.success(), "{synced:?}");
    project
}

fn for_each_kind(run: impl Fn(&Project)) {
    if !support::real_quality_tools() {
        return;
    }
    for kind in ["lib", "cli"] {
        run(&generate(kind));
    }
}

/// Hash every project file except environments and tool caches.
fn snapshot(root: &Path) -> BTreeMap<PathBuf, u64> {
    fn walk(root: &Path, directory: &Path, files: &mut BTreeMap<PathBuf, u64>) {
        for entry in fs::read_dir(directory).unwrap() {
            let entry = entry.unwrap();
            let path = entry.path();
            let name = entry.file_name();
            if path.is_dir() {
                if matches!(
                    name.to_str(),
                    Some(
                        ".venv"
                            | ".ruff_cache"
                            | "__pycache__"
                            | ".pytest_cache"
                    )
                ) {
                    continue;
                }
                walk(root, &path, files);
            } else {
                let mut hasher = DefaultHasher::new();
                fs::read(&path).unwrap().hash(&mut hasher);
                files.insert(
                    path.strip_prefix(root).unwrap().to_path_buf(),
                    hasher.finish(),
                );
            }
        }
    }
    let mut files = BTreeMap::new();
    walk(root, root, &mut files);
    files
}

#[test]
fn success_exits_zero_for_every_command() {
    for_each_kind(|project| {
        for args in [
            vec!["check"],
            vec!["lint"],
            vec!["fmt", "--check"],
            vec!["type"],
            vec!["test"],
        ] {
            project.assert_code(&args, 0);
        }
    });
}

#[test]
fn a_lint_violation_is_forwarded_and_isolated() {
    for_each_kind(|project| {
        project.write(&project.module("extra.py"), "import os\n");
        project.assert_code(&["lint"], 1);
        project.assert_code(&["check"], 1);
        project.assert_code(&["fmt", "--check"], 0);
        project.assert_code(&["type"], 0);
        project.assert_code(&["test"], 0);
    });
}

#[test]
fn a_format_issue_is_forwarded_and_isolated() {
    for_each_kind(|project| {
        project.write(&project.module("extra.py"), "VALUE=1\n");
        project.assert_code(&["fmt", "--check"], 1);
        project.assert_code(&["check"], 1);
        project.assert_code(&["lint"], 0);
        project.assert_code(&["type"], 0);
        project.assert_code(&["test"], 0);
    });
}

#[test]
fn a_type_error_is_forwarded_and_isolated() {
    for_each_kind(|project| {
        project.write(&project.module("extra.py"), "VALUE: int = \"text\"\n");
        project.assert_code(&["type"], 1);
        project.assert_code(&["check"], 1);
        project.assert_code(&["lint"], 0);
        project.assert_code(&["fmt", "--check"], 0);
        project.assert_code(&["test"], 0);
    });
}

#[test]
fn test_failures_and_empty_selection_forward_pytest_codes() {
    for_each_kind(|project| {
        project.assert_code(&["test", "-k", "no_such_test"], 5);
        project.write(
            &project.root.join("tests/test_failure.py"),
            "def test_failure() -> None:\n    assert 1 + 1 == 3\n",
        );
        project.assert_code(&["test"], 1);
        project.assert_code(&["check"], 0);
        project.assert_code(&["lint"], 0);
        project.assert_code(&["type"], 0);
        project.assert_code(&["fmt", "--check"], 0);
    });
}

#[test]
fn excluded_files_are_skipped_even_when_named() {
    for_each_kind(|project| {
        let relative = format!("src/{}/excluded.py", project.package);
        // Violates both lint (unused import) and format (spacing).
        project.write(&project.module("excluded.py"), "import os\nVALUE=1\n");
        let manifest = project.root.join("pyproject.toml");
        let text = fs::read_to_string(&manifest).unwrap();
        let text = text.replace(
            "line-length = 88\n",
            &format!("line-length = 88\nextend-exclude = [\"{relative}\"]\n"),
        );
        project.write(&manifest, &text);

        project.assert_code(&["lint"], 0);
        project.assert_code(&["lint", &relative], 0);
        project.assert_code(&["fmt", "--check", &relative], 0);
        project.assert_code(&["check"], 0);
        project.assert_code(&["check", &relative], 0);

        // The same content in a non-excluded file fails.
        project.write(&project.module("included.py"), "import os\nVALUE=1\n");
        project.assert_code(&["lint"], 1);
        project.assert_code(&["check"], 1);
    });
}

#[test]
fn quality_commands_write_nothing_by_default() {
    for_each_kind(|project| {
        project.write(&project.module("lint_bad.py"), "import os\n");
        project.write(&project.module("fmt_bad.py"), "VALUE=1\n");
        project.write(&project.module("type_bad.py"), "VALUE: int = \"x\"\n");
        project.write(
            &project.root.join("tests/test_failure.py"),
            "def test_failure() -> None:\n    assert 1 + 1 == 3\n",
        );
        let before = snapshot(&project.root);
        assert!(before.keys().any(|path| path.ends_with("uv.lock")));
        for args in [
            vec!["check"],
            vec!["lint"],
            vec!["type"],
            vec!["test"],
            vec!["fmt", "--check"],
        ] {
            // Each command reports its own failure and still writes nothing.
            project.assert_code(&args, 1);
            assert_eq!(snapshot(&project.root), before, "pyfaro {args:?}");
        }
    });
}

#[test]
fn check_fix_is_the_only_way_to_apply_fixes() {
    for_each_kind(|project| {
        let lint = project.module("lint_fixable.py");
        let format = project.module("format_fixable.py");
        project.write(&lint, "import os\n\nVALUE = 1\n");
        project.write(&format, "OTHER=2\n");
        let before = snapshot(&project.root);

        project.assert_code(&["check"], 1);
        assert_eq!(snapshot(&project.root), before, "plain check wrote");

        project.assert_code(&["check", "--fix"], 0);
        let after = snapshot(&project.root);
        assert_ne!(after, before);
        assert!(!fs::read_to_string(&lint).unwrap().contains("import os"));
        assert_eq!(fs::read_to_string(&format).unwrap(), "OTHER = 2\n");
        project.assert_code(&["check"], 0);
    });
}

#[test]
fn check_fix_still_fails_for_what_it_cannot_fix() {
    for_each_kind(|project| {
        project.write(&project.module("type_bad.py"), "VALUE: int = \"x\"\n");
        project.write(&project.module("fmt_bad.py"), "OTHER=2\n");
        project.assert_code(&["check", "--fix"], 1);
        assert_eq!(
            fs::read_to_string(project.module("fmt_bad.py")).unwrap(),
            "OTHER = 2\n"
        );
    });
}
