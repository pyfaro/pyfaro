use std::ffi::OsString;
use std::fs;

use crate::cli::RunArgs;
use crate::error::{Error, Result};
use crate::{process, uv};

pub(crate) fn run(args: RunArgs) -> Result<()> {
    let root = fs::canonicalize(&args.project).map_err(|source| {
        Error::io("resolve project directory", &args.project, source)
    })?;
    if !root.is_dir() {
        return Err(Error::Project(format!(
            "project path is not a directory: {}",
            root.display()
        )));
    }
    let cancelled = process::cancellation()?;
    let uv = uv::Uv::discover(args.uv, args.offline, &cancelled)?;
    let mut command = vec![
        OsString::from("run"),
        OsString::from("--project"),
        root.as_os_str().to_owned(),
    ];
    if args.locked {
        command.push(OsString::from("--locked"));
    } else if args.frozen {
        command.push(OsString::from("--frozen"));
    }
    command.push(OsString::from("--"));
    command.extend(args.command);
    uv.execute_interactive(&root, "run", &command, &cancelled)
}
