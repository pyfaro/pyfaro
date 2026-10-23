use std::ffi::{OsStr, OsString};
use std::fs;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::sync::atomic::AtomicBool;

use semver::Version;

use crate::error::{Error, Result};
use crate::process;

pub(crate) struct Uv {
    binary: PathBuf,
    offline: bool,
}

impl Uv {
    pub(crate) fn discover(
        binary: OsString,
        offline: bool,
        cancelled: &AtomicBool,
    ) -> Result<Self> {
        let (binary, _) = Self::inspect(binary, cancelled)?;
        Ok(Self { binary, offline })
    }

    pub(crate) fn inspect(
        binary: OsString,
        cancelled: &AtomicBool,
    ) -> Result<(PathBuf, String)> {
        let selected = PathBuf::from(binary);
        let binary = which::which(&selected)
            .map_err(|source| Error::UvLookup { path: selected, source })?;
        let binary = fs::canonicalize(&binary).map_err(|source| {
            Error::io("resolve uv executable", &binary, source)
        })?;
        #[cfg(windows)]
        if binary.extension().is_some_and(|extension| {
            extension.eq_ignore_ascii_case("cmd")
                || extension.eq_ignore_ascii_case("bat")
        }) {
            return Err(Error::Project(
                "uv must be a native executable, not a Windows batch wrapper"
                    .into(),
            ));
        }
        let output = tempfile::NamedTempFile::new().map_err(|error| {
            Error::io(
                "create uv version output",
                Path::new("temporary file"),
                error,
            )
        })?;
        let file = output
            .reopen()
            .map_err(|error| Error::io("open", output.path(), error))?;
        let stderr = tempfile::NamedTempFile::new().map_err(|error| {
            Error::io(
                "create uv error output",
                Path::new("temporary file"),
                error,
            )
        })?;
        let error_file = stderr
            .reopen()
            .map_err(|error| Error::io("open", stderr.path(), error))?;
        let mut command = Command::new(&binary);
        let cwd = std::env::current_dir().map_err(|error| {
            Error::io("resolve", Path::new("current directory"), error)
        })?;
        command
            .current_dir(cwd)
            .arg("--version")
            .stdin(Stdio::null())
            .stdout(file)
            .stderr(error_file);
        let status = process::run(command, cancelled)?;
        if !status.success() {
            return Err(Error::UvProbe {
                path: binary,
                status,
                stderr: process::read_diagnostic(stderr.path())?,
            });
        }
        let version = fs::read_to_string(output.path())
            .map_err(|error| Error::io("read", output.path(), error))?;
        let version = version.lines().next().unwrap_or_default().trim();
        validate_version(version)?;
        Ok((binary, version.to_owned()))
    }

    pub(crate) fn lock(
        &self,
        root: &Path,
        cancelled: &AtomicBool,
    ) -> Result<()> {
        self.execute(root, "lock", &["lock", "--no-build"], cancelled)
    }

    pub(crate) fn init(
        &self,
        root: &Path,
        cancelled: &AtomicBool,
    ) -> Result<()> {
        self.execute(
            root,
            "init",
            &[
                "init",
                "--bare",
                "--vcs",
                "none",
                "--no-workspace",
                "--no-pin-python",
                "--no-readme",
                "--author-from",
                "none",
                "--no-config",
                "--no-package",
            ],
            cancelled,
        )
    }

    pub(crate) fn execute<A: AsRef<OsStr>>(
        &self,
        root: &Path,
        operation: &'static str,
        args: &[A],
        cancelled: &AtomicBool,
    ) -> Result<()> {
        self.execute_with_input(root, operation, args, cancelled, false, false)
    }

    pub(crate) fn execute_dependency<A: AsRef<OsStr>>(
        &self,
        root: &Path,
        operation: &'static str,
        args: &[A],
        cancelled: &AtomicBool,
    ) -> Result<()> {
        self.execute_with_input(root, operation, args, cancelled, false, true)
    }

    pub(crate) fn execute_python<A: AsRef<OsStr>>(
        &self,
        root: &Path,
        operation: &'static str,
        args: &[A],
        downloads: bool,
        cancelled: &AtomicBool,
    ) -> Result<()> {
        let mut command = self.command(root, !downloads);
        command
            .env_remove("UV_NO_PROJECT")
            .env_remove("UV_NO_CONFIG")
            .env_remove("UV_PYTHON")
            .args(args);
        self.run_command(command, operation, cancelled, false)
    }

    pub(crate) fn execute_interactive<A: AsRef<OsStr>>(
        &self,
        root: &Path,
        operation: &'static str,
        args: &[A],
        cancelled: &AtomicBool,
    ) -> Result<()> {
        self.execute_with_input(root, operation, args, cancelled, true, false)
    }

