//! Render-thread scheduling around one I/O worker. This module owns CPU state,
//! never GL objects or focus. A failed catalogue withdraws live images; packaged
//! fallbacks remain independently owned by the shell.
use crate::app_artwork::SourceMap;
use crate::artwork_cache::{self as cache, ArtworkCache, ResidentKey, ResidentView};
use crate::artwork_worker::{SessionSettlement, Worker};
use std::time::{Duration, Instant};

pub const DEFAULT_FETCH_BUDGET: Duration = Duration::from_secs(2);
pub const MAX_FETCH_BUDGET: Duration = Duration::from_secs(30);

#[derive(Default)]
pub struct Frame {
    pub evicted: Vec<ResidentKey>,
    pub changed: Option<ResidentKey>,
}
/// Scheduling/transition evidence, independent of image/decode success.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum TransitionReadiness {
    Disabled,
    Ready,
    Draining,
    BlockedUnknown,
}
pub struct Runtime {
    cache: ArtworkCache,
    worker: Option<Worker>,
    sources: SourceMap,
    evicted: Vec<ResidentKey>,
    active: bool,
    stopped: bool,
    settlement_unknown: bool,
    initial_pending: bool,
    submitted: u64,
    completed: u64,
    stale: u64,
    rejected: u64,
    // Counts explicit Runtime withdrawal requests accepted by Worker::cancel,
    // including repeated requests while the same Work slot remains owned.
    cancellation_requests: u64,
    visible: Vec<String>,
    favourites: Vec<String>,
    fetch_budget: Duration,
}
pub struct Finished {
    pub evicted: Vec<ResidentKey>,
    pub joined: bool,
    pub panicked: bool,
    pub outstanding_resolved: bool,
    /// Lifecycle evidence survives disposal; an empty cache task is not closure.
    pub settlement: SessionSettlement,
}
impl Runtime {
    pub fn new(worker: Worker, config: cache::Config) -> Result<Self, cache::Error> {
        Self::with_fetch_budget(worker, config, DEFAULT_FETCH_BUDGET)
    }
    /// Set the finite deadline supplied to each worker job. This does not
    /// interrupt a blocked provider/codec or bound the time needed to join it.
    pub fn with_fetch_budget(
        worker: Worker,
        config: cache::Config,
        fetch_budget: Duration,
    ) -> Result<Self, cache::Error> {
        if fetch_budget.is_zero() || fetch_budget > MAX_FETCH_BUDGET {
            return Err(cache::Error::InvalidConfig);
        }
        Ok(Self {
            cache: ArtworkCache::new(config)?,
            worker: Some(worker),
            sources: SourceMap::new(),
            evicted: Vec::new(),
            active: true,
            stopped: false,
            settlement_unknown: false,
            initial_pending: true,
            submitted: 0,
            completed: 0,
            stale: 0,
            rejected: 0,
            cancellation_requests: 0,
            visible: Vec::new(),
            favourites: Vec::new(),
            fetch_budget,
        })
    }
    pub fn set_active(&mut self, active: bool) {
        self.active = active;
    }
    pub fn active(&self) -> bool {
        self.active && !self.stopped && !self.initial_pending && !self.settlement_unknown
    }
    /// No filesystem, LS2, join, cancellation or image application on this path.
    /// Ready before the first job additionally requires worker-side initializer
    /// evidence; it never grants authority to recover an older pending record.
    pub fn transition_readiness(&self) -> TransitionReadiness {
        if self.settlement_unknown {
            TransitionReadiness::BlockedUnknown
        } else if self.initial_pending {
            TransitionReadiness::Draining
        } else if self.stopped || self.worker.is_none() {
            TransitionReadiness::Disabled
        } else if self.worker.as_ref().is_some_and(Worker::has_outstanding)
            || self.cache.stats().in_flight != 0
        {
            TransitionReadiness::Draining
        } else {
            TransitionReadiness::Ready
        }
    }
    pub fn update_catalogue(&mut self, sources: Option<&SourceMap>) -> Result<(), cache::Error> {
        let empty = SourceMap::new();
        let source = sources.unwrap_or(&empty);
        let update = self.cache.update_catalogue(source)?;
        self.cache.set_interest(&self.visible, &self.favourites)?;
        self.evicted.extend(update.evicted);
        self.sources = source.clone();
        if sources.is_none() {
            if let Some(worker) = &mut self.worker {
                if worker.cancel() {
                    self.cancellation_requests = self.cancellation_requests.saturating_add(1);
                }
            }
        }
        Ok(())
    }
    pub fn set_interest(
        &mut self,
        visible: &[String],
        favourites: &[String],
    ) -> Result<(), cache::Error> {
        self.cache.set_interest(visible, favourites)?;
        self.visible = visible.to_vec();
        self.favourites = favourites.to_vec();
        Ok(())
    }
    pub fn request_refresh(&mut self, id: &str) -> bool {
        self.cache.request_refresh(id)
    }
    /// Poll while suspended so worker-owned results never block shutdown. Apply
    /// at most one CPU change per frame; the shell can therefore upload at most
    /// one image. Suspension neither dispatches jobs nor uploads images.
    pub fn frame(&mut self, now_ms: u64) -> Frame {
        let mut frame = Frame {
            evicted: std::mem::take(&mut self.evicted),
            changed: None,
        };
        if self.initial_pending {
            if let Some(initial) = self.worker.as_mut().and_then(Worker::initial_settlement) {
                self.initial_pending = false;
                self.settlement_unknown |= initial == SessionSettlement::Unknown;
            }
        }
        if let Some(done) = self.worker.as_mut().and_then(Worker::poll) {
            self.completed = self.completed.saturating_add(1);
            // Sticky for this generation: later known jobs cannot erase ambiguity.
            self.settlement_unknown |= done.settlement == SessionSettlement::Unknown;
            let result = if done.settlement == SessionSettlement::Unknown {
                // Resolve ownership without admitting even CPU bytes from an
                // unclosed session. Status/settlement remain worker evidence.
                cache::FetchResult::Failed
            } else {
                done.result
            };
            if self.cache.submit_result(done.work, result) == cache::SubmitOutcome::Stale {
                self.stale = self.stale.saturating_add(1);
            }
        }
        if self.active() {
            // Drawing references alone do not touch CPU LRU state. Refresh
            // active residents before a background result can choose a victim.
            for id in self.visible.iter().chain(&self.favourites) {
                let _ = self.cache.resident(id);
            }
            if let Some(applied) = self.cache.apply_next(now_ms) {
                frame.evicted.extend(applied.evicted);
                frame.changed = applied.changed;
                self.stale = self.stale.saturating_add(u64::from(applied.stale));
            }
            if let Some(work) = self.cache.next_work(now_ms) {
                let source = self
                    .sources
                    .get(&work.source().app_id)
                    .and_then(|s| s.preferred())
                    .filter(|s| s.key() == work.source())
                    .cloned();
                match (self.worker.as_mut(), source) {
                    (Some(worker), Some(source)) => {
                        match worker.try_submit(work, source, Instant::now() + self.fetch_budget) {
                            Ok(()) => self.submitted = self.submitted.saturating_add(1),
                            Err(rejected) => {
                                self.rejected = self.rejected.saturating_add(1);
                                self.cache
                                    .submit_result(rejected.work, cache::FetchResult::Failed);
                            }
                        }
                    }
                    _ => {
                        self.rejected = self.rejected.saturating_add(1);
                        self.cache.submit_result(work, cache::FetchResult::Failed);
                    }
                }
            }
        }
        frame
    }
    pub fn resident(&mut self, id: &str) -> Option<ResidentView<'_>> {
        self.cache.resident(id)
    }
    pub fn source(&self, id: &str) -> Option<&crate::app_artwork::SourceKey> {
        self.sources
            .get(id)
            .and_then(|s| s.preferred())
            .map(|s| s.key())
    }
    pub fn receipt(&self) -> serde_json::Value {
        let s = self.cache.stats();
        serde_json::json!({"schema":1,"active":self.active(),"requested_active":self.active,"stopped":self.stopped,"submitted":self.submitted,
            "transition_readiness":format!("{:?}",self.transition_readiness()),"settlement_unknown":self.settlement_unknown,"initial_pending":self.initial_pending,
            "completed":self.completed,"stale_results":self.stale,"rejected":self.rejected,"cancellation_requests":self.cancellation_requests,
            "entries":s.entries,"queued":s.queued,"in_flight":s.in_flight,"pending_results":s.pending_results,
            "pending_result_capacity_bytes":s.pending_result_bytes,"resident_entries":s.resident_entries,
            "resident_capacity_bytes":s.resident_bytes,"closed":s.closed})
    }
    pub fn finish(&mut self) -> Finished {
        self.stopped = true;
        self.active = false;
        let done = self.worker.take().map(Worker::finish);
        self.settlement_unknown |= self.initial_pending
            || done
                .as_ref()
                .is_some_and(|d| d.outstanding.is_some() || d.panicked);
        self.initial_pending = false;
        // The worker is joined before its still-owned cache task is cancelled.
        // Failed is an ownership acknowledgement; shutdown discards its result.
        if let Some(work) = done.as_ref().and_then(|d| d.outstanding.clone()) {
            self.cache.submit_result(work, cache::FetchResult::Failed);
        }
        let stopped = self.cache.shutdown();
        let mut evicted = std::mem::take(&mut self.evicted);
        evicted.extend(stopped.evicted);
        Finished {
            evicted,
            joined: done.as_ref().is_none_or(|d| d.joined),
            panicked: done.as_ref().is_some_and(|d| d.panicked),
            outstanding_resolved: stopped.in_flight.is_none(),
            settlement: if self.settlement_unknown {
                SessionSettlement::Unknown
            } else if self.submitted == 0 {
                SessionSettlement::NoExternalSession
            } else {
                SessionSettlement::KnownClosed
            },
        }
    }
}
impl Drop for Runtime {
    fn drop(&mut self) {
        let _ = self.finish();
    }
}

