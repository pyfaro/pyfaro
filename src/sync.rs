use crate::cli::SyncArgs;
use crate::config::Config;
use crate::error::Result;
use crate::{process, project, uv};

pub(crate) fn run(args: SyncArgs) -> Result<()> {
    let root = if let Some(project) = args.project.as_deref() {
        project::select(Some(project))?
    } else {
        project::discover(
            args.path.as_deref().unwrap_or_else(|| std::path::Path::new(".")),
        )?
    };
    let config = Config::load(&root)?;
    let cancelled = process::cancellation()?;
    let uv = uv::Uv::discover(args.uv, args.offline, &cancelled)?;
    let mut command = vec!["sync"];
    if args.locked {
        command.push("--locked");
    } else if args.frozen {
        command.push("--frozen");
    }
    uv.execute(config.root(), "sync", &command, &cancelled)
}
