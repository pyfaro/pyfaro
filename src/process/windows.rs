use std::io;
use std::os::windows::process::CommandExt;
use std::path::{Path, PathBuf};
use std::process::{Command, ExitStatus};
use std::sync::atomic::{AtomicBool, Ordering};
use std::thread;
use std::time::{Duration, Instant};

use subc_jobobject::{CREATE_SUSPENDED_FLAG, ContainedChild, JobObject};

use crate::error::{Error, Failures, Result};

const POLL: Duration = Duration::from_millis(20);

pub(super) fn run(
    mut command: Command,
    cancelled: &AtomicBool,
) -> Result<ExitStatus> {
    if cancelled.load(Ordering::SeqCst) {
        return Err(Error::Cancelled);
    }
    let program = PathBuf::from(command.get_program());
    command.creation_flags(CREATE_SUSPENDED_FLAG);
    let job = JobObject::new().map_err(|source| {
        Error::io("create delegated process job", &program, source)
    })?;
    let child = command.spawn().map_err(|source| Error::ProcessStart {
        path: program.clone(),
        source,
    })?;
    let pid = child.id();
    let mut contained =
        ContainedChild::contain(child, job, pid).map_err(|source| {
            Error::ProcessStart { path: program.clone(), source }
        })?;
    let operation = loop {
        if cancelled.load(Ordering::SeqCst) {
            break Err(Error::Cancelled);
        }
        match contained.child.try_wait() {
            Ok(Some(status)) => break Ok(status),
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
    let cleanup = cleanup(&contained.job, &program);
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

fn cleanup(job: &JobObject, program: &Path) -> Result<()> {
    job.terminate().map_err(|source| {
        Error::ProcessCleanup(Failures(vec![Error::io(
            "stop delegated job",
            program,
            source,
        )]))
    })?;
    let deadline = Instant::now() + Duration::from_secs(3);
    loop {
        let active = job.process_count().map_err(|source| {
            Error::ProcessCleanup(Failures(vec![Error::io(
                "inspect delegated job",
                program,
                source,
            )]))
        })?;
        if active == 0 {
            return Ok(());
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
