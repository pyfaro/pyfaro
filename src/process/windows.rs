use std::fs::File;
use std::io;
use std::os::windows::process::ExitStatusExt;
use std::path::PathBuf;
use std::process::{Command, ExitStatus};
use std::sync::atomic::{AtomicBool, Ordering};
use std::thread;
use std::time::Duration;

use win_custody::{Job, JobLimits, Launch, WaitOutcome, null_device, wait};

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
    let job = Job::create(&JobLimits::new().kill_on_close(true)).map_err(
        |source| Error::io("create delegated process job", &program, source),
    )?;
    let null = null_device()
        .map_err(|source| Error::io("open null input", &program, source))?;
    let output = io::stdout();
    let errors = io::stderr();
    let mut launch = Launch::new(&program);
    launch
        .args(command.get_args())
        .current_dir(
            command
                .get_current_dir()
                .unwrap_or_else(|| std::path::Path::new(".")),
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
    let child = launch
        .spawn_in_job(&job)
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
    let cleanup = (|| {
        job.terminate(1).map_err(|source| {
            Error::ProcessCleanup(Failures(vec![Error::io(
                "stop delegated job",
                &program,
                source,
            )]))
        })?;
        match wait(&job, Some(Duration::from_secs(3))) {
            Ok(WaitOutcome::Signaled) => Ok(()),
            Ok(WaitOutcome::TimedOut | WaitOutcome::Abandoned) => {
                Err(Error::ProcessCleanup(Failures(vec![Error::io(
                    "confirm delegated job exit",
                    &program,
                    io::Error::new(
                        io::ErrorKind::TimedOut,
                        "job still has active processes",
                    ),
                )])))
            }
            Err(source) => {
                Err(Error::ProcessCleanup(Failures(vec![Error::io(
                    "confirm delegated job exit",
                    &program,
                    source,
                )])))
            }
        }
    })();
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
