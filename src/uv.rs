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
        let selected = PathBuf::from(binary);
        let binary = which::which(&selected)
            .map_err(|source| Error::UvLookup { path: selected, source })?;
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
        let uv = Self { binary, offline };
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
        let mut command = Command::new(&uv.binary);
        let cwd = std::env::current_dir().map_err(|error| {
            Error::io("resolve", Path::new("current directory"), error)
        })?;
        command
            .current_dir(cwd)
            .arg("--version")
            .stdin(Stdio::null())
            .stdout(file);
        let status = process::run(command, cancelled)?;
        if !status.success() {
            return Err(Error::UvStatus { operation: "--version", status });
        }
        let version = fs::read_to_string(output.path())
            .map_err(|error| Error::io("read", output.path(), error))?;
        validate_version(&version)?;
        Ok(uv)
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
        let mut command = self.command(root);
        command.args(args);
        let status = process::run(command, cancelled)?;
        if status.success() {
            Ok(())
        } else {
            Err(Error::UvStatus { operation, status })
        }
    }

    fn command(&self, root: &Path) -> Command {
        let mut command = Command::new(&self.binary);
        command
            .current_dir(root)
            .stdin(Stdio::null())
            .env_remove("UV_WORKING_DIR")
            .env_remove("UV_PROJECT")
            .env_remove("UV_FROZEN")
            .env_remove("UV_LOCKED")
            .env_remove("UV_NO_SYNC")
            .env_remove("UV_INIT_BUILD_BACKEND")
            .args(["--no-python-downloads", "--no-progress"]);
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
            wrapper.into_os_string(),
            false,
            &AtomicBool::new(false),
        );
        assert!(matches!(result, Err(Error::Project(_))));
    }
}
