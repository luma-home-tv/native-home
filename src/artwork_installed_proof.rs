//! Isolated installed-provider renderer observation. No Root operation lives here.
//! Saved geometry and CPU hashes do not independently verify framebuffer pixels.
use crate::{model::Page, runtime::Window, shell::Shell};
use serde_json::{json, Value};
use std::{
    collections::BTreeSet,
    io::Write,
    os::unix::fs::{MetadataExt, OpenOptionsExt, PermissionsExt},
    path::Path,
    time::{Duration, Instant},
};

const LIMIT: Duration = Duration::from_secs(120);
const MIN_LIVE: usize = 3;
const PERIOD: Duration = Duration::from_micros(16_667);

fn rectangle(value: &Value) -> Option<[f64; 4]> {
    let values = value.as_array()?;
    if values.len() != 4 {
        return None;
    }
    let mut rect = [0.0; 4];
    for (out, value) in rect.iter_mut().zip(values) {
        *out = value.as_f64()?;
        if !out.is_finite() {
            return None;
        }
    }
    (rect[2] > 0.0
        && rect[3] > 0.0
        && (rect[0] + rect[2]).is_finite()
        && (rect[1] + rect[3]).is_finite())
    .then_some(rect)
}
fn contains(outer: [f64; 4], inner: [f64; 4]) -> bool {
    inner[0] >= outer[0]
        && inner[1] >= outer[1]
        && inner[0] + inner[2] <= outer[0] + outer[2]
        && inner[1] + inner[3] <= outer[1] + outer[3]
}
fn digest(value: &Value) -> bool {
    value.as_str().is_some_and(|text| {
        text.len() == 64
            && text
                .bytes()
                .all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte))
    })
}
/// A packaged drawing reference alone is insufficient: target comes only from
/// Shell::artwork_target's matching live CPU resident and owned GL revision.
fn eligible(card: &Value, target: &Value) -> bool {
    let Some(id) = card["id"].as_str().filter(|id| !id.is_empty()) else {
        return false;
    };
    let (Some(body), Some(clip)) = (rectangle(&card["rect"]), rectangle(&card["clip"])) else {
        return false;
    };
    let viewport = [0.0, 0.0, 1920.0, 1080.0];
    if !contains(viewport, clip) || !contains(clip, body) || !contains(viewport, body) {
        return false;
    }
    let (Some(handle), Some(width), Some(height)) = (
        target["texture_handle"].as_u64(),
        target["width"].as_u64(),
        target["height"].as_u64(),
    ) else {
        return false;
    };
    handle > 0
        && handle <= u64::from(u32::MAX)
        && width > 0
        && width <= 512
        && height > 0
        && height <= 512
        && card["texture_handle"].as_u64() == Some(handle)
        && card["width"].as_u64() == Some(width)
        && card["height"].as_u64() == Some(height)
        && target["source_key"]["app_id"].as_str() == Some(id)
        && target["revision"].as_u64().is_some_and(|value| value > 0)
        && digest(&target["encoded_sha256"])
}
fn live_targets(shell: &mut Shell, hashes: bool) -> Vec<Value> {
    let mut ids = BTreeSet::new();
    let mut handles = BTreeSet::new();
    shell
        .icon_presentation_cards()
        .into_iter()
        .filter_map(|card| {
            let id = card["id"].as_str()?.to_owned();
            let target = shell.artwork_target(&id, hashes)?;
            if !eligible(&card, &target) || (hashes && !digest(&target["rgba_sha256"])) {
                return None;
            }
            let handle = target["texture_handle"].as_u64()?;
            if !ids.insert(id) || !handles.insert(handle) {
                return None;
            }
            Some(json!({"card":card,"target":target}))
        })
        .collect()
}
fn save(path: &Path, value: &Value) -> Result<(), Box<dyn std::error::Error>> {
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
fn capture(window: &Window, path: &Path) -> Result<(), Box<dyn std::error::Error>> {
    let result = window.capture(path);
    // Window's generic capture requests 0640. This private proof normalizes the
    // newly created own file even if its write returned a partial-file error.
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
                return Err("Installed artwork capture ownership changed".into());
            }
            file.set_permissions(std::fs::Permissions::from_mode(0o600))?;
        }
        Err(error) if result.is_ok() => return Err(error.into()),
        Err(_) => (),
    }
    result
}
fn external_input() -> bool {
    let mut interrupted = false;
    while let Some(event) = crate::runtime::poll() {
        interrupted |= matches!(event.kind, 1..=8 | 10 | 11);
    }
    interrupted
}
fn backend_clean(receipt: &Value) -> bool {
    receipt["upload_failures"] == 0
        && receipt["gl"]["upload_failures"] == 0
        && receipt["gl"]["deletion_failures"] == 0
        && receipt["gl"]["observed_gl_errors"] == 0
}

