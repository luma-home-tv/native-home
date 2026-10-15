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
    let observed = (|| -> Result<(), Box<dyn std::error::Error>> {
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
        "stages":stages,"preferences_unchanged":preferences_unchanged,
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
}