    /// Like [`Uv::execute_interactive`], with an explicit working directory
    /// and extra environment variables for the uv process (and so the child).
    pub(crate) fn execute_task<A: AsRef<OsStr>>(
        &self,
        cwd: &Path,
        env: &[(OsString, OsString)],
        operation: &'static str,
        args: &[A],
        cancelled: &AtomicBool,
    ) -> Result<()> {
        let mut command = self.command(cwd, true);
        command.envs(env.iter().map(|(key, value)| (key, value)));
        command.args(args);
        self.run_command(command, operation, cancelled, true)
    }

    fn execute_with_input<A: AsRef<OsStr>>(
        &self,
        root: &Path,
        operation: &'static str,
        args: &[A],
        cancelled: &AtomicBool,
        interactive: bool,
        ignore_dev_env: bool,
    ) -> Result<()> {
        let mut command = self.command(root, true);
        if ignore_dev_env {
            command.env_remove("UV_DEV");
        }
        command.args(args);
        self.run_command(command, operation, cancelled, interactive)
    }

    fn run_command(
        &self,
        command: Command,
        operation: &'static str,
        cancelled: &AtomicBool,
        interactive: bool,
    ) -> Result<()> {
        let status = if interactive {
            process::run_interactive(command, cancelled)?
        } else {
            process::run(command, cancelled)?
        };
        if status.success() {
            Ok(())
        } else {
            Err(Error::UvStatus { operation, status })
        }
    }

    fn command(&self, root: &Path, no_python_downloads: bool) -> Command {
        let mut command = Command::new(&self.binary);
        command
            .current_dir(root)
            .stdin(Stdio::null())
            .env_remove("UV_WORKING_DIR")
            .env_remove("UV_PROJECT")
            .env_remove("UV_FROZEN")
            .env_remove("UV_LOCKED")
            .env_remove("UV_NO_SYNC")
            .env_remove("UV_ISOLATED")
            .env_remove("UV_INIT_BUILD_BACKEND");
        if no_python_downloads {
            command.arg("--no-python-downloads");
        }
        command.arg("--no-progress");
        if self.offline {
            command.arg("--offline");
        }
        command
    }
}

fn validate_version(output: &str) -> Result<()> {
    let version = output
        .strip_prefix("uv ")
        .and_then(|rest| rest.split_whitespace().next())
        .and_then(|number| Version::parse(number).ok());
    match version {
        Some(version)
            if version >= Version::new(0, 9, 26) && version.pre.is_empty() =>
        {
            Ok(())
        }
        _ => Err(Error::UvVersion(output.trim().to_owned())),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_support;
    use std::sync::atomic::AtomicBool;

    #[test]
    fn adapter_preserves_os_arguments_and_delegated_status() {
        let directory = tempfile::tempdir().unwrap();
        let root = directory.path().join("project with spaces");
        fs::create_dir(&root).unwrap();
        let cancelled = AtomicBool::new(false);
        let uv = Uv::discover(
            test_support::fixture_uv().as_os_str().to_owned(),
            true,
            &cancelled,
        )
        .unwrap();
        let args = [
            OsString::from("record"),
            OsString::from("--"),
            OsString::from("a b"),
            OsString::from("-n"),
            OsString::from("café"),
        ];
        let result = uv.execute(&root, "record", &args, &cancelled);
        assert!(
            matches!(result, Err(Error::UvStatus { status, .. }) if status.code() == Some(37))
        );
        let recorded =
            fs::read_to_string(root.join("uv-arguments.txt")).unwrap();
        assert_eq!(
            recorded.lines().collect::<Vec<_>>(),
            [
                "--no-python-downloads",
                "--no-progress",
                "--offline",
                "record",
                "--",
                "a b",
                "-n",
                "café"
            ]
        );
    }

    #[cfg(windows)]
    #[test]
    fn windows_batch_wrapper_is_rejected_before_launch() {
        let directory = tempfile::tempdir().unwrap();
        let wrapper = directory.path().join("uv.cmd");
        fs::write(&wrapper, "@echo off\r\nexit /b 0\r\n").unwrap();
        let result = Uv::discover(
            wrapper.as_os_str().to_owned(),
            false,
            &AtomicBool::new(false),
        );
        assert!(matches!(result, Err(Error::Project(_))));

        let trailing_dot = OsString::from(format!("{}.", wrapper.display()));
        let result = Uv::discover(trailing_dot, false, &AtomicBool::new(false));
        assert!(matches!(
            result,
            Err(Error::Project(_) | Error::UvLookup { .. })
        ));
    }
}
