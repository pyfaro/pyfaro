use std::path::PathBuf;
use std::process::{Command, ExitStatus};
use std::sync::atomic::{AtomicBool, Ordering};
use std::thread;
use std::time::Duration;

use process_wrap::std::*;

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
    #[cfg(target_os = "macos")]
    wrapped.wrap(ProcessGroup::leader());
    #[cfg(target_os = "windows")]
    wrapped.wrap(JobObject);
    let mut child = wrapped
        .spawn()
        .map_err(|source| Error::UvStart { path: program.clone(), source })?;
    drop(wrapped);
    loop {
        if cancelled.load(Ordering::SeqCst) {
            return match child.kill() {
                Ok(()) => Err(Error::Cancelled),
                Err(source) => Err(Error::OperationCleanup {
                    operation: Box::new(Error::Cancelled),
                    source: Box::new(Error::ProcessCleanup(Failures(vec![
                        Error::io(
                            "stop delegated process tree",
                            &program,
                            source,
                        ),
                    ]))),
                }),
            };
        }
        match child.try_wait() {
            Ok(Some(status)) => return Ok(status),
            Ok(None) => thread::sleep(POLL),
            Err(source) => {
                let operation =
                    Error::io("observe delegated process", &program, source);
                return match child.kill() {
                    Ok(()) => Err(operation),
                    Err(source) => Err(Error::OperationCleanup {
                        operation: Box::new(operation),
                        source: Box::new(Error::ProcessCleanup(Failures(
                            vec![Error::io(
                                "stop delegated process tree",
                                &program,
                                source,
                            )],
                        ))),
                    }),
                };
            }
        }
    }
}
