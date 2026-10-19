use std::fs;

use crate::cli::SyncArgs;
use crate::error::{Error, Result};
use crate::{process, uv};

pub(crate) fn run(args: SyncArgs) -> Result<()> {
    let root = fs::canonicalize(&args.path).map_err(|source| {
        Error::io("resolve project directory", &args.path, source)
    })?;
    if !root.is_dir() {
        return Err(Error::Project(format!(
            "project path is not a directory: {}",
            root.display()
        )));
    }
    let cancelled = process::cancellation()?;
    let uv = uv::Uv::discover(args.uv, args.offline, &cancelled)?;
    let mut command = vec!["sync"];
    if args.locked {
        command.push("--locked");
    } else if args.frozen {
        command.push("--frozen");
    }
    uv.execute(&root, "sync", &command, &cancelled)
}
