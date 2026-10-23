use std::io;
use std::os::windows::process::CommandExt;
use std::path::{Path, PathBuf};
use std::process::{Child, Command, ExitStatus};
use std::sync::atomic::{AtomicBool, Ordering};
use std::thread;
use std::time::{Duration, Instant};

use subc_jobobject::{CREATE_SUSPENDED_FLAG, ContainedChild, JobObject};

use super::console_stop::{ConsoleStop, Step};
use crate::error::{Error, Failures, Result};

const POLL: Duration = Duration::from_millis(20);

/// Run `command` in a Job Object. Ctrl-C terminates the job at once, unless
/// `console_grace` is set: then the command gets that long to end by itself,
/// see [`ConsoleStop`].
pub(super) fn run(
    mut command: Command,
    cancelled: &AtomicBool,
    console_grace: Option<Duration>,
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
    let operation = match console_grace {
        None => wait(&mut contained, cancelled, &program),
        Some(grace) => wait_for_stop(
            &mut contained,
            cancelled,
            ConsoleStop::new(grace),
            &program,
        ),
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

fn wait(
    contained: &mut ContainedChild<Child>,
    cancelled: &AtomicBool,
    program: &Path,
) -> Result<ExitStatus> {
    loop {
        if cancelled.load(Ordering::SeqCst) {
            return Err(Error::Cancelled);
        }
        match contained.child.try_wait() {
            Ok(Some(status)) => return Ok(status),
            Ok(None) => thread::sleep(POLL),
            Err(source) => {
                return Err(Error::io(
                    "observe delegated process",
                    program,
                    source,
                ));
            }
        }
    }
}

/// Like [`wait`], but Ctrl-C only counts as a stop request when the command
/// ends because of it. The flag is consumed on every observation, so that a
/// second Ctrl-C stays distinguishable from the first.
fn wait_for_stop(
    contained: &mut ContainedChild<Child>,
    cancelled: &AtomicBool,
    mut console: ConsoleStop,
    program: &Path,
) -> Result<ExitStatus> {
    loop {
        let interrupted = cancelled.swap(false, Ordering::SeqCst);
        let status = contained.child.try_wait().map_err(|source| {
            Error::io("observe delegated process", program, source)
        })?;
        let processes = contained.job.process_count().map_err(|source| {
            Error::io("inspect delegated job", program, source)
        })?;
        match console.observe(
            Instant::now(),
            interrupted,
            status.is_some(),
            processes,
        ) {
            // While a Ctrl-C is pending the job's process count is watched
            // without pause: a restarting worker leaves it for about two
            // milliseconds, and seeing that is how a reload is told from a
            // second Ctrl-C. The window bounds the busy time.
            Step::Wait if console.is_pending() => thread::yield_now(),
            Step::Wait => thread::sleep(POLL),
            Step::Exited => {
                return Ok(status.expect("the command has exited"));
            }
            Step::Stopped | Step::Force => return Err(Error::Cancelled),
        }
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
