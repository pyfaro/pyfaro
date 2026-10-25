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

/// The longest pause between two observations that cannot hide a restart: a
/// restarted worker leaves the job for only a couple of milliseconds, so the
/// process count may go down and up again between two observations further
/// apart than this (a busy machine does that to the supervising thread).
const BLIND: Duration = Duration::from_millis(1);

/// How long a second Ctrl-C that cannot be vouched for waits for a restart to
/// show before it forces termination.
const DOUBT: Duration = Duration::from_millis(500);

/// A Ctrl-C waiting to be told apart from a reload.
struct Pending {
    since: Instant,
    /// The processes at the event.
    at_event: u32,
    /// The fewest processes seen since the event.
    fewest: u32,
    /// When the command was last observed.
    last: Instant,
    /// There was a pause since the event in which a restart could have passed
    /// unseen.
    blind: bool,
    /// A second Ctrl-C arrived that the observations could not vouch for.
    doubt: Option<Doubt>,
}

/// A second Ctrl-C given the benefit of the doubt: it is either a user forcing
/// termination or the reload that follows a reload which went unseen.
struct Doubt {
    until: Instant,
    /// There was a pause meanwhile in which a restart could have passed unseen.
    blind: bool,
    /// The wait was already extended once.
    extended: bool,
}

