//! App-local lifetime and idle policy; never changes LG's panel or key policies.
use serde::Serialize;
use std::time::Duration;

pub const IDLE_AFTER: Duration = Duration::from_secs(120);
const PROOF_LIMIT: Duration = Duration::from_secs(30);
const SETTINGS_PENDING: Duration = Duration::from_secs(20);
const SETTINGS_FRESH: Duration = Duration::from_secs(2);
const SETTINGS_TRACE_LIMIT: usize = 64;

/// Only fixed window identities are retained, never raw service replies or app lists.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum SettingsWindowState {
    OwnSoleCard,
    SettingsCover,
    OpeningSpinner,
    Foreign,
    Unavailable,
}
#[derive(Clone, Copy, Serialize)]
#[serde(rename_all = "snake_case")]
enum HandoffEnd {
    OpeningExpired,
    CoverExpired,
    ForeignDeparture,
}
#[derive(Serialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
enum HandoffEvent {
    Queued,
    Reply {
        verified: bool,
        accepted: bool,
    },
    FocusHint,
    Window {
        observation_generation: u64,
        state: SettingsWindowState,
        observed_at: Duration,
        accepted: bool,
    },
    SceneReturned,
    Ended {
        reason: HandoffEnd,
    },
}
#[derive(Serialize)]
struct HandoffRecord {
    generation: u64,
    received_at: Duration,
    event: HandoffEvent,
}

