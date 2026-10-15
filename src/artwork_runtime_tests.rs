//! Host-only controlled Provider/Worker/cache integration. Synthetic PNG bytes
//! exercise the real bounded decoder; no filesystem, TV, Root or GL operation.
use super::*;
use crate::app_artwork::{sources_from_raw, EncodedFile, FileIdentity, Source, SourceKey};
use crate::artwork_worker::{Cancellation, Provider, ProviderError};
use serde_json::json;
use std::{
    sync::{
        atomic::{AtomicUsize, Ordering},
        mpsc, Arc,
    },
    thread,
};

const PNG: &[u8] = &[
    137, 80, 78, 71, 13, 10, 26, 10, 0, 0, 0, 13, 73, 72, 68, 82, 0, 0, 0, 2, 0, 0, 0, 1, 8, 6, 0,
    0, 0, 244, 34, 127, 138, 0, 0, 0, 14, 73, 68, 65, 84, 120, 156, 99, 224, 18, 145, 251, 15, 194,
    0, 8, 51, 2, 119, 73, 34, 188, 2, 0, 0, 0, 0, 73, 69, 78, 68, 174, 66, 96, 130,
];
struct Permit {
    inode: u64,
}
struct Gate {
    entered: mpsc::Sender<(SourceKey, Instant)>,
    permits: mpsc::Receiver<Permit>,
    dropped: mpsc::Sender<()>,
    calls: Arc<AtomicUsize>,
}
impl Provider for Gate {
    fn fetch(
        &mut self,
        source: &Source,
        deadline: Instant,
        cancellation: &Cancellation,
    ) -> Result<EncodedFile, ProviderError> {
        self.calls.fetch_add(1, Ordering::SeqCst);
        self.entered
            .send((source.key().clone(), deadline))
            .map_err(|_| ProviderError::Unavailable)?;
        loop {
            if cancellation.is_cancelled() {
                return Err(ProviderError::Cancelled);
            }
            let remaining = deadline.saturating_duration_since(Instant::now());
            if remaining.is_zero() {
                return Err(ProviderError::Expired);
            }
            match self
                .permits
                .recv_timeout(remaining.min(Duration::from_millis(5)))
            {
                Ok(permit) => {
                    let bytes = PNG.to_vec();
                    return Ok(EncodedFile {
                        identity: FileIdentity {
                            device: 1,
                            inode: permit.inode,
                            bytes: bytes.len() as u64,
                            mode: 0o100644,
                            uid: 501,
                            gid: 20,
                            modified_seconds: 1,
                            modified_nanos: 0,
                            changed_seconds: 1,
                            changed_nanos: 0,
                        },
                        digest: plx_base::sha256::sha256(&bytes),
                        bytes,
                        png_dimensions: Some([2, 1]),
                    });
                }
                Err(mpsc::RecvTimeoutError::Timeout) => (),
                Err(mpsc::RecvTimeoutError::Disconnected) => {
                    return Err(ProviderError::Unavailable)
                }
            }
        }
    }
}
impl Drop for Gate {
    fn drop(&mut self) {
        let _ = self.dropped.send(());
    }
}
struct Controls {
    entered: mpsc::Receiver<(SourceKey, Instant)>,
    permits: mpsc::Sender<Permit>,
    dropped: mpsc::Receiver<()>,
    calls: Arc<AtomicUsize>,
}
impl Controls {
    fn started(&self) -> SourceKey {
        self.started_with_deadline().0
    }
    fn started_with_deadline(&self) -> (SourceKey, Instant) {
        self.entered
            .recv_timeout(Duration::from_secs(2))
            .expect("controlled fetch starts")
    }
    fn release(&self, inode: u64) {
        self.permits
            .send(Permit { inode })
            .expect("controlled fetch exists");
    }
}
fn controlled_worker() -> (Worker, Controls) {
    let (entered_tx, entered_rx) = mpsc::channel();
    let (permit_tx, permit_rx) = mpsc::channel();
    let (drop_tx, drop_rx) = mpsc::channel();
    let calls = Arc::new(AtomicUsize::new(0));
    let worker_calls = calls.clone();
    let worker = Worker::start(move || Gate {
        entered: entered_tx,
        permits: permit_rx,
        dropped: drop_tx,
        calls: worker_calls,
    })
    .unwrap();
    (
        worker,
        Controls {
            entered: entered_rx,
            permits: permit_tx,
            dropped: drop_rx,
            calls,
        },
    )
}
fn runtime() -> (Runtime, Controls) {
    let (worker, control) = controlled_worker();
    (
        Runtime::new(worker, cache::Config::default()).unwrap(),
        control,
    )
}

