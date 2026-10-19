use std::io;
use std::os::unix::process::CommandExt;
use std::path::Path;
use std::process::{Child, Command, ExitStatus};
use std::sync::atomic::{AtomicBool, Ordering};
use std::thread;
use std::time::{Duration, Instant};

use nix::errno::Errno;
use nix::sys::signal::{Signal, killpg};
use nix::sys::wait::{Id, WaitPidFlag, WaitStatus, waitid, waitpid};
use nix::unistd::Pid;

use crate::error::{Error, Failures, Result};

const POLL: Duration = Duration::from_millis(20);

pub(super) fn run(
    command: &mut Command,
    cancelled: &AtomicBool,
    relay: bool,
) -> Result<ExitStatus> {
    if cancelled.load(Ordering::SeqCst) {
        return Err(Error::Cancelled);
    }
    let program = std::path::PathBuf::from(command.get_program());
    // Adopt orphaned descendants so that cleanup can reap them, rather than
    // relying on PID 1 or mistaking the leader's exit for group completion.
    nix::sys::prctl::set_child_subreaper(true).map_err(|error| {
        Error::io("enable Linux child reaping for", &program, error.into())
    })?;
    let mut child = command
        .process_group(0)
        .spawn()
        .map_err(|source| Error::UvStart { path: program.clone(), source })?;
    let input = if relay {
        child.stdin.take().map(super::unix_input::Relay::start)
    } else {
        None
    };
    let group = Pid::from_raw(child.id() as i32);
    let result = loop {
        if cancelled.load(Ordering::SeqCst) {
            // Graceful cancellation is best effort; forced termination and its
            // errors are handled unconditionally by cleanup below.
            let _ = killpg(group, Signal::SIGINT);
            let deadline = Instant::now() + Duration::from_millis(250);
            while Instant::now() < deadline {
                if matches!(leader_exited(group), Ok(true)) {
                    break;
                }
                thread::sleep(POLL);
            }
            break Err(Error::Cancelled);
        }
        match leader_exited(group) {
            Ok(true) => break Ok(()),
            Ok(false) => thread::sleep(POLL),
            Err(source) => {
                break Err(Error::io("observe uv", &program, source));
            }
        }
    };
    let cleanup = cleanup(&mut child, group, &program);
    let relay_result = input.map(super::unix_input::Relay::finish).transpose();
    let outcome = match (result, cleanup) {
        (Ok(()), status) => status,
        (Err(error), Ok(_)) => Err(error),
        (Err(operation), Err(source)) => Err(Error::OperationCleanup {
            operation: Box::new(operation),
            source: Box::new(source),
        }),
    };
    if let Err(source) = relay_result
        && matches!(&outcome, Ok(status) if status.success())
    {
        return Err(Error::io("relay terminal input", &program, source));
    }
    outcome
}

fn leader_exited(group: Pid) -> io::Result<bool> {
    // Keep the leader unreaped until all signals are sent. Its PID/PGID cannot
    // be reused and accidentally refer to an unrelated process group.
    match waitid(
        Id::Pid(group),
        WaitPidFlag::WEXITED | WaitPidFlag::WNOHANG | WaitPidFlag::WNOWAIT,
    ) {
        Ok(status) => Ok(status != WaitStatus::StillAlive),
        Err(Errno::EINTR) => Ok(false),
        Err(error) => Err(error.into()),
    }
}

fn cleanup(
    child: &mut Child,
    group: Pid,
    program: &Path,
) -> Result<ExitStatus> {
    let mut failures = Vec::new();
    if let Err(error) = killpg(group, Signal::SIGKILL)
        && error != Errno::ESRCH
    {
        failures.push(Error::io(
            "terminate uv process group",
            program,
            error.into(),
        ));
    }
    let deadline = Instant::now() + Duration::from_secs(3);
    let mut leader = None;
    loop {
        if leader.is_none() {
            match child.try_wait() {
                Ok(status) => leader = status,
                Err(error) => {
                    failures.push(Error::io("reap uv leader", program, error));
                    break;
                }
            }
        }
        if let Some(status) = leader {
            // Linux reparents descendants to this subreaper as their parents
            // exit; collect them all, including multiple descendant levels.
            loop {
                match waitpid(
                    Pid::from_raw(-group.as_raw()),
                    Some(WaitPidFlag::WNOHANG),
                ) {
                    Ok(WaitStatus::StillAlive) | Err(Errno::ECHILD) => break,
                    Ok(_) => continue,
                    Err(Errno::EINTR) => continue,
                    Err(error) => {
                        failures.push(Error::io(
                            "reap uv descendants",
                            program,
                            error.into(),
                        ));
                        break;
                    }
                }
            }
            // No further signals are sent after reaping the leader. Confirm
            // group disappearance independently of its cached exit status.
            match killpg(group, None) {
                Err(Errno::ESRCH) if failures.is_empty() => return Ok(status),
                Err(Errno::ESRCH) => break,
                Ok(()) => {}
                Err(error) => {
                    failures.push(Error::io(
                        "confirm uv group exit",
                        program,
                        error.into(),
                    ));
                    break;
                }
            }
        }
        if Instant::now() >= deadline {
            failures.push(Error::io(
                "confirm uv process cleanup",
                program,
                io::Error::new(
                    io::ErrorKind::TimedOut,
                    "uv process group did not fully exit",
                ),
            ));
            break;
        }
        thread::sleep(POLL);
    }
    Err(Error::ProcessCleanup(Failures(failures)))
}
