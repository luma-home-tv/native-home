//! Pure scheduling and CPU artwork ownership, separate from packaged fallbacks.
//! No worker, I/O, decoder, GL object, focus, or privileged operation lives here.
//! Callers supply monotonic milliseconds and drain returned eviction handles on
//! the render thread. Limits cover this cache's owned buffers, not a future
//! worker's heap, GPU storage, or caller-owned results. Byte accounting uses
//! Vec capacity (reserved RGBA storage), excluding allocator bookkeeping.
use crate::app_artwork::{FileIdentity, SourceKey, SourceMap};
use std::collections::{BTreeMap, BTreeSet, VecDeque};

pub const MAX_ENTRIES: usize = 512;
pub const MAX_QUEUED: usize = 8;
pub const MAX_RESULTS: usize = 2;
pub const MAX_AXIS: u32 = 512;
pub const MAX_IMAGE_BYTES: usize = 512 * 512 * 4;
pub const MAX_RESULT_BYTES: usize = 2 * MAX_IMAGE_BYTES;
pub const MAX_RESIDENT_BYTES: usize = 8 * 1024 * 1024;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Config {
    pub resident_bytes: usize,
    pub retry_delay_ms: u64,
    pub refresh_after_ms: u64,
}
impl Default for Config {
    fn default() -> Self {
        Self {
            resident_bytes: MAX_RESIDENT_BYTES,
            retry_delay_ms: 30_000,
            refresh_after_ms: 300_000,
        }
    }
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Error {
    InvalidConfig,
    TooManyEntries,
    InvalidCatalogue,
    CounterExhausted,
    Closed,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Fingerprint {
    pub content_sha256: [u8; 32],
    pub file_identity: FileIdentity,
}
#[derive(Debug)]
pub struct DecodedImage {
    pub width: u32,
    pub height: u32,
    pub rgba: Vec<u8>,
}
impl DecodedImage {
    fn valid(&self, budget: usize) -> bool {
        let bytes = u64::from(self.width)
            .checked_mul(u64::from(self.height))
            .and_then(|v| v.checked_mul(4));
        self.width > 0
            && self.height > 0
            && self.width <= MAX_AXIS
            && self.height <= MAX_AXIS
            && bytes == Some(self.rgba.len() as u64)
            && self.rgba.len() <= MAX_IMAGE_BYTES
            && self.rgba.capacity() <= MAX_IMAGE_BYTES
            && self.rgba.capacity() <= budget
    }
}
#[derive(Debug)]
pub enum FetchResult {
    Decoded {
        fingerprint: Fingerprint,
        image: DecodedImage,
    },
    /// The worker compared the fetched digest with Work::known before decoding.
    /// File identity may change while the encoded content remains identical.
    Unchanged(Fingerprint),
    Failed,
}
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Work {
    id: u64,
    epoch: u64,
    source: SourceKey,
    known: Option<Fingerprint>,
}
impl Work {
    pub fn id(&self) -> u64 {
        self.id
    }
    pub fn source(&self) -> &SourceKey {
        &self.source
    }
    pub fn known(&self) -> Option<&Fingerprint> {
        self.known.as_ref()
    }
}
/// Exact resident revision ownership, suitable for a later main-thread texture
/// map. An old handle must never delete a newer texture sharing the same app ID.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ResidentKey {
    pub source: SourceKey,
    pub revision: u64,
}
pub struct ResidentView<'a> {
    pub key: &'a ResidentKey,
    pub fingerprint: &'a Fingerprint,
    pub width: u32,
    pub height: u32,
    pub rgba: &'a [u8],
}
#[derive(Debug)]
pub struct CatalogueUpdate {
    pub generation: u64,
    pub evicted: Vec<ResidentKey>,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SubmitOutcome {
    Queued,
    Stale,
    Unmatched,
}
#[derive(Debug)]
pub struct Applied {
    pub changed: Option<ResidentKey>,
    pub evicted: Vec<ResidentKey>,
    pub stale: bool,
}
#[derive(Debug)]
pub struct Shutdown {
    pub evicted: Vec<ResidentKey>,
    /// The state machine cannot stop or join a worker. Its one owned task must
    /// still finish/cancel before the future runtime can claim worker teardown.
    pub in_flight: Option<Work>,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Stats {
    pub entries: usize,
    pub queued: usize,
    pub in_flight: usize,
    pub pending_results: usize,
    pub pending_result_bytes: usize,
    pub resident_entries: usize,
    pub resident_bytes: usize,
    pub closed: bool,
}
struct Resident {
    key: ResidentKey,
    fingerprint: Fingerprint,
    image: DecodedImage,
    used: u64,
}
struct Entry {
    source: SourceKey,
    epoch: u64,
    next_due: u64,
    refresh_again: bool,
    was_evicted: bool,
    resident: Option<Resident>,
}
struct Pending {
    work: Work,
    result: FetchResult,
}
impl Pending {
    fn bytes(&self) -> usize {
        match &self.result {
            FetchResult::Decoded { image, .. } => image.rgba.capacity(),
            _ => 0,
        }
    }
}
pub struct ArtworkCache {
    config: Config,
    entries: BTreeMap<String, Entry>,
    visible: BTreeSet<String>,
    favourites: BTreeSet<String>,
    queue: VecDeque<String>,
    in_flight: Option<Work>,
    results: VecDeque<Pending>,
    result_bytes: usize,
    resident_bytes: usize,
    generation: u64,
    epoch: u64,
    work_id: u64,
    use_id: u64,
    closed: bool,
}
impl ArtworkCache {
    pub fn new(config: Config) -> Result<Self, Error> {
        if config.resident_bytes == 0
            || config.resident_bytes > MAX_RESIDENT_BYTES
            || config.retry_delay_ms == 0
            || config.refresh_after_ms == 0
        {
            return Err(Error::InvalidConfig);
        }
        Ok(Self {
            config,
            entries: BTreeMap::new(),
            visible: BTreeSet::new(),
            favourites: BTreeSet::new(),
            queue: VecDeque::new(),
            in_flight: None,
            results: VecDeque::new(),
            result_bytes: 0,
            resident_bytes: 0,
            generation: 0,
            epoch: 0,
            work_id: 0,
            use_id: 0,
            closed: false,
        })
    }
    /// Use only the sealed preferred Source supplied by metadata admission.
    /// A catalogue refresh is not a request to redecode unchanged artwork.
    pub fn update_catalogue(&mut self, sources: &SourceMap) -> Result<CatalogueUpdate, Error> {
        if self.closed {
            return Err(Error::Closed);
        }
        if sources.len() > MAX_ENTRIES {
            return Err(Error::TooManyEntries);
        }
        let mut keys = BTreeMap::new();
        for (id, app) in sources {
            if id != &app.app_id {
                return Err(Error::InvalidCatalogue);
            }
            if let Some(source) = app.preferred() {
                if source.app_id() != id {
                    return Err(Error::InvalidCatalogue);
                }
                keys.insert(id.clone(), source.key().clone());
            }
        }
        let generation = self
            .generation
            .checked_add(1)
            .ok_or(Error::CounterExhausted)?;
        let changed = keys
            .iter()
            .filter(|(id, key)| self.entries.get(*id).is_none_or(|e| e.source != **key))
            .count();
        self.epoch
            .checked_add(changed as u64)
            .ok_or(Error::CounterExhausted)?;
        let mut old = std::mem::take(&mut self.entries);
        let mut evicted = Vec::new();
        for (id, source) in keys {
            let entry = match old.remove(&id) {
                Some(entry) if entry.source == source => entry,
                previous => {
                    if let Some(resident) = previous.and_then(|e| e.resident) {
                        self.remove_resident(resident, &mut evicted);
                    }
                    self.epoch += 1;
                    Entry {
                        source,
                        epoch: self.epoch,
                        next_due: 0,
                        refresh_again: false,
                        was_evicted: false,
                        resident: None,
                    }
                }
            };
            self.entries.insert(id, entry);
        }
        for entry in old.into_values() {
            if let Some(resident) = entry.resident {
                self.remove_resident(resident, &mut evicted);
            }
        }
        self.generation = generation;
        self.queue.clear();
        self.visible.retain(|id| self.entries.contains_key(id));
        self.favourites.retain(|id| self.entries.contains_key(id));
        // Stable entry epochs let an unchanged app's useful result survive a
        // newer catalogue generation. Removed/replaced results drop their Vec.
        self.results
            .retain(|p| Self::current(&self.entries, &p.work));
        self.result_bytes = self.results.iter().map(Pending::bytes).sum();
        Ok(CatalogueUpdate {
            generation,
            evicted,
        })
    }
    pub fn set_interest(&mut self, visible: &[String], favourites: &[String]) -> Result<(), Error> {
        if self.closed {
            return Err(Error::Closed);
        }
        if visible.len() > MAX_ENTRIES || favourites.len() > MAX_ENTRIES {
            return Err(Error::TooManyEntries);
        }
        let visible: BTreeSet<String> = visible
            .iter()
            .filter(|id| self.entries.contains_key(*id))
            .cloned()
            .collect();
        let favourites: BTreeSet<String> = favourites
            .iter()
            .filter(|id| self.entries.contains_key(*id))
            .cloned()
            .collect();
        for id in visible
            .difference(&self.visible)
            .chain(favourites.difference(&self.favourites))
        {
            if let Some(entry) = self.entries.get_mut(id).filter(|e| e.was_evicted) {
                entry.next_due = 0;
            }
        }
        self.visible = visible;
        self.favourites = favourites;
        self.queue.clear();
        Ok(())
    }
    fn current(entries: &BTreeMap<String, Entry>, work: &Work) -> bool {
        entries
            .get(&work.source.app_id)
            .is_some_and(|e| e.epoch == work.epoch && e.source == work.source)
    }
    fn busy(&self, id: &str) -> bool {
        self.in_flight
            .as_ref()
            .is_some_and(|w| w.source.app_id == id && Self::current(&self.entries, w))
            || self
                .results
                .iter()
                .any(|p| p.work.source.app_id == id && Self::current(&self.entries, &p.work))
    }
    pub fn request_refresh(&mut self, id: &str) -> bool {
        if self.closed {
            return false;
        }
        let busy = self.busy(id);
        let Some(entry) = self.entries.get_mut(id) else {
            return false;
        };
        entry.next_due = 0;
        entry.refresh_again |= busy;
        self.queue.clear();
        true
    }
    /// Dispatches at most one task. Two result slots reserve enough capacity for
    /// its maximum image before allowing another dispatch; slow UI drains are bounded.
    pub fn next_work(&mut self, now_ms: u64) -> Option<Work> {
        if self.closed
            || self.in_flight.is_some()
            || self.results.len() >= MAX_RESULTS
            || self.result_bytes > MAX_RESULT_BYTES - MAX_IMAGE_BYTES
        {
            return None;
        }
        let id = self.work_id.checked_add(1)?;
        let mut due = self
            .entries
            .iter()
            .filter(|(id, e)| e.next_due <= now_ms && !self.busy(id))
            .map(|(id, e)| {
                (
                    if self.visible.contains(id) {
                        0
                    } else if self.favourites.contains(id) {
                        1
                    } else {
                        2
                    },
                    e.next_due,
                    id.clone(),
                )
            })
            .collect::<Vec<_>>();
        due.sort();
        self.queue = due
            .into_iter()
            .take(MAX_QUEUED)
            .map(|(_, _, id)| id)
            .collect();
        let app_id = self.queue.pop_front()?;
        let entry = self.entries.get(&app_id)?;
        let work = Work {
            id,
            epoch: entry.epoch,
            source: entry.source.clone(),
            known: entry.resident.as_ref().map(|r| r.fingerprint.clone()),
        };
        self.work_id = id;
        self.in_flight = Some(work.clone());
        Some(work)
    }
    pub fn submit_result(&mut self, work: Work, mut result: FetchResult) -> SubmitOutcome {
        if self.in_flight.as_ref() != Some(&work) {
            return SubmitOutcome::Unmatched;
        }
        self.in_flight = None;
        if self.closed || !Self::current(&self.entries, &work) {
            return SubmitOutcome::Stale;
        }
        match &result {
            FetchResult::Decoded { image, .. } if !image.valid(self.config.resident_bytes) => {
                result = FetchResult::Failed
            }
            FetchResult::Unchanged(f)
                if work
                    .known
                    .as_ref()
                    .is_none_or(|known| known.content_sha256 != f.content_sha256) =>
            {
                result = FetchResult::Failed
            }
            _ => (),
        }
        // If a caller unnecessarily decoded identical content, still preserve
        // the existing resident revision and avoid an unnecessary texture update.
        if let FetchResult::Decoded { fingerprint, .. } = &result {
            if self
                .entries
                .get(&work.source.app_id)
                .and_then(|e| e.resident.as_ref())
                .is_some_and(|r| r.fingerprint.content_sha256 == fingerprint.content_sha256)
            {
                result = FetchResult::Unchanged(fingerprint.clone());
            }
        }
        let pending = Pending { work, result };
        // Capacity is reserved by next_work. Retain a checked failure boundary
        // rather than admitting an oversized or extra result if that changes.
        if self.results.len() >= MAX_RESULTS
            || pending.bytes() > MAX_RESULT_BYTES - self.result_bytes
        {
            return SubmitOutcome::Stale;
        }
        self.result_bytes += pending.bytes();
        self.results.push_back(pending);
        SubmitOutcome::Queued
    }
    fn remove_resident(&mut self, resident: Resident, evicted: &mut Vec<ResidentKey>) {
        self.resident_bytes -= resident.image.rgba.capacity();
        evicted.push(resident.key);
    }
    fn next_use(&mut self) -> u64 {
        self.use_id = self.use_id.saturating_add(1);
        self.use_id
    }
    pub fn apply_next(&mut self, now_ms: u64) -> Option<Applied> {
        let pending = self.results.pop_front()?;
        self.result_bytes -= pending.bytes();
        let mut applied = Applied {
            changed: None,
            evicted: Vec::new(),
            stale: false,
        };
        if self.closed || !Self::current(&self.entries, &pending.work) {
            applied.stale = true;
            return Some(applied);
        }
        let app_id = pending.work.source.app_id.clone();
        let mut entry = self.entries.remove(&app_id).expect("current entry");
        entry.next_due = now_ms.saturating_add(self.config.refresh_after_ms);
        match pending.result {
            FetchResult::Decoded { fingerprint, image } => {
                entry.was_evicted = false;
                if let Some(old) = entry.resident.take() {
                    self.remove_resident(old, &mut applied.evicted);
                }
                while self.resident_bytes + image.rgba.capacity() > self.config.resident_bytes {
                    let victim = self
                        .entries
                        .iter()
                        .filter_map(|(id, e)| e.resident.as_ref().map(|r| (r.used, id.clone())))
                        .min()
                        .map(|(_, id)| id)
                        .expect("resident bytes account for owned entries");
                    let old = self
                        .entries
                        .get_mut(&victim)
                        .and_then(|e| e.resident.take())
                        .expect("resident victim");
                    self.remove_resident(old, &mut applied.evicted);
                    // Keep its refresh deadline: immediate background reloads
                    // would thrash and starve unseen catalogue entries. A new
                    // visible/favourite interest may request its pixels sooner.
                    self.entries
                        .get_mut(&victim)
                        .expect("victim entry")
                        .was_evicted = true;
                }
                let key = ResidentKey {
                    source: entry.source.clone(),
                    revision: pending.work.id,
                };
                applied.changed = Some(key.clone());
                self.resident_bytes += image.rgba.capacity();
                entry.resident = Some(Resident {
                    key,
                    fingerprint,
                    image,
                    used: self.next_use(),
                });
            }
            FetchResult::Unchanged(fingerprint) => {
                if let Some(resident) = entry
                    .resident
                    .as_mut()
                    .filter(|r| r.fingerprint.content_sha256 == fingerprint.content_sha256)
                {
                    resident.fingerprint = fingerprint;
                } else {
                    // Its known image was evicted while this refresh ran.
                    // Fetch once more without a known digest to obtain pixels.
                    entry.next_due = 0;
                }
            }
            FetchResult::Failed => {
                entry.was_evicted = false;
                entry.next_due = now_ms.saturating_add(self.config.retry_delay_ms);
            }
        }
        if entry.refresh_again {
            entry.next_due = 0;
            entry.refresh_again = false;
        }
        self.entries.insert(app_id, entry);
        Some(applied)
    }
    pub fn resident(&mut self, app_id: &str) -> Option<ResidentView<'_>> {
        let used = self.next_use();
        let resident = self.entries.get_mut(app_id)?.resident.as_mut()?;
        resident.used = used;
        Some(ResidentView {
            key: &resident.key,
            fingerprint: &resident.fingerprint,
            width: resident.image.width,
            height: resident.image.height,
            rgba: &resident.image.rgba,
        })
    }
    pub fn shutdown(&mut self) -> Shutdown {
        self.closed = true;
        self.queue.clear();
        self.results.clear();
        self.result_bytes = 0;
        self.visible.clear();
        self.favourites.clear();
        let mut evicted = Vec::new();
        for entry in std::mem::take(&mut self.entries).into_values() {
            if let Some(resident) = entry.resident {
                self.remove_resident(resident, &mut evicted);
            }
        }
        Shutdown {
            evicted,
            in_flight: self.in_flight.clone(),
        }
    }
    pub fn stats(&self) -> Stats {
        Stats {
            entries: self.entries.len(),
            queued: self.queue.len(),
            in_flight: usize::from(self.in_flight.is_some()),
            pending_results: self.results.len(),
            pending_result_bytes: self.result_bytes,
            resident_entries: self
                .entries
                .values()
                .filter(|e| e.resident.is_some())
                .count(),
            resident_bytes: self.resident_bytes,
            closed: self.closed,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::app_artwork::sources_from_raw;
    use serde_json::json;
    fn catalogue(rows: &[(&str, &str, &str)]) -> SourceMap {
        let apps = rows.iter().map(|(id, version, icon)| json!({"id":id,"title":id,"version":version,"icon":icon,"folderPath":format!("/usr/palm/applications/{id}"),"visible":true,"type":"web"})).collect::<Vec<_>>();
        let points = rows
            .iter()
            .map(|(id, _, _)| json!({"id":id}))
            .collect::<Vec<_>>();
        sources_from_raw(&json!({"replies":{"apps":{"returnValue":true,"apps":apps},"launch_points":{"returnValue":true,"launchPoints":points}}}), crate::identity::PROOF).unwrap()
    }
    fn cache(bytes: usize) -> ArtworkCache {
        ArtworkCache::new(Config {
            resident_bytes: bytes,
            ..Config::default()
        })
        .unwrap()
    }
    fn fingerprint(digest: u8, inode: u64) -> Fingerprint {
        Fingerprint {
            content_sha256: [digest; 32],
            file_identity: FileIdentity {
                device: 1,
                inode,
                bytes: 100,
                mode: 0o100644,
                uid: 0,
                gid: 0,
                modified_seconds: 1,
                modified_nanos: 0,
                changed_seconds: 1,
                changed_nanos: 0,
            },
        }
    }
    fn image(width: u32, height: u32) -> DecodedImage {
        DecodedImage {
            width,
            height,
            rgba: vec![127; width as usize * height as usize * 4],
        }
    }
    fn decoded(digest: u8, width: u32, height: u32) -> FetchResult {
        FetchResult::Decoded {
            fingerprint: fingerprint(digest, 1),
            image: image(width, height),
        }
    }
    fn finish(cache: &mut ArtworkCache, work: Work, digest: u8, now: u64) -> Applied {
        assert_eq!(
            cache.submit_result(work, decoded(digest, 1, 1)),
            SubmitOutcome::Queued
        );
        cache.apply_next(now).unwrap()
    }
    #[test]
    fn unchanged_poll_preserves_resident_and_inflight_across_catalogue_generations() {
        let sources = catalogue(&[("app.a", "1", "icon.png")]);
        let mut c = cache(64);
        c.update_catalogue(&sources).unwrap();
        let work = c.next_work(0).unwrap();
        let later = c.update_catalogue(&sources).unwrap();
        assert_eq!(later.generation, 2);
        let applied = finish(&mut c, work, 1, 0);
        let key = applied.changed.unwrap();
        assert!(applied.evicted.is_empty());
        c.update_catalogue(&sources).unwrap();
        assert!(c.next_work(1).is_none());
        assert_eq!(c.resident("app.a").unwrap().key, &key);
    }
    #[test]
    fn pending_result_survives_unchanged_poll_and_age_refresh_supplies_known_digest() {
        let sources = catalogue(&[("app.a", "1", "icon.png")]);
        let mut c = cache(64);
        c.update_catalogue(&sources).unwrap();
        let w = c.next_work(0).unwrap();
        c.submit_result(w, decoded(1, 1, 1));
        c.update_catalogue(&sources).unwrap();
        assert_eq!(c.stats().pending_results, 1);
        let old = c.apply_next(0).unwrap().changed.unwrap();
        assert!(c.next_work(299_999).is_none());
        let w = c.next_work(300_000).unwrap();
        assert_eq!(w.known().unwrap().content_sha256, [1; 32]);
        c.submit_result(w, FetchResult::Unchanged(fingerprint(1, 2)));
        let applied = c.apply_next(300_000).unwrap();
        assert!(applied.changed.is_none() && applied.evicted.is_empty());
        assert_eq!(c.resident("app.a").unwrap().key, &old);
    }
    #[test]
    fn new_install_version_and_path_changes_have_distinct_epochs_and_no_old_pixels() {
        let mut c = cache(64);
        c.update_catalogue(&catalogue(&[("app.a", "1", "icon.png")]))
            .unwrap();
        let w = c.next_work(0).unwrap();
        let old = finish(&mut c, w, 1, 0).changed.unwrap();
        let update = c
            .update_catalogue(&catalogue(&[
                ("app.a", "2", "icon.png"),
                ("app.b", "1", "new.png"),
            ]))
            .unwrap();
        assert_eq!(update.evicted, vec![old]);
        assert!(c.resident("app.a").is_none());
        let outdated = c.next_work(1).unwrap();
        assert_eq!(outdated.source().version, "2");
        c.update_catalogue(&catalogue(&[
            ("app.a", "2", "replacement.png"),
            ("app.b", "1", "new.png"),
        ]))
        .unwrap();
        assert_eq!(
            c.submit_result(outdated, decoded(2, 1, 1)),
            SubmitOutcome::Stale
        );
        let fresh = c.next_work(1).unwrap();
        assert_eq!(fresh.source().relative, vec!["replacement.png"]);
        finish(&mut c, fresh, 3, 1);
        let installed = c.next_work(1).unwrap();
        assert_eq!(installed.source().app_id, "app.b");
    }
    #[test]
    fn same_path_refresh_updates_identity_or_content_and_exact_revision_ownership() {
        let mut c = cache(64);
        c.update_catalogue(&catalogue(&[("app.a", "1", "icon.png")]))
            .unwrap();
        let w = c.next_work(0).unwrap();
        let old = finish(&mut c, w, 1, 0).changed.unwrap();
        assert!(c.request_refresh("app.a"));
        let w = c.next_work(1).unwrap();
        assert_eq!(w.known().unwrap().content_sha256, [1; 32]);
        assert_eq!(
            c.submit_result(w, FetchResult::Unchanged(fingerprint(1, 99))),
            SubmitOutcome::Queued
        );
        let applied = c.apply_next(1).unwrap();
        assert!(applied.changed.is_none() && applied.evicted.is_empty());
        let view = c.resident("app.a").unwrap();
        assert_eq!(view.key, &old);
        assert_eq!(view.fingerprint.file_identity.inode, 99);
        c.request_refresh("app.a");
        let w = c.next_work(2).unwrap();
        let update = finish(&mut c, w, 2, 2);
        assert_eq!(update.evicted, vec![old.clone()]);
        let new = update.changed.unwrap();
        assert_eq!(new.source, old.source);
        assert_ne!(new.revision, old.revision);
    }
    #[test]
    fn removal_drops_ready_queued_results_and_inflight_without_starting_two_workers() {
        let mut c = cache(64);
        let all = catalogue(&[("app.a", "1", "a.png"), ("app.b", "1", "b.png")]);
        c.update_catalogue(&all).unwrap();
        let a = c.next_work(0).unwrap();
        assert_eq!(c.submit_result(a, decoded(1, 1, 1)), SubmitOutcome::Queued);
        let b = c.next_work(0).unwrap();
        assert!(c.next_work(0).is_none());
        c.update_catalogue(&SourceMap::new()).unwrap();
        assert_eq!(c.stats().pending_result_bytes, 0);
        assert!(c.apply_next(0).is_none());
        c.update_catalogue(&all).unwrap();
        assert!(c.next_work(0).is_none());
        assert_eq!(c.submit_result(b, decoded(2, 1, 1)), SubmitOutcome::Stale);
        assert!(c.next_work(0).is_some());
    }
    #[test]
    fn visible_then_favourites_then_catalogue_eventually_covers_more_than_64_apps() {
        let ids = (0..100).map(|n| format!("app.{n:03}")).collect::<Vec<_>>();
        let rows = ids
            .iter()
            .map(|id| (id.as_str(), "1", "icon.png"))
            .collect::<Vec<_>>();
        let mut c = cache(1024);
        c.update_catalogue(&catalogue(&rows)).unwrap();
        c.set_interest(&[ids[90].clone()], &[ids[80].clone()])
            .unwrap();
        let mut seen = BTreeSet::new();
        for i in 0..100 {
            let w = c.next_work(0).unwrap();
            if i == 0 {
                assert_eq!(w.source().app_id, ids[90]);
            }
            if i == 1 {
                assert_eq!(w.source().app_id, ids[80]);
            }
            assert!(seen.insert(w.source().app_id.clone()));
            assert!(c.stats().queued <= MAX_QUEUED);
            finish(&mut c, w, 1, 0);
        }
        assert_eq!(seen.len(), 100);
        assert!(c.next_work(0).is_none());
    }
    #[test]
    fn pending_result_bytes_slots_and_actual_resident_lru_bytes_are_bounded() {
        let mut c = cache(MAX_RESIDENT_BYTES);
        c.update_catalogue(&catalogue(&[
            ("app.a", "1", "a"),
            ("app.b", "1", "b"),
            ("app.c", "1", "c"),
        ]))
        .unwrap();
        for _ in 0..2 {
            let w = c.next_work(0).unwrap();
            assert_eq!(
                c.submit_result(w, decoded(1, 512, 512)),
                SubmitOutcome::Queued
            );
        }
        assert_eq!(c.stats().pending_results, 2);
        assert_eq!(c.stats().pending_result_bytes, MAX_RESULT_BYTES);
        assert!(c.next_work(0).is_none());
        c.apply_next(0);
        assert_eq!(c.stats().pending_result_bytes, MAX_IMAGE_BYTES);
        assert!(c.next_work(0).is_some());
        let mut c = cache(8);
        c.update_catalogue(&catalogue(&[
            ("app.a", "1", "a"),
            ("app.b", "1", "b"),
            ("app.c", "1", "c"),
        ]))
        .unwrap();
        let a = c.next_work(0).unwrap();
        let a_key = finish(&mut c, a, 1, 0).changed.unwrap();
        let b = c.next_work(0).unwrap();
        let b_key = finish(&mut c, b, 1, 0).changed.unwrap();
        c.resident("app.a").unwrap();
        let w = c.next_work(0).unwrap();
        let applied = finish(&mut c, w, 1, 0);
        assert_eq!(applied.evicted, vec![b_key]);
        assert_eq!(c.resident("app.a").unwrap().key, &a_key);
        assert!(c.resident("app.b").is_none());
        assert_eq!(c.stats().resident_bytes, 8);
    }
    #[test]
    fn small_lru_does_not_starve_new_apps_and_interest_reloads_only_evicted_entries() {
        let ids = (0..100).map(|n| format!("app.{n:03}")).collect::<Vec<_>>();
        let rows = ids
            .iter()
            .map(|id| (id.as_str(), "1", "icon.png"))
            .collect::<Vec<_>>();
        let mut c = cache(8);
        c.update_catalogue(&catalogue(&rows)).unwrap();
        let mut seen = BTreeSet::new();
        for _ in 0..100 {
            let w = c.next_work(0).unwrap();
            assert!(seen.insert(w.source().app_id.clone()));
            finish(&mut c, w, 1, 0);
            assert!(c.stats().resident_bytes <= 8);
        }
        assert_eq!(seen.len(), 100);
        assert!(c.next_work(0).is_none());
        assert!(c.resident(&ids[0]).is_none());
        c.set_interest(&[ids[0].clone()], &[]).unwrap();
        let w = c.next_work(1).unwrap();
        assert_eq!(w.source().app_id, ids[0]);
        assert!(w.known().is_none());
        c.submit_result(w, FetchResult::Failed);
        c.apply_next(1);
        c.set_interest(&[], &[]).unwrap();
        c.set_interest(&[ids[0].clone()], &[]).unwrap();
        assert!(c.next_work(1).is_none());
    }
    #[test]
    fn reserved_pixel_capacity_counts_even_when_logical_image_is_smaller() {
        let mut c = cache(64);
        c.update_catalogue(&catalogue(&[("app.a", "1", "a")]))
            .unwrap();
        let mut rgba = Vec::with_capacity(32);
        rgba.extend_from_slice(&[1, 2, 3, 4]);
        let actual = rgba.capacity();
        let w = c.next_work(0).unwrap();
        c.submit_result(
            w,
            FetchResult::Decoded {
                fingerprint: fingerprint(1, 1),
                image: DecodedImage {
                    width: 1,
                    height: 1,
                    rgba,
                },
            },
        );
        assert_eq!(c.stats().pending_result_bytes, actual);
        c.apply_next(0);
        assert_eq!(c.stats().resident_bytes, actual);
        c.request_refresh("app.a");
        let w = c.next_work(1).unwrap();
        let mut too_large = Vec::with_capacity(MAX_IMAGE_BYTES + 1);
        too_large.extend_from_slice(&[1, 2, 3, 4]);
        c.submit_result(
            w,
            FetchResult::Decoded {
                fingerprint: fingerprint(2, 1),
                image: DecodedImage {
                    width: 1,
                    height: 1,
                    rgba: too_large,
                },
            },
        );
        assert_eq!(c.stats().pending_result_bytes, 0);
        c.apply_next(1);
        assert_eq!(c.stats().resident_bytes, actual);
    }
    #[test]
    fn refresh_of_evicted_inflight_image_refetches_pixels_without_accepting_empty_unchanged() {
        let mut c = cache(8);
        c.update_catalogue(&catalogue(&[
            ("app.a", "1", "a"),
            ("app.b", "1", "b"),
            ("app.c", "1", "c"),
        ]))
        .unwrap();
        let a = c.next_work(0).unwrap();
        let a_key = finish(&mut c, a, 1, 0).changed.unwrap();
        let b = c.next_work(0).unwrap();
        finish(&mut c, b, 2, 0);
        let queued = c.next_work(0).unwrap();
        assert_eq!(queued.source().app_id, "app.c");
        c.submit_result(queued, decoded(3, 1, 1));
        c.request_refresh("app.a");
        let refreshing = c.next_work(0).unwrap();
        assert!(refreshing.known().is_some());
        assert_eq!(c.apply_next(0).unwrap().evicted, vec![a_key]);
        assert!(c.resident("app.a").is_none());
        c.submit_result(refreshing, FetchResult::Unchanged(fingerprint(1, 2)));
        assert!(c.apply_next(0).unwrap().changed.is_none());
        let refetch = c.next_work(0).unwrap();
        assert_eq!(refetch.source().app_id, "app.a");
        assert!(refetch.known().is_none());
    }
    #[test]
    fn negative_results_retry_after_delay_without_destroying_packaged_fallback() {
        let mut c = cache(64);
        c.update_catalogue(&catalogue(&[("app.a", "1", "a")]))
            .unwrap();
        let w = c.next_work(0).unwrap();
        c.submit_result(w, FetchResult::Failed);
        c.apply_next(5);
        assert!(c.next_work(30_004).is_none());
        let retry = c.next_work(30_005).unwrap();
        assert!(retry.known().is_none());
        let old = finish(&mut c, retry, 1, 30_005).changed.unwrap();
        c.request_refresh("app.a");
        let w = c.next_work(30_006).unwrap();
        c.submit_result(w, FetchResult::Failed);
        c.apply_next(30_006);
        assert_eq!(c.resident("app.a").unwrap().key, &old);
        // Packaged fallback is not stored in this module and cannot be removed
        // by a negative fetch, eviction, catalogue refresh or shutdown.
    }
    #[test]
    fn invalid_images_and_false_unchanged_results_are_negative_not_resident() {
        let mut c = cache(MAX_RESIDENT_BYTES);
        c.update_catalogue(&catalogue(&[("app.a", "1", "a")]))
            .unwrap();
        for result in [
            FetchResult::Decoded {
                fingerprint: fingerprint(1, 1),
                image: DecodedImage {
                    width: 513,
                    height: 1,
                    rgba: vec![0; 2052],
                },
            },
            FetchResult::Decoded {
                fingerprint: fingerprint(1, 1),
                image: DecodedImage {
                    width: 2,
                    height: 2,
                    rgba: vec![0; 4],
                },
            },
            FetchResult::Unchanged(fingerprint(1, 1)),
        ] {
            c.request_refresh("app.a");
            let w = c.next_work(0).unwrap();
            c.submit_result(w, result);
            let applied = c.apply_next(0).unwrap();
            assert!(applied.changed.is_none());
            assert!(c.resident("app.a").is_none());
        }
    }
    #[test]
    fn refresh_requested_during_work_is_not_lost_and_duplicate_results_cannot_clear_new_work() {
        let mut c = cache(64);
        c.update_catalogue(&catalogue(&[("app.a", "1", "a")]))
            .unwrap();
        let old = c.next_work(0).unwrap();
        c.request_refresh("app.a");
        finish(&mut c, old.clone(), 1, 0);
        let new = c.next_work(0).unwrap();
        assert_ne!(new.id(), old.id());
        assert_eq!(
            c.submit_result(old, FetchResult::Failed),
            SubmitOutcome::Unmatched
        );
        assert_eq!(c.stats().in_flight, 1);
        finish(&mut c, new, 2, 0);
        assert!(c.next_work(0).is_none());
    }
    #[test]
    fn shutdown_frees_owned_buffers_returns_evictions_and_preserves_worker_join_obligation() {
        let mut c = cache(64);
        let sources = catalogue(&[
            ("app.a", "1", "a"),
            ("app.b", "1", "b"),
            ("app.c", "1", "c"),
        ]);
        c.update_catalogue(&sources).unwrap();
        let w = c.next_work(0).unwrap();
        let key = finish(&mut c, w, 1, 0).changed.unwrap();
        let pending = c.next_work(0).unwrap();
        c.submit_result(pending, decoded(2, 1, 1));
        let w = c.next_work(0).unwrap();
        assert_eq!(c.stats().pending_result_bytes, 4);
        let shut = c.shutdown();
        assert_eq!(shut.evicted, vec![key]);
        assert_eq!(shut.in_flight, Some(w.clone()));
        assert!(c.next_work(0).is_none());
        assert_eq!(c.stats().resident_bytes, 0);
        assert_eq!(c.stats().pending_result_bytes, 0);
        assert_eq!(c.update_catalogue(&sources).unwrap_err(), Error::Closed);
        assert_eq!(c.submit_result(w, decoded(2, 1, 1)), SubmitOutcome::Stale);
        assert_eq!(c.stats().in_flight, 0);
        assert!(c.shutdown().evicted.is_empty());
    }
    #[test]
    fn malformed_or_oversized_catalogues_fail_atomically() {
        let mut c = cache(64);
        let mut sources = catalogue(&[("app.a", "1", "a")]);
        c.update_catalogue(&sources).unwrap();
        let w = c.next_work(0).unwrap();
        sources.get_mut("app.a").unwrap().app_id = "app.other".into();
        assert_eq!(
            c.update_catalogue(&sources).unwrap_err(),
            Error::InvalidCatalogue
        );
        assert_eq!(c.stats().entries, 1);
        finish(&mut c, w, 1, 0);
        assert!(c.resident("app.a").is_some());
        let ids = (0..513).map(|n| format!("app.{n}")).collect::<Vec<_>>();
        let mut oversized = SourceMap::new();
        // AppSources without a Source are valid fallback-only entries, but
        // still count toward the catalogue admission bound.
        for id in ids {
            oversized.insert(
                id.clone(),
                crate::app_artwork::AppSources {
                    app_id: id,
                    version: None,
                    icon: None,
                    large_icon: None,
                    rejections: vec![],
                },
            );
        }
        assert_eq!(
            c.update_catalogue(&oversized).unwrap_err(),
            Error::TooManyEntries
        );
        assert_eq!(c.stats().entries, 1);
        assert!(ArtworkCache::new(Config {
            resident_bytes: 0,
            ..Config::default()
        })
        .is_err());
    }
}
