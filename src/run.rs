use std::ffi::{OsStr, OsString};
use std::path::Path;

use crate::cli::RunArgs;
use crate::error::{Error, Result};
use crate::{process, project, uv};

pub(crate) fn run(args: RunArgs) -> Result<()> {
    let root = project::select(args.project.as_deref())?;
    let working_directory = std::env::current_dir().map_err(|source| {
        Error::io("resolve working directory", Path::new("."), source)
    })?;
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
    let program = Path::new(&args.command[0]);
    if program.extension().is_some_and(|extension| {
        extension.eq_ignore_ascii_case(OsStr::new("py"))
    }) && working_directory.join(program).is_file()
    {
        command.push(OsString::from("python"));
        command.push(OsString::from("--"));
    }
    command.extend(args.command);
    uv.execute_interactive(&working_directory, "run", &command, &cancelled)
}
