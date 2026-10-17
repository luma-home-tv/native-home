use super::*;
use std::{
    os::unix::fs::{symlink, PermissionsExt},
    sync::atomic::{AtomicU64, Ordering},
    time::{Duration, Instant},
};
// Explicit host fixture owner injection; the public loader remains Root0:GID0.
fn fixture_load(
    assets: &Path,
    device: bool,
    interactive: bool,
    app: &str,
    cli: bool,
) -> io::Result<Option<Marker>> {
    load_with_owner(
        assets,
        device,
        interactive,
        app,
        cli,
        (unsafe { libc::geteuid() }, unsafe { libc::getegid() }),
    )
}
struct Temp(std::path::PathBuf);
impl Temp {
    fn new() -> Self {
        static N: AtomicU64 = AtomicU64::new(0);
        let p = std::env::temp_dir().join(format!(
            "luma-passive-host-{}-{}",
            std::process::id(),
            N.fetch_add(1, Ordering::Relaxed)
        ));
        std::fs::create_dir(&p).unwrap();
        std::fs::set_permissions(&p, std::fs::Permissions::from_mode(0o700)).unwrap();
        Self(p)
    }
    fn assets(&self) {
        for n in [
            NAME,
            "interactive-session.json",
            "artwork-cache-producer.json",
        ] {
            let raw = if n == NAME {
                r#"{"schema":2,"lifetime_seconds":240,"snapshot_interval_ms":1000}"#
            } else {
                "{}"
            };
            std::fs::write(self.0.join(n), raw).unwrap();
            std::fs::set_permissions(self.0.join(n), std::fs::Permissions::from_mode(0o644))
                .unwrap();
        }
    }
}
impl Drop for Temp {
    fn drop(&mut self) {
        std::fs::remove_dir_all(&self.0).unwrap();
    }
}
#[test]
fn marker_off_creates_no_metric_files_and_strict_selected_modes_refuse() {
    let t = Temp::new();
    assert!(
        fixture_load(&t.0, true, true, crate::identity::PREVIEW, false)
            .unwrap()
            .is_none()
    );
    assert_eq!(std::fs::read_dir(&t.0).unwrap().count(), 0);
    t.assets();
    assert!(
        fixture_load(&t.0, true, true, crate::identity::PREVIEW, false)
            .unwrap()
            .is_some()
    );
    for (device, interactive, app, cli) in [
        (false, true, crate::identity::PREVIEW, false),
        (true, false, crate::identity::PREVIEW, false),
        (true, true, crate::identity::PROOF, false),
        (true, true, crate::identity::PREVIEW, true),
    ] {
        assert!(fixture_load(&t.0, device, interactive, app, cli).is_err());
    }
    std::fs::write(t.0.join("session-proof.json"), "{}").unwrap();
    assert!(fixture_load(&t.0, true, true, crate::identity::PREVIEW, false).is_err());
}
#[test]
fn marker_closed_schema_bounds_and_symlinks_fail_closed() {
    for raw in [
        r#"{}"#,
        r#"{"schema":2,"schema":2,"lifetime_seconds":240,"snapshot_interval_ms":1000}"#,
        r#"{"schema":2,"lifetime_seconds":241,"snapshot_interval_ms":1000}"#,
        r#"{"schema":2,"lifetime_seconds":240,"snapshot_interval_ms":10}"#,
        r#"{"schema":true,"lifetime_seconds":240,"snapshot_interval_ms":1000}"#,
        r#"{"schema":2,"lifetime_seconds":240,"snapshot_interval_ms":1000,"Proof":true}"#,
    ] {
        assert!(Marker::parse(raw.as_bytes()).is_err());
    }
    assert!(Marker::parse(&[b'x'; 257]).is_err());
    let t = Temp::new();
    t.assets();
    std::fs::remove_file(t.0.join(NAME)).unwrap();
    symlink(t.0.join("missing"), t.0.join(NAME)).unwrap();
    assert!(fixture_load(&t.0, true, true, crate::identity::PREVIEW, false).is_err());
}
#[test]
fn actual_normal_selection_requires_all_workers_capabilities_and_cache() {
    let p = NormalProfile {
        live_worker: true,
        action_worker: true,
        pointer_worker: true,
        description_worker: true,
        app_launches: true,
        home_launches: true,
        ordinary_cache: true,
    };
    assert!(p.validate().is_ok());
    for i in 0..7 {
        let mut q = p;
        match i {
            0 => q.live_worker = false,
            1 => q.action_worker = false,
            2 => q.pointer_worker = false,
            3 => q.description_worker = false,
            4 => q.app_launches = false,
            5 => q.home_launches = false,
            _ => q.ordinary_cache = false,
        };
        assert!(q.validate().is_err());
    }
}
#[test]
fn phase_and_whole_session_monitor_semantics_are_explicit() {
    let mut m = crate::performance::Monitor::default();
    m.input(4, Duration::from_millis(2));
    m.presented(
        Duration::from_millis(10),
        Duration::from_millis(3),
        Duration::from_millis(1),
    );
    m.suspended();
    let r = snapshot(1.0, "Home", Some("apps"), "None", false, true, m.report());
    assert_eq!(r["phase"], "idle_black");
    assert_eq!(r["metrics"]["frames"], 1);
    assert_eq!(
        r["metrics"]["dequeued_input_to_next_swap_submission"]["samples"],
        1
    );
    assert_eq!(r["physical_or_GPU_completion_qualified"], false);
    assert_eq!(
        snapshot(2.0, "Home", None, "None", true, true, m.report())["phase"],
        "hidden"
    );
    let huge = "x".repeat(257);
    assert_eq!(
        snapshot(2.0, "Home", Some(&huge), "None", false, false, m.report())["focus_unavailable"],
        true
    );
}
#[test]
fn one_slot_queue_drops_without_waiting_for_worker_io() {
    let (tx, rx) = mpsc::sync_channel(1);
    let (release, gate) = mpsc::channel();
    let worker = thread::spawn(move || {
        let _ = gate.recv_timeout(Duration::from_secs(1));
        let mut r = WriterReport::default();
        for _ in rx {
            r.published += 1;
        }
        r
    });
    let mut w = Writer {
        tx: Some(tx),
        worker: Some(worker),
        dropped: 0,
    };
    w.submit(json!({"n":1}));
    w.submit(json!({"n":2}));
    release.send(()).unwrap();
    let r = w.finish();
    assert_eq!(r.published, 1);
    assert_eq!(r.dropped, 1);
    assert!(r.worker_joined && !r.worker_panicked);
}
#[test]
fn real_snapshot_worker_owns_json_io_and_joins_with_exact_epoch() {
    let t = Temp::new();
    let mut w = Writer::start(&t.0).unwrap();
    w.submit(json!({"sample":1}));
    let r = w.finish();
    assert!(r.worker_joined && r.io_on_snapshot_worker && !r.worker_panicked);
    assert_eq!(r.published, 1);
    assert!(r.error.is_none());
    let p = t.0.join("performance-current.json");
    assert_eq!(
        serde_json::from_slice::<Value>(&std::fs::read(&p).unwrap()).unwrap(),
        json!({"sample":1})
    );
    assert_eq!(p.metadata().unwrap().mode() & 0o7777, 0o600);
    assert!(!t.0.join("performance-current-next.json").exists());
    assert!(Writer::start(&t.0).is_err());
}
#[test]
fn oversize_refuses_before_file_creation_and_existing_or_linked_paths_are_not_adopted() {
    let t = Temp::new();
    let mut w = Writer::start(&t.0).unwrap();
    w.submit(json!({"huge":"x".repeat(CAP+1)}));
    let r = w.finish();
    assert_eq!(r.published, 0);
    assert!(r.error.is_some());
    assert_eq!(std::fs::read_dir(&t.0).unwrap().count(), 0);
    symlink(t.0.join("absent"), t.0.join("performance-current.json")).unwrap();
    assert!(Writer::start(&t.0).is_err());
    let link = Temp::new();
    let p = link.0.join("dir-link");
    symlink(&t.0, &p).unwrap();
    assert!(Writer::start(&p).is_err());
}
#[test]
fn changed_current_epoch_is_never_overwritten() {
    let t = Temp::new();
    let mut w = Writer::start(&t.0).unwrap();
    w.submit(json!({"sample":1}));
    let p = t.0.join("performance-current.json");
    let until = Instant::now() + Duration::from_secs(2);
    while !p.exists() {
        assert!(Instant::now() < until);
        thread::sleep(Duration::from_millis(1));
    }
    std::fs::write(&p, b"external changed epoch").unwrap();
    w.submit(json!({"sample":2}));
    let r = w.finish();
    assert!(r.error.is_some());
    assert_eq!(std::fs::read(&p).unwrap(), b"external changed epoch");
}
#[test]
fn installed_marker_policy_is_root_zero_group_zero_not_device_flag() {
    let t = Temp::new();
    t.assets();
    let actual_owner = (unsafe { libc::geteuid() }, unsafe { libc::getegid() });
    assert!(
        fixture_load(&t.0, true, true, crate::identity::PREVIEW, false)
            .unwrap()
            .is_some()
    );
    let result = load(&t.0, true, true, crate::identity::PREVIEW, false);
    if actual_owner == (0, 0) {
        assert!(result.unwrap().is_some());
    } else {
        assert!(result.is_err());
    }
    assert!(load_with_owner(
        &t.0,
        true,
        true,
        crate::identity::PREVIEW,
        false,
        (actual_owner.0, actual_owner.1.wrapping_add(1))
    )
    .is_err());
    assert!(load_with_owner(
        &t.0,
        true,
        true,
        crate::identity::PREVIEW,
        false,
        (actual_owner.0.wrapping_add(1), actual_owner.1)
    )
    .is_err());
}
#[test]
fn directory_permission_change_before_submit_refuses_without_creating_file() {
    let t = Temp::new();
    let mut w = Writer::start(&t.0).unwrap();
    std::fs::set_permissions(&t.0, std::fs::Permissions::from_mode(0o755)).unwrap();
    w.submit(json!({"sample":1}));
    let r = w.finish();
    assert_eq!(r.published, 0);
    assert!(r.error.is_some() && r.worker_joined && !r.worker_panicked);
    assert_eq!(std::fs::read_dir(&t.0).unwrap().count(), 0);
    assert_eq!(t.0.metadata().unwrap().mode() & 0o7777, 0o755);
}
#[test]
fn replaced_named_directory_refuses_without_following_replacement() {
    let t = Temp::new();
    let mut w = Writer::start(&t.0).unwrap();
    let held = t.0.with_extension("old-held");
    std::fs::rename(&t.0, &held).unwrap();
    std::fs::create_dir(&t.0).unwrap();
    std::fs::set_permissions(&t.0, std::fs::Permissions::from_mode(0o700)).unwrap();
    w.submit(json!({"sample":1}));
    let r = w.finish();
    assert_eq!(r.published, 0);
    assert!(r.error.is_some());
    assert_eq!(std::fs::read_dir(&t.0).unwrap().count(), 0);
    assert_eq!(std::fs::read_dir(&held).unwrap().count(), 0);
    std::fs::remove_dir(&held).unwrap();
}
#[test]
fn actual_worker_rechecks_private_temp_mode_after_write_and_never_repairs_it() {
    let t = Temp::new();
    let mut w = Writer::start_inner(
        &t.0,
        Some(Box::new(|stage, file| {
            if stage == PublishStage::AfterWrite {
                file.set_permissions(std::fs::Permissions::from_mode(0o644))
                    .unwrap();
            }
        })),
    )
    .unwrap();
    w.submit(json!({"sample":1}));
    let r = w.finish();
    assert_eq!(r.published, 0);
    assert!(r.error.is_some());
    assert!(!t.0.join("performance-current.json").exists());
    assert_eq!(
        t.0.join("performance-current-next.json")
            .metadata()
            .unwrap()
            .mode()
            & 0o7777,
        0o644
    );
}
#[test]
fn actual_worker_rechecks_named_temp_and_preserves_replacement() {
    let t = Temp::new();
    let path = t.0.join("performance-current-next.json");
    let named = path.clone();
    let mut w = Writer::start_inner(
        &t.0,
        Some(Box::new(move |stage, _| {
            if stage == PublishStage::BeforeRename {
                std::fs::remove_file(&named).unwrap();
                std::fs::write(&named, b"external owned-name replacement").unwrap();
                std::fs::set_permissions(&named, std::fs::Permissions::from_mode(0o600)).unwrap();
            }
        })),
    )
    .unwrap();
    w.submit(json!({"sample":1}));
    let r = w.finish();
    assert_eq!(r.published, 0);
    assert!(r.error.is_some());
    assert!(!t.0.join("performance-current.json").exists());
    assert_eq!(
        std::fs::read(path).unwrap(),
        b"external owned-name replacement"
    );
}
#[test]
fn actual_worker_rechecks_directory_immediately_before_rename() {
    let t = Temp::new();
    let path = t.0.clone();
    let mut w = Writer::start_inner(
        &t.0,
        Some(Box::new(move |stage, _| {
            if stage == PublishStage::BeforeRename {
                std::fs::set_permissions(&path, std::fs::Permissions::from_mode(0o755)).unwrap();
            }
        })),
    )
    .unwrap();
    w.submit(json!({"sample":1}));
    let r = w.finish();
    assert_eq!(r.published, 0);
    assert!(r.error.is_some());
    assert!(!t.0.join("performance-current.json").exists());
    assert!(t.0.join("performance-current-next.json").exists());
}
#[test]
fn actual_worker_refuses_new_current_name_before_rename() {
    let t = Temp::new();
    let current = t.0.join("performance-current.json");
    let path = current.clone();
    let mut w = Writer::start_inner(
        &t.0,
        Some(Box::new(move |stage, _| {
            if stage == PublishStage::BeforeRename {
                std::fs::write(&path, b"external current").unwrap();
            }
        })),
    )
    .unwrap();
    w.submit(json!({"sample":1}));
    let r = w.finish();
    assert_eq!(r.published, 0);
    assert!(r.error.is_some());
    assert_eq!(std::fs::read(current).unwrap(), b"external current");
}
#[test]
fn actual_worker_verifies_published_mode_without_repair_or_unlink() {
    let t = Temp::new();
    let mut w = Writer::start_inner(
        &t.0,
        Some(Box::new(|stage, file| {
            if stage == PublishStage::AfterRename {
                file.set_permissions(std::fs::Permissions::from_mode(0o644))
                    .unwrap();
            }
        })),
    )
    .unwrap();
    w.submit(json!({"sample":1}));
    let r = w.finish();
    assert_eq!(r.published, 0);
    assert!(r.error.is_some());
    let current = t.0.join("performance-current.json");
    assert_eq!(current.metadata().unwrap().mode() & 0o7777, 0o644);
    assert_eq!(
        serde_json::from_slice::<Value>(&std::fs::read(current).unwrap()).unwrap(),
        json!({"sample":1})
    );
}
#[test]
fn own_directory_timestamp_changes_do_not_block_second_publication() {
    let t = Temp::new();
    let mut w = Writer::start(&t.0).unwrap();
    w.submit(json!({"sample":1}));
    let path = t.0.join("performance-current.json");
    let until = Instant::now() + Duration::from_secs(2);
    while !path.exists() {
        assert!(Instant::now() < until);
        thread::sleep(Duration::from_millis(1));
    }
    w.submit(json!({"sample":2}));
    let r = w.finish();
    assert_eq!(r.published, 2);
    assert!(r.error.is_none());
}
#[test]
fn actual_blocked_worker_has_one_slot_backpressure_and_drop_joins() {
    use std::sync::{mpsc, Arc, Mutex};
    let t = Temp::new();
    let (entered_tx, entered_rx) = mpsc::channel();
    let (release_tx, release_rx) = mpsc::channel();
    let release = Arc::new(Mutex::new(Some(release_rx)));
    let mut w = Writer::start_inner(
        &t.0,
        Some(Box::new(move |stage, _| {
            if stage == PublishStage::AfterWrite {
                if let Some(rx) = release.lock().unwrap().take() {
                    entered_tx.send(()).unwrap();
                    rx.recv_timeout(Duration::from_secs(3)).unwrap();
                }
            }
        })),
    )
    .unwrap();
    w.submit(json!({"sample":1}));
    entered_rx.recv_timeout(Duration::from_secs(2)).unwrap();
    w.submit(json!({"sample":2}));
    w.submit(json!({"sample":3}));
    assert_eq!(w.dropped, 1);
    let (dropping_tx, dropping_rx) = mpsc::channel();
    let (done_tx, done_rx) = mpsc::channel();
    let handle = thread::spawn(move || {
        dropping_tx.send(()).unwrap();
        drop(w);
        done_tx.send(()).unwrap();
    });
    dropping_rx.recv_timeout(Duration::from_secs(2)).unwrap();
    assert!(done_rx.recv_timeout(Duration::from_millis(50)).is_err());
    release_tx.send(()).unwrap();
    done_rx.recv_timeout(Duration::from_secs(2)).unwrap();
    handle.join().unwrap();
    assert_eq!(
        serde_json::from_slice::<Value>(
            &std::fs::read(t.0.join("performance-current.json")).unwrap()
        )
        .unwrap(),
        json!({"sample":2})
    );
}
#[test]
fn actual_worker_refuses_temp_hardlink_after_write_without_unlink() {
    let t = Temp::new();
    let path = t.0.join("performance-current-next.json");
    let extra = t.0.join("external-link");
    let link = extra.clone();
    let mut w = Writer::start_inner(
        &t.0,
        Some(Box::new(move |stage, _| {
            if stage == PublishStage::AfterWrite {
                std::fs::hard_link(&path, &link).unwrap();
            }
        })),
    )
    .unwrap();
    w.submit(json!({"sample":1}));
    let r = w.finish();
    assert!(r.error.is_some());
    assert_eq!(r.published, 0);
    assert_eq!(extra.metadata().unwrap().nlink(), 2);
    assert!(!t.0.join("performance-current.json").exists());
}
#[test]
fn actual_worker_refuses_named_temp_symlink_after_write_without_following() {
    let t = Temp::new();
    let target = Temp::new();
    let victim = target.0.join("unchanged");
    std::fs::write(&victim, b"external target").unwrap();
    let path = t.0.join("performance-current-next.json");
    let victim_path = victim.clone();
    let mut w = Writer::start_inner(
        &t.0,
        Some(Box::new(move |stage, _| {
            if stage == PublishStage::BeforeRename {
                std::fs::remove_file(&path).unwrap();
                symlink(&victim_path, &path).unwrap();
            }
        })),
    )
    .unwrap();
    w.submit(json!({"sample":1}));
    let r = w.finish();
    assert!(r.error.is_some());
    assert_eq!(r.published, 0);
    assert_eq!(std::fs::read(&victim).unwrap(), b"external target");
    assert!(t.0.join("performance-current-next.json").is_symlink());
}

