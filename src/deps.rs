use std::ffi::OsString;

use crate::cli::DependencyArgs;
use crate::config::Config;
use crate::error::Result;
use crate::{process, project, uv};

pub(crate) enum Action {
    Add,
    Remove,
}

impl Action {
    fn name(&self) -> &'static str {
        match self {
            Self::Add => "add",
            Self::Remove => "remove",
        }
    }
}

pub(crate) fn run(action: Action, args: DependencyArgs) -> Result<()> {
    let root = project::select(args.project.as_deref())?;
    let config = Config::load(&root)?;
    let cancelled = process::cancellation()?;
    let uv = uv::Uv::discover(args.uv, args.offline, &cancelled)?;

    let mut command =
        vec![OsString::from(action.name()), OsString::from("--no-sync")];
    if args.dev {
        command.push(OsString::from("--dev"));
    }
    if let Some(group) = args.group {
        command.push(OsString::from(format!("--group={group}")));
    }
    command.push(OsString::from("--"));
    command.extend(args.packages);
    uv.execute_dependency(config.root(), action.name(), &command, &cancelled)
}
