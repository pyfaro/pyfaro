use std::fs::File;
use std::io;
use std::os::windows::io::AsRawHandle;
use std::os::windows::process::ExitStatusExt;
use std::path::{Path, PathBuf};
use std::process::{Command, ExitStatus};
use std::sync::atomic::{AtomicBool, Ordering};
use std::thread;
use std::time::{Duration, Instant};

use win_custody::{Launch, Process, ProcessAccess, null_device};
use win32job::{ExtendedLimitInfo, Job};

use crate::error::{Error, Failures, Result};

const POLL: Duration = Duration::from_millis(20);

pub(super) fn run(
    command: Command,
    cancelled: &AtomicBool,
    capture: Option<&File>,
) -> Result<ExitStatus> {
    if cancelled.load(Ordering::SeqCst) {
        return Err(Error::Cancelled);
    }
    let program = PathBuf::from(command.get_program());
    let mut limits = ExtendedLimitInfo::new();
    limits.limit_kill_on_job_close();
    let job = Job::create_with_limit_info(&limits).map_err(|source| {
        Error::io(
            "create delegated process job",
            &program,
            io::Error::other(source),
        )
    })?;
    let null = null_device()
        .map_err(|source| Error::io("open null input", &program, source))?;
    let output = io::stdout();
    let errors = io::stderr();
    let mut launch = Launch::new(&program);
    launch
        .args(command.get_args())
        .current_dir(
            command.get_current_dir().unwrap_or_else(|| Path::new(".")),
        )
        .create_no_window(false)
        .stdin(&null)
        .stderr(&errors);
    if let Some(file) = capture {
        launch.stdout(file);
    } else {
        launch.stdout(&output);
    }
    for (name, value) in command.get_envs() {
        if let Some(value) = value {
            launch.env(name, value);
        } else {
            launch.env_remove(name);
        }
    }
    let suspended = launch
        .spawn_suspended()
        .map_err(|source| Error::UvStart { path: program.clone(), source })?;
    job.assign_process(suspended.process().as_raw_handle() as isize).map_err(
        |source| Error::UvStart {
            path: program.clone(),
            source: io::Error::other(source),
        },
    )?;
    let child = suspended
        .resume()
        .map_err(|source| Error::UvStart { path: program.clone(), source })?;
    let operation = loop {
        if cancelled.load(Ordering::SeqCst) {
            break Err(Error::Cancelled);
        }
        match child.exit_code() {
            Ok(Some(code)) => break Ok(ExitStatus::from_raw(code)),
            Ok(None) => thread::sleep(POLL),
            Err(source) => {
                break Err(Error::io(
                    "observe delegated process",
                    &program,
                    source,
                ));
            }
        }
    };
    let cleanup = cleanup(&job, &program);
    match (operation, cleanup) {
        (Ok(status), Ok(())) => Ok(status),
        (Err(error), Ok(())) => Err(error),
        (Ok(_), Err(source)) => Err(source),
        (Err(operation), Err(source)) => Err(Error::OperationCleanup {
            operation: Box::new(operation),
            source: Box::new(source),
        }),
    }
}

fn cleanup(job: &Job, program: &Path) -> Result<()> {
    let deadline = Instant::now() + Duration::from_secs(3);
    loop {
        let processes = job.query_process_id_list().map_err(|source| {
            Error::ProcessCleanup(Failures(vec![Error::io(
                "inspect delegated job",
                program,
                io::Error::other(source),
            )]))
        })?;
        if processes.is_empty() {
            return Ok(());
        }
        for pid in processes {
            match Process::open(pid as u32, ProcessAccess::Terminate) {
                Ok(process) => {
                    if let Err(source) = process.terminate(1)
                        && source.raw_os_error() != Some(87)
                    {
                        return Err(Error::ProcessCleanup(Failures(vec![
                            Error::io(
                                "stop delegated job process",
                                program,
                                source,
                            ),
                        ])));
                    }
                }
                Err(source) if source.raw_os_error() == Some(87) => {}
                Err(source) => {
                    return Err(Error::ProcessCleanup(Failures(vec![
                        Error::io(
                            "open delegated job process",
                            program,
                            source,
                        ),
                    ])));
                }
            }
        }
        if Instant::now() >= deadline {
            return Err(Error::ProcessCleanup(Failures(vec![Error::io(
                "confirm delegated job exit",
                program,
                io::Error::new(
                    io::ErrorKind::TimedOut,
                    "job still has active processes",
                ),
            )])));
        }
        thread::sleep(POLL);
    }
}
