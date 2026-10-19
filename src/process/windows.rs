use std::path::{Path, PathBuf};
use std::process::{Command, ExitStatus};
use std::sync::atomic::{AtomicBool, Ordering};
use std::thread;
use std::time::Duration;

use process_wrap::std::{ChildWrapper, CommandWrap, JobObject};

use crate::error::{Error, Failures, Result};

const POLL: Duration = Duration::from_millis(20);

pub(super) fn run(
    command: Command,
    cancelled: &AtomicBool,
) -> Result<ExitStatus> {
    if cancelled.load(Ordering::SeqCst) {
        return Err(Error::Cancelled);
    }
    let program = PathBuf::from(command.get_program());
    let mut wrapped = CommandWrap::from(command);
    wrapped.wrap(JobObject);
    let mut child = wrapped
        .spawn()
        .map_err(|source| Error::UvStart { path: program.clone(), source })?;
    drop(wrapped);
    let result = loop {
        if cancelled.load(Ordering::SeqCst) {
            break Err(Error::Cancelled);
        }
        match child.try_wait() {
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
    match (result, cleanup(child.as_mut(), &program)) {
        (Ok(status), Ok(())) => Ok(status),
        (Err(error), Ok(())) => Err(error),
        (Ok(_), Err(source)) => Err(source),
        (Err(operation), Err(source)) => Err(Error::OperationCleanup {
            operation: Box::new(operation),
            source: Box::new(source),
        }),
    }
}

fn cleanup(child: &mut dyn ChildWrapper, program: &Path) -> Result<()> {
    child.start_kill().map_err(|source| {
        Error::ProcessCleanup(Failures(vec![Error::io(
            "stop delegated job",
            program,
            source,
        )]))
    })?;
    child.wait().map_err(|source| {
        Error::ProcessCleanup(Failures(vec![Error::io(
            "confirm delegated job exit",
            program,
            source,
        )]))
    })?;
    Ok(())
}
