//! Real existing Worker+bounded decoder, controlled provider observations only.
//! Fixture publications do not exercise Root, durable cache, or native authority.
use super::*;
use crate::{
    app_artwork::{self, EncodedFile, FileIdentity},
    artwork_preparation_events::Publication,
    artwork_worker::{Cancellation, Provider, ProviderError},
};
use serde_json::json;
use std::sync::{
    mpsc::{self, SyncSender},
    Arc, Condvar, Mutex,
};
#[derive(Clone, Copy)]
enum Case {
    Good,
    PublishedHeld,
    Refusal,
    Missing,
    BadIndex,
    BadSequence,
    BadKey,
    BadDigest,
    PublishFailed,
    BadDecode,
    Unknown,
}
struct Gate {
    entered: SyncSender<()>,
    released: Arc<(Mutex<bool>, Condvar)>,
}
struct FixtureProvider {
    events: SyncSender<Attempt>,
    cases: Vec<Case>,
    n: usize,
    gate: Option<Gate>,
    calls: Arc<Mutex<Vec<crate::app_artwork::SourceKey>>>,
}
fn png() -> Vec<u8> {
    vec![
        137, 80, 78, 71, 13, 10, 26, 10, 0, 0, 0, 13, 73, 72, 68, 82, 0, 0, 0, 2, 0, 0, 0, 1, 8, 6,
        0, 0, 0, 244, 34, 127, 138, 0, 0, 0, 17, 73, 68, 65, 84, 120, 156, 99, 224, 18, 145, 99,
        208, 48, 178, 105, 0, 0, 3, 199, 1, 83, 206, 227, 180, 69, 0, 0, 0, 0, 73, 69, 78, 68, 174,
        66, 96, 130,
    ]
}
fn encoded(bytes: Vec<u8>) -> EncodedFile {
    EncodedFile {
        identity: FileIdentity {
            device: 1,
            inode: 1,
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
        png_dimensions: None,
    }
}
impl Provider for FixtureProvider {
    fn initial_settlement(&self) -> SessionSettlement {
        SessionSettlement::NoExternalSession
    }
    fn transition_settlement(&self) -> SessionSettlement {
        if matches!(self.cases[self.n - 1], Case::Unknown) {
            SessionSettlement::Unknown
        } else {
            SessionSettlement::KnownClosed
        }
    }
    fn fetch(
        &mut self,
        source: &Source,
        _: Instant,
        _: &Cancellation,
    ) -> Result<EncodedFile, ProviderError> {
        self.calls.lock().unwrap().push(source.key().clone());
        let case = self.cases[self.n];
        self.n += 1;
        if !matches!(case, Case::PublishedHeld) {
            hold(self.gate.take());
        }
        let file = encoded(if matches!(case, Case::BadDecode) {
            b"not an image".to_vec()
        } else {
            png()
        });
        let mut event = Attempt {
            sequence: self.n as u8,
            source_key: source.key().clone(),
            observation_index: Some(self.n as u8),
            current_ticket_resolved: true,
            settlement: if matches!(case, Case::Unknown) {
                Settlement::Unknown
            } else {
                Settlement::KnownClosed
            },
            encoded_returned: !matches!(case, Case::Refusal),
            provider_error: matches!(case, Case::Refusal).then(|| "Denied".to_owned()),
            publication: (!matches!(case, Case::Refusal)).then(|| Publication {
                encoded_sha256: file.digest,
                source_identity: file.identity.clone(),
                producer: crate::durable_icon_cache::ProducerVersion::new(
                    &"a".repeat(40),
                    [1; 32],
                    [2; 32],
                )
                .unwrap(),
                encoded_bytes: file.bytes.len(),
                succeeded: !matches!(case, Case::PublishFailed),
                receipt_written: true,
            }),
        };
        match case {
            Case::BadIndex => event.observation_index = Some(16),
            Case::BadSequence => event.sequence = 1,
            Case::BadKey => event.source_key.version = "other".into(),
            Case::BadDigest => event.publication.as_mut().unwrap().encoded_sha256 = [0; 32],
            _ => {}
        }
        if !matches!(case, Case::Missing) {
            self.events.try_send(event).unwrap();
        }
        if matches!(case, Case::PublishedHeld) {
            hold(self.gate.take());
        }
        if matches!(case, Case::Refusal) {
            Err(ProviderError::Denied)
        } else {
            Ok(file)
        }
    }
}
fn hold(gate: Option<Gate>) {
    if let Some(gate) = gate {
        gate.entered.send(()).unwrap();
        let (lock, condition) = &*gate.released;
        let ready = lock.lock().unwrap();
        let (ready, timeout) = condition
            .wait_timeout_while(ready, Duration::from_secs(2), |v| !*v)
            .unwrap();
        assert!(
            !timeout.timed_out() && *ready,
            "test must release held actual fetch"
        );
    }
}
fn map(count: usize) -> SourceMap {
    let apps = (0..count).map(|i| {let id=format!("fixture.{i:03}"); json!({"id":id,"title":"Fixture","visible":true,"type":"native","version":"1","folderPath":format!("/usr/palm/applications/{id}"),"icon":"icon.png"})}).collect::<Vec<_>>();
    let points = apps
        .iter()
        .map(|v| json!({"id":v["id"]}))
        .collect::<Vec<_>>();
    app_artwork::sources_from_raw(&json!({"replies":{"apps":{"returnValue":true,"apps":apps},"launch_points":{"returnValue":true,"launchPoints":points}}}),crate::identity::PREVIEW).unwrap()
}
fn view(map: &SourceMap, stamp: u32) -> Catalogue<'_> {
    Catalogue::new(Some(map), Some(stamp), false, true)
}
type Calls = Arc<Mutex<Vec<crate::app_artwork::SourceKey>>>;
fn setup(
    map: &SourceMap,
    cases: Vec<Case>,
    gate: Option<Gate>,
) -> (Runner, Calls, SyncSender<Attempt>) {
    let mut plan = Plan::from_current(view(map, 0), 0).unwrap();
    let batch = plan.next_batch(view(map, 0), 0, None).unwrap();
    let (tx, rx) = mpsc::sync_channel(1);
    let spare = tx.clone();
    let calls = Arc::new(Mutex::new(Vec::new()));
    let observed = calls.clone();
    let worker = Worker::start(move || FixtureProvider {
        events: tx,
        cases,
        n: 0,
        gate,
        calls: observed,
    })
    .unwrap();
    (
        Runner::with_worker(plan, batch, worker, rx, Instant::now()),
        calls,
        spare,
    )
}
fn step_until(runner: &mut Runner, map: &SourceMap, target: Step) {
    let end = Instant::now() + Duration::from_secs(2);
    loop {
        if runner.poll(view(map, 0), 0, false) == target {
            return;
        }
        assert!(Instant::now() < end);
        std::thread::sleep(Duration::from_millis(1));
    }
}
fn finish(runner: &mut Runner, map: &SourceMap) -> Report {
    let end = Instant::now() + Duration::from_secs(2);
    while runner.stopped().is_none() || runner.has_outstanding() {
        runner.poll(view(map, 0), 0, false);
        assert!(Instant::now() < end);
        std::thread::sleep(Duration::from_millis(1));
    }
    runner.finish(view(map, 0), || (0, false)).unwrap()
}
#[test]
fn first_batch_executes_sixteen_exact_keys_once_without_gl_or_catalogue_completeness() {
    let map = map(24);
    let (mut r, calls, _) = setup(&map, vec![Case::Good; 16], None);
    let report = finish(&mut r, &map);
    let calls = calls.lock().unwrap();
    assert_eq!(calls.len(), 16);
    assert_eq!(report.submitted, 16);
    assert_eq!(report.prepared_locally, 16);
    assert!(report.batch_prepared_locally && report.local_lifecycle_known && report.joined);
    assert!(
        !report.all_catalogue_prepared
            && !report.native_pixels_qualified
            && !report.root_authority_granted
    );
    assert_eq!(
        calls.as_slice(),
        report
            .jobs
            .iter()
            .map(|j| j.source_key.clone())
            .collect::<Vec<_>>()
            .as_slice()
    );
    assert!(report
        .jobs
        .iter()
        .all(|j| j.worker_status == "Decoded" && j.event_paired));
}
#[test]
fn known_refusal_and_published_but_undecodable_or_failed_publication_are_partial_not_unknown() {
    let map = map(4);
    let (mut r, _, _) = setup(
        &map,
        vec![
            Case::Refusal,
            Case::BadDecode,
            Case::PublishFailed,
            Case::Good,
        ],
        None,
    );
    let report = finish(&mut r, &map);
    assert!(report.local_lifecycle_known);
    assert!(!report.batch_prepared_locally);
    assert_eq!(report.prepared_locally, 1);
    assert_eq!(report.submitted, 4);
    assert_eq!(report.jobs[1].worker_status, "DecodeFailed");
    assert!(
        report.jobs[1]
            .attempt
            .as_ref()
            .unwrap()
            .publication
            .as_ref()
            .unwrap()
            .succeeded
    );
    assert!(!report.jobs[1].prepared_locally);
}
#[test]
fn consumed_prior_success_cannot_cover_next_missing_event_or_skipped_fetch_shape() {
    let map = map(3);
    let (mut r, calls, _) = setup(&map, vec![Case::Good, Case::Missing, Case::Good], None);
    let report = finish(&mut r, &map);
    assert_eq!(calls.lock().unwrap().len(), 2);
    assert_eq!(report.stopped, Stop::Unknown);
    assert_eq!(report.jobs.len(), 2);
    assert!(report.jobs[1].attempt.is_none());
    assert_eq!(report.jobs[1].worker_settlement, Settlement::KnownClosed);
    assert!(!report.local_lifecycle_known);
}
#[test]
fn wrong_sequence_index_key_or_unknown_settlement_never_realigns_on_later_jobs() {
    for bad in [
        Case::BadSequence,
        Case::BadIndex,
        Case::BadKey,
        Case::Unknown,
    ] {
        let map = map(3);
        let (mut r, calls, _) = setup(&map, vec![Case::Good, bad, Case::Good], None);
        let report = finish(&mut r, &map);
        assert_eq!(calls.lock().unwrap().len(), 2);
        assert_eq!(report.stopped, Stop::Unknown);
        assert!(!report.local_lifecycle_known);
    }
}
#[test]
fn unsolicited_event_before_next_submit_is_not_reused_as_a_work_receipt() {
    let map = map(2);
    let (mut r, calls, tx) = setup(&map, vec![Case::Good; 2], None);
    step_until(&mut r, &map, Step::Completed);
    let mut stale = r.records()[0].attempt.as_ref().unwrap().clone();
    stale.sequence = 2;
    stale.observation_index = Some(2);
    stale.source_key = map["fixture.001"].preferred().unwrap().key().clone();
    tx.try_send(stale).unwrap();
    assert_eq!(r.poll(view(&map, 0), 0, false), Step::Stopped);
    let report = r.finish(view(&map, 0), || (0, false)).unwrap();
    assert_eq!(calls.lock().unwrap().len(), 1);
    assert_eq!(report.unexpected_events.len(), 1);
    assert!(!report.local_lifecycle_known);
}
type Release = Arc<(Mutex<bool>, Condvar)>;
fn held() -> (Gate, Receiver<()>, Release) {
    let (tx, rx) = mpsc::sync_channel(1);
    let released = Arc::new((Mutex::new(false), Condvar::new()));
    (
        Gate {
            entered: tx,
            released: released.clone(),
        },
        rx,
        released,
    )
}
fn release(value: &Release) {
    *value.0.lock().unwrap() = true;
    value.1.notify_all();
}
#[test]
fn departure_drains_without_cancel_and_finish_cannot_drop_a_held_work() {
    let map = map(2);
    let (gate, entered, released) = held();
    let (mut r, calls, _) = setup(&map, vec![Case::Good; 2], Some(gate));
    step_until(&mut r, &map, Step::Submitted);
    entered.recv_timeout(Duration::from_secs(2)).unwrap();
    assert_eq!(r.poll(view(&map, 0), 0, true), Step::Waiting);
    assert!(r.finish(view(&map, 0), || (0, false)).is_none());
    release(&released);
    let report = finish(&mut r, &map);
    assert_eq!(report.stopped, Stop::InputOrDeparture);
    assert!(report.local_lifecycle_known);
    assert_eq!(calls.lock().unwrap().len(), 1);
    assert_eq!(report.jobs[0].worker_status, "Decoded");
    assert!(!report.batch_prepared_locally);
}
#[test]
fn caller_failure_drains_the_actual_job_without_claiming_input_or_departure() {
    let map = map(2);
    let (gate, entered, released) = held();
    let (mut r, calls, _) = setup(&map, vec![Case::Good; 2], Some(gate));
    step_until(&mut r, &map, Step::Submitted);
    entered.recv_timeout(Duration::from_secs(2)).unwrap();
    r.stop_for_caller_failure();
    assert_eq!(r.poll(view(&map, 0), 0, false), Step::Waiting);
    assert!(r.finish(view(&map, 0), || (0, false)).is_none());
    release(&released);
    let report = finish(&mut r, &map);
    assert_eq!(report.stopped, Stop::CallerFailure);
    assert!(report.local_lifecycle_known);
    assert_eq!(calls.lock().unwrap().len(), 1);
    assert_eq!(report.jobs[0].worker_status, "Decoded");
    assert!(!report.batch_prepared_locally);
}
#[test]
fn empty_enumeration_is_known_no_work_without_a_prepared_batch_claim() {
    let map = map(0);
    let (mut r, calls, _) = setup(&map, vec![], None);
    let report = finish(&mut r, &map);
    assert_eq!(report.submitted, 0);
    assert!(calls.lock().unwrap().is_empty());
    assert!(report.local_lifecycle_known);
    assert!(!report.batch_prepared_locally);
    assert!(!report.all_catalogue_prepared);
}
#[test]
fn stale_catalogue_with_held_published_result_withdraws_current_coverage_and_never_retries_aba() {
    let map = map(2);
    let (gate, entered, released) = held();
    let (mut r, calls, _) = setup(&map, vec![Case::Good; 2], Some(gate));
    step_until(&mut r, &map, Step::Submitted);
    entered.recv_timeout(Duration::from_secs(2)).unwrap();
    r.poll(view(&map, 0), 15001, false);
    release(&released);
    let report = finish(&mut r, &map);
    assert_eq!(report.stopped, Stop::CatalogueWithdrawn);
    assert_eq!(calls.lock().unwrap().len(), 1);
    assert!(report.local_lifecycle_known);
    assert!(!report.jobs[0].catalogue_still_current && !report.jobs[0].prepared_locally);
}
#[test]
fn fingerprint_contradiction_cannot_turn_publication_into_prepared_coverage() {
    let map = map(1);
    let (mut r, _, _) = setup(&map, vec![Case::BadDigest], None);
    let report = finish(&mut r, &map);
    assert!(report.local_lifecycle_known);
    assert_eq!(report.prepared_locally, 0);
    assert!(!report.batch_prepared_locally);
}
#[test]
fn original_dispatch_budget_is_not_reset_and_existing_work_still_drains() {
    let map = map(2);
    let (gate, entered, released) = held();
    let (mut r, calls, _) = setup(&map, vec![Case::Good; 2], Some(gate));
    step_until(&mut r, &map, Step::Submitted);
    entered.recv_timeout(Duration::from_secs(2)).unwrap();
    r.began = Instant::now() - (BATCH_BUDGET - JOB_BUDGET - DRAIN_RESERVE);
    r.poll(view(&map, 0), 0, false);
    assert!(r.finish(view(&map, 0), || (0, false)).is_none());
    release(&released);
    let report = finish(&mut r, &map);
    assert_eq!(calls.lock().unwrap().len(), 1);
    assert_eq!(report.stopped, Stop::Budget);
    assert!(report.local_lifecycle_known && !report.batch_prepared_locally);
}
#[test]
fn late_departure_after_final_completion_revokes_full_batch_claim_before_finish() {
    let map = map(1);
    let (mut r, _, _) = setup(&map, vec![Case::Good], None);
    step_until(&mut r, &map, Step::Completed);
    r.poll(view(&map, 0), 0, true);
    let report = r.finish(view(&map, 0), || (0, false)).unwrap();
    assert_eq!(report.stopped, Stop::InputOrDeparture);
    assert!(report.local_lifecycle_known && !report.batch_prepared_locally);
}
#[test]
fn post_join_clock_and_departure_observation_cannot_inherit_pre_join_freshness() {
    for departed in [false, true] {
        let map = map(1);
        let (mut r, _, _) = setup(&map, vec![Case::Good], None);
        step_until(&mut r, &map, Step::Completed);
        let mut count = 0;
        let report = r
            .finish(view(&map, 0), || {
                count += 1;
                if count == 1 {
                    (0, false)
                } else {
                    (15001, departed)
                }
            })
            .unwrap();
        assert_eq!(count, 2);
        assert!(report.local_lifecycle_known);
        assert!(!report.batch_prepared_locally);
        assert!(matches!(
            report.stopped,
            Stop::CatalogueWithdrawn | Stop::InputOrDeparture
        ));
    }
}

#[test]
fn publication_then_actual_worker_cancellation_keeps_closure_but_never_prepared_pixels() {
    let map = map(2);
    let (gate, entered, released) = held();
    let (mut r, calls, _) = setup(&map, vec![Case::PublishedHeld, Case::Good], Some(gate));
    step_until(&mut r, &map, Step::Submitted);
    entered.recv_timeout(Duration::from_secs(2)).unwrap();
    assert!(r.worker.as_mut().unwrap().cancel()); // Test only; Runner never calls this API.
    r.poll(view(&map, 0), 0, true);
    release(&released);
    let report = finish(&mut r, &map);
    assert_eq!(calls.lock().unwrap().len(), 1);
    assert_eq!(report.jobs[0].worker_status, "Cancelled");
    assert!(
        report.jobs[0]
            .attempt
            .as_ref()
            .unwrap()
            .publication
            .as_ref()
            .unwrap()
            .succeeded
    );
    assert!(
        report.local_lifecycle_known
            && !report.jobs[0].prepared_locally
            && !report.batch_prepared_locally
    );
}
