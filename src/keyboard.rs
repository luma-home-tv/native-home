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

/// Narrow rollout gate; loader symbols alone do not qualify behaviour.
pub fn qualified(info: &serde_json::Value) -> bool {
    info["returnValue"] == true
        && matches!(
            info["modelName"].as_str(),
            Some("OLED55G26LA" | "OLED65C26LD")
        )
        && info["firmwareVersion"] == "23.25.55"
        && info["sdkVersion"] == "9.2.2"
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Command {
    None,
    Start,
    Stop,
    Finish,
    Unavailable,
}
#[derive(Default, Debug, serde::Serialize)]
pub struct Controller {
    #[serde(skip)]
    panel: PanelSession,
    #[serde(skip)]
    opened_at: Option<u32>,
    #[serde(skip)]
    settled_at: Option<u32>,
    pub opens: u32,
    pub stops: u32,
    pub dismissals: u32,
    pub failures: u32,
}
impl Controller {
    pub fn active(&self) -> bool {
        self.opened_at.is_some()
    }
    pub fn started(&mut self, success: bool, now: u32) {
        self.panel = PanelSession::default();
        if success {
            self.opened_at = Some(now);
            self.opens += 1;
        } else {
            self.failures += 1;
        }
    }
    /// Drain delayed app-local release events after giving up the LG text surface.
    pub fn accepts_input(&self, now: u32) -> bool {
        self.settled_at.is_none_or(|at| now.wrapping_sub(at) >= 200)
    }
    pub fn begin_input_drain(&mut self, now: u32) {
        self.settled_at = Some(now);
    }
    pub fn stopped(&mut self, now: u32) {
        self.begin_input_drain(now);
        if self.opened_at.take().is_some() {
            self.stops += 1;
        }
        self.panel = PanelSession::default();
    }
    pub fn reconcile(
        &mut self,
        wanted: bool,
        allowed: bool,
        visible: bool,
        status: crate::runtime::KeyboardStatus,
        now: u32,
    ) -> Command {
        if let Some(opened) = self.opened_at {
            if !wanted {
                return Command::Stop;
            }
            if !allowed || !visible || status.focused == 0 {
                return Command::Finish;
            }
            if self.panel.observe(status.panel_flag != 0) {
                self.dismissals += 1;
                return Command::Finish;
            }
            if !self.panel.seen && now.wrapping_sub(opened) >= 3000 {
                self.failures += 1;
                return Command::Unavailable;
            }
            Command::None
        } else if wanted {
            if allowed
                && visible
                && status.support != 0
                && status.focused != 0
                && (status.major, status.minor, status.patch) == (2, 0, 14)
            {
                Command::Start
            } else {
                self.failures += 1;
                Command::Unavailable
            }
        } else {
            Command::None
        }
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn rollout_requires_real_known_model_and_firmware() {
        let info = serde_json::json!({"returnValue":true,"modelName":"OLED55G26LA","firmwareVersion":"23.25.55","sdkVersion":"9.2.2"});
        assert!(qualified(&info));
        for key in ["returnValue", "modelName", "firmwareVersion", "sdkVersion"] {
            let mut changed = info.clone();
            changed[key] = serde_json::Value::Null;
            assert!(!qualified(&changed));
        }
        let mut changed = info;
        changed["modelName"] = "OLED65C26LD".into();
        assert!(qualified(&changed));
        changed["modelName"] = "unknown".into();
        assert!(!qualified(&changed));
    }
    #[test]
    fn controller_handles_pending_panel_dismissal_expiry_and_ownership_loss() {
        use Command::*;
        let mut c = Controller::default();
        let mut status = crate::runtime::KeyboardStatus {
            support: 1,
            focused: 1,
            major: 2,
            minor: 0,
            patch: 14,
            ..Default::default()
        };
        assert_eq!(c.reconcile(true, false, true, status, 0), Unavailable);
        assert_eq!(c.reconcile(true, true, true, status, 0), Start);
        c.started(true, 0);
        assert_eq!(c.reconcile(true, true, true, status, 100), None);
        status.panel_flag = 1;
        assert_eq!(c.reconcile(true, true, true, status, 200), None);
        status.panel_flag = 0;
        assert_eq!(c.reconcile(true, true, true, status, 300), Finish);
        c.stopped(300);
        assert!(!c.active());
        assert!(!c.accepts_input(300));
        assert!(!c.accepts_input(499));
        assert!(c.accepts_input(500));
        assert_eq!(c.failures, 1);
        assert_eq!((c.opens, c.stops, c.dismissals), (1, 1, 1));
        c.started(true, u32::MAX - 100);
        assert_eq!(c.reconcile(true, true, true, status, 3000), Unavailable);
        c.stopped(3000);
        assert_eq!(c.failures, 2);
        c.started(true, 4000);
        assert_eq!(c.reconcile(true, true, false, status, 4010), Finish);
        assert_eq!(c.reconcile(false, true, true, status, 4010), Stop);
    }

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
