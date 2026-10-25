use std::ffi::{OsStr, OsString};
use std::path::{Path, PathBuf};

use crate::cli::RunArgs;
use crate::config::Config;
use crate::error::{Error, Result};
use crate::{process, project, redact, uv};

/// What to hand to `uv run ... --`, and where and with which variables.
enum Launch {
    Program {
        working_directory: PathBuf,
        command: Vec<OsString>,
    },
    Task {
        working_directory: PathBuf,
        env: Vec<(OsString, OsString)>,
        command: Vec<OsString>,
    },
}

pub(crate) fn run(args: RunArgs) -> Result<()> {
    if args.task.is_none() && args.command.is_empty() {
        return Err(missing_target(args.project.as_deref())?);
    }
    let root = match project::select(args.project.as_deref()) {
        // `pyfaro run python` outside a project is almost always a forgotten
        // `--`: report it as the usage error it most likely is.
        Err(error @ Error::NoProject { .. }) if args.task.is_some() => {
            return Err(Error::Usage(format!(
                "{error}\nto run a program, use pyfaro run -- <program>"
            )));
        }
        selected => selected?,
    };
    let config = Config::load(&root)?;
    let launch = match args.task {
        Some(name) => task_launch(&config, &name, args.command)?,
        None => program_launch(&config, args.command)?,
    };
    let cancelled = process::cancellation()?;
    let uv = uv::Uv::discover(args.uv, args.offline, &cancelled)?;
    let mut command = vec![
        OsString::from("run"),
        OsString::from("--project"),
        config.root().as_os_str().to_owned(),
    ];
    if args.locked {
        command.push(OsString::from("--locked"));
    } else if args.frozen {
        command.push(OsString::from("--frozen"));
    }
    command.push(OsString::from("--"));
    match launch {
        Launch::Program { working_directory, command: program } => {
            command.extend(program);
            uv.execute_interactive(
                &working_directory,
                "run",
                &command,
                &cancelled,
            )
        }
        Launch::Task { working_directory, env, command: task } => {
            command.extend(task);
            uv.execute_task(
                &working_directory,
                &env,
                "run",
                &command,
                &cancelled,
            )
        }
    }
}

fn program_launch(config: &Config, args: Vec<OsString>) -> Result<Launch> {
    let working_directory = std::env::current_dir().map_err(|source| {
        Error::io("resolve working directory", Path::new("."), source)
    })?;
    if let Some(name) = args[0].to_str()
        && config.tasks().get(name).is_some()
    {
        eprintln!(
            "{}",
            redact::text(&format!(
                "pyfaro: running the program \"{name}\"; a task with the same name exists, run it with pyfaro run {name}"
            ))
        );
    }
    let mut command = Vec::with_capacity(args.len() + 2);
    let program = Path::new(&args[0]);
    if program.extension().is_some_and(|extension| {
        extension.eq_ignore_ascii_case(OsStr::new("py"))
    }) && working_directory.join(program).is_file()
    {
        command.push(OsString::from("python"));
        command.push(OsString::from("--"));
    }
    command.extend(args);
    Ok(Launch::Program { working_directory, command })
}

fn task_launch(
    config: &Config,
    name: &OsStr,
    extra: Vec<OsString>,
) -> Result<Launch> {
    let tasks = config.tasks();
    let Some(task) = name.to_str().and_then(|name| tasks.get(name)) else {
        return Err(unknown_task(config, &name.to_string_lossy()));
    };
    let invocation = task.invocation(config.root(), extra)?;
    Ok(Launch::Task {
        working_directory: invocation.cwd,
        env: invocation.env,
        command: invocation.command,
    })
}

fn unknown_task(config: &Config, name: &str) -> Error {
    let metadata = config.root().join("pyproject.toml");
    let tasks = config.tasks();
    let known = if tasks.is_empty() {
        "no tasks are defined under [tool.pyfaro.tasks]".to_owned()
    } else {
        format!("tasks defined:\n{}", tasks.listing())
    };
    Error::Project(format!(
        "no task named \"{name}\" in {}\n{known}\nto run a program, use pyfaro run -- {name}",
        metadata.display()
    ))
}

/// `pyfaro run` with neither a task nor `--`: a usage error that still helps
/// by listing the tasks of the project, when there is one.
fn missing_target(explicit: Option<&Path>) -> Result<Error> {
    let tasks = match project::select(explicit) {
        Ok(root) => {
            let config = Config::load(&root)?;
            let tasks = config.tasks();
            if tasks.is_empty() {
                String::new()
            } else {
                format!(
                    "\nTasks defined in {}:\n{}\n",
                    root.join("pyproject.toml").display(),
                    tasks.listing()
                )
            }
        }
        Err(Error::NoProject { .. }) => String::new(),
        Err(error) => return Err(error),
    };
    Ok(Error::Usage(format!(
        "the following required arguments were not provided:\n  <TASK> or -- <PROGRAM>...\n\n\
         Usage: pyfaro run [OPTIONS] <TASK> [-- <ARGS>...]\n       \
         pyfaro run [OPTIONS] -- <PROGRAM> [ARGS]...\n{tasks}\n\
         For more information, try '--help'."
    )))
}
