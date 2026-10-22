//! Shared end-to-end harness.
//!
//! ```ignore
//! #[test]
//! fn sync_creates_the_environment() {
//!     e2e_guard!();
//!     let sandbox = Sandbox::new();
//!     let project = sandbox.project(Kind::Lib); // create only
//!     project.pyfaro(&["sync"]).run().success();
//! }
//! ```
//!
//! Everything runs inside a [`Sandbox`] whose path contains a space and a
//! non-ASCII character, with an isolated environment (no inherited
//! `VIRTUAL_ENV`, `UV_PROJECT_ENVIRONMENT` or user Ruff/ty configuration, and a
//! sandbox-local `UV_PYTHON_INSTALL_DIR`). The shared uv cache is kept for speed.

#![allow(dead_code)]

use std::collections::BTreeMap;
use std::ffi::OsStr;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

use sha2::{Digest, Sha256};
use tempfile::TempDir;

/// Versions the suite is tested against.
pub const UV: &str = "0.9.26";
pub const RUFF: &str = "0.16.10";
pub const TY: &str = "0.0.85";
pub const PYTEST: &str = "9.1.1";

/// True when `PYFARO_E2E=1`.
pub fn enabled() -> bool {
    std::env::var("PYFARO_E2E").as_deref() == Ok("1")
}

/// Skip the current test with a printed note unless `PYFARO_E2E=1`, then
/// check that the uv on PATH is the tested version.
macro_rules! e2e_guard {
    () => {
        if !$crate::harness::enabled() {
            eprintln!("skipped: set PYFARO_E2E=1 to run end-to-end tests");
            return;
        }
        $crate::harness::assert_tested_uv();
    };
}

/// Assert the uv on PATH is [`UV`].
pub fn assert_tested_uv() {
    let output = Command::new("uv")
        .arg("--version")
        .output()
        .expect("uv must be on PATH");
    let text = String::from_utf8_lossy(&output.stdout);
    assert!(
        text.split_whitespace().nth(1) == Some(UV),
        "e2e tests need uv {UV}, found: {text}"
    );
}

/// Which `pyfaro create` preset to generate.
#[derive(Clone, Copy)]
pub enum Kind {
    Lib,
    Cli,
}

/// A disposable directory (`pyfaro e2é <random>`), removed on drop.
pub struct Sandbox {
    dir: TempDir,
}

impl Sandbox {
    pub fn new() -> Self {
        let dir = tempfile::Builder::new()
            .prefix("pyfaro e2é ")
            .tempdir()
            .expect("create sandbox");
        Self { dir }
    }

    pub fn path(&self) -> &Path {
        self.dir.path()
    }

    pub fn join(&self, relative: impl AsRef<Path>) -> PathBuf {
        self.path().join(relative)
    }

    /// Write a file (creating parent directories) relative to the sandbox.
    pub fn write(&self, relative: impl AsRef<Path>, contents: &str) -> PathBuf {
        write_file(&self.join(relative), contents)
    }

    /// A `pyfaro` invocation with cwd = the sandbox root.
    pub fn pyfaro<I, S>(&self, args: I) -> Cmd
    where
        I: IntoIterator<Item = S>,
        S: AsRef<OsStr>,
    {
        Cmd::pyfaro(self.path(), args)
    }

    /// A direct `uv` invocation with cwd = the sandbox root.
    pub fn uv<I, S>(&self, args: I) -> Cmd
    where
        I: IntoIterator<Item = S>,
        S: AsRef<OsStr>,
    {
        Cmd::uv(self.path(), args)
    }

    /// Generate a project with `pyfaro create` (not synced) in
    /// `<sandbox>/project`.
    pub fn project(&self, kind: Kind) -> Project {
        let (kind_arg, name, package) = match kind {
            Kind::Lib => ("lib", "acme-widget", "acme_widget"),
            Kind::Cli => ("cli", "acme-tools", "acme_tools"),
        };
        let root = self.join("project");
        self.pyfaro(["create", kind_arg, name, "--python", "3.12", "--path"])
            .arg(&root)
            .run()
            .success();
        Project { root, name, package }
    }

    /// [`Sandbox::project`] followed by `pyfaro sync`, which installs the
    /// pinned Ruff, ty and pytest into `.venv`.
    pub fn synced_project(&self, kind: Kind) -> Project {
        let project = self.project(kind);
        project.pyfaro(["sync"]).run().success();
        project
    }
}

/// A generated project; `pyfaro`/`uv` run with cwd = the project root.
pub struct Project {
    pub root: PathBuf,
    /// Distribution name, e.g. `acme-widget`.
    pub name: &'static str,
    /// Import package, e.g. `acme_widget`.
    pub package: &'static str,
}

