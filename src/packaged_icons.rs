//! Ordinary packaged fallback only. One worker owns anchored reads/decode; this
//! render-thread controller alone owns GL handles. No live-source, Root session,
//! Ticket, action, keyboard or filesystem-write API exists here. Bounds describe
//! owned payloads/texture estimates, not total heap/GPU use or a hard shutdown
//! time: a blocked filesystem/codec call cannot be interrupted by these flags.
use crate::{
    artwork_textures::TextureBackend,
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
    relative: RelativeIconPath,
}
struct Job {
    key: Key,
    abandoned: Arc<AtomicBool>,
}
enum Pixels {
    Loaded(DecodedIcon),
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
}
impl Loader for PackagedIconReader {
    fn decode(&mut self, path: &RelativeIconPath) -> io::Result<DecodedIcon> {
        PackagedIconReader::decode(self, path)
    }
}
struct Worker {
    jobs: Option<SyncSender<Job>>,
    replies: Option<Receiver<Reply>>,
    stop: Arc<AtomicBool>,
    join: Option<JoinHandle<()>>,
}
impl Worker {
    fn start<L: Loader + 'static>(
        factory: impl FnOnce() -> io::Result<L> + Send + 'static,
    ) -> io::Result<Self> {
        let (jobs, incoming) = mpsc::sync_channel::<Job>(1);
        let (complete, replies) = mpsc::sync_channel(1);
        let stop = Arc::new(AtomicBool::new(false));
        let stopped = stop.clone();
        let join = thread::Builder::new()
            .name("packaged-icons".into())
            .spawn(move || {
                let mut loader = factory().ok();
                while let Ok(job) = incoming.recv() {
                    if stopped.load(Ordering::Acquire) {
                        break;
                    }
                    let mut pixels = if job.abandoned.load(Ordering::Acquire) {
                        Pixels::Abandoned
                    } else {
                        match loader
                            .as_mut()
                            .and_then(|reader| reader.decode(&job.key.relative).ok())
                        {
                            Some(image) if valid_image(&image) => Pixels::Loaded(image),
                            _ => Pixels::Failed,
                        }
                    };
                    if stopped.load(Ordering::Acquire) {
                        break;
                    }
                    if job.abandoned.load(Ordering::Acquire) {
                        pixels = Pixels::Abandoned;
                    }
                    // Never block worker exit on a main-thread result drain. There is
                    // only one outstanding job, so a full result slot is unexpected.
                    if complete
                        .try_send(Reply {
                            key: job.key,
                            pixels,
                        })
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
pub struct PackagedIcons<B: TextureBackend> {
    backend: B,
    worker: Option<Worker>,
    bindings: Bindings,
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
        key.epoch == self.epoch && self.bindings.get(&key.id) == Some(&key.relative)
    }
    pub fn texture(&self, id: &str) -> Option<TextureView<B::Handle>> {
        let owned = self.textures.get(id)?;
        self.current(&owned.key).then_some(TextureView {
            handle: owned.handle,
            width: owned.width,
            height: owned.height,
        })
    }
    /// A known packaged source is still eligible for demand loading. Callers
    /// can keep its image area blank while warming instead of showing a
    /// missing-art monogram. This is no live-source or version claim.
    pub fn will_load(&self, id: &str) -> bool {
        !self.closed
            && !self.stats.worker_closed
            && self.bindings.contains_key(id)
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
            .filter(|id| self.bindings.contains_key(*id) || self.textures.contains_key(*id))
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
            Some(Ok(reply)) => {
                self.stats.completed = self.stats.completed.saturating_add(1);
                let matching = self.pending.take().is_some_and(|pending| {
                    pending.key == reply.key && !pending.abandoned.load(Ordering::Acquire)
                });
                if matching && self.current(&reply.key) {
                    match reply.pixels {
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
        if let Some(reply) = self.ready.take() {
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
            if let Some(id) = visible.iter().copied().find(|id| {
                self.bindings.contains_key(*id)
                    && !self.failed.contains(*id)
                    && self.texture(id).is_none()
            }) {
                let key = Key {
                    epoch: self.epoch,
                    id: id.into(),
                    relative: self.bindings[id].clone(),
                };
                let abandoned = Arc::new(AtomicBool::new(false));
                let job = Job {
                    key: key.clone(),
                    abandoned: abandoned.clone(),
                };
                if self
                    .worker
                    .as_ref()
                    .and_then(|worker| worker.jobs.as_ref())
                    .is_some_and(|sender| sender.try_send(job).is_ok())
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
    pub fn stats(&self) -> Stats {
        Stats {
            texture_entries: self.textures.len(),
            texture_bytes_estimate: self.bytes,
            unavailable_sources: self.failed.len(),
            ..self.stats
        }
    }
    pub fn finish(&mut self) -> Finish {
        if let Some(finished) = self.finished {
            return finished;
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
