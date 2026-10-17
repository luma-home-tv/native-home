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
                r#"{"schema":1,"lifetime_seconds":240,"snapshot_interval_ms":1000}"#
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
        r#"{"schema":1,"schema":1,"lifetime_seconds":240,"snapshot_interval_ms":1000}"#,
        r#"{"schema":1,"lifetime_seconds":241,"snapshot_interval_ms":1000}"#,
        r#"{"schema":1,"lifetime_seconds":240,"snapshot_interval_ms":10}"#,
        r#"{"schema":true,"lifetime_seconds":240,"snapshot_interval_ms":1000}"#,
        r#"{"schema":1,"lifetime_seconds":240,"snapshot_interval_ms":1000,"Proof":true}"#,
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
