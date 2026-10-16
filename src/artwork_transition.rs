//! App-local pause composition and one typed deferred external intent. No I/O,
//! service/URI/command selection, provider activation, recovery or authority.
use crate::{actions::Request, artwork_runtime::TransitionReadiness, model::Page};
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[repr(u8)]
pub enum PauseReason {
    KeyboardIntent,
    KeyboardActive,
    ExternalIntent,
    ForegroundLost,
    Idle,
    Quarantine,
    Manual,
}
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct PauseReasons(u8);
impl PauseReasons {
    pub fn set(&mut self, reason: PauseReason, paused: bool) {
        let bit = 1u8 << reason as u8;
        if paused {
            self.0 |= bit;
        } else {
            self.0 &= !bit;
        }
    }
    pub fn contains(self, reason: PauseReason) -> bool {
        self.0 & (1u8 << reason as u8) != 0
    }
    pub fn is_empty(self) -> bool {
        self.0 == 0
    }
    pub fn bits(self) -> u8 {
        self.0
    }
}
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Anchor {
    pub page: Page,
    pub focused: Option<String>,
    pub key: Option<u32>,
}
#[derive(Clone, Debug)]
struct Intent {
    request: Request,
    anchor: Anchor,
    generation: u64,
}
#[derive(Default, Debug)]
pub struct DeferredExternal {
    pending: Option<Intent>,
    submitted: Option<Request>,
    reply_verified: bool,
    focus_returned: bool,
    generation: u64,
}
impl DeferredExternal {
    /// Same intent coalesces. The most recent different unsubmitted intent wins.
    /// An in-flight action is never replaced or automatically submitted again.
    pub fn offer(&mut self, request: Request, anchor: Anchor) -> bool {
        if self.submitted.is_some() {
            return false;
        }
        if self
            .pending
            .as_ref()
            .is_some_and(|p| p.request == request && p.anchor == anchor)
        {
            return true;
        }
        self.generation = self.generation.wrapping_add(1);
        self.pending = Some(Intent {
            request,
            anchor,
            generation: self.generation,
        });
        true
    }
    pub fn cancel_pending(&mut self) {
        self.pending = None;
    }
    pub fn revalidate(&mut self, anchor: &Anchor, keyboard_intent: bool, exiting: bool) {
        if keyboard_intent || exiting || self.pending.as_ref().is_some_and(|p| &p.anchor != anchor)
        {
            self.cancel_pending();
        }
    }
    pub fn pending_request(&self) -> Option<&Request> {
        self.pending.as_ref().map(|p| &p.request)
    }
    pub fn pending_generation(&self) -> Option<u64> {
        self.pending.as_ref().map(|p| p.generation)
    }
    pub fn holds_pause(&self) -> bool {
        self.pending.is_some() || self.submitted.is_some()
    }
    pub fn take_ready(
        &mut self,
        readiness: TransitionReadiness,
        context_ready: bool,
    ) -> Option<Request> {
        if self.submitted.is_none()
            && context_ready
            && matches!(
                readiness,
                TransitionReadiness::Ready | TransitionReadiness::Disabled
            )
        {
            self.pending.take().map(|p| p.request)
        } else {
            None
        }
    }
    pub fn submitted(&mut self, request: Request) {
        debug_assert!(self.submitted.is_none());
        self.submitted = Some(request);
        self.reply_verified = false;
        self.focus_returned = false;
    }
    /// Only the matching existing action reply may resolve its local lifecycle.
    /// This is not Root/helper session evidence and grants no artwork authority.
    pub fn reply(&mut self, request: &Request, verified: bool) {
        if self.submitted.as_ref() != Some(request) {
            return;
        }
        self.reply_verified = verified;
        // A generic unverified action receipt includes admitted-but-readback
        // failure and transport-unknown. Neither proves a closed handoff.
        if verified && self.focus_returned {
            self.submitted = None;
        }
    }
    pub fn focus_returned(&mut self, observed_departure: bool) {
        // Duplicate/fresh FocusGained without an observed prior loss is not an
        // action-return lifecycle observation, even after a verified reply.
        if observed_departure && self.submitted.is_some() {
            self.focus_returned = true;
            if self.reply_verified {
                self.submitted = None;
            }
        }
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    fn anchor() -> Anchor {
        Anchor {
            page: Page::Applications,
            focused: Some("synthetic.app.a".into()),
            key: Some(1),
        }
    }
    fn request(id: &str) -> Request {
        Request::Application { id: id.into() }
    }
    #[test]
    fn reasons_compose_and_repeated_set_is_idempotent() {
        let mut set = PauseReasons::default();
        for reason in [
            PauseReason::KeyboardIntent,
            PauseReason::ForegroundLost,
            PauseReason::Idle,
        ] {
            set.set(reason, true);
            set.set(reason, true);
        }
        set.set(PauseReason::ForegroundLost, false);
        set.set(PauseReason::Idle, false);
        assert!(set.contains(PauseReason::KeyboardIntent));
        assert!(!set.is_empty());
        set.set(PauseReason::KeyboardIntent, false);
        assert!(set.is_empty());
    }
    #[test]
    fn last_current_typed_intent_only_and_no_queue_before_known_drain() {
        let mut d = DeferredExternal::default();
        d.offer(request("synthetic.a"), anchor());
        let first = d.pending_generation();
        d.offer(request("synthetic.a"), anchor());
        assert_eq!(first, d.pending_generation());
        d.offer(request("synthetic.b"), anchor());
        assert_ne!(first, d.pending_generation());
        assert!(d.take_ready(TransitionReadiness::Draining, true).is_none());
        assert!(d
            .take_ready(TransitionReadiness::BlockedUnknown, true)
            .is_none());
        assert!(d.take_ready(TransitionReadiness::Ready, false).is_none());
        assert_eq!(
            d.take_ready(TransitionReadiness::Ready, true),
            Some(request("synthetic.b"))
        );
        assert!(d.take_ready(TransitionReadiness::Ready, true).is_none());
    }
    #[test]
    fn back_or_changed_focus_or_keyboard_cancels_pending_without_requeue() {
        let mut d = DeferredExternal::default();
        d.offer(request("synthetic.a"), anchor());
        d.cancel_pending();
        assert!(!d.holds_pause());
        d.offer(request("synthetic.a"), anchor());
        let mut changed = anchor();
        changed.key = Some(2);
        d.revalidate(&changed, false, false);
        assert!(!d.holds_pause());
        d.offer(request("synthetic.a"), anchor());
        d.revalidate(&anchor(), true, false);
        assert!(d.take_ready(TransitionReadiness::Ready, true).is_none());
    }
    #[test]
    fn matching_reply_and_return_lifecycle_only_no_inflight_replacement() {
        let mut d = DeferredExternal::default();
        d.submitted(request("synthetic.a"));
        assert!(!d.offer(request("synthetic.b"), anchor()));
        d.reply(&request("foreign"), false);
        assert!(d.holds_pause());
        d.reply(&request("synthetic.a"), true);
        assert!(d.holds_pause());
        d.focus_returned(true);
        assert!(!d.holds_pause());
        d.submitted(request("synthetic.a"));
        d.focus_returned(true);
        assert!(d.holds_pause());
        d.reply(&request("synthetic.a"), true);
        assert!(!d.holds_pause());
    }
    #[test]
    fn focus_gain_without_loss_cannot_clear_submitted_pause_before_or_after_reply() {
        for reply_first in [false, true] {
            let mut d = DeferredExternal::default();
            d.submitted(request("synthetic.a"));
            if reply_first {
                d.reply(&request("synthetic.a"), true);
            }
            d.focus_returned(false);
            if !reply_first {
                d.reply(&request("synthetic.a"), true);
            }
            assert!(d.holds_pause(), "no observed departure means no return");
            assert!(!d.offer(request("synthetic.b"), anchor()));
            d.focus_returned(true);
            assert!(
                !d.holds_pause(),
                "observed loss then gain plus matching reply closes local lifecycle"
            );
        }
    }
    #[test]
    fn unverified_transport_or_admitted_readback_failure_never_releases_submitted_pause() {
        for admitted in [false, true] {
            for gain_first in [false, true] {
                let mut d = DeferredExternal::default();
                let req = request("synthetic.a");
                d.submitted(req.clone());
                let receipt = crate::actions::Receipt {
                    request: req.clone(),
                    target: admitted.then(|| "synthetic.a".to_string()),
                    admitted,
                    verified: false,
                    detail: if admitted {
                        "synthetic readback timed out"
                    } else {
                        "synthetic RPC transport unknown"
                    }
                    .into(),
                };
                if gain_first {
                    d.focus_returned(true);
                }
                d.reply(&receipt.request, receipt.verified);
                if !gain_first {
                    d.focus_returned(true);
                }
                assert!(
                    d.holds_pause(),
                    "observed gain is not closure for an unverified action"
                );
                assert!(!d.offer(request("synthetic.b"), anchor()));
                assert!(d.take_ready(TransitionReadiness::Ready, true).is_none());
                // Only an actual matching verified observation plus departure
                // can close it; the controller invents neither observation.
                d.reply(&req, true);
                assert!(!d.holds_pause());
            }
        }
    }
}
