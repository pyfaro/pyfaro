//! Telling a stop request from a console broadcast.
//!
//! A Windows console delivers Ctrl-C to every process attached to it, so the
//! event is not necessarily meant for Pyfaro's child: Uvicorn's reloader, for
//! one, restarts its worker with `os.kill(pid, signal.CTRL_C_EVENT)`. The
//! decisions live here, independent of the platform, so that they can be
//! tested everywhere; `windows.rs` feeds them observations.

use std::time::{Duration, Instant};

/// What the supervising loop does next.
#[derive(Debug, PartialEq, Eq)]
pub(super) enum Step {
    /// Nothing to decide yet: keep observing.
    Wait,
    /// The command ended on its own, without any Ctrl-C pending.
    Exited,
    /// The command ended after a Ctrl-C: the user's stop request.
    Stopped,
    /// Terminate the tree: a second Ctrl-C arrived while the first was
    /// pending, or only part of the tree obeyed the first.
    Force,
}

/// A Ctrl-C waiting to be told apart from a reload.
struct Pending {
    since: Instant,
    /// The processes at the event.
    at_event: u32,
    /// The fewest processes seen since the event.
    fewest: u32,
}

/// The first Ctrl-C starts a grace window instead of ending the command.
/// The command ending inside the window makes it a stop request; the window
/// elapsing, or a process starting while the command lives on, shows that it
/// was a broadcast for somebody else, which is then forgotten. A command that
/// lost processes and started none by the end of the window obeyed only part
/// of a stop request (Uvicorn's reloader can swallow one): the rest of its
/// tree is terminated.
pub(super) struct ConsoleStop {
    grace: Duration,
    pending: Option<Pending>,
}

impl ConsoleStop {
    pub(super) fn new(grace: Duration) -> Self {
        Self { grace, pending: None }
    }

    /// A Ctrl-C is waiting to be told apart from a reload.
    pub(super) fn is_pending(&self) -> bool {
        self.pending.is_some()
    }

