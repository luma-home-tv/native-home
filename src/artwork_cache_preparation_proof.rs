//! Explicit Preview preparation recipe. No normal-startup selection or prepared
//! artwork upload comes from this module; Shell may upload packaged fallbacks.
//! Caller must keep RootRuntime/actions/persistence
//! off. Every owned job is polled and the worker joined before report I/O/errors.
use crate::{
    artwork_cache_preparation::{Cursor, Plan},
    artwork_cache_preparation_runner::Runner,
    broker_protocol::Marker,
    model::Page,
    runtime::Window,
    shell::Shell,
};
use serde_json::{json, Value};
use std::{
    io::Write,
    os::unix::fs::OpenOptionsExt,
    path::Path,
    time::{Duration, Instant},
};

const PERIOD: Duration = Duration::from_millis(17);
const CATALOGUE_WAIT: Duration = Duration::from_secs(15);
const REPORT_LIMIT: usize = 2 * 1024 * 1024;

fn interrupted() -> bool {
    for _ in 0..256 {
        let Some(event) = crate::runtime::poll() else {
            return false;
        };
        if matches!(event.kind, 1..=8 | 10 | 11) {
            return true;
        }
    }
    // A saturated event queue is a reason to stop, never permission to dispatch.
    true
}
fn save(output: &Path, report: &Value) -> Result<(), Box<dyn std::error::Error>> {
    let mut raw = serde_json::to_vec(report)?;
    raw.push(b'\n');
    if raw.len() > REPORT_LIMIT {
        return Err("Preparation report exceeds bound".into());
    }
    let mut file = std::fs::OpenOptions::new()
        .create_new(true)
        .write(true)
        .mode(0o600)
        .custom_flags(libc::O_NOFOLLOW | libc::O_CLOEXEC)
        .open(output.join("artwork-cache-preparation.json"))?;
    file.write_all(&raw)?;
    file.sync_all()?;
    Ok(())
}
fn present(shell: &mut Shell, window: &Window, now_ms: u32) -> bool {
    shell.tick(now_ms, 1.0 / 60.0);
    if window.graphics_state().error != 0 {
        return false;
    }
    shell.draw();
    let state = window.graphics_state();
    if state.error != 0
        || state.scissor_enabled
        || state.framebuffer != 0
        || state.viewport != [0, 0, 1920, 1080]
    {
        return false;
    }
    window.swap();
    true
}