#[test]
fn fetch_budget_rejects_zero_or_over_thirty_seconds_without_dispatch() {
    for budget in [
        Duration::ZERO,
        Duration::from_secs(30) + Duration::from_nanos(1),
    ] {
        let (worker, control) = controlled_worker();
        assert!(matches!(
            Runtime::with_fetch_budget(worker, cache::Config::default(), budget),
            Err(cache::Error::InvalidConfig)
        ));
        control
            .dropped
            .recv_timeout(Duration::from_secs(2))
            .unwrap();
        assert_eq!(control.calls.load(Ordering::SeqCst), 0);
        assert!(control.entered.try_recv().is_err());
    }
}

#[test]
fn default_and_configured_budgets_reach_actual_worker_provider_deadlines() {
    for configured in [None, Some(Duration::from_secs(30))] {
        let (worker, control) = controlled_worker();
        let mut runtime = match configured {
            None => Runtime::new(worker, cache::Config::default()),
            Some(budget) => Runtime::with_fetch_budget(worker, cache::Config::default(), budget),
        }
        .unwrap();
        runtime
            .update_catalogue(Some(&sources(&[("fixture.budget", "1")])))
            .unwrap();
        let before_submit = Instant::now();
        assert!(runtime.frame(0).changed.is_none());
        let (source, deadline) = control.started_with_deadline();
        let after_arrival = Instant::now();
        let expected = configured.unwrap_or(Duration::from_secs(2));
        assert_eq!(source.app_id, "fixture.budget");
        assert!(deadline >= before_submit + expected);
        assert!(deadline <= after_arrival + expected);
        assert!(
            runtime.resident("fixture.budget").is_none(),
            "No result was released"
        );
        control.release(70);
        let applied = drive_until(&mut runtime, 1, |r| r.completed == 1);
        assert_eq!(applied.changed.len(), 1);
        assert_eq!(
            runtime
                .resident("fixture.budget")
                .unwrap()
                .fingerprint
                .file_identity
                .inode,
            70
        );
        let done = runtime.finish();
        assert!(done.joined && !done.panicked && done.outstanding_resolved);
    }
}

#[test]
fn configured_long_budget_does_not_publish_cancelled_inflight_content() {
    let (worker, control) = controlled_worker();
    let mut runtime =
        Runtime::with_fetch_budget(worker, cache::Config::default(), Duration::from_secs(30))
            .unwrap();
    runtime
        .update_catalogue(Some(&sources(&[("fixture.cancel_budget", "1")])))
        .unwrap();
    assert!(runtime.frame(0).changed.is_none());
    control.started_with_deadline();
    assert!(runtime.resident("fixture.cancel_budget").is_none());
    runtime.update_catalogue(None).unwrap();
    // Even a provider racing to return released bytes must not make withdrawn
    // work resident: cancellation is also checked by the real Worker.
    control.release(71);
    let applied = drive_until(&mut runtime, 1, |r| r.completed == 1);
    assert!(applied.changed.is_empty());
    assert!(runtime.resident("fixture.cancel_budget").is_none());
    assert!(runtime.source("fixture.cancel_budget").is_none());
    let done = runtime.finish();
    assert!(done.joined && !done.panicked && done.outstanding_resolved);
    assert!(done.evicted.is_empty());
}
#[test]
fn visible_priority_survives_first_catalogue_and_withdrawal_recovery() {
    for withdraw in [false, true] {
        let (mut runtime, control) = runtime();
        runtime.set_interest(&["fixture.z".into()], &[]).unwrap();
        let map = sources(&[("fixture.a", "1"), ("fixture.z", "1")]);
        runtime.update_catalogue(Some(&map)).unwrap();
        if withdraw {
            runtime.update_catalogue(None).unwrap();
            runtime.update_catalogue(Some(&map)).unwrap();
        }
        runtime.frame(0);
        assert_eq!(control.started().app_id, "fixture.z");
        let done = runtime.finish();
        assert!(done.joined && done.outstanding_resolved);
    }
}
fn sources(rows: &[(&str, &str)]) -> SourceMap {
    let apps=rows.iter().map(|(id,version)|json!({"id":id,"title":"Fixture","type":"native","visible":true,"version":version,"folderPath":format!("/usr/palm/applications/{id}"),"icon":"icon.png"})).collect::<Vec<_>>();
    let points = rows
        .iter()
        .map(|(id, _)| json!({"id":id}))
        .collect::<Vec<_>>();
    sources_from_raw(&json!({"replies":{"apps":{"returnValue":true,"apps":apps},"launch_points":{"returnValue":true,"launchPoints":points}}}),crate::identity::PROOF).unwrap()
}
#[derive(Default)]
struct Changes {
    changed: Vec<ResidentKey>,
    evicted: Vec<ResidentKey>,
}
fn drive_until(runtime: &mut Runtime, now_ms: u64, ready: impl Fn(&Runtime) -> bool) -> Changes {
    let deadline = Instant::now() + Duration::from_secs(2);
    let mut events = Changes::default();
    loop {
        let frame = runtime.frame(now_ms);
        events.evicted.extend(frame.evicted);
        if let Some(key) = frame.changed {
            events.changed.push(key);
        }
        if ready(runtime) {
            return events;
        }
        assert!(
            Instant::now() < deadline,
            "controlled completion did not reach expected state"
        );
        thread::yield_now();
    }
}
fn first_resident(runtime: &mut Runtime, control: &Controls, id: &str) -> ResidentKey {
    runtime
        .update_catalogue(Some(&sources(&[(id, "1")])))
        .unwrap();
    assert!(runtime.frame(0).changed.is_none());
    assert_eq!(control.started().app_id, id);
    control.release(1);
    let events = drive_until(runtime, 1, |r| r.completed == 1);
    assert_eq!(events.changed.len(), 1);
    assert!(events.evicted.is_empty());
    events.changed[0].clone()
}