/// Local scheduler counters only. in_flight includes queued or completed-but-
/// unpolled Work; it is explicitly NOT an observation of the HBC/Root stage.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
struct SchedulingSample {
    submitted: u64,
    completed: u64,
    in_flight: u64,
    uploads: u64,
    cancellation_requests: u64,
    active: bool,
}
impl SchedulingSample {
    fn read(value: &Value) -> Option<Self> {
        Some(Self {
            submitted: value["runtime"]["submitted"].as_u64()?,
            completed: value["runtime"]["completed"].as_u64()?,
            in_flight: value["runtime"]["in_flight"].as_u64()?,
            uploads: value["gl"]["uploads"].as_u64()?,
            cancellation_requests: value["runtime"]["cancellation_requests"].as_u64()?,
            active: value["runtime"]["active"].as_bool()?,
        })
    }
    fn paused_from(self, baseline: Self) -> bool {
        !self.active
            && self.submitted == baseline.submitted
            && self.uploads == baseline.uploads
            && self.cancellation_requests == baseline.cancellation_requests
            && self.in_flight <= 1
            && self.completed <= self.submitted
    }
    fn first_draining(self, baseline: Self) -> bool {
        self.paused_from(baseline) && self.in_flight == 1 && self.completed == 0
    }
}
fn scheduling_frame(
    shell: &mut Shell,
    window: &Window,
    now: &mut u32,
    poll_live: &mut impl FnMut(&mut Shell),
    edge: Option<bool>,
    interrupted: &mut bool,
    unexpected: &mut usize,
) -> Result<(), Box<dyn std::error::Error>> {
    let began = Instant::now();
    *interrupted |= external_input();
    if *interrupted {
        return Err("Scheduling observation interrupted by real input or departure".into());
    }
    poll_live(shell);
    *now = now.wrapping_add(17);
    shell.input_frame(*now, 1.0 / 60.0, |shell| {
        if let Some(down) = edge {
            shell.press(down, plx_machine::machine::PressFrom::Key);
        }
        Ok::<(), Box<dyn std::error::Error>>(())
    })?;
    *unexpected += shell.take_requests().len();
    if *unexpected != 0 || shell.should_exit() {
        return Err("Scheduling observation requested action or exit".into());
    }
    if window.graphics_state().error != 0 {
        return Err("Scheduling observation has pending GL error".into());
    }
    shell.draw();
    let graphics = window.graphics_state();
    if graphics.error != 0
        || graphics.scissor_enabled
        || graphics.framebuffer != 0
        || graphics.viewport != [0, 0, 1920, 1080]
    {
        return Err("Scheduling observation draw target changed".into());
    }
    window.swap();
    if let Some(left) = PERIOD.checked_sub(began.elapsed()) {
        std::thread::sleep(left);
    }
    Ok(())
}
fn scheduling_component(
    shell: &mut Shell,
    window: &Window,
    deadline: Instant,
    now: &mut u32,
    poll_live: &mut impl FnMut(&mut Shell),
    report: &mut Value,
) -> Result<(), Box<dyn std::error::Error>> {
    use crate::{artwork_runtime::TransitionReadiness as Ready, artwork_transition::PauseReason};
    let preferences = shell.preferences().clone();
    let mut interrupted = false;
    let mut unexpected = 0usize;
    let mut frames = 0u64;
    *report = json!({"schema":1,"qualified":false,"scope":"Ordinary Shell queued/unpolled Work scheduling and local intent cancellation only","stage":"prepare_local_editor","root_hbc_stage_at_pause_independently_observed":false,"provider_session_independently_qualified":false,"LG_keyboard_start_requested":false,"external_action_requested":false,"recipe_worker_cancel_requested":false,"old_pending_cleared":false,"physical_input_qualified":false,"main_handoff_qualified":false,"performance_qualified":false});
    let result = (|| -> Result<(), Box<dyn std::error::Error>> {
        shell.set_artwork_active(false);
        shell.open(Page::Applications);
        if !shell.focus_drawer_control(3) {
            return Err("Local keypad entry is unavailable".into());
        }
        // Real fixed controller timing, entirely paused; no text input or LG
        // keyboard call. Control3 selects only the app-local keypad editor.
        for frame in 0..30 {
            if Instant::now() >= deadline {
                return Err("Scheduling preparation deadline".into());
            }
            scheduling_frame(
                shell,
                window,
                now,
                poll_live,
                match frame {
                    0 => Some(true),
                    6 => Some(false),
                    _ => None,
                },
                &mut interrupted,
                &mut unexpected,
            )?;
            frames += 1;
            if shell.preferences() != &preferences {
                return Err("Scheduling preparation changed preferences".into());
            }
            if shell.drawer_state().editing {
                break;
            }
        }
        if !shell.drawer_state().editing || shell.drawer_state().system_keyboard() {
            return Err("Local keypad intent did not open cleanly".into());
        }
        while !shell.artwork_catalogue_current()
            || shell.artwork_transition_readiness() == Ready::Draining
        {
            if Instant::now() >= deadline {
                return Err("Scheduling initialization/catalogue deadline".into());
            }
            scheduling_frame(
                shell,
                window,
                now,
                poll_live,
                None,
                &mut interrupted,
                &mut unexpected,
            )?;
            frames += 1;
            if shell.preferences() != &preferences {
                return Err("Scheduling initialization changed preferences".into());
            }
        }
        let before = shell.live_artwork_receipt();
        let initial = SchedulingSample::read(&before).ok_or("Missing scheduling counters")?;
        report["before_dispatch"] = before;
        if shell.artwork_transition_readiness() != Ready::Ready
            || initial.submitted != 0
            || initial.completed != 0
            || initial.in_flight != 0
            || initial.cancellation_requests != 0
            || initial.active
        {
            return Err(
                "Scheduling recipe requires initialized, paused, never-dispatched worker".into(),
            );
        }
        shell.set_artwork_active(true);
        loop {
            if Instant::now() >= deadline {
                return Err("Scheduling first queued Work deadline".into());
            }
            scheduling_frame(
                shell,
                window,
                now,
                poll_live,
                None,
                &mut interrupted,
                &mut unexpected,
            )?;
            frames += 1;
            let value = shell.live_artwork_receipt();
            let count = SchedulingSample::read(&value).ok_or("Missing queued Work counters")?;
            if shell.artwork_transition_readiness() == Ready::BlockedUnknown {
                return Err("Scheduling provider initialization or session is unknown".into());
            }
            if count.in_flight == 1 {
                report["owned_work_before_intent"] = value;
                break;
            }
        }
        let baseline = SchedulingSample::read(&report["owned_work_before_intent"])
            .ok_or("Missing owned Work sample")?;
        if baseline.submitted != 1
            || baseline.completed != 0
            || baseline.cancellation_requests != initial.cancellation_requests
        {
            return Err("Scheduling first Work changed before intent".into());
        }
        // Existing app-local editor intent only. No Controller::Start or Window
        // text_start call exists in this recipe. It grants no LG capability.
        shell.use_system_keyboard(true);
        shell.set_artwork_pause(PauseReason::KeyboardIntent, true);
        report["stage"] = json!("keyboard_intent_pause");
        report["paused_before_poll"] = shell.live_artwork_receipt();
        if !shell.drawer_state().system_keyboard() || shell.keyboard_transition_ready() {
            return Err("Queued Work did not defer local keyboard readiness".into());
        }
        scheduling_frame(
            shell,
            window,
            now,
            poll_live,
            None,
            &mut interrupted,
            &mut unexpected,
        )?;
        frames += 1;
        let paused = shell.live_artwork_receipt();
        let counts = SchedulingSample::read(&paused).ok_or("Missing paused Work sample")?;
        report["first_paused_frame"] = paused;
        if !counts.first_draining(baseline)
            || shell.artwork_transition_readiness() != Ready::Draining
        {
            // Honest bounded nonqualification if completion beat this sample;
            // never assert that Root/HBC was held merely because a job existed.
            return Err(
                "Queued Work settled before the required draining sample or counters changed"
                    .into(),
            );
        }
        // Back remains responsive without cancelling the worker. Manual keeps
        // scheduling paused after Back legitimately removes KeyboardIntent.
        shell.set_artwork_active(false);
        shell.back();
        report["back_while_work_owned"] =
            json!(shell.artwork_transition_readiness() == Ready::Draining);
        if shell.drawer_state().editing
            || shell.page() != Page::Applications
            || shell.preferences() != &preferences
            || !shell.take_requests().is_empty()
        {
            return Err("Local Back did not cancel only editor intent".into());
        }
        report["local_intent_cancelled"] = json!(true);
        report["stage"] = json!("poll_only_drain_after_local_Back");
        loop {
            if Instant::now() >= deadline {
                return Err("Scheduling known-closure observation deadline".into());
            }
            scheduling_frame(
                shell,
                window,
                now,
                poll_live,
                None,
                &mut interrupted,
                &mut unexpected,
            )?;
            frames += 1;
            let value = shell.live_artwork_receipt();
            let count = SchedulingSample::read(&value).ok_or("Missing drain sample")?;
            if !count.paused_from(baseline) || shell.preferences() != &preferences {
                return Err("Paused scheduling counters/preferences changed".into());
            }
            match shell.artwork_transition_readiness() {
                Ready::Ready => {
                    report["after_drain"] = value;
                    if count.completed != 1 || count.in_flight != 0 {
                        return Err("Readiness did not classify exactly first Work".into());
                    }
                    break;
                }
                Ready::Draining => (),
                Ready::Disabled | Ready::BlockedUnknown => {
                    return Err("Queued Work closure is unknown or runtime vanished".into())
                }
            }
        }
        shell.open(Page::Home);
        shell.set_artwork_active(true); // Clears only the recipe's Manual owner.
        if shell.artwork_transition_readiness() != Ready::Ready {
            return Err("Scheduling return did not retain classified readiness".into());
        }
        report["stage"] = json!("complete");
        report["qualified"] = json!(true);
        report["ready_after_classified_first_Work"] = json!(true);
        report["all_current_session_receipts_require_external_reconciliation"] = json!(true);
        report["measured_runtime_cancellation_requests_unchanged_zero"] = json!(true);
        Ok(())
    })();
    report["polled_drawn_frames"] = json!(frames);
    report["interrupted"] = json!(interrupted);
    report["unexpected_request_count"] = json!(unexpected);
    report["preferences_unchanged"] = json!(shell.preferences() == &preferences);
    report["error"] = json!(result.as_ref().err().map(|error| error.to_string()));
    result
}

