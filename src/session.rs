//! App-local lifetime and idle policy; never changes LG's panel or key policies.
use std::time::Duration;

pub const IDLE_AFTER: Duration = Duration::from_secs(120);
const PROOF_LIMIT: Duration = Duration::from_secs(30);

/// Keep only our own underlying window alive during a verified Settings handoff.
#[derive(Default)]
pub struct SettingsHandoff {
    queued_at: Option<Duration>,
    confirmed_at: Option<Duration>,
    visible_at: Option<Duration>,
    generation: u64,
    returned_while_pending: bool,
}
impl SettingsHandoff {
    pub fn queued(&mut self, now: Duration) {
        self.generation = self.generation.wrapping_add(1);
        self.returned_while_pending = false;
        self.queued_at = Some(now);
        self.confirmed_at = None;
        self.visible_at = None;
    }
    pub fn reply(&mut self, now: Duration, verified: bool) {
        if self.queued_at.is_some() {
            if verified && !self.returned_while_pending {
                self.confirmed_at = Some(now);
            } else {
                self.returned();
            }
        }
    }
    pub fn window(&mut self, generation: u64, now: Duration, covering_own_card: bool) {
        if generation == self.generation && self.confirmed_at.is_some() {
            if covering_own_card {
                self.visible_at = Some(now);
            } else {
                self.returned();
            }
        }
    }
    pub fn returned(&mut self) {
        self.queued_at = None;
        self.confirmed_at = None;
        self.visible_at = None;
        self.returned_while_pending = false;
    }
    pub fn focus_returned(&mut self) {
        if self.queued_at.is_some() && self.confirmed_at.is_none() {
            // Preserve the single in-flight operation until its reply arrives.
            self.returned_while_pending = true;
        } else {
            self.returned();
        }
    }
    pub fn observation_generation(&self) -> Option<u64> {
        self.confirmed_at.map(|_| self.generation)
    }
    pub fn pending(&self) -> bool {
        self.queued_at.is_some()
    }
    pub fn keep_hidden(&self, now: Duration) -> bool {
        if self.returned_while_pending {
            return false;
        }
        if let Some(confirmed) = self.confirmed_at {
            let stamp = self.visible_at.unwrap_or(confirmed);
            now.checked_sub(stamp)
                .is_some_and(|age| age <= Duration::from_secs(2))
        } else {
            self.queued_at
                .and_then(|stamp| now.checked_sub(stamp))
                .is_some_and(|age| age <= Duration::from_secs(20))
        }
    }
}

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
    fn settings_handoff_is_bounded_until_confirmed_and_requires_fresh_own_overlay() {
        let mut handoff = SettingsHandoff::default();
        assert!(!handoff.keep_hidden(Duration::ZERO));
        handoff.queued(Duration::ZERO);
        assert!(handoff.keep_hidden(Duration::from_secs(19)));
        assert!(!handoff.keep_hidden(Duration::from_secs(21)));
        handoff.reply(Duration::from_secs(1), true);
        assert!(handoff.keep_hidden(Duration::from_secs(2)));
        assert!(!handoff.keep_hidden(Duration::from_secs(4)));
        handoff.window(handoff.generation, Duration::from_secs(4), true);
        assert!(handoff.keep_hidden(Duration::from_secs(5)));
        handoff.window(handoff.generation, Duration::from_secs(5), false);
        assert!(!handoff.pending());
        handoff.queued(Duration::from_secs(6));
        handoff.reply(Duration::from_secs(7), false);
        assert!(!handoff.keep_hidden(Duration::from_secs(7)));
    }
    #[test]
    fn returned_focus_and_old_observations_cannot_extend_a_later_handoff() {
        let mut handoff = SettingsHandoff::default();
        handoff.queued(Duration::ZERO);
        let old = handoff.generation;
        handoff.focus_returned();
        assert!(handoff.pending());
        assert!(!handoff.keep_hidden(Duration::ZERO));
        handoff.reply(Duration::from_secs(1), true);
        assert!(!handoff.pending());
        handoff.queued(Duration::from_secs(2));
        handoff.reply(Duration::from_secs(2), true);
        handoff.window(old, Duration::from_secs(5), true);
        assert!(!handoff.keep_hidden(Duration::from_secs(5)));
    }
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