#[test]
fn old_marker_refuses_and_typed_diagnostics_encode_on_existing_worker() {
    assert!(
        Marker::parse(br#"{"schema":1,"lifetime_seconds":240,"snapshot_interval_ms":1000}"#)
            .is_err()
    );
    let t = Temp::new();
    let caller = std::thread::current().id();
    let observed = std::sync::Arc::new(std::sync::Mutex::new(None));
    let capture = observed.clone();
    let hook: PublishHook = Box::new(move |stage, _| {
        if stage == PublishStage::AfterWrite {
            *capture.lock().unwrap() = Some(std::thread::current().id());
        }
    });
    let mut w = Writer::start_inner(&t.0, Some(hook)).unwrap();
    w.submit_observed(
        crate::draw_snapshot::base(
            0.,
            crate::draw_diagnostics::Page::Home,
            false,
            false,
            crate::performance::Monitor::default().report(),
        ),
        crate::draw_diagnostics::Summary::default(),
    );
    let report = w.finish();
    assert!(report.error.is_none());
    assert!(report.worker_joined);
    assert_eq!(report.draw_diagnostic_snapshots, 1);
    assert_ne!(observed.lock().unwrap().unwrap(), caller);
    let raw = std::fs::read(t.0.join("performance-current.json")).unwrap();
    assert!(crate::draw_snapshot::parse(&raw).is_ok());
}

#[test]
fn typed_queue_is_nonblocking_and_disconnection_and_bad_envelope_stay_explicit() {
    let (tx, rx) = mpsc::sync_channel(1);
    let mut w = Writer {
        tx: Some(tx),
        worker: None,
        dropped: 0,
    };
    let base = || {
        crate::draw_snapshot::base(
            0.,
            crate::draw_diagnostics::Page::Home,
            false,
            false,
            crate::performance::Monitor::default().report(),
        )
    };
    w.submit_observed(base(), crate::draw_diagnostics::Summary::default());
    w.submit_observed(base(), crate::draw_diagnostics::Summary::default());
    assert_eq!(w.dropped, 1);
    drop(rx);
    w.submit_observed(base(), crate::draw_diagnostics::Summary::default());
    assert_eq!(w.dropped, 2);
    let t = Temp::new();
    let mut worker = Writer::start(&t.0).unwrap();
    let mut bad = base();
    bad["private_field"] = json!("x".repeat(CAP));
    worker.submit_observed(bad, crate::draw_diagnostics::Summary::default());
    let report = worker.finish();
    assert!(report.worker_joined && report.error.is_some());
    assert_eq!(report.published, 0);
    assert_eq!(std::fs::read_dir(&t.0).unwrap().count(), 0);
}

fn observed_fixture(count: u64) -> (Value, crate::draw_diagnostics::Summary) {
    use crate::draw_diagnostics::{Clock, Collector, Page, Phase, Probe, Timed};
    struct ClockStep(u64);
    impl Clock for ClockStep {
        fn micros(&mut self) -> u64 {
            self.0 += 1;
            self.0
        }
    }
    let mut collector = Collector::default();
    let mut monitor = crate::performance::Monitor::default();
    for i in 0..count {
        let mut probe = Timed::new(ClockStep(0));
        for phase in [Phase::Clear, Phase::Chrome, Phase::Content, Phase::Finalize] {
            probe.cut(phase);
        }
        let draw = Duration::from_micros(20000 + i * 10000);
        let at = Duration::from_millis(30 * i);
        collector.record(
            Some(collector.context(Page::Home)),
            probe.finish(),
            draw,
            Duration::ZERO,
            at,
        );
        monitor.presented(at, draw, Duration::ZERO);
    }
    (
        crate::draw_snapshot::base(1., Page::Home, false, false, monitor.report()),
        collector.summary(),
    )
}
#[test]
fn final_dropped_summary_never_qualifies_saturated_or_unpublished_tail() {
    use std::sync::{Arc, Mutex};
    for saturated in [false, true] {
        let (mut old_base, mut old) = observed_fixture(1);
        let (mut new_base, mut final_summary) = observed_fixture(2);
        if saturated {
            // Review-only numeric boundary, not a 240-second reachability
            // claim. Actual Collector overflow controls are in its own tests.
            for (base, summary) in [
                (&mut old_base, &mut old),
                (&mut new_base, &mut final_summary),
            ] {
                summary.frames_seen = u64::MAX;
                for row in summary.top.iter_mut().flatten() {
                    row.frame_sequence = u64::MAX;
                    row.context.as_mut().unwrap().first_draw = false;
                }
                base["metrics"]["frames"] = json!(u64::MAX);
                for key in ["draw_submission", "swap_call"] {
                    base["metrics"][key]["samples"] = json!(u64::MAX);
                    base["metrics"][key]["mean_ms"] = json!(
                        if key == "draw_submission" {
                            if summary.slow_seen == 1 {
                                20.
                            } else {
                                50.
                            }
                        } else {
                            0.
                        } / u64::MAX as f64
                    );
                    for q in ["p50_upper_ms", "p95_upper_ms", "p99_upper_ms"] {
                        base["metrics"][key][q] = json!(1);
                    }
                }
            }
            final_summary.saturated = true;
        }
        assert!(old.valid() && final_summary.valid());
        assert_ne!(old, final_summary);
        let t = Temp::new();
        let (entered_tx, entered_rx) = mpsc::channel();
        let (release_tx, release_rx) = mpsc::channel();
        let release = Arc::new(Mutex::new(Some(release_rx)));
        let mut writer = Writer::start_inner(
            &t.0,
            Some(Box::new(move |stage, _| {
                if stage == PublishStage::AfterWrite {
                    if let Some(rx) = release.lock().unwrap().take() {
                        entered_tx.send(()).unwrap();
                        rx.recv_timeout(Duration::from_secs(3)).unwrap();
                    }
                }
            })),
        )
        .unwrap();
        writer.submit_observed(old_base.clone(), old);
        entered_rx.recv_timeout(Duration::from_secs(2)).unwrap();
        writer.submit_observed(old_base, old);
        writer.submit_observed(new_base, final_summary);
        assert_eq!(writer.dropped, 1);
        release_tx.send(()).unwrap();
        let report = writer.finish();
        assert!(report.worker_joined && !report.worker_panicked && report.error.is_none());
        assert_eq!(report.draw_diagnostic_snapshots, 2);
        let raw = std::fs::read(t.0.join("performance-current.json")).unwrap();
        let saved = crate::draw_snapshot::parse(&raw).unwrap();
        assert_eq!(saved.diagnostics(), &old);
        assert_eq!(
            report.draw_diagnostic_frames_published == final_summary.frames_seen,
            saturated
        );
        assert!(!report.draw_diagnostics_complete(&final_summary));
        assert!(report.draw_diagnostics_complete(&old));
    }
}
#[test]
fn genuine_final_publication_qualifies_only_unsaturated_healthy_collector() {
    let (base, summary) = observed_fixture(2);
    let t = Temp::new();
    let mut writer = Writer::start(&t.0).unwrap();
    writer.submit_observed(base, summary);
    let mut report = writer.finish();
    assert!(report.draw_diagnostics_complete(&summary));
    let saved =
        crate::draw_snapshot::parse(&std::fs::read(t.0.join("performance-current.json")).unwrap())
            .unwrap();
    assert_eq!(saved.diagnostics(), &summary);
    let mut saturated = summary;
    saturated.saturated = true;
    assert!(!report.draw_diagnostics_complete(&saturated));
    report.worker_joined = false;
    assert!(!report.draw_diagnostics_complete(&summary));
    report.worker_joined = true;
    report.worker_panicked = true;
    assert!(!report.draw_diagnostics_complete(&summary));
    report.worker_panicked = false;
    report.error = Some("Observed publication failure".into());
    assert!(!report.draw_diagnostics_complete(&summary));
    report.error = None;
    report.draw_diagnostic_snapshots = 0;
    assert!(!report.draw_diagnostics_complete(&summary));
}
