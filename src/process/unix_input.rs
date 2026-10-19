use std::io::{self, Read, Write};
use std::os::fd::AsFd;
use std::process::ChildStdin;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::thread::{self, JoinHandle};

use nix::poll::{PollFd, PollFlags, PollTimeout, poll};

pub(super) struct Relay {
    done: Arc<AtomicBool>,
    thread: JoinHandle<io::Result<()>>,
}

impl Relay {
    pub(super) fn start(mut child_input: ChildStdin) -> Self {
        let done = Arc::new(AtomicBool::new(false));
        let signal = Arc::clone(&done);
        let thread = thread::spawn(move || {
            let mut input = io::stdin();
            let mut buffer = [0_u8; 4096];
            while !signal.load(Ordering::SeqCst) {
                let ready = poll(
                    &mut [PollFd::new(input.as_fd(), PollFlags::POLLIN)],
                    PollTimeout::from(20_u8),
                )
                .map_err(io::Error::other)?;
                if ready == 0 {
                    continue;
                }
                let count = input.read(&mut buffer)?;
                if count == 0 {
                    break;
                }
                if let Err(error) = child_input.write_all(&buffer[..count]) {
                    if error.kind() == io::ErrorKind::BrokenPipe {
                        break;
                    }
                    return Err(error);
                }
            }
            Ok(())
        });
        Self { done, thread }
    }

    pub(super) fn finish(self) -> io::Result<()> {
        self.done.store(true, Ordering::SeqCst);
        self.thread
            .join()
            .map_err(|_| io::Error::other("terminal input relay panicked"))?
    }
}