impl Project {
    pub fn path(&self, relative: impl AsRef<Path>) -> PathBuf {
        self.root.join(relative)
    }

    /// `src/<package>/<file>`.
    pub fn module(&self, file: &str) -> PathBuf {
        self.root.join("src").join(self.package).join(file)
    }

    /// Write a file (creating parent directories) relative to the root.
    pub fn write(&self, relative: impl AsRef<Path>, contents: &str) -> PathBuf {
        write_file(&self.path(relative), contents)
    }

    pub fn read(&self, relative: impl AsRef<Path>) -> String {
        fs::read_to_string(self.path(relative)).expect("read project file")
    }

    pub fn pyfaro<I, S>(&self, args: I) -> Cmd
    where
        I: IntoIterator<Item = S>,
        S: AsRef<OsStr>,
    {
        Cmd::pyfaro(&self.root, args)
    }

    pub fn uv<I, S>(&self, args: I) -> Cmd
    where
        I: IntoIterator<Item = S>,
        S: AsRef<OsStr>,
    {
        Cmd::uv(&self.root, args)
    }

    /// Install exact tool versions into `.venv` with
    /// `uv pip install name==version`, e.g. `[("ruff", RUFF)]`.
    pub fn install_tools(&self, tools: &[(&str, &str)]) {
        let specs: Vec<String> =
            tools.iter().map(|(n, v)| format!("{n}=={v}")).collect();
        self.uv(["pip", "install", "--python"])
            .arg(self.root.join(".venv"))
            .args(&specs)
            .run()
            .success();
    }

    /// Hash of every project file except environments and caches.
    pub fn snapshot(&self) -> Snapshot {
        snapshot(&self.root)
    }
}

fn write_file(path: &Path, contents: &str) -> PathBuf {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).expect("create parent directories");
    }
    fs::write(path, contents).expect("write file");
    path.to_path_buf()
}

/// Builder for one process run.
pub struct Cmd {
    command: Command,
    description: String,
    stdin: Option<Vec<u8>>,
}

impl Cmd {
    /// The real built binary, with the isolated environment applied.
    pub fn pyfaro<I, S>(cwd: &Path, args: I) -> Self
    where
        I: IntoIterator<Item = S>,
        S: AsRef<OsStr>,
    {
        Self::new(env!("CARGO_BIN_EXE_pyfaro"), cwd, args)
    }

    /// `uv` from PATH, with the isolated environment applied.
    pub fn uv<I, S>(cwd: &Path, args: I) -> Self
    where
        I: IntoIterator<Item = S>,
        S: AsRef<OsStr>,
    {
        Self::new("uv", cwd, args)
    }

    fn new<I, S>(program: &str, cwd: &Path, args: I) -> Self
    where
        I: IntoIterator<Item = S>,
        S: AsRef<OsStr>,
    {
        let mut command = Command::new(program);
        command.current_dir(cwd);
        for key in [
            "VIRTUAL_ENV",
            "UV_PROJECT_ENVIRONMENT",
            "UV_PYTHON",
            "UV_NO_CONFIG",
            "RUFF_CONFIG",
            "PYTHONPATH",
        ] {
            command.env_remove(key);
        }
        // Sandboxed tool config and Python installs; the uv cache stays shared.
        let sandbox = cwd
            .ancestors()
            .find(|p| {
                p.file_name().is_some_and(|n| {
                    n.to_string_lossy().starts_with("pyfaro e2é ")
                })
            })
            .unwrap_or(cwd);
        command.env("UV_PYTHON_INSTALL_DIR", sandbox.join("python"));
        command.env("XDG_CONFIG_HOME", sandbox.join("config"));
        command.env("APPDATA", sandbox.join("config"));
        command.args(args);
        let description = format!("{program} (cwd {})", cwd.display());
        Self { command, description, stdin: None }
    }

    pub fn arg(mut self, arg: impl AsRef<OsStr>) -> Self {
        self.command.arg(arg);
        self
    }

    pub fn args<I, S>(mut self, args: I) -> Self
    where
        I: IntoIterator<Item = S>,
        S: AsRef<OsStr>,
    {
        self.command.args(args);
        self
    }

    pub fn cwd(mut self, dir: impl AsRef<Path>) -> Self {
        self.command.current_dir(dir);
        self
    }

    pub fn env(
        mut self,
        key: impl AsRef<OsStr>,
        value: impl AsRef<OsStr>,
    ) -> Self {
        self.command.env(key, value);
        self
    }

    pub fn env_remove(mut self, key: impl AsRef<OsStr>) -> Self {
        self.command.env_remove(key);
        self
    }

