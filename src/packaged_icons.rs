//! Ordinary packaged fallback and opt-in durable-cache reads. One worker owns anchored reads/decode; this
//! render-thread controller alone owns GL handles. Fresh Source metadata is
//! supplied independently; cache bytes grant no source or Root authority. No Root session,
//! Ticket, action, keyboard or cache-publication API exists here. The optional
//! cache opener can create only its fixed own private directory. Bounds describe
//! owned payloads/texture estimates, not total heap/GPU use or a hard shutdown
//! time: a blocked filesystem/codec call cannot be interrupted by these flags.
use crate::{
    app_artwork::{Source, SourceMap},
    artwork_textures::TextureBackend,
    durable_icon_cache::{self as disk, CachedIcon, ProducerVersion, Selection, Snapshot},
    packaged_icon_reader::{
        DecodedIcon, PackagedIconReader, RelativeIconPath, MAX_AXIS, MAX_RGBA_BYTES,
    },
};
use std::{
    collections::{BTreeMap, BTreeSet},
    fs::OpenOptions,
    io,
    marker::PhantomData,
    os::unix::fs::OpenOptionsExt,
    path::PathBuf,
    rc::Rc,
    sync::{
        atomic::{AtomicBool, Ordering},
        mpsc::{self, Receiver, SyncSender, TryRecvError},
        Arc,
    },
    thread::{self, JoinHandle},
};
pub const MAX_SOURCES: usize = 512;
pub const MAX_TEXTURES: usize = 64;
pub const MAX_TEXTURE_BYTES: usize = 16 * 1024 * 1024;
pub type Bindings = BTreeMap<String, RelativeIconPath>;
#[derive(Clone, Debug, PartialEq, Eq)]
struct Key {
    epoch: u64,
    id: String,
    relative: Option<RelativeIconPath>,
    dynamic: Option<Box<DynamicKey>>,
}
#[derive(Clone, Debug, PartialEq, Eq)]
struct DynamicKey {
    source: Source,
    producer: ProducerVersion,
    selection: Selection,
}
struct Job {
    key: Key,
    abandoned: Arc<AtomicBool>,
    cached: Option<(Arc<Snapshot>, Box<disk::Demand>)>,
}
enum Pixels {
    Loaded(DecodedIcon),
    Cached(Box<CachedIcon>),
    Fallback(DecodedIcon),
    Failed,
    Abandoned,
}
struct Reply {
    key: Key,
    pixels: Pixels,
}
struct Pending {
    key: Key,
    abandoned: Arc<AtomicBool>,
}
trait Loader {
    fn decode(&mut self, path: &RelativeIconPath) -> io::Result<DecodedIcon>;
    fn adopt(&mut self) -> io::Result<Adoption> {
        Ok(Adoption {
            association: None,
            producer: None,
            snapshot: None,
        })
    }
    fn decode_cached(
        &mut self,
        snapshot: &Snapshot,
        demand: disk::Demand,
    ) -> io::Result<CachedIcon> {
        snapshot.decode(demand)
    }
}
struct Adoption {
    association: Option<Box<crate::cache_consumer_profile::ConsumerAssociation>>,
    // Independently supplied/profile-loaded on worker, NEVER cache metadata.
    producer: Option<ProducerVersion>,
    snapshot: Option<Arc<Snapshot>>,
}
struct CacheLoader {
    association: Option<crate::cache_consumer_profile::ConsumerAssociation>,
    packaged: PackagedIconReader,
    cache: Option<disk::Cache>,
    producer: Option<ProducerVersion>,
}
impl Loader for CacheLoader {
    fn decode(&mut self, path: &RelativeIconPath) -> io::Result<DecodedIcon> {
        self.packaged.decode(path)
    }
    fn adopt(&mut self) -> io::Result<Adoption> {
        Ok(Adoption {
            association: self.association.clone().map(Box::new),
            producer: self.producer.clone(),
            snapshot: self
                .cache
                .as_ref()
                .map(|cache| cache.snapshot().map(Arc::new))
                .transpose()?,
        })
    }
}
impl Loader for PackagedIconReader {
    fn decode(&mut self, path: &RelativeIconPath) -> io::Result<DecodedIcon> {
        PackagedIconReader::decode(self, path)
    }
}
enum Command {
    Decode(Job),
    Adopt(u64),
}
enum Event {
    Image(Reply),
    Adopted { request: u64, adoption: Adoption },
}
struct Worker {
    jobs: Option<SyncSender<Command>>,
    replies: Option<Receiver<Event>>,
    stop: Arc<AtomicBool>,
    join: Option<JoinHandle<()>>,
}
impl Worker {
    fn start<L: Loader + 'static>(
        factory: impl FnOnce() -> io::Result<L> + Send + 'static,
    ) -> io::Result<Self> {
        let (jobs, incoming) = mpsc::sync_channel::<Command>(1);
        let (complete, replies) = mpsc::sync_channel(1);
        let stop = Arc::new(AtomicBool::new(false));
        let stopped = stop.clone();
        let join = thread::Builder::new()
            .name("packaged-icons".into())
            .spawn(move || {
                let mut loader = factory().ok();
                while let Ok(command) = incoming.recv() {
                    if stopped.load(Ordering::Acquire) {
                        break;
                    }
                    let job = match command {
                        Command::Adopt(request) => {
                            // Ordered after the prior Decode and its sole result
                            // drain. No new thread or UI filesystem work exists.
                            let adoption =
                                loader
                                    .as_mut()
                                    .and_then(|l| l.adopt().ok())
                                    .unwrap_or(Adoption {
                                        association: None,
                                        producer: None,
                                        snapshot: None,
                                    });
                            if stopped.load(Ordering::Acquire) {
                                break;
                            }
                            if complete
                                .try_send(Event::Adopted { request, adoption })
                                .is_err()
                            {
                                break;
                            }
                            continue;
                        }
                        Command::Decode(job) => job,
                    };
                    let mut pixels = if job.abandoned.load(Ordering::Acquire) {
                        Pixels::Abandoned
                    } else if let Some(reader) = loader.as_mut() {
                        if let Some((snapshot, demand)) = job.cached {
                            match reader.decode_cached(&snapshot, *demand) {
                                Ok(icon) => Pixels::Cached(Box::new(icon)),
                                Err(_) if !job.abandoned.load(Ordering::Acquire) => job
                                    .key
                                    .relative
                                    .as_ref()
                                    .and_then(|p| reader.decode(p).ok())
                                    .filter(valid_image)
                                    .map(Pixels::Fallback)
                                    .unwrap_or(Pixels::Failed),
                                Err(_) => Pixels::Abandoned,
                            }
                        } else {
                            job.key
                                .relative
                                .as_ref()
                                .and_then(|p| reader.decode(p).ok())
                                .filter(valid_image)
                                .map(Pixels::Loaded)
                                .unwrap_or(Pixels::Failed)
                        }
                    } else {
                        Pixels::Failed
                    };
                    if stopped.load(Ordering::Acquire) {
                        break;
                    }
                    if job.abandoned.load(Ordering::Acquire) {
                        pixels = Pixels::Abandoned;
                    }
                    if complete
                        .try_send(Event::Image(Reply {
                            key: job.key,
                            pixels,
                        }))
                        .is_err()
                    {
                        break;
                    }
                }
            })?;
        Ok(Self {
            jobs: Some(jobs),
            replies: Some(replies),
            stop,
            join: Some(join),
        })
    }
    fn finish(&mut self) -> bool {
        self.stop.store(true, Ordering::Release);
        drop(self.replies.take()); // Drop a queued RGBA buffer before waiting.
        drop(self.jobs.take()); // Wake an idle recv; no new job can arrive.
        self.join.take().is_none_or(|join| join.join().is_ok())
    }
}
impl Drop for Worker {
    fn drop(&mut self) {
        let _ = self.finish();
    }
}
fn valid_image(image: &DecodedIcon) -> bool {
    let size = u64::from(image.width)
        .checked_mul(u64::from(image.height))
        .and_then(|n| n.checked_mul(4));
    image.width > 0
        && image.height > 0
        && image.width <= MAX_AXIS
        && image.height <= MAX_AXIS
        && size == Some(image.rgba.len() as u64)
        && image.rgba.len() <= MAX_RGBA_BYTES
        && image.rgba.capacity() <= MAX_RGBA_BYTES
}
fn valid_bindings(bindings: &Bindings) -> bool {
    bindings.len() <= MAX_SOURCES
        && bindings.keys().all(|id| {
            !id.is_empty()
                && id.len() <= 180
                && id
                    .bytes()
                    .all(|b| b.is_ascii_alphanumeric() || b"._-".contains(&b))
        })
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ConfigureError {
    InvalidBindings,
    EpochExhausted,
    Closed,
}
/// Metadata/index readiness only, never image, lifecycle or speed qualification.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum CacheReadiness {
    #[default]
    Disabled,
    AwaitingSources,
    Pending,
    Ready,
    Unavailable,
}
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Stats {
    pub texture_entries: usize,
    pub texture_bytes_estimate: usize,
    pub submitted: u64,
    pub completed: u64,
    pub discarded: u64,
    pub unavailable_sources: usize,
    pub upload_failures: u64,
    pub aliased_handles: u64,
    pub worker_closed: bool,
    pub cache_readiness: CacheReadiness,
    pub cache_adoptions: u64,
    pub cache_uploads: u64,
    pub packaged_uploads: u64,
}
#[derive(Default, Debug, PartialEq, Eq)]
pub struct FrameUpdate {
    pub visible_pixels_changed: bool,
    pub evicted: Vec<String>,
    pub capacity_deferred: bool,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct TextureView<H> {
    pub handle: H,
    pub width: u32,
    pub height: u32,
}
struct Owned<H> {
    key: Key,
    handle: H,
    width: u32,
    height: u32,
    bytes: usize,
    touched: u64,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Finish {
    pub worker_joined: bool,
    pub worker_panicked: bool,
    pub remaining_textures: usize,
}
/// Construct/use/drop on the thread owning the GL context. Texture views are
/// borrowed drawing references, not deletion ownership. After each frame the
/// caller MUST refresh all card references before the next draw (including
/// evicted IDs); no focus/layout rebuild is needed for pixel changes.
enum ExpectedPolicy {
    Supplied(ProducerVersion),
    WorkerProfile,
}
struct CacheMode {
    expected: ExpectedPolicy,
    producer: Option<ProducerVersion>,
    sources: BTreeMap<String, Source>,
    snapshot: Option<Arc<Snapshot>>,
    selected: BTreeMap<String, Selection>,
    failed: BTreeSet<String>,
    request: u64,
    adopting: Option<u64>,
    refresh: bool,
}
pub struct PackagedIcons<B: TextureBackend> {
    backend: B,
    worker: Option<Worker>,
    bindings: Bindings,
    cache: Option<CacheMode>,
    epoch: u64,
    pending: Option<Pending>,
    ready: Option<Reply>,
    failed: BTreeSet<String>,
    textures: BTreeMap<String, Owned<B::Handle>>,
    bytes: usize,
    clock: u64,
    reset_pending: bool,
    closed: bool,
    finished: Option<Finish>,
    stats: Stats,
    loaded_cache_association: Option<crate::cache_consumer_profile::ConsumerAssociation>,
    limits: (usize, usize),
    _render_thread: PhantomData<Rc<()>>,
}
impl<B: TextureBackend> PackagedIcons<B> {
    pub fn start(root: PathBuf, bindings: Bindings, backend: B) -> io::Result<Self> {
        Self::start_using(
            bindings,
            backend,
            (MAX_TEXTURES, MAX_TEXTURE_BYTES),
            move || {
                // Opening/anchoring and every subsequent read/decode happen here,
                // on the worker. Native callers supply their own fixed app assets.
                let directory = OpenOptions::new()
                    .read(true)
                    .custom_flags(libc::O_DIRECTORY | libc::O_NOFOLLOW | libc::O_CLOEXEC)
                    .open(root)?;
                PackagedIconReader::from_directory(directory)
            },
        )
    }
    /// Opt-in only. Expected producer must be independently artifact-admitted;
    /// no cache record supplies it. Root replacement needs finish+new controller.
    /// Both anchors, scan and decode are opened/run on this same ordinary worker.
    pub fn start_with_cache(
        root: PathBuf,
        bindings: Bindings,
        backend: B,
        config: disk::Config,
        producer: ProducerVersion,
    ) -> io::Result<Self> {
        let supplied = producer.clone();
        Self::start_cache_using(
            root,
            bindings,
            backend,
            config,
            ExpectedPolicy::Supplied(producer),
            move || {
                Ok(Some(
                    crate::cache_consumer_profile::ReadExpectation::supplied(supplied),
                ))
            },
            disk::Cache::open_current,
        )
    }
    /// Native opt-in only: fixed packaged profile and actual self-ELF measurement
    /// run on THIS image worker. Unavailable profile/cache leaves package usable.
    /// Profile/root replacement is not hot reloaded: finish+new controller.
    pub fn start_current_cache(
        root: PathBuf,
        bindings: Bindings,
        backend: B,
        config: disk::Config,
    ) -> io::Result<Self> {
        Self::start_cache_using(
            root,
            bindings,
            backend,
            config,
            ExpectedPolicy::WorkerProfile,
            crate::cache_consumer_profile::load_current,
            disk::Cache::open_current,
        )
    }
    fn start_cache_using(
        root: PathBuf,
        bindings: Bindings,
        backend: B,
        config: disk::Config,
        expected: ExpectedPolicy,
        load_expected: impl FnOnce() -> io::Result<Option<crate::cache_consumer_profile::ReadExpectation>>
            + Send
            + 'static,
        open_cache: impl FnOnce(disk::Config) -> io::Result<disk::Cache> + Send + 'static,
    ) -> io::Result<Self> {
        let mut controller = Self::start_using(
            bindings,
            backend,
            (MAX_TEXTURES, MAX_TEXTURE_BYTES),
            move || {
                let directory = OpenOptions::new()
                    .read(true)
                    .custom_flags(libc::O_DIRECTORY | libc::O_NOFOLLOW | libc::O_CLOEXEC)
                    .open(root)?;
                let packaged = PackagedIconReader::from_directory(directory)?;
                // Independent expectation ONCE, before scan; no record can fill it.
                let (producer, association) = match load_expected().ok().flatten() {
                    Some(expected) => {
                        let (producer, association) = expected.into_parts();
                        (Some(producer), association)
                    }
                    None => (None, None),
                };
                let cache = producer.as_ref().and_then(|_| open_cache(config).ok());
                Ok(CacheLoader {
                    packaged,
                    cache,
                    producer,
                    association,
                })
            },
        )?;
        controller.cache = Some(CacheMode {
            expected,
            producer: None,
            sources: BTreeMap::new(),
            snapshot: None,
            selected: BTreeMap::new(),
            failed: BTreeSet::new(),
            request: 0,
            adopting: None,
            refresh: false,
        });
        Ok(controller)
    }
    /// Pure current-catalogue selection. None MUST be called on failure/expiry,
    /// independently of privileged Runtime presence. Unchanged polls do nothing.
    pub fn update_sources(&mut self, sources: Option<&SourceMap>) -> Result<(), ConfigureError> {
        if self.closed {
            return Err(ConfigureError::Closed);
        }
        let mut selected = BTreeMap::new();
        if let Some(sources) = sources {
            if sources.len() > MAX_SOURCES {
                self.update_sources(None)?;
                return Err(ConfigureError::InvalidBindings);
            }
            for (id, entry) in sources {
                if let Some(source) = entry.preferred() {
                    if !crate::model::valid_app_id(id) || source.app_id() != id.as_str() {
                        self.update_sources(None)?;
                        return Err(ConfigureError::InvalidBindings);
                    }
                    selected.insert(id.clone(), source.clone());
                }
            }
        }
        let Some(cache) = &mut self.cache else {
            return Ok(());
        };
        if cache.sources == selected {
            return Ok(());
        }
        cache.sources = selected;
        self.refresh_cache()
    }
    /// Explicit refresh only, never routine unchanged polling. Withdraw first;
    /// the old single job/result must drain before an Adopt can enter its slot.
    /// No UI I/O, join or new root opening takes place here.
    pub fn refresh_cache(&mut self) -> Result<(), ConfigureError> {
        if self.closed {
            return Err(ConfigureError::Closed);
        }
        let epoch = self
            .epoch
            .checked_add(1)
            .ok_or(ConfigureError::EpochExhausted)?;
        let Some(cache) = &mut self.cache else {
            return Ok(());
        };
        let request = cache
            .request
            .checked_add(1)
            .ok_or(ConfigureError::EpochExhausted)?;
        if let Some(old) = cache.snapshot.take() {
            old.generation().withdraw();
        }
        cache.selected.clear();
        cache.producer = None;
        cache.failed.clear();
        cache.request = request;
        cache.refresh = !cache.sources.is_empty();
        self.epoch = epoch;
        if let Some(pending) = &self.pending {
            pending.abandoned.store(true, Ordering::Release);
        }
        self.ready = None;
        self.failed.clear();
        self.reset_pending = true;
        Ok(())
    }
    fn start_using<L: Loader + 'static>(
        bindings: Bindings,
        backend: B,
        limits: (usize, usize),
        factory: impl FnOnce() -> io::Result<L> + Send + 'static,
    ) -> io::Result<Self> {
        if !valid_bindings(&bindings)
            || limits.0 == 0
            || limits.0 > MAX_TEXTURES
            || limits.1 < 4
            || limits.1 > MAX_TEXTURE_BYTES
        {
            return Err(io::Error::new(
                io::ErrorKind::InvalidInput,
                "Invalid packaged icon configuration",
            ));
        }
        Ok(Self {
            backend,
            worker: Some(Worker::start(factory)?),
            bindings,
            cache: None,
            epoch: 1,
            pending: None,
            ready: None,
            failed: BTreeSet::new(),
            textures: BTreeMap::new(),
            bytes: 0,
            clock: 0,
            reset_pending: false,
            closed: false,
            finished: None,
            stats: Stats::default(),
            loaded_cache_association: None,
            limits,
            _render_thread: PhantomData,
        })
    }
    /// New configuration of this same anchored package, not an OS app-version
    /// measurement or hot root adoption. Actual package/root replacement needs
    /// finish+new (normally a new process). No UI-thread I/O or join occurs here.
    pub fn reconfigure(&mut self, bindings: Bindings) -> Result<(), ConfigureError> {
        if self.closed {
            return Err(ConfigureError::Closed);
        }
        if !valid_bindings(&bindings) {
            return Err(ConfigureError::InvalidBindings);
        }
        let epoch = self
            .epoch
            .checked_add(1)
            .ok_or(ConfigureError::EpochExhausted)?;
        if let Some(pending) = &self.pending {
            pending.abandoned.store(true, Ordering::Release);
        }
        self.epoch = epoch;
        self.bindings = bindings;
        self.failed.clear();
        self.ready = None;
        self.reset_pending = true;
        Ok(())
    }
    fn current(&self, key: &Key) -> bool {
        key.epoch == self.epoch
            && self.bindings.get(&key.id) == key.relative.as_ref()
            && match &key.dynamic {
                None => true,
                Some(dynamic) => self.cache.as_ref().is_some_and(|cache| {
                    cache.producer.as_ref() == Some(&dynamic.producer)
                        && cache.sources.get(&key.id) == Some(&dynamic.source)
                        && cache.selected.get(&key.id) == Some(&dynamic.selection)
                        && !cache.failed.contains(&key.id)
                        && cache
                            .snapshot
                            .as_ref()
                            .is_some_and(|s| s.generation().current())
                }),
            }
    }
    pub fn texture(&self, id: &str) -> Option<TextureView<B::Handle>> {
        let owned = self.textures.get(id)?;
        self.current(&owned.key).then_some(TextureView {
            handle: owned.handle,
            width: owned.width,
            height: owned.height,
        })
    }
    /// Diagnostic origin only, for an actually owned current cached texture.
    /// Package/fallback pixels never report a cached source. No I/O or authority.
    pub fn current_cached_source(&self, id: &str) -> Option<&crate::app_artwork::SourceKey> {
        let owned = self.textures.get(id)?;
        if !self.current(&owned.key) {
            return None;
        }
        Some(owned.key.dynamic.as_ref()?.source.key())
    }
    /// A current ordinary binding is eligible for demand loading. Keep its
    /// image area blank while warming instead of a missing-art monogram. Caller
    /// supplied Source metadata is not a continuous source-file freshness claim.
    pub fn will_load(&self, id: &str) -> bool {
        !self.closed
            && !self.stats.worker_closed
            && (self.bindings.contains_key(id)
                || self.cache.as_ref().is_some_and(|cache| {
                    cache.sources.contains_key(id)
                        && (cache.refresh
                            || cache.adopting.is_some()
                            || cache.selected.contains_key(id) && !cache.failed.contains(id))
                }))
            && !self.failed.contains(id)
    }
    fn evict(&mut self, id: &str, update: &mut FrameUpdate, visible: &BTreeSet<&str>) {
        if let Some(old) = self.textures.remove(id) {
            self.bytes -= old.bytes;
            self.backend.delete(old.handle);
            update.visible_pixels_changed |= visible.contains(id);
            update.evicted.push(id.into());
        }
    }
    fn room(&mut self, bytes: usize, visible: &BTreeSet<&str>, update: &mut FrameUpdate) -> bool {
        while self.textures.len() >= self.limits.0
            || self.bytes.saturating_add(bytes) > self.limits.1
        {
            let victim = self
                .textures
                .iter()
                .filter(|(id, _)| !visible.contains(id.as_str()))
                .min_by_key(|(_, value)| value.touched)
                .map(|(id, _)| id.clone());
            let Some(victim) = victim else {
                return false;
            };
            self.evict(&victim, update, visible);
        }
        true
    }
    /// Poll even while paused, but start no job/upload and invalidate no idle
    /// canvas. The caller supplies actual At::Drawn+clip visible IDs, in priority
    /// order, and its frontend-only gate (hidden/idle/keyboard/external intent).
    /// This loader never delays those transitions and reads no Root settlement.
    pub fn frame(&mut self, visible: &[&str], allowed: bool) -> FrameUpdate {
        let mut update = FrameUpdate::default();
        if self.closed {
            return update;
        }
        self.clock = self.clock.saturating_add(1);
        let shown = visible
            .iter()
            .copied()
            .filter(|id| {
                self.bindings.contains_key(*id)
                    || self.textures.contains_key(*id)
                    || self
                        .cache
                        .as_ref()
                        .is_some_and(|c| c.sources.contains_key(*id))
            })
            .collect::<BTreeSet<_>>();
        if let Some(pending) = &self.pending {
            if !allowed || !shown.contains(pending.key.id.as_str()) || !self.current(&pending.key) {
                pending.abandoned.store(true, Ordering::Release);
            }
        }
        if self.ready.as_ref().is_some_and(|reply| {
            !allowed || !shown.contains(reply.key.id.as_str()) || !self.current(&reply.key)
        }) {
            self.ready = None;
            self.stats.discarded = self.stats.discarded.saturating_add(1);
        }
        let polled = self
            .worker
            .as_ref()
            .and_then(|worker| worker.replies.as_ref())
            .map(Receiver::try_recv);
        match polled {
            Some(Ok(Event::Adopted { request, adoption })) => {
                if let Some(cache) = &mut self.cache {
                    let current =
                        cache.adopting.take() == Some(request) && cache.request == request;
                    let allowed_producer =
                        adoption
                            .producer
                            .as_ref()
                            .is_some_and(|producer| match &cache.expected {
                                ExpectedPolicy::Supplied(expected) => expected == producer,
                                ExpectedPolicy::WorkerProfile => true,
                            });
                    if current {
                        cache.refresh = false;
                    }
                    if current && allowed_producer {
                        if let Some(association) = adoption.association {
                            self.loaded_cache_association = Some(*association);
                        }
                        if let (Some(snapshot), Some(producer)) =
                            (adoption.snapshot, adoption.producer)
                        {
                            for (id, source) in &cache.sources {
                                if let disk::Lookup::Ready(demand) =
                                    snapshot.lookup(source, &producer)
                                {
                                    cache.selected.insert(id.clone(), demand.selection());
                                }
                            }
                            cache.producer = Some(producer);
                            cache.snapshot = Some(snapshot);
                            self.stats.cache_adoptions =
                                self.stats.cache_adoptions.saturating_add(1);
                        }
                    } else if let Some(snapshot) = adoption.snapshot {
                        snapshot.generation().withdraw();
                    }
                }
            }
            Some(Ok(Event::Image(reply))) => {
                self.stats.completed = self.stats.completed.saturating_add(1);
                let matching = self.pending.take().is_some_and(|pending| {
                    pending.key == reply.key && !pending.abandoned.load(Ordering::Acquire)
                });
                if matching && self.current(&reply.key) {
                    match reply.pixels {
                        Pixels::Fallback(image) => {
                            if let Some(cache) = &mut self.cache {
                                cache.failed.insert(reply.key.id.clone());
                            }
                            let mut key = reply.key;
                            key.dynamic = None;
                            if allowed && shown.contains(key.id.as_str()) {
                                self.ready = Some(Reply {
                                    key,
                                    pixels: Pixels::Loaded(image),
                                });
                            } else {
                                self.stats.discarded = self.stats.discarded.saturating_add(1);
                            }
                        }
                        Pixels::Cached(icon)
                            if allowed && shown.contains(reply.key.id.as_str()) =>
                        {
                            self.ready = Some(Reply {
                                key: reply.key,
                                pixels: Pixels::Cached(icon),
                            });
                        }
                        Pixels::Failed => {
                            self.failed.insert(reply.key.id);
                        }
                        Pixels::Loaded(image)
                            if allowed && shown.contains(reply.key.id.as_str()) =>
                        {
                            self.ready = Some(Reply {
                                key: reply.key,
                                pixels: Pixels::Loaded(image),
                            });
                        }
                        _ => {
                            self.stats.discarded = self.stats.discarded.saturating_add(1);
                        }
                    }
                } else {
                    self.stats.discarded = self.stats.discarded.saturating_add(1);
                }
            }
            Some(Err(TryRecvError::Disconnected)) => {
                self.pending = None;
                self.stats.worker_closed = true;
            }
            _ => {}
        }
        if !allowed {
            return update;
        }
        if self.reset_pending {
            for id in self.textures.keys().cloned().collect::<Vec<_>>() {
                self.evict(&id, &mut update, &shown);
            }
            self.reset_pending = false;
        }
        for id in &shown {
            if let Some(entry) = self.textures.get_mut(*id) {
                entry.touched = self.clock;
            }
        }
        if let Some(mut reply) = self.ready.take() {
            reply.pixels = match reply.pixels {
                Pixels::Cached(icon) => {
                    let admitted = if self.current(&reply.key) {
                        match (self.cache.as_ref(), reply.key.dynamic.as_ref()) {
                            (Some(cache), Some(dynamic))
                                if icon.selection() == dynamic.selection =>
                            {
                                cache.snapshot.as_ref().and_then(|snapshot| {
                                    icon.admit(
                                        &snapshot.generation(),
                                        &dynamic.source,
                                        &dynamic.producer,
                                    )
                                })
                            }
                            _ => None,
                        }
                    } else {
                        None
                    };
                    match admitted {
                        Some(image) => Pixels::Loaded(image),
                        None => {
                            if let Some(cache) = &mut self.cache {
                                cache.failed.insert(reply.key.id.clone());
                            }
                            self.stats.discarded = self.stats.discarded.saturating_add(1);
                            Pixels::Abandoned
                        }
                    }
                }
                other => other,
            };
            if let Pixels::Loaded(image) = reply.pixels {
                if !self.current(&reply.key)
                    || !shown.contains(reply.key.id.as_str())
                    || !valid_image(&image)
                {
                    self.stats.discarded = self.stats.discarded.saturating_add(1);
                } else if !self.room(image.rgba.len(), &shown, &mut update) {
                    update.capacity_deferred = true;
                    self.ready = Some(Reply {
                        key: reply.key,
                        pixels: Pixels::Loaded(image),
                    });
                } else {
                    match self.backend.upload(image.width, image.height, &image.rgba) {
                        Err(_) => {
                            self.stats.upload_failures =
                                self.stats.upload_failures.saturating_add(1);
                            self.failed.insert(reply.key.id);
                        }
                        Ok(handle) if self.textures.values().any(|old| old.handle == handle) => {
                            // This handle is already owned. Deleting the returned
                            // alias would destroy somebody else's valid texture.
                            self.stats.aliased_handles =
                                self.stats.aliased_handles.saturating_add(1);
                            self.failed.insert(reply.key.id);
                        }
                        Ok(handle) => {
                            if reply.key.dynamic.is_some() {
                                self.stats.cache_uploads =
                                    self.stats.cache_uploads.saturating_add(1);
                            } else {
                                self.stats.packaged_uploads =
                                    self.stats.packaged_uploads.saturating_add(1);
                            }
                            self.bytes += image.rgba.len();
                            self.textures.insert(
                                reply.key.id.clone(),
                                Owned {
                                    key: reply.key,
                                    handle,
                                    width: image.width,
                                    height: image.height,
                                    bytes: image.rgba.len(),
                                    touched: self.clock,
                                },
                            );
                            update.visible_pixels_changed = true; // Content changed even if a deleted numeric handle/dims were reused.
                        }
                    }
                }
            }
        }
        if self.pending.is_none() && self.ready.is_none() && !self.stats.worker_closed {
            if let Some(cache) = &mut self.cache {
                if cache.adopting.is_some() {
                    return update;
                }
                if cache.refresh {
                    let request = cache.request;
                    if self
                        .worker
                        .as_ref()
                        .and_then(|w| w.jobs.as_ref())
                        .is_some_and(|sender| sender.try_send(Command::Adopt(request)).is_ok())
                    {
                        cache.adopting = Some(request);
                    } else {
                        self.stats.worker_closed = true;
                    }
                    return update;
                }
            }
            if let Some(id) = visible.iter().copied().find(|id| {
                self.will_load(id) && !self.failed.contains(*id) && self.texture(id).is_none()
            }) {
                let key = Key {
                    epoch: self.epoch,
                    id: id.into(),
                    relative: self.bindings.get(id).cloned(),
                    dynamic: None,
                };
                let mut key = key;
                let cached = self.cache.as_ref().and_then(|cache| {
                    if cache.failed.contains(id) {
                        return None;
                    }
                    let snapshot = cache.snapshot.as_ref()?;
                    let source = cache.sources.get(id)?;
                    let producer = cache.producer.as_ref()?;
                    match snapshot.lookup(source, producer) {
                        disk::Lookup::Ready(demand)
                            if cache.selected.get(id) == Some(&demand.selection()) =>
                        {
                            key.dynamic = Some(Box::new(DynamicKey {
                                source: source.clone(),
                                producer: producer.clone(),
                                selection: demand.selection(),
                            }));
                            Some((snapshot.clone(), demand))
                        }
                        _ => None,
                    }
                });
                let abandoned = Arc::new(AtomicBool::new(false));
                let job = Job {
                    key: key.clone(),
                    abandoned: abandoned.clone(),
                    cached,
                };
                if self
                    .worker
                    .as_ref()
                    .and_then(|worker| worker.jobs.as_ref())
                    .is_some_and(|sender| sender.try_send(Command::Decode(job)).is_ok())
                {
                    self.pending = Some(Pending { key, abandoned });
                    self.stats.submitted = self.stats.submitted.saturating_add(1);
                } else {
                    self.stats.worker_closed = true;
                }
            }
        }
        update
    }
    /// One worker-loaded association observation; independent of current image readiness.
    pub fn cache_association_observation(
        &self,
    ) -> Option<&crate::cache_consumer_profile::ConsumerAssociation> {
        self.loaded_cache_association.as_ref()
    }
    pub fn stats(&self) -> Stats {
        Stats {
            texture_entries: self.textures.len(),
            texture_bytes_estimate: self.bytes,
            unavailable_sources: self.failed.len(),
            cache_readiness: match &self.cache {
                _ if self.closed => CacheReadiness::Disabled,
                None => CacheReadiness::Disabled,
                Some(cache) if cache.sources.is_empty() => CacheReadiness::AwaitingSources,
                Some(cache) if cache.refresh || cache.adopting.is_some() => CacheReadiness::Pending,
                Some(cache) if cache.producer.is_some() && cache.snapshot.is_some() => {
                    CacheReadiness::Ready
                }
                Some(_) => CacheReadiness::Unavailable,
            },
            ..self.stats
        }
    }
    pub fn finish(&mut self) -> Finish {
        if let Some(finished) = self.finished {
            return finished;
        }
        if let Some(cache) = &mut self.cache {
            if let Some(snapshot) = cache.snapshot.take() {
                snapshot.generation().withdraw();
            }
            cache.selected.clear();
            cache.sources.clear();
        }
        let joined = self.worker.take().is_none_or(|mut worker| worker.finish());
        self.pending = None;
        self.ready = None;
        for old in std::mem::take(&mut self.textures).into_values() {
            self.backend.delete(old.handle);
        }
        self.bytes = 0;
        self.closed = true;
        let finished = Finish {
            worker_joined: true,
            worker_panicked: !joined,
            remaining_textures: 0,
        };
        self.finished = Some(finished);
        finished
    }
}
impl<B: TextureBackend> Drop for PackagedIcons<B> {
    fn drop(&mut self) {
        let _ = self.finish();
    }
}
#[cfg(test)]
#[path = "packaged_icons_tests.rs"]
mod tests;

#[cfg(test)]
#[path = "packaged_icons_cache_tests.rs"]
mod cache_tests;
