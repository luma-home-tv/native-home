//! Isolated negative-only pending-guard observation. No pending record is ever
//! cleared here; this module neither starts nor reconciles a Root helper.
use crate::{broker_protocol::Marker, runtime::Window, shell::Shell};
use serde_json::Value;
use std::{error::Error, path::Path};

#[cfg(any(target_arch = "arm", test))]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Phase {
    CreateAndRetain,
    NewProviderBlocked,
}
#[cfg(any(target_arch = "arm", test))]
impl Phase {
    fn selected(pending: bool) -> Self {
        if pending {
            Self::NewProviderBlocked
        } else {
            Self::CreateAndRetain
        }
    }
    fn name(self) -> &'static str {
        match self {
            Self::CreateAndRetain => "created_and_retained",
            Self::NewProviderBlocked => "new_provider_blocked",
        }
    }
}
#[cfg(any(target_arch = "arm", test))]
fn require_unavailable(
    result: Result<(), crate::artwork_worker::ProviderError>,
) -> Result<(), &'static str> {
    if result == Err(crate::artwork_worker::ProviderError::Unavailable) {
        Ok(())
    } else {
        Err("Existing pending record did not block a new provider as Unavailable")
    }
}

#[cfg(target_arch = "arm")]
fn reject_input() -> Result<(), Box<dyn Error>> {
    while let Some(event) = crate::runtime::poll() {
        if matches!(event.kind, 1..=8 | 10 | 11) {
            return Err("Pending guard proof interrupted by real input or departure".into());
        }
    }
    Ok(())
}
#[cfg(target_arch = "arm")]
fn save(path: &Path, value: &Value) -> Result<(), Box<dyn Error>> {
    use std::{io::Write, os::unix::fs::OpenOptionsExt};
    let mut file = std::fs::OpenOptions::new()
        .write(true)
        .create_new(true)
        .mode(0o600)
        .custom_flags(libc::O_NOFOLLOW | libc::O_CLOEXEC)
        .open(path)?;
    serde_json::to_writer(&mut file, value)?;
    file.write_all(b"\n")?;
    file.sync_all()?;
    Ok(())
}
#[cfg(target_arch = "arm")]
fn capture(window: &Window, path: &Path) -> Result<(), Box<dyn Error>> {
    use std::os::unix::fs::{MetadataExt, OpenOptionsExt, PermissionsExt};
    let result = window.capture(path);
    // Normalize our own new generic capture even if capture left a partial file.
    match std::fs::OpenOptions::new()
        .read(true)
        .custom_flags(libc::O_NOFOLLOW | libc::O_CLOEXEC)
        .open(path)
    {
        Ok(file) => {
            let metadata = file.metadata()?;
            if !metadata.is_file()
                || metadata.uid() != unsafe { libc::geteuid() }
                || metadata.nlink() != 1
            {
                return Err("Pending guard capture ownership changed".into());
            }
            file.set_permissions(std::fs::Permissions::from_mode(0o600))?;
        }
        Err(error) if result.is_ok() => return Err(error.into()),
        Err(_) => (),
    }
    result
}