/// Retain only our own window during a bounded, scene-observed Settings handoff.
/// SDL focus is a hint: it cannot prove Settings finished opening or returned.
#[derive(Default)]
pub struct SettingsHandoff {
    queued_at: Option<Duration>,
    verified: bool,
    visible_at: Option<Duration>,
    generation: u64,
    records: Vec<HandoffRecord>,
    trace_truncated: bool,
    scene_returns: u32,
    last_window: Option<(u64, SettingsWindowState, bool)>,
    repeated_windows_omitted: bool,
    ended: Option<HandoffEnd>,
    scene_suspended: bool,
}
impl SettingsHandoff {
    fn record(&mut self, now: Duration, event: HandoffEvent) {
        if self.records.len() < SETTINGS_TRACE_LIMIT {
            self.records.push(HandoffRecord {
                generation: self.generation,
                received_at: now,
                event,
            });
        } else {
            // Preserve the first origin plus the most recent finite sequence.
            self.records.remove(1);
            self.records.push(HandoffRecord {
                generation: self.generation,
                received_at: now,
                event,
            });
            self.trace_truncated = true;
        }
    }
    pub fn queued(&mut self, now: Duration) {
        self.generation = self.generation.wrapping_add(1);
        self.queued_at = Some(now);
        self.verified = false;
        self.visible_at = None;
        self.last_window = None;
        self.ended = None;
        self.scene_suspended = false;
        self.record(now, HandoffEvent::Queued);
    }
    pub fn reply(&mut self, now: Duration, verified: bool) {
        // A receipt cannot erase or renew an already elapsed opening/cover bound.
        let accepted = !self.must_close(now) && self.pending();
        self.record(now, HandoffEvent::Reply { verified, accepted });
        if accepted {
            if verified {
                self.verified = true;
            } else {
                self.clear();
            }
        }
    }
    /// `observed_at` is the worker's read START, not channel receipt time.
    /// A delayed observation cannot renew a cover or qualify a return.
    pub fn window(
        &mut self,
        generation: u64,
        observed_at: Duration,
        now: Duration,
        state: SettingsWindowState,
    ) -> bool {
        // Consume prior expiry BEFORE a fresh-looking observation can replace it.
        let fresh = !self.must_close(now)
            && generation == self.generation
            && self.verified
            && self.queued_at.is_some_and(|queued| observed_at >= queued)
            && now
                .checked_sub(observed_at)
                .is_some_and(|age| age <= SETTINGS_FRESH)
            && self.visible_at.is_none_or(|last| observed_at >= last)
            && (state != SettingsWindowState::SettingsCover
                || self.visible_at.is_some()
                || self
                    .queued_at
                    .is_some_and(|queued| now.saturating_sub(queued) <= SETTINGS_PENDING));
        let trace_key = (generation, state, fresh);
        if self.last_window == Some(trace_key) {
            self.repeated_windows_omitted = true;
        } else {
            self.record(
                now,
                HandoffEvent::Window {
                    observation_generation: generation,
                    state,
                    observed_at,
                    accepted: fresh,
                },
            );
            self.last_window = Some(trace_key);
        }
        if !fresh {
            return false;
        }
        match state {
            SettingsWindowState::SettingsCover => {
                // First cover also passed the original opening deadline above.
                self.visible_at = Some(observed_at);
                self.scene_suspended = true;
            }
            SettingsWindowState::OpeningSpinner => {
                // Suspension only: spinner never supplies cover or return evidence.
                self.scene_suspended = true;
            }
            SettingsWindowState::OwnSoleCard
                if self
                    .visible_at
                    .is_some_and(|last| now.saturating_sub(last) <= SETTINGS_FRESH) =>
            {
                self.scene_returns = self.scene_returns.saturating_add(1);
                self.scene_suspended = false;
                self.record(now, HandoffEvent::SceneReturned);
                self.clear();
                return true;
            }
            SettingsWindowState::Foreign => {
                self.clear();
                self.ended = Some(HandoffEnd::ForeignDeparture);
                self.record(
                    now,
                    HandoffEvent::Ended {
                        reason: HandoffEnd::ForeignDeparture,
                    },
                );
            }
            // Own-before-cover, spinner and unavailable never mean a return, and
            // never refresh a cover timestamp. Existing hidden-window expiry applies.
            _ => {}
        }
        false
    }
    fn clear(&mut self) {
        self.queued_at = None;
        self.verified = false;
        self.visible_at = None;
    }
    pub fn focus_returned(&mut self, now: Duration) {
        if self.pending() {
            self.record(now, HandoffEvent::FocusHint);
        }
    }
    pub fn observation_generation(&self) -> Option<u64> {
        self.verified
            .then_some(self.generation)
            .filter(|_| self.pending())
    }
    pub fn pending(&self) -> bool {
        self.queued_at.is_some()
    }
    pub fn keep_hidden(&self, now: Duration) -> bool {
        let (stamp, limit) = match self.visible_at {
            Some(stamp) => (Some(stamp), SETTINGS_FRESH),
            None => (self.queued_at, SETTINGS_PENDING),
        };
        stamp
            .and_then(|stamp| now.checked_sub(stamp))
            .is_some_and(|age| age <= limit)
    }
    /// Consume expiry independently of SDL hidden state, before another action
    /// can be dispatched. The caller ends this lifetime, never retries the intent.
    pub fn must_close(&mut self, now: Duration) -> bool {
        if self.ended.is_some() {
            return true;
        }
        if self.pending() && !self.keep_hidden(now) {
            let reason = if self.visible_at.is_some() {
                HandoffEnd::CoverExpired
            } else {
                HandoffEnd::OpeningExpired
            };
            self.clear();
            self.ended = Some(reason);
            self.record(now, HandoffEvent::Ended { reason });
            return true;
        }
        false
    }
    /// App-local drawing/input suspension; diagnostic records are not consulted.
    pub fn scene_suspended(&self) -> bool {
        self.scene_suspended
    }
    pub fn diagnostics(&self) -> serde_json::Value {
        serde_json::json!({"schema":1,"records":self.records,"trace_truncated":self.trace_truncated,
            "scene_returns":self.scene_returns,"ended":self.ended,"scene_suspended":self.scene_suspended,"repeated_windows_omitted":self.repeated_windows_omitted,"observations_are_authority":false,
            "sdl_focus_counter_is_scene_return":false})
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
    fn ms(n: u64) -> Duration {
        Duration::from_millis(n)
    }
    fn observe(h: &mut SettingsHandoff, at: u64, now: u64, state: SettingsWindowState) -> bool {
        h.window(h.generation, ms(at), ms(now), state)
    }
    #[test]
    fn early_focus_and_reply_do_not_settle_opening_or_freshen_original_deadline() {
        let mut h = SettingsHandoff::default();
        h.queued(ms(0));
        h.focus_returned(ms(100));
        assert!(h.pending());
        assert!(h.keep_hidden(ms(19_999)));
        h.reply(ms(15_000), true);
        assert!(h.pending());
        assert!(!observe(
            &mut h,
            15_100,
            15_100,
            SettingsWindowState::OwnSoleCard
        ));
        assert!(!observe(
            &mut h,
            15_200,
            15_200,
            SettingsWindowState::OpeningSpinner
        ));
        assert!(h.keep_hidden(ms(20_000)));
        assert!(!h.keep_hidden(ms(20_001)));
        assert_eq!(h.diagnostics()["scene_returns"], 0);
    }
    #[test]
    fn genuine_fresh_cover_then_own_scene_is_return_without_needing_sdl_focus() {
        let mut h = SettingsHandoff::default();
        h.queued(ms(0));
        h.reply(ms(1_000), true);
        assert!(!observe(
            &mut h,
            1_100,
            1_200,
            SettingsWindowState::SettingsCover
        ));
        h.focus_returned(ms(1_300));
        assert!(h.pending());
        assert!(observe(
            &mut h,
            1_400,
            1_500,
            SettingsWindowState::OwnSoleCard
        ));
        assert!(!h.pending());
        assert!(!h.keep_hidden(ms(1_500)));
        assert_eq!(h.diagnostics()["scene_returns"], 1);
    }
    #[test]
    fn unavailable_and_spinner_do_not_refresh_cover_or_fabricate_return() {
        for state in [
            SettingsWindowState::Unavailable,
            SettingsWindowState::OpeningSpinner,
        ] {
            let mut h = SettingsHandoff::default();
            h.queued(ms(0));
            h.reply(ms(100), true);
            observe(&mut h, 200, 200, SettingsWindowState::SettingsCover);
            assert!(!observe(&mut h, 1_900, 1_900, state));
            assert!(!h.keep_hidden(ms(2_201)));
            assert!(h.pending());
            assert!(!observe(
                &mut h,
                2_300,
                2_300,
                SettingsWindowState::OwnSoleCard
            ));
        }
    }
    #[test]
    fn stale_receipts_old_generations_future_and_prequeue_times_cannot_extend_hidden_lifetime() {
        let mut h = SettingsHandoff::default();
        h.queued(ms(100));
        h.reply(ms(200), true);
        assert!(!observe(
            &mut h,
            99,
            201,
            SettingsWindowState::SettingsCover
        ));
        assert!(!observe(
            &mut h,
            500,
            201,
            SettingsWindowState::SettingsCover
        ));
        assert!(!observe(
            &mut h,
            300,
            2_301,
            SettingsWindowState::SettingsCover
        ));
        observe(&mut h, 400, 500, SettingsWindowState::SettingsCover);
        assert!(!h.keep_hidden(ms(2_401))); // start stamp, never channel receipt500
        let old = h.generation;
        h.queued(ms(3_000));
        h.reply(ms(3_100), true);
        assert!(!h.window(
            old,
            ms(3_200),
            ms(3_200),
            SettingsWindowState::SettingsCover
        ));
        assert!(!h.keep_hidden(ms(23_001)));
        assert!(!observe(
            &mut h,
            23_100,
            23_100,
            SettingsWindowState::SettingsCover
        ));
        assert!(!h.keep_hidden(ms(23_100)));
    }
    #[test]
    fn refused_action_or_foreign_departure_never_grants_a_return() {
        let mut h = SettingsHandoff::default();
        h.queued(ms(0));
        h.reply(ms(100), false);
        assert!(!h.pending());
        assert!(!h.keep_hidden(ms(100)));
        h.queued(ms(200));
        h.reply(ms(300), true);
        assert!(!observe(&mut h, 400, 400, SettingsWindowState::Foreign));
        assert!(!h.pending());
        assert!(!h.keep_hidden(ms(400)));
        assert_eq!(h.diagnostics()["scene_returns"], 0);
    }
    #[test]
    fn no_sdl_hide_still_expires_opening_and_cover_without_scene_return_or_later_reply_revival() {
        for covered in [false, true] {
            let mut h = SettingsHandoff::default();
            h.queued(ms(0));
            h.reply(ms(100), true);
            if covered {
                observe(&mut h, 200, 200, SettingsWindowState::SettingsCover);
            }
            let boundary = if covered { 2_200 } else { 20_000 };
            assert!(!h.must_close(ms(boundary)));
            assert!(h.must_close(ms(boundary + 1)));
            assert!(!h.pending());
            assert!(h.observation_generation().is_none());
            h.reply(ms(boundary + 2), true);
            assert!(!h.pending());
            assert!(h.must_close(ms(boundary + 3)));
            assert_eq!(h.diagnostics()["scene_returns"], 0);
        }
        let mut h = SettingsHandoff::default();
        h.queued(ms(0));
        h.reply(ms(100), true);
        observe(&mut h, 200, 200, SettingsWindowState::Foreign);
        assert!(h.must_close(ms(200)));
        assert_eq!(h.diagnostics()["ended"], "foreign_departure");
    }
    #[test]
    fn expired_cover_cannot_be_renewed_then_return_in_same_delayed_batch() {
        let mut h = SettingsHandoff::default();
        h.queued(ms(0));
        h.reply(ms(100), true);
        observe(&mut h, 200, 200, SettingsWindowState::SettingsCover);
        // No must_close/tick between last cover and delivery of these fresh reads.
        assert!(!observe(
            &mut h,
            2_900,
            3_000,
            SettingsWindowState::SettingsCover
        ));
        assert!(!observe(
            &mut h,
            2_950,
            3_000,
            SettingsWindowState::OwnSoleCard
        ));
        assert!(!h.pending());
        assert!(h.must_close(ms(3_000)));
        assert_eq!(h.diagnostics()["ended"], "cover_expired");
        assert_eq!(h.diagnostics()["scene_returns"], 0);
        let records = h.diagnostics()["records"].as_array().unwrap().clone();
        assert_eq!(records[records.len() - 1]["event"]["accepted"], false);
    }
    #[test]
    fn late_true_or_false_reply_consumes_original_expiry_before_clearing_or_confirming() {
        for verified in [false, true] {
            let mut h = SettingsHandoff::default();
            h.queued(ms(0));
            h.reply(ms(20_001), verified); // no preceding must_close/tick
            assert!(!h.pending());
            assert!(h.must_close(ms(20_001)));
            assert_eq!(h.diagnostics()["ended"], "opening_expired");
            assert_eq!(h.diagnostics()["scene_returns"], 0);
            assert_eq!(
                h.diagnostics()["records"]
                    .as_array()
                    .unwrap()
                    .last()
                    .unwrap()["event"]["accepted"],
                false
            );
        }
    }
    #[test]
    fn known_cover_and_spinner_suspend_without_sdl_hide_but_only_cover_then_own_resumes() {
        let mut h = SettingsHandoff::default();
        h.queued(ms(0));
        h.reply(ms(100), true);
        observe(&mut h, 200, 200, SettingsWindowState::OpeningSpinner);
        assert!(h.scene_suspended());
        h.focus_returned(ms(300));
        assert!(h.scene_suspended());
        assert!(!observe(&mut h, 400, 400, SettingsWindowState::OwnSoleCard));
        assert!(h.scene_suspended());
        observe(&mut h, 500, 500, SettingsWindowState::SettingsCover);
        assert!(h.scene_suspended());
        assert!(observe(&mut h, 600, 600, SettingsWindowState::OwnSoleCard));
        assert!(!h.scene_suspended());
    }
    #[test]
    fn timeline_diagnostics_are_finite_and_do_not_drive_lifetime() {
        let mut h = SettingsHandoff::default();
        h.queued(ms(0));
        for _ in 0..200 {
            h.focus_returned(ms(1));
        }
        assert_eq!(
            h.diagnostics()["records"].as_array().unwrap().len(),
            SETTINGS_TRACE_LIMIT
        );
        assert_eq!(h.diagnostics()["trace_truncated"], true);
        h.reply(ms(10), true);
        observe(&mut h, 20, 20, SettingsWindowState::SettingsCover);
        assert!(observe(&mut h, 30, 30, SettingsWindowState::OwnSoleCard));
        assert!(!h.pending());
        assert!(serde_json::to_vec(&h.diagnostics()).unwrap().len() < 16_384);
        assert_eq!(h.diagnostics()["records"][0]["event"]["kind"], "queued");
        for generation in 0..100 {
            h.window(
                generation,
                Duration::MAX,
                Duration::MAX,
                SettingsWindowState::Unavailable,
            );
        }
        assert_eq!(
            h.diagnostics()["records"].as_array().unwrap().len(),
            SETTINGS_TRACE_LIMIT
        );
        assert!(serde_json::to_vec(&h.diagnostics()).unwrap().len() < 32_768);
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
