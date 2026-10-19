#[cfg(any(target_os = "linux", target_os = "macos"))]
use std::io;
use std::io::{IsTerminal, Read};
use std::path::Path;
use std::process::{Command, ExitStatus, Stdio};
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};

use crate::error::Error;
use crate::error::Result;

#[cfg(target_os = "linux")]
mod linux;
#[cfg(target_os = "macos")]
mod macos;
#[cfg(any(target_os = "linux", target_os = "macos"))]
mod unix_input;
#[cfg(target_os = "windows")]
mod windows;

#[cfg(any(target_os = "linux", target_os = "macos"))]
#[derive(Clone, Copy, PartialEq, Eq)]
enum LeaderState {
    Running,
    Exited,
    Stopped,
}

pub(crate) fn cancellation() -> Result<Arc<AtomicBool>> {
    let cancelled = Arc::new(AtomicBool::new(false));
    let signal = Arc::clone(&cancelled);
    ctrlc::set_handler(move || signal.store(true, Ordering::SeqCst))?;
    Ok(cancelled)
}

pub(crate) fn read_diagnostic(path: &Path) -> Result<String> {
    let mut captured = Vec::new();
    std::fs::File::open(path)
        .map_err(|error| Error::io("read", path, error))?
        .take(4096)
        .read_to_end(&mut captured)
        .map_err(|error| Error::io("read", path, error))?;
    Ok(String::from_utf8_lossy(&captured).trim().to_owned())
}

pub(crate) fn run(
    command: Command,
    cancelled: &AtomicBool,
) -> Result<ExitStatus> {
    run_with_input(command, cancelled, false)
}

pub(crate) fn run_interactive(
    mut command: Command,
    cancelled: &AtomicBool,
) -> Result<ExitStatus> {
    let relay = cfg!(any(target_os = "linux", target_os = "macos"))
        && std::io::stdin().is_terminal();
    command.stdin(if relay { Stdio::piped() } else { Stdio::inherit() });
    run_with_input(command, cancelled, relay)
}

fn run_with_input(
    command: Command,
    cancelled: &AtomicBool,
    relay: bool,
) -> Result<ExitStatus> {
    #[cfg(target_os = "linux")]
    {
        let mut command = command;
        linux::run(&mut command, cancelled, relay)
    }
    #[cfg(target_os = "macos")]
    {
        macos::run(command, cancelled, relay)
    }
    #[cfg(target_os = "windows")]
    {
        let _ = relay;
        windows::run(command, cancelled)
    }
    #[cfg(not(any(
        target_os = "linux",
        target_os = "macos",
        target_os = "windows"
    )))]
    {
        let _ = (command, cancelled, relay);
        Err(crate::error::Error::Project(
            "delegated commands are unavailable on this platform".into(),
        ))
    }
}

#[cfg(any(target_os = "linux", target_os = "macos"))]
fn finish_with_relay(
    operation: Result<()>,
    cleanup: Result<ExitStatus>,
    relay: io::Result<Option<()>>,
    program: &Path,
) -> Result<ExitStatus> {
    let outcome = match (operation, cleanup) {
        (Ok(()), status) => status,
        (Err(error), Ok(_)) => Err(error),
        (Err(operation), Err(source)) => Err(Error::OperationCleanup {
            operation: Box::new(operation),
            source: Box::new(source),
        }),
    };
    if let Err(source) = relay
        && matches!(&outcome, Ok(status) if status.success())
    {
        return Err(Error::io("relay terminal input", program, source));
    }
    outcome
}

#[cfg(all(test, target_os = "linux"))]
mod tests {
    use crate::error::Error;
    use crate::test_support;
    use std::fs;
    use std::os::unix::process::ExitStatusExt;
    use std::process::Stdio;
    use std::thread;
    use std::time::{Duration, Instant};

    use super::*;