/// Ordinary Proof only. A successful local observation supplies no evidence
/// about remote process closure, recovery, normal activation or performance.
#[cfg(target_arch = "arm")]
pub fn run(
    shell: &mut Shell,
    window: &Window,
    output: &Path,
    marker: &Marker,
    now: &mut u32,
) -> Result<Value, Box<dyn Error>> {
    use crate::{
        artwork_installed_provider::InstalledProvider,
        artwork_session_guard::{Guard, PendingIdentity},
        broker_installation::Profile,
        model::Page,
    };
    use serde_json::json;
    use std::time::{Duration, Instant};
    marker.validate()?;
    let own = crate::broker_probe::own_identity()?;
    if own.ui_sha256 != marker.ui_sha256 {
        return Err("Pending guard marker differs from measured Proof executable".into());
    }
    if !shell.live_artwork_receipt()["runtime"].is_null() {
        return Err("Pending guard proof requires no artwork worker".into());
    }
    reject_input()?;
    let preferences = shell.preferences().clone();
    let guard = Guard::open(Profile::Proof)?;
    let pending_before = guard.pending()?;
    let phase = Phase::selected(pending_before);
    let mut report = json!({"schema":1,"app_id":Profile::Proof.app_id(),"own_before":own,
        "own_after":null,"marker":marker,"phase":phase.name(),"pending_before":pending_before,
        "pending_after":null,"constructor":{"attempted":false,"outcome":"not_attempted"},
        "ticket_deliberately_dropped":false,"hold_ms":0,"hold_target_ms":8000,
        "frame":"guard-relaunch-home.ppm","preferences_unchanged":false,"requests_empty":false,
        "unexpected_request_count":0,"verified":false,"error":null,
        "root_dispatch_requested":false,"hbc_calls_requested":false,"worker_started":false,
        "resource_reads":false,"setting_writes":false,"application_launches":false,
        "pending_record_cleared":false,"recovery_performed":false,"remote_closure_qualified":false,
        "normal_loader_qualified":false,"production_startup_qualified":false,"performance_validated":false});
    let mut requests = 0usize;
    let observed = (|| -> Result<(), Box<dyn Error>> {
        match phase {
            Phase::CreateAndRetain => {
                let ticket = guard.begin(PendingIdentity {
                    schema: 1,
                    profile: Profile::Proof,
                    caller_pid: own.pid,
                    caller_start_ticks: own.start_ticks,
                    ui_sha256: own.ui_sha256.clone(),
                    nonce: marker.nonce.clone(),
                })?;
                drop(ticket); // Intentionally no finish_known or recovery path.
                report["ticket_deliberately_dropped"] = json!(true);
            }
            Phase::NewProviderBlocked => {
                report["constructor"]["attempted"] = json!(true);
                let result = InstalledProvider::new(marker.clone()).map(drop);
                report["constructor"]["outcome"] = json!(match result {
                    Ok(()) => "unexpected_success".to_owned(),
                    Err(error) => format!("{error:?}"),
                });
                require_unavailable(result)?;
            }
        }
        if !guard.pending()? {
            return Err("Pending guard record was not retained after local phase".into());
        }
        shell.open(Page::Home);
        let began = Instant::now();
        while began.elapsed() < Duration::from_secs(8) {
            let frame_start = Instant::now();
            reject_input()?;
            *now = now.wrapping_add(17);
            shell.tick(*now, 1.0 / 60.0);
            requests += shell.take_requests().len();
            if requests != 0
                || shell.preferences() != &preferences
                || shell.should_exit()
                || !shell.live_artwork_receipt()["runtime"].is_null()
            {
                return Err(
                    "Pending guard proof changed preferences, worker state or requested an action"
                        .into(),
                );
            }
            shell.draw();
            window.swap();
            if let Some(left) = Duration::from_micros(16_667).checked_sub(frame_start.elapsed()) {
                std::thread::sleep(left);
            }
        }
        report["hold_ms"] = json!(began.elapsed().as_millis());
        reject_input()?;
        let before_draw = window.graphics_state();
        shell.draw();
        let graphics = window.graphics_state();
        report["before_draw"] = json!(before_draw);
        report["graphics"] = json!(graphics);
        if before_draw.error != 0
            || graphics.error != 0
            || graphics.scissor_enabled
            || graphics.framebuffer != 0
            || graphics.viewport != [0, 0, 1920, 1080]
        {
            return Err("Pending guard Home draw target is unqualified".into());
        }
        capture(window, &output.join("guard-relaunch-home.ppm"))?;
        window.swap();
        let after = crate::broker_probe::own_identity()?;
        report["own_after"] = json!(after);
        if after != own {
            return Err("Pending guard caller identity changed".into());
        }
        requests += shell.take_requests().len();
        reject_input()?;
        if requests != 0 || shell.preferences() != &preferences || shell.should_exit() {
            return Err("Pending guard final UI invariants changed".into());
        }
        if !guard.pending()? {
            return Err("Pending guard record disappeared during observation".into());
        }
        report["pending_after"] = json!(true);
        report["presentation"] = shell.presentation_probe();
        Ok(())
    })();
    report["preferences_unchanged"] = json!(shell.preferences() == &preferences);
    report["requests_empty"] = json!(requests == 0);
    report["unexpected_request_count"] = json!(requests);
    report["verified"] = json!(observed.is_ok());
    if let Err(error) = &observed {
        report["error"] = json!(error.to_string());
    }
    // Neither errors nor Drop paths clear the pending name. Save failure evidence
    // where possible; a save/capture error likewise leaves the guard untouched.
    save(&output.join("guard-relaunch-proof.json"), &report)?;
    observed?;
    Ok(report)
}

#[cfg(not(target_arch = "arm"))]
pub fn run(
    _shell: &mut Shell,
    _window: &Window,
    _output: &Path,
    _marker: &Marker,
    _now: &mut u32,
) -> Result<Value, Box<dyn Error>> {
    Err(std::io::Error::new(
        std::io::ErrorKind::Unsupported,
        "Pending guard relaunch proof requires the native ordinary Proof profile",
    )
    .into())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::artwork_worker::ProviderError;
    #[test]
    fn phase_selection_has_no_age_pid_nonce_or_recovery_branch() {
        assert_eq!(Phase::selected(false), Phase::CreateAndRetain);
        assert_eq!(Phase::selected(false).name(), "created_and_retained");
        assert_eq!(Phase::selected(true), Phase::NewProviderBlocked);
        assert_eq!(Phase::selected(true).name(), "new_provider_blocked");
    }
    #[test]
    fn only_unavailable_is_the_expected_constructor_refusal_not_host_native_proof() {
        // Pure disposition policy only; no host Native constructor is invoked.
        assert!(require_unavailable(Err(ProviderError::Unavailable)).is_ok());
        assert!(require_unavailable(Ok(())).is_err());
        for error in [
            ProviderError::Denied,
            ProviderError::Changed,
            ProviderError::Invalid,
            ProviderError::Cancelled,
            ProviderError::Expired,
        ] {
            assert!(require_unavailable(Err(error)).is_err());
        }
    }
}