#[cfg(test)]
#[path = "artwork_runtime_tests.rs"]
mod tests;

#[cfg(test)]
mod settlement_tests {
    use super::*;
    use crate::app_artwork::{sources_from_raw, EncodedFile, Source};
    use crate::artwork_worker::{Cancellation, Provider, ProviderError};
    use std::sync::mpsc;

    struct Held {
        entered: mpsc::Sender<()>,
        release: mpsc::Receiver<()>,
        settlement: SessionSettlement,
    }
    impl Provider for Held {
        fn initial_settlement(&self) -> SessionSettlement {
            SessionSettlement::NoExternalSession
        }
        fn fetch(
            &mut self,
            _: &Source,
            _: Instant,
            _: &Cancellation,
        ) -> Result<EncodedFile, ProviderError> {
            self.entered.send(()).unwrap();
            self.release.recv_timeout(Duration::from_secs(2)).unwrap();
            // Deliberately no image: remote closure and decode success differ.
            Err(ProviderError::Invalid)
        }
        fn transition_settlement(&self) -> SessionSettlement {
            self.settlement
        }
    }
    fn controlled(
        settlement: SessionSettlement,
    ) -> (Runtime, mpsc::Receiver<()>, mpsc::Sender<()>) {
        let (entered_tx, entered_rx) = mpsc::channel();
        let (release_tx, release_rx) = mpsc::channel();
        let w = Worker::start(move || Held {
            entered: entered_tx,
            release: release_rx,
            settlement,
        })
        .unwrap();
        let mut runtime = Runtime::new(w, cache::Config::default()).unwrap();
        let id = "synthetic.fixture.transition";
        let raw = serde_json::json!({"replies":{"apps":{"returnValue":true,"apps":[{"id":id,"title":"Fixture","type":"native","visible":true,"version":"1","folderPath":format!("/usr/palm/applications/{id}"),"icon":"icon.png"}]},"launch_points":{"returnValue":true,"launchPoints":[{"id":id}]}}});
        let sources = sources_from_raw(&raw, crate::identity::PROOF).unwrap();
        let until = Instant::now() + Duration::from_secs(2);
        while runtime.initial_pending {
            runtime.frame(0);
            assert!(Instant::now() < until, "controlled worker initialization");
            std::thread::yield_now();
        }
        runtime.update_catalogue(Some(&sources)).unwrap();
        runtime.set_interest(&[id.to_string()], &[]).unwrap();
        (runtime, entered_rx, release_tx)
    }
    fn polled(runtime: &mut Runtime) {
        let until = Instant::now() + Duration::from_secs(2);
        while runtime.completed == 0 {
            let frame = runtime.frame(1);
            assert!(frame.changed.is_none());
            assert!(
                Instant::now() < until,
                "bounded held worker did not complete"
            );
            std::thread::yield_now();
        }
    }
    #[test]
    fn paused_runtime_drains_known_failed_image_without_dispatch_or_apply() {
        let (mut runtime, entered, release) = controlled(SessionSettlement::KnownClosed);
        assert_eq!(runtime.transition_readiness(), TransitionReadiness::Ready);
        runtime.frame(0);
        entered.recv_timeout(Duration::from_secs(2)).unwrap();
        runtime.set_active(false);
        assert_eq!(
            runtime.transition_readiness(),
            TransitionReadiness::Draining
        );
        for _ in 0..10 {
            assert!(runtime.frame(1).changed.is_none());
        }
        assert_eq!(runtime.submitted, 1);
        release.send(()).unwrap();
        polled(&mut runtime);
        assert_eq!(runtime.transition_readiness(), TransitionReadiness::Ready);
        assert_eq!(runtime.cache.stats().in_flight, 0);
        assert_eq!(runtime.submitted, 1);
        let done = runtime.finish();
        assert!(done.joined && done.outstanding_resolved);
        assert_eq!(
            runtime.transition_readiness(),
            TransitionReadiness::Disabled
        );
    }
    #[test]
    fn empty_work_slot_with_unknown_settlement_stays_blocked_after_resume() {
        let (mut runtime, entered, release) = controlled(SessionSettlement::Unknown);
        runtime.frame(0);
        entered.recv_timeout(Duration::from_secs(2)).unwrap();
        runtime.set_active(false);
        release.send(()).unwrap();
        polled(&mut runtime);
        assert_eq!(runtime.cache.stats().in_flight, 0);
        assert_eq!(
            runtime.transition_readiness(),
            TransitionReadiness::BlockedUnknown
        );
        runtime.set_active(true);
        runtime.request_refresh("synthetic.fixture.transition");
        for _ in 0..10 {
            runtime.frame(2);
        }
        assert!(!runtime.active());
        assert_eq!(runtime.submitted, 1);
        assert_eq!(
            runtime.transition_readiness(),
            TransitionReadiness::BlockedUnknown
        );
        runtime.finish();
    }
    #[test]
    fn initializer_pending_is_not_ready_and_unknown_init_is_sticky() {
        let (entered_tx, entered_rx) = mpsc::channel();
        let (release_tx, release_rx) = mpsc::channel();
        let worker = Worker::start(move || {
            entered_tx.send(()).unwrap();
            release_rx.recv_timeout(Duration::from_secs(2)).unwrap();
            crate::artwork_worker::DirectProvider
        })
        .unwrap();
        let mut runtime = Runtime::new(worker, cache::Config::default()).unwrap();
        entered_rx.recv_timeout(Duration::from_secs(2)).unwrap();
        runtime.set_active(false);
        for _ in 0..10 {
            runtime.frame(0);
            assert_eq!(
                runtime.transition_readiness(),
                TransitionReadiness::Draining
            );
            assert_eq!(runtime.submitted, 0);
        }
        release_tx.send(()).unwrap();
        let until = Instant::now() + Duration::from_secs(2);
        while runtime.initial_pending {
            runtime.frame(0);
            assert!(Instant::now() < until);
            std::thread::yield_now();
        }
        assert_eq!(runtime.transition_readiness(), TransitionReadiness::Ready);
        assert_eq!(
            runtime.finish().settlement,
            SessionSettlement::NoExternalSession
        );

        struct Ambiguous;
        impl Provider for Ambiguous {
            fn fetch(
                &mut self,
                _: &Source,
                _: Instant,
                _: &Cancellation,
            ) -> Result<EncodedFile, ProviderError> {
                panic!("unknown initialization must prevent dispatch")
            }
        }
        let mut runtime = Runtime::new(
            Worker::start(|| Ambiguous).unwrap(),
            cache::Config::default(),
        )
        .unwrap();
        let until = Instant::now() + Duration::from_secs(2);
        while runtime.initial_pending {
            runtime.frame(0);
            assert!(Instant::now() < until);
            std::thread::yield_now();
        }
        assert_eq!(
            runtime.transition_readiness(),
            TransitionReadiness::BlockedUnknown
        );
        assert!(!runtime.receipt()["active"].as_bool().unwrap());
        assert_eq!(runtime.finish().settlement, SessionSettlement::Unknown);
        assert_eq!(
            runtime.transition_readiness(),
            TransitionReadiness::BlockedUnknown
        );
    }
    #[test]
    fn finish_cannot_reclassify_completed_unpolled_work_as_disabled() {
        for disposition in [SessionSettlement::Unknown, SessionSettlement::KnownClosed] {
            let (mut runtime, entered, release) = controlled(disposition);
            runtime.frame(0);
            entered.recv_timeout(Duration::from_secs(2)).unwrap();
            release.send(()).unwrap();
            // Whether the completion already reached its channel or is still
            // returning, it has not been classified by the Runtime.
            let closed = runtime.finish();
            assert!(closed.joined && closed.outstanding_resolved);
            assert_eq!(closed.settlement, SessionSettlement::Unknown);
            assert_eq!(
                runtime.transition_readiness(),
                TransitionReadiness::BlockedUnknown
            );
        }
    }
    #[test]
    fn decoded_unknown_never_enters_pending_or_resident_cpu_cache() {
        struct DecodedUnknown;
        impl Provider for DecodedUnknown {
            fn initial_settlement(&self) -> SessionSettlement {
                SessionSettlement::NoExternalSession
            }
            fn fetch(
                &mut self,
                _: &Source,
                _: Instant,
                _: &Cancellation,
            ) -> Result<EncodedFile, ProviderError> {
                let bytes = vec![
                    137, 80, 78, 71, 13, 10, 26, 10, 0, 0, 0, 13, 73, 72, 68, 82, 0, 0, 0, 2, 0, 0,
                    0, 1, 8, 6, 0, 0, 0, 244, 34, 127, 138, 0, 0, 0, 14, 73, 68, 65, 84, 120, 156,
                    99, 224, 18, 145, 251, 15, 194, 0, 8, 51, 2, 119, 73, 34, 188, 2, 0, 0, 0, 0,
                    73, 69, 78, 68, 174, 66, 96, 130,
                ];
                Ok(EncodedFile {
                    identity: crate::app_artwork::FileIdentity {
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
                    png_dimensions: Some([2, 1]),
                })
            }
        }
        let (donor, _, _) = controlled(SessionSettlement::KnownClosed);
        let sources = donor.sources.clone();
        drop(donor);
        let mut runtime = Runtime::new(
            Worker::start(|| DecodedUnknown).unwrap(),
            cache::Config::default(),
        )
        .unwrap();
        runtime.update_catalogue(Some(&sources)).unwrap();
        runtime
            .set_interest(&["synthetic.fixture.transition".into()], &[])
            .unwrap();
        let until = Instant::now() + Duration::from_secs(2);
        while runtime.submitted == 0 {
            runtime.frame(0);
            assert!(Instant::now() < until);
            std::thread::yield_now();
        }
        runtime.set_active(false);
        polled(&mut runtime);
        let stats = runtime.cache.stats();
        assert_eq!(
            runtime.transition_readiness(),
            TransitionReadiness::BlockedUnknown
        );
        assert_eq!(
            (
                stats.pending_results,
                stats.pending_result_bytes,
                stats.resident_entries,
                stats.resident_bytes
            ),
            // One bounded Failed tombstone acknowledges Work ownership; it
            // contains no image allocation and cannot be applied while blocked.
            (1, 0, 0, 0)
        );
        assert_eq!(runtime.finish().settlement, SessionSettlement::Unknown);
    }
    #[test]
    fn catalogue_withdrawal_counts_only_accepted_worker_cancellation_requests() {
        let (mut runtime, entered, release) = controlled(SessionSettlement::KnownClosed);
        runtime.frame(0);
        entered.recv_timeout(Duration::from_secs(2)).unwrap();
        assert_eq!(runtime.receipt()["cancellation_requests"], 0);
        runtime.set_active(false);
        runtime.update_catalogue(None).unwrap();
        assert_eq!(runtime.receipt()["cancellation_requests"], 1);
        release.send(()).unwrap();
        polled(&mut runtime);
        assert_eq!(runtime.receipt()["cancellation_requests"], 1);
        // Worker no longer owns a slot, so another withdrawal is not counted.
        runtime.update_catalogue(None).unwrap();
        assert_eq!(runtime.receipt()["cancellation_requests"], 1);
        runtime.finish();
    }
    #[test]
    fn ordinary_manual_pause_and_known_drain_request_no_cancellation() {
        let (mut runtime, entered, release) = controlled(SessionSettlement::KnownClosed);
        runtime.frame(0);
        entered.recv_timeout(Duration::from_secs(2)).unwrap();
        runtime.set_active(false);
        for _ in 0..10 {
            runtime.frame(1);
        }
        assert_eq!(runtime.receipt()["cancellation_requests"], 0);
        assert_eq!(
            runtime.transition_readiness(),
            TransitionReadiness::Draining
        );
        release.send(()).unwrap();
        polled(&mut runtime);
        assert_eq!(runtime.transition_readiness(), TransitionReadiness::Ready);
        assert_eq!(runtime.receipt()["cancellation_requests"], 0);
        runtime.finish();
    }
}
