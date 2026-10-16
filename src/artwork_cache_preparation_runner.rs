//! Explicit diagnostic, one finite <=16-key batch on the existing image Worker.
//! No GL, Root policy, Main activation, retry, eviction or cross-process ledger.
//! Call poll with the SAME Main live_origin NOW as Catalogue.observed_ms.
//! A caller must latch no-draw immediately on input/departure, then keep polling
//! the current Work. finish never cancels it. Actual join can block in syscalls.
use crate::{
    app_artwork::{AppSources, IconKind, Source, SourceMap},
    artwork_cache::{ArtworkCache, Config, FetchResult, Work},
    artwork_cache_preparation::{Batch, Catalogue, Cursor, Plan},
    artwork_preparation_events::{Attempt, Settlement},
    artwork_worker::{SessionSettlement, Status, Worker},
};
use serde::Serialize;
use std::{
    sync::mpsc::{Receiver, TryRecvError},
    time::{Duration, Instant},
};

pub const BATCH_BUDGET: Duration = Duration::from_secs(180);
pub const DRAIN_RESERVE: Duration = Duration::from_secs(30);
pub const JOB_BUDGET: Duration = Duration::from_secs(30);
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum Stop {
    Complete,
    CatalogueWithdrawn,
    InputOrDeparture,
    CallerFailure,
    Budget,
    Unknown,
    SubmissionRejected,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Step {
    Waiting,
    Submitted,
    Completed,
    Stopped,
}
#[derive(Debug, Serialize)]
pub struct JobRecord {
    pub attempt: Option<Attempt>,
    pub source_key: crate::app_artwork::SourceKey,
    pub work_id: u64,
    pub worker_settlement: Settlement,
    pub event_paired: bool,
    pub worker_status: String,
    pub catalogue_still_current: bool,
    pub prepared_locally: bool,
}
#[derive(Debug, Serialize)]
pub struct Report {
    pub schema: u8,
    pub planning: crate::artwork_cache_preparation::PlanningManifest,
    pub jobs: Vec<JobRecord>,
    pub unexpected_events: Vec<Attempt>,
    pub submitted: usize,
    pub completed: usize,
    pub prepared_locally: usize,
    pub stopped: Stop,
    pub joined: bool,
    pub panicked: bool,
    pub outstanding: bool,
    pub local_lifecycle_known: bool,
    pub batch_prepared_locally: bool,
    pub external_process_closure_verified: bool,
    pub all_catalogue_prepared: bool,
    pub current_cache_body_coverage_verified: bool,
    pub native_pixels_qualified: bool,
    #[serde(rename = "Root_authority_granted")]
    pub root_authority_granted: bool,
}
struct Active {
    work: Work,
    scheduler: ArtworkCache,
}
pub struct Runner {
    plan: Plan,
    batch: Batch,
    worker: Option<Worker>,
    events: Receiver<Attempt>,
    active: Option<Active>,
    records: Vec<JobRecord>,
    unexpected: Vec<Attempt>,
    submitted: usize,
    stopped: Option<Stop>,
    unknown: bool,
    initial: bool,
    began: Instant,
}
impl Runner {
    /// Explicit native Preview diagnostic factory only. Plan validation happens
    /// before worker creation; the fixed sidecar/cache load stays on that worker.
    pub fn start(
        catalogue: Catalogue<'_>,
        now_ms: u32,
        cursor: Option<&Cursor>,
        marker: crate::broker_protocol::Marker,
        output: std::path::PathBuf,
    ) -> Result<Self, Box<dyn std::error::Error>> {
        let mut plan = Plan::from_current(catalogue, now_ms)
            .map_err(|e| format!("Preparation plan: {e:?}"))?;
        let batch = plan
            .next_batch(catalogue, now_ms, cursor)
            .map_err(|e| format!("Preparation batch: {e:?}"))?;
        let began = Instant::now();
        let (worker, events) =
            crate::artwork_installed_provider::InstalledProvider::start_preview_preparation_worker(
                marker, output,
            )?;
        Ok(Self::with_worker(plan, batch, worker, events, began))
    }
    fn with_worker(
        plan: Plan,
        batch: Batch,
        worker: Worker,
        events: Receiver<Attempt>,
        began: Instant,
    ) -> Self {
        Self {
            plan,
            batch,
            worker: Some(worker),
            events,
            active: None,
            records: Vec::new(),
            unexpected: Vec::new(),
            submitted: 0,
            stopped: None,
            unknown: false,
            initial: false,
            began,
        }
    }
    fn stop(&mut self, reason: Stop) {
        // A later unknown must never be hidden by an earlier ordinary decline.
        if reason == Stop::Unknown {
            self.unknown = true;
            self.stopped = Some(reason);
        } else if self.stopped.is_none()
            || (self.stopped == Some(Stop::Complete) && reason != Stop::Complete)
        {
            self.stopped = Some(reason);
        }
    }
    pub fn has_outstanding(&self) -> bool {
        self.active.is_some()
    }
    pub fn stopped(&self) -> Option<Stop> {
        self.stopped
    }
    pub fn records(&self) -> &[JobRecord] {
        &self.records
    }
    /// Stop new work on a caller invariant/presentation failure, without
    /// cancelling or dropping the current job. Unknown still overrides this.
    pub fn stop_for_caller_failure(&mut self) {
        self.stop(Stop::CallerFailure);
    }
    /// No drawing, filesystem or remote operation occurs on this thread here.
    /// Caller stops new requests and drawing on departure before invoking poll.
    /// Logical budget expiry stops dispatch; it does NOT drop/cancel active Work.
    pub fn poll(&mut self, catalogue: Catalogue<'_>, now_ms: u32, departed: bool) -> Step {
        if self.worker.is_none() {
            return Step::Stopped;
        }
        if departed {
            self.stop(Stop::InputOrDeparture);
        }
        if self.began.elapsed() >= BATCH_BUDGET - JOB_BUDGET - DRAIN_RESERVE {
            self.stop(Stop::Budget);
        }
        if let Some(active) = self.active.as_ref() {
            if self
                .plan
                .before_dispatch(catalogue, now_ms, active.work.source())
                .is_err()
            {
                self.stop(Stop::CatalogueWithdrawn);
            }
        }
        if self.active.is_some() {
            let Some(completed) = self.worker.as_mut().and_then(Worker::poll) else {
                return Step::Waiting;
            };
            let active = self
                .active
                .take()
                .expect("active Work retained until completion");
            let current = self
                .plan
                .before_dispatch(catalogue, now_ms, completed.work.source())
                .is_ok();
            if !current {
                self.stop(Stop::CatalogueWithdrawn);
            }
            // A matching event is sent before fetch returns. Missing, duplicate,
            // disconnected or mismatched observations cannot become known-no-start.
            let event = self.events.try_recv();
            let no_extra = match self.events.try_recv() {
                Err(TryRecvError::Empty) => true,
                Ok(extra) => {
                    self.unexpected.push(extra);
                    false
                }
                Err(TryRecvError::Disconnected) => false,
            };
            let paired = event
                .as_ref()
                .is_ok_and(|e| e.matches_closed(self.submitted, active.work.source()));
            let closed = completed.work == active.work
                && completed.settlement == SessionSettlement::KnownClosed
                && paired
                && no_extra;
            let decoded = completed.status == Status::Decoded;
            let fingerprint = match &completed.result {
                FetchResult::Decoded { fingerprint, .. } => Some(fingerprint),
                _ => None,
            };
            let prepared = closed
                && current
                && decoded
                && event.as_ref().is_ok_and(|e| {
                    e.publication.as_ref().is_some_and(|p| {
                        p.succeeded
                            && p.receipt_written
                            && fingerprint.is_some_and(|f| {
                                f.content_sha256 == p.encoded_sha256
                                    && f.file_identity == p.source_identity
                                    && f.file_identity.bytes == p.encoded_bytes as u64
                            })
                    })
                });
            // Never upload/retain the decoded pixels in this metadata-only runner.
            let source_key = completed.work.source().clone();
            let work_id = completed.work.id();
            drop(completed.result);
            let mut scheduler = active.scheduler;
            let submitted = scheduler.submit_result(active.work, FetchResult::Failed);
            let drained = scheduler.apply_next(0).is_some();
            let internal = scheduler.shutdown();
            if !closed
                || submitted != crate::artwork_cache::SubmitOutcome::Queued
                || !drained
                || internal.in_flight.is_some()
            {
                self.stop(Stop::Unknown);
            }
            self.records.push(JobRecord {
                attempt: event.ok(),
                source_key,
                work_id,
                worker_settlement: completed.settlement.into(),
                event_paired: paired && no_extra,
                worker_status: format!("{:?}", completed.status),
                catalogue_still_current: current,
                prepared_locally: prepared,
            });
            if self.submitted == self.batch.sources().len() && self.stopped.is_none() {
                self.stop(Stop::Complete);
            }
            return Step::Completed;
        }
        if self.stopped.is_some() {
            return Step::Stopped;
        }
        if !self.initial {
            match self.worker.as_mut().and_then(Worker::initial_settlement) {
                None => return Step::Waiting,
                Some(SessionSettlement::NoExternalSession) => self.initial = true,
                Some(_) => {
                    self.stop(Stop::Unknown);
                    return Step::Stopped;
                }
            }
        }
        let Some(planned) = self.batch.sources().get(self.submitted) else {
            self.stop(Stop::Complete);
            return Step::Stopped;
        };
        let source = match self.plan.before_dispatch(catalogue, now_ms, planned.key()) {
            Ok(source) => source.clone(),
            Err(_) => {
                self.stop(Stop::CatalogueWithdrawn);
                return Step::Stopped;
            }
        };
        // Consume no event speculatively: a previous/early observation must not
        // survive a Worker pre-fetch interruption and masquerade as this job.
        match self.events.try_recv() {
            Err(TryRecvError::Empty) => {}
            Ok(extra) => {
                self.unexpected.push(extra);
                self.stop(Stop::Unknown);
                return Step::Stopped;
            }
            Err(TryRecvError::Disconnected) => {
                self.stop(Stop::Unknown);
                return Step::Stopped;
            }
        }
        let mut scheduler = one_source_scheduler(&source);
        let Some(work) = scheduler.next_work(0) else {
            self.stop(Stop::Unknown);
            return Step::Stopped;
        };
        // Budget recheck immediately before actual submit; no per-job reset of
        // the original batch deadline and no future retry after rejection.
        let now = Instant::now();
        if now >= self.began + BATCH_BUDGET - JOB_BUDGET - DRAIN_RESERVE {
            self.stop(Stop::Budget);
            return Step::Stopped;
        }
        let deadline = (now + JOB_BUDGET).min(self.began + BATCH_BUDGET);
        let result = self.worker.as_mut().expect("worker present").try_submit(
            work.clone(),
            source,
            deadline,
        );
        if result.is_err() {
            self.stop(Stop::SubmissionRejected);
            return Step::Stopped;
        }
        self.submitted += 1;
        self.active = Some(Active { work, scheduler });
        Step::Submitted
    }
    fn final_observation(&mut self, catalogue: Catalogue<'_>, now_ms: u32, departed: bool) {
        if departed {
            self.stop(Stop::InputOrDeparture);
        }
        if self.plan.next_batch(catalogue, now_ms, None).is_err() {
            self.stop(Stop::CatalogueWithdrawn);
        }
        if self.began.elapsed() >= BATCH_BUDGET {
            self.stop(Stop::Budget);
        }
    }
    /// None means keep polling the current actual Work (or stop explicitly).
    /// This never uses Worker.finish to cancel an unpolled completion. Join may
    /// still block on provider/SDK destruction; there is no hard shutdown bound.
    pub fn finish(
        &mut self,
        catalogue: Catalogue<'_>,
        mut observe: impl FnMut() -> (u32, bool),
    ) -> Option<Report> {
        if self.active.is_some() || self.stopped.is_none() {
            return None;
        }
        // Both observations use the original live clock and the caller's actual
        // input/departure latch. Join does not turn old metadata into fresh data.
        let (now_ms, departed) = observe();
        self.final_observation(catalogue, now_ms, departed);
        let finished = self.worker.take()?.finish();
        let (now_ms, departed) = observe();
        self.final_observation(catalogue, now_ms, departed);
        let no_extra = match self.events.try_recv() {
            Err(TryRecvError::Empty | TryRecvError::Disconnected) => true,
            Ok(extra) => {
                self.unexpected.push(extra);
                false
            }
        };
        if finished.panicked
            || finished.outstanding.is_some()
            || !no_extra
            || self.records.len() != self.submitted
        {
            self.stop(Stop::Unknown);
        }
        let known = self.initial
            && !self.unknown
            && !finished.panicked
            && finished.joined
            && finished.outstanding.is_none()
            && no_extra
            && self.records.len() == self.submitted;
        let prepared = self.records.iter().filter(|r| r.prepared_locally).count();
        Some(Report {
            schema: 1,
            planning: self.batch.manifest(),
            jobs: std::mem::take(&mut self.records),
            unexpected_events: std::mem::take(&mut self.unexpected),
            submitted: self.submitted,
            completed: self.submitted,
            prepared_locally: prepared,
            stopped: self.stopped.unwrap_or(Stop::Unknown),
            joined: finished.joined,
            panicked: finished.panicked,
            outstanding: finished.outstanding.is_some(),
            local_lifecycle_known: known,
            batch_prepared_locally: known
                && self.stopped == Some(Stop::Complete)
                && !self.batch.sources().is_empty()
                && prepared == self.batch.sources().len(),
            external_process_closure_verified: false,
            all_catalogue_prepared: false,
            current_cache_body_coverage_verified: false,
            native_pixels_qualified: false,
            root_authority_granted: false,
        })
    }
}
fn one_source_scheduler(source: &Source) -> ArtworkCache {
    let mut map = SourceMap::new();
    let (icon, large_icon) = match source.key().kind {
        IconKind::Icon => (Some(source.clone()), None),
        IconKind::LargeIcon => (None, Some(source.clone())),
    };
    // This copies a sealed Source selected by Plan, never constructs a Source or
    // accepts a new path. The tiny existing scheduler only mints Work ownership.
    map.insert(
        source.app_id().to_owned(),
        AppSources {
            app_id: source.app_id().to_owned(),
            version: Some(source.key().version.clone()),
            icon,
            large_icon,
            rejections: Vec::new(),
        },
    );
    let mut scheduler = ArtworkCache::new(Config::default()).expect("fixed valid cache config");
    scheduler.update_catalogue(&map).expect("one sealed Source");
    scheduler
}
#[cfg(test)]
#[path = "artwork_cache_preparation_runner_tests.rs"]
mod tests;