    #[test]
    fn relay_failure_preserves_a_failed_command_status() {
        let failed = ExitStatus::from_raw(23 << 8);
        let result = finish_with_relay(
            Ok(()),
            Ok(failed),
            Err(io::Error::other("relay failed")),
            Path::new("uv"),
        );
        assert_eq!(result.unwrap().code(), Some(23));
    }

    #[test]
    fn cancellation_stops_and_reaps_the_entire_process_tree() {
        let directory = tempfile::tempdir().unwrap();
        let cancelled = Arc::new(AtomicBool::new(false));
        let signal = Arc::clone(&cancelled);
        let ready = directory.path().join("heartbeat");
        let executable = test_support::fixture_uv();
        let trigger = thread::spawn(move || {
            let deadline = Instant::now() + Duration::from_secs(5);
            while !ready.exists() && Instant::now() < deadline {
                thread::sleep(Duration::from_millis(20));
            }
            let started = ready.exists();
            signal.store(true, Ordering::SeqCst);
            started
        });
        let mut command = Command::new(executable);
        command
            .arg("lock")
            .current_dir(directory.path())
            .env("PYFARO_FIXTURE_MODE", "wait")
            .stdout(Stdio::null())
            .stderr(Stdio::null());
        let result = run(command, &cancelled);
        assert!(trigger.join().unwrap(), "fixture did not start");
        assert!(matches!(result, Err(Error::Cancelled)), "{result:?}");
        let pid: i32 =
            fs::read_to_string(directory.path().join("uv-descendant.pid"))
                .unwrap()
                .parse()
                .unwrap();
        assert_eq!(
            nix::sys::signal::kill(nix::unistd::Pid::from_raw(pid), None),
            Err(nix::errno::Errno::ESRCH)
        );
    }
}

#[cfg(all(test, any(target_os = "macos", target_os = "windows")))]
mod portable_tests {
    use super::*;
    use crate::test_support;
    use std::fs;
    use std::process::Stdio;
    use std::thread;
    use std::time::{Duration, Instant};

    #[test]
    fn cancellation_stops_the_process_tree() {
        let directory = tempfile::tempdir().unwrap();
        let cancelled = Arc::new(AtomicBool::new(false));
        let signal = Arc::clone(&cancelled);
        let heartbeat = directory.path().join("heartbeat");
        let ready = heartbeat.clone();
        let trigger = thread::spawn(move || {
            let deadline = Instant::now() + Duration::from_secs(5);
            while !ready.exists() && Instant::now() < deadline {
                thread::sleep(Duration::from_millis(20));
            }
            let started = ready.exists();
            signal.store(true, Ordering::SeqCst);
            started
        });
        let mut command = Command::new(test_support::fixture_uv());
        command
            .arg("lock")
            .current_dir(directory.path())
            .env("PYFARO_FIXTURE_MODE", "wait")
            .stdout(Stdio::null())
            .stderr(Stdio::null());
        let result = run(command, &cancelled);
        assert!(trigger.join().unwrap(), "fixture did not start");
        assert!(
            matches!(result, Err(crate::error::Error::Cancelled)),
            "{result:?}"
        );
        let before = fs::read(&heartbeat).unwrap();
        thread::sleep(Duration::from_millis(150));
        assert_eq!(fs::read(&heartbeat).unwrap(), before);
    }

    #[test]
    fn an_exited_leader_does_not_leave_a_running_descendant() {
        let directory = tempfile::tempdir().unwrap();
        let mut command = Command::new(test_support::fixture_uv());
        command
            .arg("lock")
            .current_dir(directory.path())
            .env("PYFARO_FIXTURE_MODE", "exit-with-child")
            .stdout(Stdio::null())
            .stderr(Stdio::null());
        let status = run(command, &AtomicBool::new(false)).unwrap();
        assert_eq!(status.code(), Some(42));
        let heartbeat = directory.path().join("heartbeat");
        let before = fs::read(&heartbeat).unwrap();
        thread::sleep(Duration::from_millis(150));
        assert_eq!(fs::read(&heartbeat).unwrap(), before);
    }
}
