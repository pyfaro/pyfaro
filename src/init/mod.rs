mod config;
mod files;

use std::fs;
use std::io;
use std::io::Write;

use crate::cli::InitArgs;
use crate::error::{Error, Failures, Result};
use crate::process;
use crate::project;
use crate::uv;
use files::Snapshot;

pub(crate) fn run(args: InitArgs) -> Result<()> {
    let root = project::target(&args.path)?;
    let metadata = Snapshot::read(root.join("pyproject.toml"))?;
    let lockfile = Snapshot::read(root.join("uv.lock"))?;
    if !metadata.exists() && lockfile.exists() {
        return Err(Error::Project("uv.lock exists without pyproject.toml; restore project metadata before running init".into()));
    }
    for ancestor in root.ancestors() {
        let metadata = ancestor.join("pyproject.toml");
        match fs::read_to_string(&metadata) {
            Ok(source) if config::is_workspace(&source)? => {
                return Err(Error::Project(format!(
                    "uv workspaces are not supported by this alpha (found {}); initialize a standalone project",
                    metadata.display()
                )));
            }
            Ok(_) => {}
            Err(error) if error.kind() == io::ErrorKind::NotFound => {}
            Err(error) => return Err(Error::io("read", &metadata, error)),
        }
    }
    let path = root.join("pyproject.toml");
    let original = metadata.text()?;
    let proposed = original.map(config::prepare).transpose()?;

    print_plan(&args, original, proposed.as_deref()).map_err(Error::Output)?;
    if !args.apply {
        return Ok(());
    }
    if !cfg!(target_os = "linux") {
        return Err(Error::Project(
            "init --apply is supported on Linux only in this alpha; preview is available on all platforms".into()
        ));
    }

    let cancelled = process::cancellation()?;
    let uv = uv::Uv::discover(args.uv, args.offline, &cancelled)?;
    metadata.verify_unchanged()?;
    lockfile.verify_unchanged()?;
    io::stdout().flush().map_err(Error::Output)?;
    let result = (|| {
        if original.is_none() {
            fs::create_dir_all(&root).map_err(|error| {
                Error::io("create project directory", &root, error)
            })?;
            uv.init(&root, &cancelled)?;
        }
        let current_metadata = Snapshot::read(path.clone())?;
        let current = current_metadata.text()?.ok_or_else(|| {
            Error::Project("uv init did not create pyproject.toml".into())
        })?;
        let proposed = config::prepare(current)?;
        if proposed != current {
            current_metadata.write(proposed.as_bytes())?;
        }
        uv.lock(&root, &cancelled)
    })();
    if let Err(operation) = result {
        if operation.cleanup_incomplete() {
            return Err(Error::RecoverySkipped(Box::new(operation)));
        }
        let failures: Vec<_> = [metadata.restore(), lockfile.restore()]
            .into_iter()
            .filter_map(std::result::Result::err)
            .collect();
        if !failures.is_empty() {
            return Err(Error::Rollback {
                operation: Box::new(operation),
                source: Failures(failures),
            });
        }
        return Err(operation);
    }
    writeln!(
        io::stdout().lock(),
        "Initialized {} with Pyfaro and uv.",
        args.path.display()
    )
    .map_err(Error::Output)?;
    Ok(())
}

fn print_plan(
    args: &InitArgs,
    original: Option<&str>,
    proposed: Option<&str>,
) -> io::Result<()> {
    let mut output = io::stdout().lock();
    writeln!(output, "Initialization plan for {}:", args.path.display())?;
    if original.is_none() {
        writeln!(
            output,
            "  Run uv init --bare (no Git, README, or Python pin files)."
        )?;
    }
    if original.is_some() && proposed == original {
        writeln!(output, "  [tool.pyfaro] is already configured.")?;
    } else {
        writeln!(
            output,
            "  Ensure the following configuration in pyproject.toml:"
        )?;
        writeln!(output, "\n[tool.pyfaro]\nschema-version = 1\n")?;
    }
    writeln!(output, "  Run uv lock --no-build to create or update uv.lock.")?;
    if !args.apply {
        writeln!(
            output,
            "Preview only: rerun with --apply to initialize this project."
        )?;
    }

    Ok(())
}