#[test]
fn decoded_arrival_becomes_resident_and_exact_changed_revision() {
    let (mut runtime, control) = runtime();
    let key = first_resident(&mut runtime, &control, "fixture.arrival");
    let resident = runtime.resident("fixture.arrival").unwrap();
    assert_eq!(resident.key, &key);
    assert_eq!((resident.width, resident.height), (2, 1));
    assert_eq!(resident.rgba, [10, 20, 30, 255].repeat(2));
    assert_eq!(runtime.submitted, 1);
    assert_eq!(runtime.cache.stats().resident_bytes, 8);
    let done = runtime.finish();
    assert!(done.joined && !done.panicked && done.outstanding_resolved);
    assert_eq!(done.evicted, vec![key]);
    control
        .dropped
        .recv_timeout(Duration::from_secs(2))
        .unwrap();
}
#[test]
fn unchanged_catalogue_refresh_during_fetch_preserves_useful_work() {
    let (mut runtime, control) = runtime();
    let map = sources(&[("fixture.unchanged", "1")]);
    runtime.update_catalogue(Some(&map)).unwrap();
    runtime.frame(0);
    let requested = control.started();
    runtime.update_catalogue(Some(&map)).unwrap();
    assert_eq!(runtime.cache.stats().in_flight, 1);
    control.release(10);
    let events = drive_until(&mut runtime, 1, |r| r.completed == 1);
    assert_eq!(events.changed.len(), 1);
    assert_eq!(events.changed[0].source, requested);
    assert_eq!(runtime.stale, 0);
    assert_eq!(runtime.submitted, 1);
    assert_eq!(
        runtime
            .resident("fixture.unchanged")
            .unwrap()
            .fingerprint
            .file_identity
            .inode,
        10
    );
    runtime.finish();
}
#[test]
fn delayed_old_result_after_readd_or_version_change_cannot_resurrect() {
    for replace_version in [false, true] {
        let (mut runtime, control) = runtime();
        let old = sources(&[("fixture.replace", "1")]);
        runtime.update_catalogue(Some(&old)).unwrap();
        runtime.frame(0);
        assert_eq!(control.started().version, "1");
        let current = if replace_version {
            sources(&[("fixture.replace", "2")])
        } else {
            runtime.update_catalogue(Some(&SourceMap::new())).unwrap();
            old.clone()
        };
        runtime.update_catalogue(Some(&current)).unwrap();
        control.release(11);
        let stale = drive_until(&mut runtime, 1, |r| r.completed == 1 && r.submitted == 2);
        assert!(stale.changed.is_empty());
        assert!(runtime.resident("fixture.replace").is_none());
        assert_eq!(runtime.stale, 1);
        let fresh = control.started();
        assert_eq!(fresh.version, if replace_version { "2" } else { "1" });
        control.release(22);
        let accepted = drive_until(&mut runtime, 2, |r| r.completed == 2);
        assert_eq!(accepted.changed.len(), 1);
        assert_eq!(
            runtime
                .resident("fixture.replace")
                .unwrap()
                .fingerprint
                .file_identity
                .inode,
            22
        );
        runtime.finish();
    }
}
#[test]
fn none_catalogue_withdraws_resident_and_returns_exact_eviction() {
    let (mut runtime, control) = runtime();
    let old = first_resident(&mut runtime, &control, "fixture.withdraw");
    runtime.update_catalogue(None).unwrap();
    let frame = runtime.frame(2);
    assert_eq!(frame.evicted, vec![old]);
    assert!(frame.changed.is_none());
    assert!(runtime.resident("fixture.withdraw").is_none());
    assert!(runtime.source("fixture.withdraw").is_none());
    assert_eq!(runtime.cache.stats().entries, 0);
    assert_eq!(runtime.cache.stats().resident_bytes, 0);
    assert_eq!(control.calls.load(Ordering::SeqCst), 1);
    runtime.finish();
}
#[test]
fn suspension_drains_into_bounded_pending_without_apply_or_dispatch() {
    let (mut runtime, control) = runtime();
    runtime
        .update_catalogue(Some(&sources(&[("fixture.a", "1"), ("fixture.b", "1")])))
        .unwrap();
    runtime.frame(0);
    assert_eq!(control.started().app_id, "fixture.a");
    runtime.set_active(false);
    control.release(1);
    let held = drive_until(&mut runtime, 1, |r| r.completed == 1);
    assert!(held.changed.is_empty());
    assert!(runtime.resident("fixture.a").is_none());
    assert_eq!(runtime.submitted, 1);
    let stats = runtime.cache.stats();
    assert_eq!(stats.pending_results, 1);
    assert!(stats.pending_result_bytes <= cache::MAX_RESULT_BYTES);
    assert_eq!(stats.in_flight, 0);
    assert!(control.entered.try_recv().is_err());
    runtime.set_active(true);
    let frame = runtime.frame(2);
    assert_eq!(
        frame.changed.as_ref().map(|key| key.source.app_id.as_str()),
        Some("fixture.a")
    );
    assert_eq!(control.started().app_id, "fixture.b");
    assert_eq!(runtime.submitted, 2);
    let done = runtime.finish();
    assert!(done.joined && done.outstanding_resolved);
    assert_eq!(done.evicted.len(), 1);
}
#[test]
fn identical_digest_refresh_updates_file_identity_without_new_revision() {
    let (mut runtime, control) = runtime();
    let old = first_resident(&mut runtime, &control, "fixture.refresh");
    assert!(runtime.request_refresh("fixture.refresh"));
    runtime.frame(2);
    control.started();
    control.release(99);
    let events = drive_until(&mut runtime, 3, |r| r.completed == 2);
    assert!(events.changed.is_empty() && events.evicted.is_empty());
    let view = runtime.resident("fixture.refresh").unwrap();
    assert_eq!(view.key, &old);
    assert_eq!(view.fingerprint.file_identity.inode, 99);
    runtime.finish();
}
#[test]
fn finish_inflight_joins_acknowledges_cache_work_and_evicts_exact_resident() {
    let (mut runtime, control) = runtime();
    runtime
        .update_catalogue(Some(&sources(&[("fixture.a", "1"), ("fixture.b", "1")])))
        .unwrap();
    runtime.frame(0);
    assert_eq!(control.started().app_id, "fixture.a");
    control.release(1);
    let events = drive_until(&mut runtime, 1, |r| r.completed == 1 && r.submitted == 2);
    let resident = events.changed[0].clone();
    assert_eq!(control.started().app_id, "fixture.b");
    let done = runtime.finish();
    assert!(done.joined && !done.panicked && done.outstanding_resolved);
    assert_eq!(done.evicted, vec![resident]);
    control
        .dropped
        .recv_timeout(Duration::from_secs(2))
        .unwrap();
    let stats = runtime.cache.stats();
    assert!(stats.closed);
    assert_eq!(
        (stats.in_flight, stats.pending_results, stats.resident_bytes),
        (0, 0, 0)
    );
    let frame = runtime.frame(2);
    assert!(frame.changed.is_none() && frame.evicted.is_empty());
    assert_eq!(control.calls.load(Ordering::SeqCst), 2);
    assert!(runtime.finish().evicted.is_empty());
}
