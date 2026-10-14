//! App-local lifetime and idle policy; never changes LG's panel or key policies.
use std::time::Duration;

pub const IDLE_AFTER: Duration = Duration::from_secs(120);
const PROOF_LIMIT: Duration = Duration::from_secs(30);

pub struct Session {
    interactive: bool,
    last_input: Duration,
    idle: bool,
    pub wakes: u32,
}

impl Session {
    pub fn new(interactive: bool) -> Self {
        Self {
            interactive,
            last_input: Duration::ZERO,
            idle: false,
            wakes: 0,
        }
    }
    pub fn expired(&self, elapsed: Duration) -> bool {
        !self.interactive && elapsed >= PROOF_LIMIT
    }
    pub fn idle(&mut self, elapsed: Duration) -> bool {
        self.idle = self.interactive && elapsed.saturating_sub(self.last_input) >= IDLE_AFTER;
        self.idle
    }
    /// The first input wakes the canvas only, avoiding a launch/pin on a hidden card.
    pub fn input(&mut self, elapsed: Duration) -> bool {
        let was_idle = self.idle(elapsed);
        self.last_input = elapsed;
        self.idle = false;
        if was_idle {
            self.wakes = self.wakes.saturating_add(1);
        }
        !was_idle
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn diagnostic_stays_bounded_and_does_not_blank() {
        let mut session = Session::new(false);
        assert!(!session.expired(Duration::from_secs(29)));
        assert!(session.expired(PROOF_LIMIT));
        assert!(!session.idle(IDLE_AFTER));
    }
    #[test]
    fn interactive_survives_proof_timeout_and_blanks_at_two_minutes() {
        let mut session = Session::new(true);
        assert!(!session.expired(Duration::from_secs(3600)));
        assert!(!session.idle(Duration::from_secs(119)));
        assert!(session.idle(IDLE_AFTER));
        assert!(!session.input(IDLE_AFTER));
        assert_eq!(session.wakes, 1);
        assert!(session.input(Duration::from_secs(121)));
        assert!(!session.idle(Duration::from_secs(240)));
        assert!(session.idle(Duration::from_secs(241)));
    }
    #[test]
    fn activity_at_idle_boundary_is_wake_only_even_before_next_frame() {
        let mut session = Session::new(true);
        assert!(!session.input(IDLE_AFTER));
        assert!(!session.idle(IDLE_AFTER));
    }
}
