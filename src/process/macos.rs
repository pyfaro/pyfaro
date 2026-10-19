use std::io;
use std::path::{Path, PathBuf};
use std::process::{Command, ExitStatus};
use std::sync::atomic::{AtomicBool, Ordering};
use std::thread;
use std::time::{Duration, Instant};

use nix::errno::Errno;
use nix::sys::signal::{Signal, killpg};
use nix::unistd::Pid;
use process_wrap::std::{CommandWrap, ProcessGroup};
use rustix::process::{Pid as RustixPid, WaitId, WaitIdOptions, waitid};

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
    wrapped.wrap(ProcessGroup::leader());
    let mut child = wrapped
        .spawn()
        .map_err(|source| Error::UvStart { path: program.clone(), source })?;
    drop(wrapped);
    let group = Pid::from_raw(child.id() as i32);
    let leader = RustixPid::from_raw(child.id() as i32).ok_or_else(|| {
        Error::Project("delegated process has no valid process ID".into())
    })?;
    let result = loop {
        if cancelled.load(Ordering::SeqCst) {
            let _ = child.signal(Signal::SIGINT as i32);
            let deadline = Instant::now() + Duration::from_millis(250);
            while Instant::now() < deadline {
                if matches!(leader_exited(leader), Ok(true)) {
                    break;
                }
                thread::sleep(POLL);
            }
            break Err(Error::Cancelled);
        }
        match leader_exited(leader) {
            Ok(true) => break Ok(()),
            Ok(false) => thread::sleep(POLL),
            Err(source) => {
                break Err(Error::io(
                    "observe delegated process",
                    &program,
                    source,
                ));
            }
        }
    };
    match (result, cleanup(child.as_mut(), group, &program)) {
        (Ok(()), status) => status,
        (Err(error), Ok(_)) => Err(error),
        (Err(operation), Err(source)) => Err(Error::OperationCleanup {
            operation: Box::new(operation),
            source: Box::new(source),
        }),
    }
}

fn leader_exited(leader: RustixPid) -> io::Result<bool> {
    let flags =
        WaitIdOptions::EXITED | WaitIdOptions::NOHANG | WaitIdOptions::NOWAIT;
    waitid(WaitId::Pid(leader), flags)
        .map(|status| status.is_some())
        .map_err(|error| io::Error::from_raw_os_error(error.raw_os_error()))
}

fn cleanup(
    child: &mut dyn process_wrap::std::ChildWrapper,
    group: Pid,
    program: &Path,
) -> Result<ExitStatus> {
    let mut failures = Vec::new();
    if let Err(source) = child.start_kill()
        && source.raw_os_error() != Some(Errno::ESRCH as i32)
        && source.raw_os_error() != Some(Errno::EPERM as i32)
    {
        failures.push(Error::io(
            "stop delegated process group",
            program,
            source,
        ));
    }
    let deadline = Instant::now() + Duration::from_secs(3);
    let mut leader = None;
    loop {
        if leader.is_none() {
            match child.try_wait() {
                Ok(status) => leader = status,
                Err(source) => {
                    failures.push(Error::io(
                        "reap delegated process",
                        program,
                        source,
                    ));
                    break;
                }
            }
        }
        if let Some(status) = leader {
            match killpg(group, None) {
                Err(Errno::ESRCH) if failures.is_empty() => return Ok(status),
                Err(Errno::ESRCH) => break,
                Err(Errno::EPERM) => {
                    match group_has_live_members(group, program) {
                        Ok(false) if failures.is_empty() => return Ok(status),
                        Ok(false) => break,
                        Ok(true) => {
                            failures.push(Error::io(
                                "confirm delegated process group exit",
                                program,
                                io::Error::from(Errno::EPERM),
                            ));
                            break;
                        }
                        Err(error) => {
                            failures.push(error);
                            break;
                        }
                    }
                }
                Ok(()) => {}
                Err(source) => {
                    failures.push(Error::io(
                        "confirm delegated process group exit",
                        program,
                        source.into(),
                    ));
                    break;
                }
            }
        }
        if Instant::now() >= deadline {
            failures.push(Error::io(
                "confirm delegated process group cleanup",
                program,
                io::Error::new(
                    io::ErrorKind::TimedOut,
                    "process group still exists",
                ),
            ));
            break;
        }
        thread::sleep(POLL);
    }
    Err(Error::ProcessCleanup(Failures(failures)))
}

fn group_has_live_members(group: Pid, program: &Path) -> Result<bool> {
    let output = Command::new("/bin/ps")
        .args(["-A", "-o", "pgid=", "-o", "stat="])
        .output()
        .map_err(|source| {
            Error::io("inspect delegated process group", program, source)
        })?;
    if !output.status.success() {
        return Err(Error::io(
            "inspect delegated process group",
            program,
            io::Error::other("ps could not enumerate process states"),
        ));
    }
    let listing = std::str::from_utf8(&output.stdout).map_err(|source| {
        Error::io("parse process states", program, io::Error::other(source))
    })?;
    for line in listing.lines() {
        let mut fields = line.split_whitespace();
        let process_group =
            fields.next().and_then(|field| field.parse::<i32>().ok());
        let state = fields.next();
        let (Some(process_group), Some(state)) = (process_group, state) else {
            return Err(Error::io(
                "parse process states",
                program,
                io::Error::new(io::ErrorKind::InvalidData, "invalid ps output"),
            ));
        };
        if process_group == group.as_raw() && !state.starts_with('Z') {
            return Ok(true);
        }
    }
    Ok(false)
}