/// A bounded local batch report is not external Root/helper closure or final
/// cache-body/all-catalogue readiness. The native operator must grade those
/// independently before privileged cleanup or replacing a retained package.
#[allow(clippy::too_many_arguments)]
pub fn run(
    shell: &mut Shell,
    window: &Window,
    output: &Path,
    marker: &Marker,
    cursor: Option<&Cursor>,
    preflight: Value,
    live_origin: Instant,
    mut poll_live: impl FnMut(&mut Shell),
) -> Result<Value, Box<dyn std::error::Error>> {
    let preferences = shell.preferences().clone();
    let mut report = json!({"schema":1,"profile":"preview","recipe":"catalogue_cache_preparation",
        "qualified":false,"preflight":preflight,"resource_worker_created":false,
        "resource_dispatch_enabled":false,"submitted_jobs":0,"root_exec_attempts":0,
        "outcome":"preflight_declined","capture_requested":false,
        "normal_loader_qualified":false,"production_startup_qualified":false,
        "performance_qualified":false,"external_process_closure_verified":false,
        "all_catalogue_prepared":false,"current_cache_body_coverage_verified":false,
        "native_pixels_qualified":false,"Root_authority_granted":false});
    if !crate::preview_artwork_admission::admitted(&report["preflight"]) {
        save(output, &report)?;
        return Ok(report);
    }
    shell.open(Page::Home);
    let catalogue_deadline = Instant::now() + CATALOGUE_WAIT;
    let mut departed = false;
    let mut caller_failed = false;
    let mut unexpected_requests = 0usize;
    let plan = loop {
        departed |= interrupted();
        poll_live(shell);
        unexpected_requests += shell.take_requests().len();
        caller_failed |=
            unexpected_requests != 0 || shell.preferences() != &preferences || shell.should_exit();
        if departed || caller_failed || Instant::now() >= catalogue_deadline {
            report["outcome"] = json!(if departed {
                "input_or_departure_before_worker"
            } else if caller_failed {
                "caller_failure_before_worker"
            } else {
                "catalogue_unavailable"
            });
            report["interrupted"] = json!(departed);
            report["caller_failed"] = json!(caller_failed);
            report["unexpected_requests"] = json!(unexpected_requests);
            save(output, &report)?;
            return Ok(report);
        }
        let now_ms = live_origin.elapsed().as_millis() as u32;
        if let Ok(plan) = Plan::from_current(shell.cache_preparation_catalogue(), now_ms) {
            break plan;
        }
        caller_failed |= !present(shell, window, now_ms);
        std::thread::sleep(PERIOD);
    };
    let expected_keyset = plan.keyset_sha256().to_owned();
    departed |= interrupted();
    if departed {
        report["outcome"] = json!("input_or_departure_before_worker");
        report["interrupted"] = json!(true);
        save(output, &report)?;
        return Ok(report);
    }
    let mut runner = match Runner::start(
        shell.cache_preparation_catalogue(),
        live_origin.elapsed().as_millis() as u32,
        cursor,
        marker.clone(),
        output.to_owned(),
    ) {
        Ok(runner) => runner,
        Err(error) => {
            report["outcome"] = json!("plan_or_worker_creation_refused");
            report["error"] = json!(error.to_string().chars().take(256).collect::<String>());
            save(output, &report)?;
            return Ok(report);
        }
    };
    report["resource_worker_created"] = json!(true);
    report["resource_dispatch_enabled"] = json!(true);
    report["root_exec_attempts"] = Value::Null; // External evidence must count actual dispatches.
                                                // From this point, do not return or perform fallible report I/O while an
                                                // owned job is outstanding. A logical deadline does not cancel a syscall.
    loop {
        let frame_began = Instant::now();
        departed |= interrupted();
        poll_live(shell);
        unexpected_requests += shell.take_requests().len();
        caller_failed |=
            unexpected_requests != 0 || shell.preferences() != &preferences || shell.should_exit();
        if caller_failed {
            runner.stop_for_caller_failure();
        }
        runner.poll(
            shell.cache_preparation_catalogue(),
            live_origin.elapsed().as_millis() as u32,
            departed,
        );
        if runner.stopped().is_some() && !runner.has_outstanding() {
            break;
        }
        if !departed && !caller_failed {
            caller_failed |= !present(shell, window, live_origin.elapsed().as_millis() as u32);
            if caller_failed {
                runner.stop_for_caller_failure();
            }
        }
        if let Some(left) = PERIOD.checked_sub(frame_began.elapsed()) {
            std::thread::sleep(left);
        }
    }
    let local = runner.finish(shell.cache_preparation_catalogue(), || {
        departed |= interrupted();
        (live_origin.elapsed().as_millis() as u32, departed)
    });
    // The loop establishes stopped/no-outstanding. None can now only mean the
    // worker was already taken; it cannot conceal an active Work on this path.
    let local = local.ok_or("Preparation finish returned no report after drain")?;
    // The borrow held across join cannot see newer live updates. Refresh again
    // and compare the full current metadata before reporting a current batch.
    departed |= interrupted();
    poll_live(shell);
    let final_current = Plan::from_current(
        shell.cache_preparation_catalogue(),
        live_origin.elapsed().as_millis() as u32,
    )
    .is_ok_and(|plan| plan.keyset_sha256() == expected_keyset);
    unexpected_requests += shell.take_requests().len();
    let preferences_unchanged = shell.preferences() == &preferences;
    caller_failed |= unexpected_requests != 0 || !preferences_unchanged || shell.should_exit();
    let local_ready = local.batch_prepared_locally && final_current && !departed && !caller_failed;
    report["submitted_jobs"] = json!(local.submitted);
    report["outcome"] = json!(if !local.local_lifecycle_known {
        "lifecycle_unknown"
    } else if local_ready {
        "batch_prepared_locally"
    } else {
        "partial_or_stopped"
    });
    report["post_join_live_polled"] = json!(true);
    report["post_join_current_keyset_matches"] = json!(final_current);
    report["new_service_reply_after_join_qualified"] = json!(false);
    report["batch_prepared_locally_current"] = json!(local_ready);
    report["interrupted"] = json!(departed);
    report["caller_failed"] = json!(caller_failed);
    report["unexpected_requests"] = json!(unexpected_requests);
    report["preferences_unchanged"] = json!(preferences_unchanged);
    report["local"] = serde_json::to_value(local)?;
    save(output, &report)?;
    Ok(report)
}