pub fn run(
    shell: &mut Shell,
    window: &Window,
    output: &Path,
    now: &mut u32,
    mut poll_live: impl FnMut(&mut Shell),
) -> Result<Value, Box<dyn std::error::Error>> {
    let began = Instant::now();
    let deadline = began + LIMIT;
    let preferences = shell.preferences().clone();
    let mut stages = Vec::new();
    let mut interrupted = false;
    let mut unexpected_requests = 0usize;
    let scheduling_enabled =
        cfg!(target_arch = "arm") && plx_base::paths::app_id() == crate::identity::PROOF;
    let mut scheduling = Value::Null;
    let observed = (|| -> Result<(), Box<dyn std::error::Error>> {
        if scheduling_enabled {
            let result = scheduling_component(
                shell,
                window,
                deadline,
                now,
                &mut poll_live,
                &mut scheduling,
            );
            interrupted |= scheduling["interrupted"] == true;
            unexpected_requests +=
                scheduling["unexpected_request_count"].as_u64().unwrap_or(0) as usize;
            result?;
        }
        for (page, name) in [(Page::Home, "home"), (Page::Applications, "applications")] {
            shell.open(page);
            let mut focus_after_catalogue = None;
            let mut fresh_seen = false;
            loop {
                let frame_start = Instant::now();
                if frame_start >= deadline {
                    return Err("Installed artwork presentation deadline exceeded".into());
                }
                interrupted |= external_input();
                if interrupted {
                    return Err("Installed artwork interrupted by input or window departure".into());
                }
                poll_live(shell);
                *now = now.wrapping_add(17);
                shell.tick(*now, 1.0 / 60.0);
                unexpected_requests += shell.take_requests().len();
                if shell.preferences() != &preferences
                    || unexpected_requests != 0
                    || shell.should_exit()
                {
                    return Err(
                        "Installed artwork changed preferences or requested an action".into(),
                    );
                }
                let current = shell.artwork_catalogue_current();
                if current && !fresh_seen {
                    focus_after_catalogue = shell.focused_item();
                    fresh_seen = true;
                }
                if fresh_seen && shell.focused_item() != focus_after_catalogue {
                    return Err(
                        "Installed artwork arrival changed the fresh-catalogue focus".into(),
                    );
                }
                let before_draw = window.graphics_state();
                if before_draw.error != 0
                    || before_draw.scissor_enabled
                    || before_draw.framebuffer != 0
                    || before_draw.viewport != [0, 0, 1920, 1080]
                {
                    return Err("Installed artwork has a pending GL state error".into());
                }
                shell.draw();
                let graphics = window.graphics_state();
                if graphics.error != 0
                    || graphics.scissor_enabled
                    || graphics.framebuffer != 0
                    || graphics.viewport != [0, 0, 1920, 1080]
                {
                    return Err("Installed artwork draw target or GL state is unqualified".into());
                }
                let live = live_targets(shell, false);
                if current && fresh_seen && live.len() >= MIN_LIVE {
                    // Hash only the stage being saved, not every waiting frame.
                    let targets = live_targets(shell, true);
                    if targets.len() < MIN_LIVE {
                        return Err(
                            "Live artwork dimensions, hashes or drawing references changed".into(),
                        );
                    }
                    let receipt = shell.live_artwork_receipt();
                    if !backend_clean(&receipt) || receipt["runtime"]["active"] != true {
                        return Err(
                            "Installed artwork backend observed an allocation or GL failure".into(),
                        );
                    }
                    let handles = targets
                        .iter()
                        .map(|row| row["target"]["texture_handle"].as_u64().unwrap() as u32)
                        .collect::<Vec<_>>();
                    let validity = window.texture_validity(&handles);
                    let after_resources = window.graphics_state();
                    if after_resources.error != 0
                        || validity.as_array().is_none_or(|rows| {
                            rows.len() != handles.len()
                                || rows.iter().any(|row| row["valid"] != true)
                        })
                    {
                        return Err("Installed artwork live GL texture is unavailable".into());
                    }
                    let frame = format!("installed-artwork-{name}.ppm");
                    // Capture the just-drawn buffer before swap. No pixel oracle
                    // or external visual predicate can discard this raw frame.
                    capture(window, &output.join(&frame))?;
                    window.swap();
                    let stage = json!({"schema":1,"page":format!("{page:?}"),"frame":frame,
                        "catalogue_current":current,"focus_after_catalogue":focus_after_catalogue,
                        "focused_item":shell.focused_item(),"preferences_unchanged":true,
                        "cards":shell.icon_presentation_cards(),"live_targets":targets,
                        "fully_visible_live_count":handles.len(),"before_draw":before_draw,
                        "graphics":graphics,"resource_validity":validity,"after_resources":after_resources,
                        "runtime":receipt["runtime"],"textures":receipt["textures"],"gl":receipt["gl"],
                        "presentation":shell.presentation_probe()});
                    save(
                        &output.join(format!("installed-artwork-{name}.json")),
                        &stage,
                    )?;
                    stages.push(stage);
                    break;
                }
                window.swap();
                if let Some(left) = PERIOD.checked_sub(frame_start.elapsed()) {
                    std::thread::sleep(left);
                }
            }
        }
        Ok(())
    })();
    let mut error = observed.err().map(|error| error.to_string());
    // This flag prevents new dispatch before the first drain poll, including on
    // failure. The Provider still owns finishing/reconciling any submitted job.
    shell.set_artwork_active(false);
    let before_drain = shell.live_artwork_receipt();
    let drain_started = Instant::now();
    let mut drain_error = None;
    loop {
        interrupted |= external_input();
        let receipt = shell.live_artwork_receipt();
        if receipt["runtime"]["in_flight"].as_u64() == Some(0) {
            break;
        }
        if receipt["runtime"].is_null() {
            drain_error = Some("Installed artwork runtime disappeared".to_owned());
            break;
        }
        if Instant::now() >= deadline {
            drain_error = Some("Installed artwork drain deadline exceeded".to_owned());
            break;
        }
        poll_live(shell);
        *now = now.wrapping_add(17);
        shell.tick(*now, 1.0 / 60.0);
        unexpected_requests += shell.take_requests().len();
        std::thread::sleep(Duration::from_millis(5));
    }
    let before_finish = shell.live_artwork_receipt();
    let drained = before_finish["runtime"]["in_flight"].as_u64() == Some(0);
    let shutdown = shell.finish_live_artwork();
    interrupted |= external_input();
    let graphics_after_shutdown = window.graphics_state();
    let shutdown_qualified = shutdown.as_ref().is_some_and(|value| {
        value["joined"] == true
            && value["panicked"] == false
            && value["outstanding_resolved"] == true
            && matches!(
                value["settlement"].as_str(),
                Some("KnownClosed" | "NoExternalSession")
            )
            && value["closed"] == true
            && value["remaining_live_textures"] == 0
            && value["gpu_cleanup_qualified"] == true
            && value["runtime"]["in_flight"] == 0
            && value["runtime"]["resident_entries"] == 0
            && value["runtime"]["pending_results"] == 0
    }) && graphics_after_shutdown.error == 0;
    unexpected_requests += shell.take_requests().len();
    let preferences_unchanged = shell.preferences() == &preferences;
    let deadline_expired = Instant::now() >= deadline;
    if error.is_none() {
        error = drain_error.clone();
    }
    if error.is_none()
        && (!drained
            || !shutdown_qualified
            || !preferences_unchanged
            || unexpected_requests != 0
            || interrupted
            || shell.should_exit()
            || deadline_expired)
    {
        error = Some(
            "Installed artwork shutdown, invariants or operational deadline unqualified".into(),
        );
    }
    let qualified = error.is_none() && stages.len() == 2;
    let report = json!({"schema":1,"qualified":qualified,"error":error,
        "scope":"Two-page current live CPU/cache/GL drawing-reference observation through the configured installed provider",
        "deadline_ms":120000,"operation_deadline_expired":deadline_expired,
        "stages":stages,"transition_scheduling":scheduling,"transition_scheduling_enabled":scheduling_enabled,"preferences_unchanged":preferences_unchanged,
        "requests_empty":unexpected_requests==0,"unexpected_request_count":unexpected_requests,
        "interrupted":interrupted,"drain":{"active_disabled":true,"before":before_drain,
            "before_finish":before_finish,"in_flight_drained":drained,"error":drain_error,
            "reported_elapsed_ms":drain_started.elapsed().as_millis() as u64},
        "shutdown":shutdown,"shutdown_qualified":shutdown_qualified,
        "graphics_after_shutdown":graphics_after_shutdown,
        "reported_elapsed_ms":began.elapsed().as_millis() as u64,
        "root_cleanup_qualified":false,"normal_loader_qualified":false,
        "production_startup_qualified":false,"performance_qualified":false,
        "physical_input_qualified":false,"pixel_content_independently_verified":false,
        "hard_join_or_process_lifetime_bound":false});
    save(&output.join("installed-artwork-proof.json"), &report)?;
    Ok(report)
}

