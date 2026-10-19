use std::process::{Command, ExitStatus};
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};

use crate::error::Result;

#[cfg(target_os = "linux")]
mod linux;
#[cfg(any(target_os = "macos", target_os = "windows"))]
mod portable;

pub(crate) fn cancellation() -> Result<Arc<AtomicBool>> {
    let cancelled = Arc::new(AtomicBool::new(false));
    let signal = Arc::clone(&cancelled);
    ctrlc::set_handler(move || signal.store(true, Ordering::SeqCst))?;
    Ok(cancelled)
}

pub(crate) fn run(
    mut command: Command,
    cancelled: &AtomicBool,
) -> Result<ExitStatus> {
    #[cfg(target_os = "linux")]
    {
        linux::run(&mut command, cancelled)
    }
    #[cfg(any(target_os = "macos", target_os = "windows"))]
    {
        portable::run(command, cancelled)
    }
    #[cfg(not(any(
        target_os = "linux",
        target_os = "macos",
        target_os = "windows"
    )))]
    {
        let _ = (command, cancelled);
        Err(crate::error::Error::Project(
            "delegated commands are unavailable on this platform".into(),
        ))
    }
}

#[cfg(all(test, target_os = "linux"))]
mod tests {
    use crate::error::Error;
    use crate::test_support;
    use std::fs;
    use std::process::Stdio;
    use std::thread;
    use std::time::{Duration, Instant};

    use super::*;

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
}
