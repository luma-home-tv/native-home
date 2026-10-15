//! Render-thread scheduling around one I/O worker. This module owns CPU state,
//! never GL objects or focus. A failed catalogue withdraws live images; packaged
//! fallbacks remain independently owned by the shell.
use crate::app_artwork::SourceMap;
use crate::artwork_cache::{self as cache, ArtworkCache, ResidentKey, ResidentView};
use crate::artwork_worker::Worker;
use std::time::{Duration, Instant};

#[derive(Default)]
pub struct Frame {
    pub evicted: Vec<ResidentKey>,
    pub changed: Option<ResidentKey>,
}
pub struct Runtime {
    cache: ArtworkCache,
    worker: Option<Worker>,
    sources: SourceMap,
    evicted: Vec<ResidentKey>,
    active: bool,
    stopped: bool,
    submitted: u64,
    completed: u64,
    stale: u64,
    rejected: u64,
    visible: Vec<String>,
    favourites: Vec<String>,
}
pub struct Finished {
    pub evicted: Vec<ResidentKey>,
    pub joined: bool,
    pub panicked: bool,
    pub outstanding_resolved: bool,
}
impl Runtime {
    pub fn new(worker: Worker, config: cache::Config) -> Result<Self, cache::Error> {
        Ok(Self {
            cache: ArtworkCache::new(config)?,
            worker: Some(worker),
            sources: SourceMap::new(),
            evicted: Vec::new(),
            active: true,
            stopped: false,
            submitted: 0,
            completed: 0,
            stale: 0,
            rejected: 0,
            visible: Vec::new(),
            favourites: Vec::new(),
        })
    }
    pub fn set_active(&mut self, active: bool) {
        self.active = active;
    }
    pub fn active(&self) -> bool {
        self.active && !self.stopped
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
                worker.cancel();
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
        if let Some(done) = self.worker.as_mut().and_then(Worker::poll) {
            self.completed = self.completed.saturating_add(1);
            if self.cache.submit_result(done.work, done.result) == cache::SubmitOutcome::Stale {
                self.stale = self.stale.saturating_add(1);
            }
        }
        if self.active && !self.stopped {
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
                    (Some(worker), Some(source)) => match worker.try_submit(
                        work,
                        source,
                        Instant::now() + Duration::from_secs(2),
                    ) {
                        Ok(()) => self.submitted = self.submitted.saturating_add(1),
                        Err(rejected) => {
                            self.rejected = self.rejected.saturating_add(1);
                            self.cache
                                .submit_result(rejected.work, cache::FetchResult::Failed);
                        }
                    },
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
        serde_json::json!({"schema":1,"active":self.active,"stopped":self.stopped,"submitted":self.submitted,
            "completed":self.completed,"stale_results":self.stale,"rejected":self.rejected,
            "entries":s.entries,"queued":s.queued,"in_flight":s.in_flight,"pending_results":s.pending_results,
            "pending_result_capacity_bytes":s.pending_result_bytes,"resident_entries":s.resident_entries,
            "resident_capacity_bytes":s.resident_bytes,"closed":s.closed})
    }
    pub fn finish(&mut self) -> Finished {
        self.stopped = true;
        self.active = false;
        let done = self.worker.take().map(Worker::finish);
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