#[cfg(test)]
mod tests {
    use super::*;
    fn pair() -> (Value, Value) {
        (
            json!({"id":"fixture.app","rect":[600,300,200,120],"clip":[500,250,1000,500],
            "texture_handle":7,"width":128,"height":64}),
            json!({"source_key":{"app_id":"fixture.app"},"revision":1,
            "texture_handle":7,"width":128,"height":64,"encoded_sha256":"a".repeat(64)}),
        )
    }
    #[test]
    fn live_reference_requires_complete_visible_matching_card() {
        let (card, target) = pair();
        assert!(eligible(&card, &target));
        for (key, value) in [
            ("texture_handle", json!(0)),
            ("width", json!(127)),
            ("id", json!("fixture.other")),
            ("rect", json!([450, 300, 200, 120])),
            ("clip", json!([500, 250, 1000, 100])),
        ] {
            let mut changed = card.clone();
            changed[key] = value;
            assert!(!eligible(&changed, &target));
        }
        assert!(
            !eligible(&card, &Value::Null),
            "A packaged fallback is not a live target"
        );
    }
    #[test]
    fn live_shape_and_digest_cannot_be_coerced_or_exceed_decoder_bound() {
        let (card, target) = pair();
        for (key, value) in [
            ("texture_handle", json!(true)),
            ("width", json!(513)),
            ("height", json!(0)),
            ("revision", json!(0)),
            ("encoded_sha256", json!("A".repeat(64))),
        ] {
            let mut changed = target.clone();
            changed[key] = value;
            assert!(!eligible(&card, &changed));
        }
    }
    #[test]
    fn geometry_rejects_invalid_extents_and_viewport_overflow() {
        for value in [
            json!([0, 0, 0, 1]),
            json!([0, 0, -1, 1]),
            json!([0, 0, 1]),
            json!([true, 0, 1, 1]),
            json!([0, 0, null, 1]),
            json!([1e308, 0, 1e308, 1]),
        ] {
            assert!(rectangle(&value).is_none());
        }
        let (mut card, target) = pair();
        card["clip"] = json!([-1, 0, 1921, 1080]);
        assert!(!eligible(&card, &target));
        card["clip"] = json!([0, 0, 1920, 1080]);
        card["rect"] = json!([1800, 1000, 200, 120]);
        assert!(!eligible(&card, &target));
        assert!(contains(
            [0.0, 0.0, 1920.0, 1080.0],
            [0.0, 0.0, 1920.0, 1080.0]
        ));
    }
    fn scheduling_value() -> Value {
        json!({"runtime":{"submitted":1,"completed":0,"in_flight":1,"active":false,"cancellation_requests":0},"gl":{"uploads":0}})
    }
    #[test]
    fn scheduling_sample_requires_typed_native_counters_not_coerced_flags() {
        let original = scheduling_value();
        assert!(SchedulingSample::read(&original).is_some());
        for (path, value) in [
            ("/runtime/submitted", json!(true)),
            ("/runtime/in_flight", json!("1")),
            ("/runtime/active", json!(0)),
            ("/gl/uploads", Value::Null),
            ("/runtime/cancellation_requests", json!(false)),
        ] {
            let mut bad = original.clone();
            *bad.pointer_mut(path).unwrap() = value;
            assert!(SchedulingSample::read(&bad).is_none());
        }
    }
    #[test]
    fn first_draining_observation_rejects_completion_before_sample_without_inventing_remote_hold() {
        let sample = SchedulingSample::read(&scheduling_value()).unwrap();
        assert!(sample.first_draining(sample));
        let done = SchedulingSample {
            completed: 1,
            in_flight: 0,
            ..sample
        };
        assert!(done.paused_from(sample));
        assert!(!done.first_draining(sample));
        // This sample certifies only Work ownership. No HBC event or Root stage
        // is an input to this scheduler classifier, and none is inferred.
    }
    #[test]
    fn paused_observation_rejects_extra_submit_upload_or_effective_active() {
        let sample = SchedulingSample::read(&scheduling_value()).unwrap();
        for bad in [
            SchedulingSample {
                submitted: 2,
                ..sample
            },
            SchedulingSample {
                uploads: 1,
                ..sample
            },
            SchedulingSample {
                cancellation_requests: 1,
                ..sample
            },
            SchedulingSample {
                active: true,
                ..sample
            },
            SchedulingSample {
                in_flight: 2,
                ..sample
            },
            SchedulingSample {
                completed: 2,
                ..sample
            },
        ] {
            assert!(!bad.paused_from(sample));
        }
    }
}
