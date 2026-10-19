use crate::cli::SyncArgs;
use crate::error::Result;
use crate::{process, project, uv};

pub(crate) fn run(args: SyncArgs) -> Result<()> {
    let root =
        project::select(args.project.as_deref().or(args.path.as_deref()))?;
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
