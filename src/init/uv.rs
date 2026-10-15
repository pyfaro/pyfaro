use std::ffi::OsString;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::sync::atomic::AtomicBool;

use semver::Version;

use crate::error::{Error, Result};
use crate::process;

pub(super) struct Uv {
    binary: PathBuf,
    offline: bool,
}

impl Uv {
    pub fn discover(
        binary: OsString,
        offline: bool,
        cancelled: &AtomicBool,
    ) -> Result<Self> {
        let selected = PathBuf::from(binary);
        let binary = which::which(&selected)
            .map_err(|source| Error::UvLookup { path: selected, source })?;
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
        let status = process::run(&mut command, cancelled)?;
        if !status.success() {
            return Err(Error::UvStatus { operation: "--version", status });
        }
        let version = fs::read_to_string(output.path())
            .map_err(|error| Error::io("read", output.path(), error))?;
        validate_version(&version)?;
        Ok(uv)
    }

    pub fn lock(&self, root: &Path, cancelled: &AtomicBool) -> Result<()> {
        let mut command = self.command(root);
        command.args(["lock", "--no-build"]);
        let status = process::run(&mut command, cancelled)?;
        if status.success() {
            Ok(())
        } else {
            Err(Error::UvStatus { operation: "lock", status })
        }
    }

    pub fn init(&self, root: &Path, cancelled: &AtomicBool) -> Result<()> {
        let mut command = self.command(root);
        command.args([
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
        ]);
        let status = process::run(&mut command, cancelled)?;
        if status.success() {
            Ok(())
        } else {
            Err(Error::UvStatus { operation: "init", status })
        }
    }

    fn command(&self, root: &Path) -> Command {
        let mut command = Command::new(&self.binary);
        command
            .current_dir(root)
            .env_remove("UV_WORKING_DIR")
            .env_remove("UV_PROJECT")
            .env_remove("UV_FROZEN")
            .env_remove("UV_INIT_BUILD_BACKEND")
            .args(["--no-python-downloads", "--no-progress"])
            .stdin(Stdio::null());
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