    /// Pipe these bytes to the process's standard input (default: null).
    pub fn stdin(mut self, input: impl Into<Vec<u8>>) -> Self {
        self.stdin = Some(input.into());
        self
    }

    pub fn run(mut self) -> Outcome {
        let output = match self.stdin.take() {
            None => self.command.output().expect("start process"),
            Some(input) => {
                use std::io::Write;
                use std::process::Stdio;
                let mut child = self
                    .command
                    .stdin(Stdio::piped())
                    .stdout(Stdio::piped())
                    .stderr(Stdio::piped())
                    .spawn()
                    .expect("start process");
                // A process that exits without reading closes the pipe.
                let _ = child.stdin.take().unwrap().write_all(&input);
                child.wait_with_output().expect("wait for process")
            }
        };
        Outcome {
            description: format!(
                "{} {:?}",
                self.description,
                self.command.get_args().collect::<Vec<_>>()
            ),
            code: output.status.code(),
            stdout: String::from_utf8_lossy(&output.stdout).into_owned(),
            stderr: String::from_utf8_lossy(&output.stderr).into_owned(),
        }
    }
}

/// Result of a run. Assertion helpers return `&Self` so they chain.
pub struct Outcome {
    description: String,
    /// `None` when the process was killed by a signal.
    pub code: Option<i32>,
    pub stdout: String,
    pub stderr: String,
}

impl Outcome {
    fn fail(&self, message: &str) -> ! {
        panic!(
            "{message}\ncommand: {}\nexit code: {:?}\nstdout:\n{}\nstderr:\n{}",
            self.description, self.code, self.stdout, self.stderr
        )
    }

    pub fn code(&self, expected: i32) -> &Self {
        if self.code != Some(expected) {
            self.fail(&format!("expected exit code {expected}"));
        }
        self
    }

    pub fn success(&self) -> &Self {
        self.code(0)
    }

    pub fn stdout_contains(&self, needle: &str) -> &Self {
        if !self.stdout.contains(needle) {
            self.fail(&format!("stdout does not contain {needle:?}"));
        }
        self
    }

    pub fn stderr_contains(&self, needle: &str) -> &Self {
        if !self.stderr.contains(needle) {
            self.fail(&format!("stderr does not contain {needle:?}"));
        }
        self
    }

    pub fn stdout_eq(&self, expected: &str) -> &Self {
        if self.stdout != expected {
            self.fail(&format!("stdout is not exactly {expected:?}"));
        }
        self
    }

    pub fn stderr_eq(&self, expected: &str) -> &Self {
        if self.stderr != expected {
            self.fail(&format!("stderr is not exactly {expected:?}"));
        }
        self
    }

    /// Like [`Outcome::stdout_eq`] but accepts CRLF line endings, which
    /// Python's text-mode stdout produces on Windows.
    pub fn stdout_text_eq(&self, expected: &str) -> &Self {
        if self.stdout.replace("\r\n", "\n") != expected {
            self.fail(&format!("stdout is not exactly {expected:?}"));
        }
        self
    }

    pub fn stdout_empty(&self) -> &Self {
        self.stdout_eq("")
    }

    pub fn stderr_empty(&self) -> &Self {
        if !self.stderr.is_empty() {
            self.fail("stderr is not empty");
        }
        self
    }
}

/// Relative path (with `/` separators) to SHA-256 hex digest.
pub type Snapshot = BTreeMap<String, String>;

/// Hash every file under `root`, skipping `.venv`, tool caches and
/// `__pycache__`. Compare two snapshots to assert a command wrote nothing.
pub fn snapshot(root: &Path) -> Snapshot {
    fn walk(root: &Path, dir: &Path, files: &mut Snapshot) {
        for entry in fs::read_dir(dir).expect("read directory") {
            let path = entry.expect("directory entry").path();
            if path.is_dir() {
                let name = path.file_name().unwrap().to_string_lossy();
                if matches!(
                    name.as_ref(),
                    ".venv"
                        | ".ruff_cache"
                        | ".pytest_cache"
                        | ".mypy_cache"
                        | "__pycache__"
                ) {
                    continue;
                }
                walk(root, &path, files);
            } else {
                let digest =
                    Sha256::digest(fs::read(&path).expect("read file"));
                let relative = path
                    .strip_prefix(root)
                    .unwrap()
                    .components()
                    .map(|c| c.as_os_str().to_string_lossy().into_owned())
                    .collect::<Vec<_>>()
                    .join("/");
                files.insert(relative, format!("{digest:x}"));
            }
        }
    }
    let mut files = Snapshot::new();
    walk(root, root, &mut files);
    files
}