/// The first Ctrl-C starts a grace window instead of ending the command.
/// The command ending inside the window makes it a stop request; the window
/// elapsing, or a process starting while the command lives on, shows that it
/// was a broadcast for somebody else, which is then forgotten. A command that
/// lost processes and started none by the end of the window obeyed only part
/// of a stop request (Uvicorn's reloader can swallow one): the rest of its
/// tree is terminated.
///
/// A second Ctrl-C while the first is pending forces termination at once when
/// the command was watched without a gap since the first and no process
/// started. Process starts are seen by sampling, though, and a reload that
/// happened in a gap leaves the first Ctrl-C pending with nothing to show for
/// it, so after a gap the second one may just as well be the next reload:
/// termination then waits a moment for that reload to show.
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
        let Some(pending) = &mut self.pending else {
            if interrupted {
                self.pending = Some(Pending {
                    since: now,
                    at_event: processes,
                    fewest: processes,
                    last: now,
                    blind: false,
                    doubt: None,
                });
            }
            return Step::Wait;
        };
        if now.duration_since(pending.last) > BLIND {
            pending.blind = true;
            if let Some(doubt) = &mut pending.doubt {
                doubt.blind = true;
            }
        }
        pending.last = now;
        // Nothing starts while a command shuts down, so a new process means
        // the command is restarting a part of itself.
        let restarted = processes > pending.fewest;
        pending.fewest = pending.fewest.min(processes);
        if restarted {
            self.pending = None;
            // The Ctrl-C of this very observation is the next one.
            return self.observe(now, interrupted, false, processes);
        }
        if interrupted {
            let shutting_down = pending.fewest < pending.at_event;
            if shutting_down || !pending.blind || pending.doubt.is_some() {
                return Step::Force;
            }
            pending.doubt = Some(Doubt {
                until: now + DOUBT,
                blind: false,
                extended: false,
            });
            return Step::Wait;
        }
        if let Some(doubt) = &mut pending.doubt {
            if now >= doubt.until {
                // A pause meanwhile means the wait proved nothing yet.
                if doubt.blind && !doubt.extended {
                    doubt.until = now + DOUBT;
                    doubt.blind = false;
                    doubt.extended = true;
                } else {
                    self.pending = None;
                    return Step::Force;
                }
            }
        } else if now.duration_since(pending.since) >= self.grace {
            let partial = pending.fewest < pending.at_event;
            self.pending = None;
            if partial {
                return Step::Force;
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

    fn after_ms(start: Instant, milliseconds: u64) -> Instant {
        start + Duration::from_millis(milliseconds)
    }

    /// Observe the command every half millisecond for `milliseconds`, as a
    /// supervisor that nothing starves does; returns the time of the last
    /// observation and every decision other than waiting.
    fn watch(
        stop: &mut ConsoleStop,
        from: Instant,
        milliseconds: u64,
        processes: u32,
    ) -> (Instant, Vec<Step>) {
        let mut steps = Vec::new();
        let mut now = from;
        for _ in 0..milliseconds * 2 {
            now += Duration::from_micros(500);
            match stop.observe(now, false, false, processes) {
                Step::Wait => {}
                step => steps.push(step),
            }
        }
        (now, steps)
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
        let (now, steps) = watch(&mut stop, start, 1_000, 3);
        assert!(steps.is_empty());
        assert_eq!(stop.observe(now, true, false, 3), Step::Force);
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
        let sixth = after(start, 6);
        assert_eq!(stop.observe(sixth, true, false, 3), Step::Wait);
        let (now, _) = watch(&mut stop, sixth, 1_000, 3);
        assert_eq!(stop.observe(now, true, false, 3), Step::Force);
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
    fn a_second_ctrl_c_after_a_pause_waits_for_the_restart_it_may_cause() {
        let start = Instant::now();
        let mut stop = ConsoleStop::new(GRACE);
        assert_eq!(stop.observe(start, true, false, 5), Step::Wait);
        // The command was not watched for a second, so the first reload may
        // have come and gone unseen.
        let second = after(start, 1);
        assert_eq!(stop.observe(second, true, false, 5), Step::Wait);
        assert!(stop.is_pending());
        // The reload this Ctrl-C started shows: a worker leaves, another
        // starts. Neither Ctrl-C is a stop request.
        let (now, steps) = watch(&mut stop, second, 200, 5);
        assert!(steps.is_empty());
        assert_eq!(stop.observe(after_ms(now, 1), false, false, 4), Step::Wait);
        assert_eq!(stop.observe(after_ms(now, 2), false, false, 5), Step::Wait);
        assert!(!stop.is_pending());
        let (now, steps) = watch(&mut stop, after_ms(now, 2), 1_000, 5);
        assert!(steps.is_empty());
        assert_eq!(stop.observe(now, false, true, 0), Step::Exited);
    }

    #[test]
    fn a_second_ctrl_c_after_a_pause_forces_when_no_restart_shows() {
        let start = Instant::now();
        let mut stop = ConsoleStop::new(GRACE);
        assert_eq!(stop.observe(start, true, false, 5), Step::Wait);
        let second = after(start, 1);
        assert_eq!(stop.observe(second, true, false, 5), Step::Wait);
        // Watched all along, nothing starts: a user forcing a stuck command.
        let (now, steps) = watch(&mut stop, second, 499, 5);
        assert!(steps.is_empty());
        let (_, steps) = watch(&mut stop, now, 2, 5);
        assert_eq!(steps, [Step::Force]);
    }

    #[test]
    fn the_wait_for_a_restart_is_extended_once_after_another_pause() {
        let start = Instant::now();
        let mut stop = ConsoleStop::new(GRACE);
        assert_eq!(stop.observe(start, true, false, 5), Step::Wait);
        let second = after(start, 1);
        assert_eq!(stop.observe(second, true, false, 5), Step::Wait);
        // Another pause: the wait proved nothing, so it runs again.
        let paused = after_ms(second, 100);
        assert_eq!(stop.observe(paused, false, false, 5), Step::Wait);
        let (now, steps) = watch(&mut stop, paused, 410, 5);
        assert!(steps.is_empty());
        assert!(stop.is_pending());
        let (now, steps) = watch(&mut stop, now, 485, 5);
        assert!(steps.is_empty());
        let (_, steps) = watch(&mut stop, now, 10, 5);
        assert_eq!(steps, [Step::Force]);
    }

    #[test]
    fn a_third_ctrl_c_forces_without_waiting() {
        let start = Instant::now();
        let mut stop = ConsoleStop::new(GRACE);
        assert_eq!(stop.observe(start, true, false, 5), Step::Wait);
        let second = after(start, 1);
        assert_eq!(stop.observe(second, true, false, 5), Step::Wait);
        assert_eq!(
            stop.observe(after_ms(second, 100), true, false, 5),
            Step::Force
        );
    }

    #[test]
    fn a_second_ctrl_c_forces_at_once_when_the_command_is_shutting_down() {
        let start = Instant::now();
        let mut stop = ConsoleStop::new(GRACE);
        assert_eq!(stop.observe(start, true, false, 5), Step::Wait);
        // Processes left, and a pause followed: nothing restarted, so the
        // first Ctrl-C was obeyed in part.
        assert_eq!(
            stop.observe(after_ms(start, 100), false, false, 4),
            Step::Wait
        );
        assert_eq!(stop.observe(after(start, 1), true, false, 4), Step::Force);
    }

    #[test]
    fn a_restart_seen_with_the_second_ctrl_c_makes_that_one_the_first() {
        let start = Instant::now();
        let mut stop = ConsoleStop::new(GRACE);
        assert_eq!(stop.observe(start, true, false, 5), Step::Wait);
        assert_eq!(
            stop.observe(after_ms(start, 100), false, false, 4),
            Step::Wait
        );
        // The worker is back, and the next reload begins in the same
        // observation.
        assert_eq!(
            stop.observe(after_ms(start, 101), true, false, 5),
            Step::Wait
        );
        assert!(stop.is_pending());
        let (now, steps) = watch(&mut stop, after_ms(start, 101), 10, 5);
        assert!(steps.is_empty());
        assert_eq!(stop.observe(now, true, false, 5), Step::Force);
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
