//! App-local tracking of a requested system keyboard. No polling, rendering or TV calls.
#[derive(Default, Debug)]
pub struct PanelSession {
    seen: bool,
    closed: bool,
}
impl PanelSession {
    /// Startup may report no panel yet. Only a subsequent hide ends this request.
    pub fn observe(&mut self, shown: bool) -> bool {
        self.closed |= self.seen && !shown;
        self.seen |= shown;
        self.closed
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn initial_absence_does_not_close_a_request_but_lg_dismissal_does() {
        let mut session = PanelSession::default();
        assert!(!session.observe(false));
        assert!(!session.observe(false));
        assert!(!session.observe(true));
        assert!(!session.observe(true));
        assert!(session.observe(false));
        assert!(session.observe(true));
        assert!(!PanelSession::default().observe(true));
    }
}
