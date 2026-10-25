use std::ffi::OsString;
use std::path::Path;

use crate::cli::{PythonArgs, PythonCommand};
use crate::config::Config;
use crate::error::{Error, Result};
use crate::{process, project, uv};

pub(crate) fn run(args: PythonArgs) -> Result<()> {
    let root = if let Some(path) = args.project.as_deref() {
        let root = project::select(Some(path))?;
        Config::load(&root)?;
        root
    } else {
        std::env::current_dir().map_err(|source| {
            Error::io("resolve working directory", Path::new("."), source)
        })?
    };
    crate::verbose::project_root(&root);
    let cancelled = process::cancellation()?;
    let uv = uv::Uv::discover(args.uv, args.offline, &cancelled)?;
    let (operation, downloads, command) = match args.command {
        PythonCommand::List { request, only_installed, all_versions } => {
            let mut command =
                vec![OsString::from("python"), OsString::from("list")];
            if only_installed {
                command.push(OsString::from("--only-installed"));
            }
            if all_versions {
                command.push(OsString::from("--all-versions"));
            }
            if let Some(request) = request {
                command.push(OsString::from("--"));
                command.push(request);
            }
            ("python list", true, command)
        }
        PythonCommand::Install { targets } => {
            let mut command =
                vec![OsString::from("python"), OsString::from("install")];
            if !targets.is_empty() {
                command.push(OsString::from("--"));
                command.extend(targets);
            }
            ("python install", true, command)
        }
        PythonCommand::Pin { request, rm } => {
            let mut command =
                vec![OsString::from("python"), OsString::from("pin")];
            if rm {
                command.push(OsString::from("--rm"));
            }
            if let Some(request) = request {
                command.push(OsString::from("--"));
                command.push(request);
            }
            ("python pin", false, command)
        }
    };
    uv.execute_python(&root, operation, &command, downloads, &cancelled)
}