    /// Fold in one observation: whether Ctrl-C arrived since the last one,
    /// whether the command's leader has exited, and how many processes its
    /// job holds.
    pub(super) fn observe(
        &mut self,
        now: Instant,
        interrupted: bool,
        exited: bool,
        processes: u32,
    ) -> Step {
        if exited {
            return if interrupted || self.pending.is_some() {
                Step::Stopped
            } else {
                Step::Exited
            };
        }
        if interrupted {
            if self.pending.is_some() {
                return Step::Force;
            }
            self.pending = Some(Pending {
                since: now,
                at_event: processes,
                fewest: processes,
            });
            return Step::Wait;
        }
        if let Some(pending) = &mut self.pending {
            // Nothing starts while a command shuts down, so a new process
            // means the command is restarting a part of itself.
            let restarted = processes > pending.fewest;
            pending.fewest = pending.fewest.min(processes);
            if restarted {
                self.pending = None;
            } else if now.duration_since(pending.since) >= self.grace {
                let partial = pending.fewest < pending.at_event;
                self.pending = None;
                if partial {
                    return Step::Force;
                }
            }
        }
        Step::Wait
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const GRACE: Duration = Duration::from_secs(5);

    fn after(start: Instant, seconds: u64) -> Instant {
        start + Duration::from_secs(seconds)
    }

    #[test]
    fn a_command_that_runs_on_its_own_is_left_alone() {
        let start = Instant::now();
        let mut stop = ConsoleStop::new(GRACE);
        assert_eq!(stop.observe(start, false, false, 3), Step::Wait);
        assert_eq!(stop.observe(after(start, 60), false, false, 3), Step::Wait);
        assert_eq!(
            stop.observe(after(start, 61), false, true, 0),
            Step::Exited
        );
    }

    #[test]
    fn a_ctrl_c_is_pending_until_it_is_settled() {
        let start = Instant::now();
        let mut stop = ConsoleStop::new(GRACE);
        assert!(!stop.is_pending());
        stop.observe(start, true, false, 3);
        assert!(stop.is_pending());
        stop.observe(after(start, 5), false, false, 3);
        assert!(!stop.is_pending());
    }

    #[test]
    fn a_command_that_ends_after_ctrl_c_was_stopped() {
        let start = Instant::now();
        let mut stop = ConsoleStop::new(GRACE);
        assert_eq!(stop.observe(start, true, false, 3), Step::Wait);
        assert_eq!(stop.observe(after(start, 1), false, false, 2), Step::Wait);
        assert_eq!(
            stop.observe(after(start, 2), false, true, 0),
            Step::Stopped
        );
    }

    #[test]
    fn exiting_in_the_very_observation_of_ctrl_c_is_a_stop() {
        let mut stop = ConsoleStop::new(GRACE);
        assert_eq!(stop.observe(Instant::now(), true, true, 0), Step::Stopped);
    }

    #[test]
    fn a_second_ctrl_c_while_one_is_pending_forces_termination() {
        let start = Instant::now();
        let mut stop = ConsoleStop::new(GRACE);
        assert_eq!(stop.observe(start, true, false, 3), Step::Wait);
        assert_eq!(stop.observe(after(start, 1), true, false, 3), Step::Force);
    }

    #[test]
    fn a_ctrl_c_that_outlives_the_window_is_forgotten() {
        let start = Instant::now();
        let mut stop = ConsoleStop::new(GRACE);
        assert_eq!(stop.observe(start, true, false, 3), Step::Wait);
        assert_eq!(stop.observe(after(start, 4), false, false, 3), Step::Wait);
        assert_eq!(stop.observe(after(start, 5), false, false, 3), Step::Wait);
        // Pending no more: the next Ctrl-C is a first one again, and a later
        // exit is not a stop.
        assert_eq!(stop.observe(after(start, 6), true, false, 3), Step::Wait);
        assert_eq!(stop.observe(after(start, 7), true, false, 3), Step::Force);
    }

    #[test]
    fn a_command_that_ends_long_after_the_window_exited_by_itself() {
        let start = Instant::now();
        let mut stop = ConsoleStop::new(GRACE);
        assert_eq!(stop.observe(start, true, false, 3), Step::Wait);
        assert_eq!(stop.observe(after(start, 6), false, false, 3), Step::Wait);
        assert_eq!(stop.observe(after(start, 7), false, true, 0), Step::Exited);
    }

    #[test]
    fn a_process_that_starts_after_ctrl_c_shows_a_reload() {
        let start = Instant::now();
        let mut stop = ConsoleStop::new(GRACE);
        assert_eq!(stop.observe(start, true, false, 3), Step::Wait);
        // The worker is gone, then its replacement starts.
        assert_eq!(stop.observe(after(start, 1), false, false, 2), Step::Wait);
        assert_eq!(stop.observe(after(start, 2), false, false, 3), Step::Wait);
        // A second reload soon after is a first Ctrl-C, not a force.
        assert_eq!(stop.observe(after(start, 3), true, false, 3), Step::Wait);
        assert_eq!(stop.observe(after(start, 4), false, false, 2), Step::Wait);
        assert_eq!(stop.observe(after(start, 5), false, false, 3), Step::Wait);
        assert_eq!(stop.observe(after(start, 6), false, true, 0), Step::Exited);
    }

    #[test]
    fn a_tree_that_only_partly_obeyed_ctrl_c_is_terminated_after_the_window() {
        let start = Instant::now();
        let mut stop = ConsoleStop::new(GRACE);
        assert_eq!(stop.observe(start, true, false, 3), Step::Wait);
        assert_eq!(stop.observe(after(start, 1), false, false, 2), Step::Wait);
        assert_eq!(stop.observe(after(start, 4), false, false, 2), Step::Wait);
        assert_eq!(stop.observe(after(start, 5), false, false, 2), Step::Force);
    }

    #[test]
    fn processes_that_only_leave_do_not_dismiss_the_ctrl_c() {
        let start = Instant::now();
        let mut stop = ConsoleStop::new(GRACE);
        assert_eq!(stop.observe(start, true, false, 3), Step::Wait);
        assert_eq!(stop.observe(after(start, 1), false, false, 2), Step::Wait);
        assert_eq!(stop.observe(after(start, 2), false, false, 1), Step::Wait);
        assert_eq!(stop.observe(after(start, 3), true, false, 1), Step::Force);
    }
}
