use super::*;
use std::{
    os::unix::fs::{symlink, PermissionsExt},
    sync::atomic::{AtomicU64, Ordering},
    time::{Duration, Instant},
};
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
    assert!(load(&t.0, true, true, crate::identity::PREVIEW, false)
        .unwrap()
        .is_none());
    assert_eq!(std::fs::read_dir(&t.0).unwrap().count(), 0);
    t.assets();
    assert!(load(&t.0, true, true, crate::identity::PREVIEW, false)
        .unwrap()
        .is_some());
    for (device, interactive, app, cli) in [
        (false, true, crate::identity::PREVIEW, false),
        (true, false, crate::identity::PREVIEW, false),
        (true, true, crate::identity::PROOF, false),
        (true, true, crate::identity::PREVIEW, true),
    ] {
        assert!(load(&t.0, device, interactive, app, cli).is_err());
    }
    std::fs::write(t.0.join("session-proof.json"), "{}").unwrap();
    assert!(load(&t.0, true, true, crate::identity::PREVIEW, false).is_err());
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
    assert!(load(&t.0, true, true, crate::identity::PREVIEW, false).is_err());
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
