//! One outstanding artwork fetch/decode, with provider ownership confined to
//! its worker thread. No GL, privileged launch, URI or filesystem fallback.
//! finish/Drop join the real thread; cancellation/deadlines cannot interrupt a
//! blocked initializer, provider, codec or filesystem syscall. There is no hard
//! total-heap or shutdown-time claim. Panic recovery requires an unwind build;
//! the native panic=abort profile still aborts on a panic.
use crate::app_artwork::{self, EncodedFile, ReadReason, Source};
use crate::artwork_cache::{
    DecodedImage, FetchResult, Fingerprint, Work, MAX_AXIS, MAX_IMAGE_BYTES,
};
use std::{
    io,
    panic::{catch_unwind, AssertUnwindSafe},
    sync::{
        atomic::{AtomicBool, Ordering},
        mpsc::{self, Receiver, SyncSender, TryRecvError, TrySendError},
        Arc,
    },
    thread::{self, JoinHandle},
    time::Instant,
};

#[derive(Clone)]
pub struct Cancellation {
    stop: Arc<AtomicBool>,
    job: Arc<AtomicBool>,
}
impl Cancellation {
    pub fn is_cancelled(&self) -> bool {
        self.stop.load(Ordering::Acquire) || self.job.load(Ordering::Acquire)
    }
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ProviderError {
    Unavailable,
    Denied,
    Changed,
    Invalid,
    Cancelled,
    Expired,
}
/// Construct the provider on the worker thread. A provider itself need not be
/// Send: its LS2 handle or other thread-local state never leaves that thread.
pub trait Provider {
    fn fetch(
        &mut self,
        source: &Source,
        deadline: Instant,
        cancellation: &Cancellation,
    ) -> Result<EncodedFile, ProviderError>;
}
#[derive(Default)]
pub struct DirectProvider;
impl Provider for DirectProvider {
    fn fetch(
        &mut self,
        source: &Source,
        deadline: Instant,
        cancellation: &Cancellation,
    ) -> Result<EncodedFile, ProviderError> {
        if cancellation.is_cancelled() {
            return Err(ProviderError::Cancelled);
        }
        if Instant::now() >= deadline {
            return Err(ProviderError::Expired);
        }
        // This ordinary entry still refuses Root. Never retry via a root reader.
        let result = app_artwork::read_encoded(source).map_err(|error| match error.reason {
            ReadReason::PrivilegedProcess => ProviderError::Denied,
            ReadReason::Changed => ProviderError::Changed,
            ReadReason::Io if matches!(error.errno, Some(libc::EACCES) | Some(libc::EPERM)) => {
                ProviderError::Denied
            }
            ReadReason::Io => ProviderError::Unavailable,
            _ => ProviderError::Invalid,
        });
        if cancellation.is_cancelled() {
            return Err(ProviderError::Cancelled);
        }
        if Instant::now() >= deadline {
            return Err(ProviderError::Expired);
        }
        result
    }
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Status {
    Decoded,
    Unchanged,
    ProviderFailed(ProviderError),
    InvalidEncoded,
    DecodeFailed,
    Cancelled,
    Expired,
    InitPanicked,
    ProviderPanicked,
    ProviderUnavailable,
    WorkerStopped,
}
#[derive(Debug)]
pub struct Completed {
    pub work: Work,
    pub result: FetchResult,
    pub status: Status,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum RejectReason {
    Busy,
    SourceMismatch,
    Expired,
    Closed,
}
#[derive(Debug)]
pub struct Rejected {
    pub work: Work,
    pub reason: RejectReason,
}
#[derive(Debug)]
pub struct Finished {
    /// A submitted Work not yet returned by poll, even if its result was ready.
    pub outstanding: Option<Work>,
    pub joined: bool,
    /// Uncaught thread panic from join; caught provider panics are Completed statuses.
    pub panicked: bool,
}
struct Job {
    work: Work,
    source: Source,
    deadline: Instant,
    cancellation: Cancellation,
}
struct Active {
    work: Work,
    cancellation: Cancellation,
}
pub struct Worker {
    jobs: Option<SyncSender<Job>>,
    results: Option<Receiver<Completed>>,
    stop: Arc<AtomicBool>,
    active: Option<Active>,
    thread: Option<JoinHandle<()>>,
}
fn failed(work: Work, status: Status) -> Completed {
    Completed {
        work,
        result: FetchResult::Failed,
        status,
    }
}
fn interrupted(deadline: Instant, cancellation: &Cancellation) -> Option<Status> {
    if cancellation.is_cancelled() {
        Some(Status::Cancelled)
    } else if Instant::now() >= deadline {
        Some(Status::Expired)
    } else {
        None
    }
}
fn encoded_valid(file: &EncodedFile) -> bool {
    !file.bytes.is_empty()
        && file.bytes.len() <= app_artwork::MAX_ENCODED_BYTES
        && file.bytes.capacity() <= app_artwork::MAX_ENCODED_BYTES
        && file.identity.bytes == file.bytes.len() as u64
        && file.identity.mode & 0o170000 == 0o100000
        && (0..1_000_000_000).contains(&file.identity.modified_nanos)
        && (0..1_000_000_000).contains(&file.identity.changed_nanos)
        && plx_base::sha256::sha256(&file.bytes) == file.digest
}
fn process<P: Provider>(provider: &mut P, job: &Job) -> (FetchResult, Status) {
    if job.work.source() != job.source.key() {
        return (FetchResult::Failed, Status::InvalidEncoded);
    }
    if let Some(status) = interrupted(job.deadline, &job.cancellation) {
        return (FetchResult::Failed, status);
    }
    let fetched = provider.fetch(&job.source, job.deadline, &job.cancellation);
    if let Some(status) = interrupted(job.deadline, &job.cancellation) {
        return (FetchResult::Failed, status);
    }
    let file = match fetched {
        Ok(file) => file,
        Err(error) => return (FetchResult::Failed, Status::ProviderFailed(error)),
    };
    // A generic provider is not allowed to assert a trusted hash or file size.
    if !encoded_valid(&file) {
        return (FetchResult::Failed, Status::InvalidEncoded);
    }
    if let Some(status) = interrupted(job.deadline, &job.cancellation) {
        return (FetchResult::Failed, status);
    }
    let fingerprint = Fingerprint {
        content_sha256: file.digest,
        file_identity: file.identity,
    };
    if job
        .work
        .known()
        .is_some_and(|known| known.content_sha256 == fingerprint.content_sha256)
    {
        return (FetchResult::Unchanged(fingerprint), Status::Unchanged);
    }
    let Some((width, height, rgba)) =
        plx_gfx::img::img_decode_owned_bounded(&file.bytes, MAX_AXIS, MAX_IMAGE_BYTES as u64)
    else {
        return (FetchResult::Failed, Status::DecodeFailed);
    };
    if let Some(status) = interrupted(job.deadline, &job.cancellation) {
        return (FetchResult::Failed, status);
    }
    let expected = u64::from(width)
        .checked_mul(u64::from(height))
        .and_then(|n| n.checked_mul(4));
    if width == 0
        || height == 0
        || width > MAX_AXIS
        || height > MAX_AXIS
        || expected != Some(rgba.len() as u64)
        || rgba.capacity() > MAX_IMAGE_BYTES
    {
        return (FetchResult::Failed, Status::DecodeFailed);
    }
    (
        FetchResult::Decoded {
            fingerprint,
            image: DecodedImage {
                width,
                height,
                rgba,
            },
        },
        Status::Decoded,
    )
}
impl Worker {
    pub fn start<F, P>(make_provider: F) -> io::Result<Self>
    where
        F: FnOnce() -> P + Send + 'static,
        P: Provider + 'static,
    {
        let (job_tx, job_rx) = mpsc::sync_channel::<Job>(1);
        let (result_tx, result_rx) = mpsc::sync_channel::<Completed>(1);
        let stop = Arc::new(AtomicBool::new(false));
        let worker_stop = stop.clone();
        let thread = thread::Builder::new()
            .name("home-artwork".into())
            .spawn(move || {
                let mut provider = catch_unwind(AssertUnwindSafe(make_provider)).ok();
                let mut unavailable = Status::InitPanicked;
                while let Ok(job) = job_rx.recv() {
                    let (mut result, mut status) =
                        if let Some(reason) = interrupted(job.deadline, &job.cancellation) {
                            (FetchResult::Failed, reason)
                        } else if let Some(p) = provider.as_mut() {
                            match catch_unwind(AssertUnwindSafe(|| process(p, &job))) {
                                Ok(value) => value,
                                Err(_) => {
                                    // A provider may have mutated itself before panicking.
                                    // Retire it on its own thread; never reuse poisoned state.
                                    let damaged = provider.take();
                                    let _ = catch_unwind(AssertUnwindSafe(|| drop(damaged)));
                                    unavailable = Status::ProviderUnavailable;
                                    (FetchResult::Failed, Status::ProviderPanicked)
                                }
                            }
                        } else {
                            (FetchResult::Failed, unavailable)
                        };
                    if let Some(reason) = interrupted(job.deadline, &job.cancellation) {
                        result = FetchResult::Failed;
                        status = reason;
                    }
                    // No blocking result send: finish drops the receiver before join.
                    if result_tx
                        .try_send(Completed {
                            work: job.work,
                            result,
                            status,
                        })
                        .is_err()
                    {
                        break;
                    }
                    if worker_stop.load(Ordering::Acquire) {
                        break;
                    }
                }
                // Provider (including a non-Send LS2 registration) drops here.
            })?;
        Ok(Self {
            jobs: Some(job_tx),
            results: Some(result_rx),
            stop,
            active: None,
            thread: Some(thread),
        })
    }
    pub fn try_submit(
        &mut self,
        work: Work,
        source: Source,
        deadline: Instant,
    ) -> Result<(), Box<Rejected>> {
        let reason = if self.stop.load(Ordering::Acquire) || self.jobs.is_none() {
            Some(RejectReason::Closed)
        } else if self.active.is_some() {
            Some(RejectReason::Busy)
        } else if work.source() != source.key() {
            Some(RejectReason::SourceMismatch)
        } else if Instant::now() >= deadline {
            Some(RejectReason::Expired)
        } else {
            None
        };
        if let Some(reason) = reason {
            return Err(Box::new(Rejected { work, reason }));
        }
        let cancellation = Cancellation {
            stop: self.stop.clone(),
            job: Arc::new(AtomicBool::new(false)),
        };
        let active = Active {
            work: work.clone(),
            cancellation: cancellation.clone(),
        };
        let job = Job {
            work,
            source,
            deadline,
            cancellation,
        };
        match self.jobs.as_ref().expect("checked sender").try_send(job) {
            Ok(()) => {
                self.active = Some(active);
                Ok(())
            }
            Err(error) => {
                let (job, reason) = match error {
                    TrySendError::Full(job) => (job, RejectReason::Busy),
                    TrySendError::Disconnected(job) => (job, RejectReason::Closed),
                };
                Err(Box::new(Rejected {
                    work: job.work,
                    reason,
                }))
            }
        }
    }
    /// Cancel only the current job, retaining its slot until poll returns it.
    pub fn cancel(&mut self) -> bool {
        if let Some(active) = &self.active {
            active.cancellation.job.store(true, Ordering::Release);
            true
        } else {
            false
        }
    }
    pub fn poll(&mut self) -> Option<Completed> {
        let active = self.active.as_ref()?;
        match self.results.as_ref()?.try_recv() {
            Ok(mut completed) => {
                if completed.work != active.work {
                    self.stop.store(true, Ordering::Release);
                    drop(self.jobs.take());
                    return self
                        .active
                        .take()
                        .map(|a| failed(a.work, Status::WorkerStopped));
                }
                if active.cancellation.is_cancelled() {
                    completed.result = FetchResult::Failed;
                    completed.status = Status::Cancelled;
                }
                self.active = None;
                Some(completed)
            }
            Err(TryRecvError::Empty) => None,
            Err(TryRecvError::Disconnected) => self
                .active
                .take()
                .map(|a| failed(a.work, Status::WorkerStopped)),
        }
    }
    fn shutdown(&mut self) -> Finished {
        self.stop.store(true, Ordering::Release);
        let outstanding = self.active.take().map(|a| {
            a.cancellation.job.store(true, Ordering::Release);
            a.work
        });
        drop(self.jobs.take());
        drop(self.results.take());
        let panicked = self
            .thread
            .take()
            .is_some_and(|handle| handle.join().is_err());
        Finished {
            outstanding,
            joined: true,
            panicked,
        }
    }
    /// This waits for actual termination. Blocking I/O/initialization is not
    /// forcibly interruptible; callers must not infer a hard join deadline.
    pub fn finish(mut self) -> Finished {
        self.shutdown()
    }
}
impl Drop for Worker {
    fn drop(&mut self) {
        let _ = self.shutdown();
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::app_artwork::{sources_from_raw, FileIdentity, SourceMap};
    use crate::artwork_cache::{ArtworkCache, Config, SubmitOutcome};
    use serde_json::json;
    use std::{
        cell::Cell,
        rc::Rc,
        sync::{atomic::AtomicUsize, mpsc},
        time::Duration,
    };
    fn catalogue(id: &str, version: &str) -> SourceMap {
        sources_from_raw(&json!({"replies":{"apps":{"returnValue":true,"apps":[{"id":id,"title":"Fixture","type":"native","visible":true,"version":version,"folderPath":format!("/usr/palm/applications/{id}"),"icon":"icon.png"}]},"launch_points":{"returnValue":true,"launchPoints":[{"id":id}]}}}),crate::identity::PROOF).unwrap()
    }
    fn fixture(id: &str) -> (ArtworkCache, Work, Source) {
        let sources = catalogue(id, "1");
        let source = sources[id].preferred().unwrap().clone();
        let mut cache = ArtworkCache::new(Config::default()).unwrap();
        cache.update_catalogue(&sources).unwrap();
        let work = cache.next_work(0).unwrap();
        (cache, work, source)
    }
    fn identity(bytes: usize) -> FileIdentity {
        FileIdentity {
            device: 1,
            inode: 1,
            bytes: bytes as u64,
            mode: 0o100644,
            uid: 501,
            gid: 20,
            modified_seconds: 1,
            modified_nanos: 0,
            changed_seconds: 1,
            changed_nanos: 0,
        }
    }
    fn encoded(bytes: Vec<u8>) -> EncodedFile {
        EncodedFile {
            identity: identity(bytes.len()),
            digest: plx_base::sha256::sha256(&bytes),
            bytes,
            png_dimensions: None,
        }
    }
    fn crc(bytes: &[u8]) -> u32 {
        let mut crc = 0xffff_ffff_u32;
        for b in bytes {
            crc ^= u32::from(*b);
            for _ in 0..8 {
                crc = (crc >> 1) ^ if crc & 1 != 0 { 0xedb8_8320 } else { 0 };
            }
        }
        !crc
    }
    fn chunk(out: &mut Vec<u8>, kind: &[u8; 4], body: &[u8]) {
        out.extend_from_slice(&(body.len() as u32).to_be_bytes());
        out.extend_from_slice(kind);
        out.extend_from_slice(body);
        let mut all = kind.to_vec();
        all.extend_from_slice(body);
        out.extend_from_slice(&crc(&all).to_be_bytes());
    }
    fn png(width: u32, height: u32) -> Vec<u8> {
        // Synthetic valid PNG with stored DEFLATE, exact CRC/Adler and RGBA bytes.
        let mut raw = Vec::new();
        for _ in 0..height {
            raw.push(0);
            raw.extend([10, 20, 30, 255].repeat(width as usize));
        }
        let mut compressed = vec![0x78, 0x01];
        let count = raw.len().div_ceil(65535);
        for (i, block) in raw.chunks(65535).enumerate() {
            compressed.push(u8::from(i + 1 == count));
            let n = block.len() as u16;
            compressed.extend_from_slice(&n.to_le_bytes());
            compressed.extend_from_slice(&(!n).to_le_bytes());
            compressed.extend_from_slice(block);
        }
        let (mut a, mut b) = (1_u32, 0_u32);
        for byte in raw {
            a = (a + u32::from(byte)) % 65521;
            b = (b + a) % 65521;
        }
        compressed.extend_from_slice(&((b << 16) | a).to_be_bytes());
        let mut out = b"\x89PNG\r\n\x1a\n".to_vec();
        let mut ihdr = width.to_be_bytes().to_vec();
        ihdr.extend_from_slice(&height.to_be_bytes());
        ihdr.extend_from_slice(&[8, 6, 0, 0, 0]);
        chunk(&mut out, b"IHDR", &ihdr);
        chunk(&mut out, b"IDAT", &compressed);
        chunk(&mut out, b"IEND", &[]);
        out
    }
    struct Bytes {
        bytes: Vec<u8>,
        calls: Arc<AtomicUsize>,
    }
    impl Provider for Bytes {
        fn fetch(
            &mut self,
            _: &Source,
            _: Instant,
            _: &Cancellation,
        ) -> Result<EncodedFile, ProviderError> {
            self.calls.fetch_add(1, Ordering::SeqCst);
            Ok(encoded(self.bytes.clone()))
        }
    }
    fn worker(bytes: Vec<u8>) -> (Worker, Arc<AtomicUsize>) {
        let calls = Arc::new(AtomicUsize::new(0));
        let observed = calls.clone();
        (
            Worker::start(move || Bytes {
                bytes,
                calls: observed,
            })
            .unwrap(),
            calls,
        )
    }
    fn deadline() -> Instant {
        Instant::now() + Duration::from_secs(2)
    }
    fn wait(worker: &mut Worker) -> Completed {
        let until = deadline();
        loop {
            if let Some(done) = worker.poll() {
                return done;
            }
            assert!(
                Instant::now() < until,
                "worker did not return bounded fixture"
            );
            thread::sleep(Duration::from_millis(1));
        }
    }
    #[test]
    fn actual_synthetic_png_decodes_on_worker_and_preserves_work() {
        let (mut cache, work, source) = fixture("fixture.png");
        let (mut worker, calls) = worker(png(2, 1));
        worker.try_submit(work.clone(), source, deadline()).unwrap();
        let done = wait(&mut worker);
        assert_eq!(done.work, work);
        assert_eq!(done.status, Status::Decoded);
        let FetchResult::Decoded { image, .. } = &done.result else {
            panic!("expected decoded PNG")
        };
        assert_eq!((image.width, image.height), (2, 1));
        assert_eq!(image.rgba, [10, 20, 30, 255].repeat(2));
        assert_eq!(
            cache.submit_result(done.work, done.result),
            SubmitOutcome::Queued
        );
        assert!(cache.apply_next(0).unwrap().changed.is_some());
        assert_eq!(calls.load(Ordering::SeqCst), 1);
        let report = worker.finish();
        assert!(report.joined && !report.panicked && report.outstanding.is_none());
    }
    #[test]
    fn non_send_provider_init_fetch_and_drop_are_same_worker_thread() {
        struct Local {
            local: Rc<Cell<u8>>,
            origin: thread::ThreadId,
            events: mpsc::Sender<thread::ThreadId>,
        }
        impl Provider for Local {
            fn fetch(
                &mut self,
                _: &Source,
                _: Instant,
                _: &Cancellation,
            ) -> Result<EncodedFile, ProviderError> {
                assert_eq!(self.origin, thread::current().id());
                self.local.set(1);
                self.events.send(thread::current().id()).unwrap();
                Ok(encoded(png(1, 1)))
            }
        }
        impl Drop for Local {
            fn drop(&mut self) {
                assert_eq!(self.local.get(), 1);
                let _ = self.events.send(thread::current().id());
            }
        }
        let (tx, rx) = mpsc::channel();
        let main = thread::current().id();
        let mut w = Worker::start(move || {
            let id = thread::current().id();
            tx.send(id).unwrap();
            Local {
                local: Rc::new(Cell::new(0)),
                origin: id,
                events: tx,
            }
        })
        .unwrap();
        let (_, work, source) = fixture("fixture.local");
        w.try_submit(work, source, deadline()).unwrap();
        assert_eq!(wait(&mut w).status, Status::Decoded);
        w.finish();
        let ids = rx.try_iter().collect::<Vec<_>>();
        assert_eq!(ids.len(), 3);
        assert!(ids.iter().all(|id| *id == ids[0] && *id != main));
    }
    #[test]
    fn one_outstanding_until_poll_rejects_backpressure_and_returns_work() {
        let (_, first, source) = fixture("fixture.one");
        let (_, second, other) = fixture("fixture.two");
        let (mut w, _) = worker(png(1, 1));
        w.try_submit(first, source, deadline()).unwrap();
        let rejected = w
            .try_submit(second.clone(), other.clone(), deadline())
            .unwrap_err();
        assert_eq!(rejected.reason, RejectReason::Busy);
        assert_eq!(rejected.work, second);
        let done = wait(&mut w);
        assert_eq!(done.status, Status::Decoded);
        w.try_submit(rejected.work, other, deadline()).unwrap();
        assert_eq!(wait(&mut w).status, Status::Decoded);
        w.finish();
    }
    #[test]
    fn exact_source_key_mismatch_and_expiry_never_call_provider() {
        let (_, work, source) = fixture("fixture.match");
        let wrong = catalogue("fixture.match", "2")["fixture.match"]
            .preferred()
            .unwrap()
            .clone();
        let (mut w, calls) = worker(png(1, 1));
        let rejected = w.try_submit(work.clone(), wrong, deadline()).unwrap_err();
        assert_eq!(rejected.reason, RejectReason::SourceMismatch);
        assert_eq!(rejected.work, work);
        assert_eq!(
            w.try_submit(work, source, Instant::now())
                .unwrap_err()
                .reason,
            RejectReason::Expired
        );
        assert_eq!(calls.load(Ordering::SeqCst), 0);
        assert!(w.finish().outstanding.is_none());
    }
    #[test]
    fn cancelled_queued_job_does_not_poison_next_job() {
        let (ready_tx, ready_rx) = mpsc::channel();
        let (release_tx, release_rx) = mpsc::channel();
        let calls = Arc::new(AtomicUsize::new(0));
        let seen = calls.clone();
        let mut w = Worker::start(move || {
            ready_tx.send(()).unwrap();
            release_rx.recv().unwrap();
            Bytes {
                bytes: png(1, 1),
                calls: seen,
            }
        })
        .unwrap();
        ready_rx.recv_timeout(Duration::from_secs(2)).unwrap();
        let (_, work, source) = fixture("fixture.cancel");
        w.try_submit(work, source, deadline()).unwrap();
        assert!(w.cancel());
        release_tx.send(()).unwrap();
        assert_eq!(wait(&mut w).status, Status::Cancelled);
        assert_eq!(calls.load(Ordering::SeqCst), 0);
        let (_, work, source) = fixture("fixture.next");
        w.try_submit(work, source, deadline()).unwrap();
        assert_eq!(wait(&mut w).status, Status::Decoded);
        assert_eq!(calls.load(Ordering::SeqCst), 1);
        w.finish();
    }
    #[test]
    fn finish_while_provider_inflight_cancels_without_blocked_result_send() {
        struct Held {
            entered: mpsc::Sender<()>,
            cancelled: mpsc::Sender<()>,
            release: mpsc::Receiver<()>,
        }
        impl Provider for Held {
            fn fetch(
                &mut self,
                _: &Source,
                deadline: Instant,
                cancellation: &Cancellation,
            ) -> Result<EncodedFile, ProviderError> {
                self.entered.send(()).unwrap();
                while !cancellation.is_cancelled() && Instant::now() < deadline {
                    thread::sleep(Duration::from_millis(1));
                }
                if !cancellation.is_cancelled() {
                    return Err(ProviderError::Expired);
                }
                self.cancelled.send(()).unwrap();
                self.release.recv_timeout(Duration::from_secs(2)).unwrap();
                Ok(encoded(png(1, 1)))
            }
        }
        let (entered_tx, entered_rx) = mpsc::channel();
        let (release_tx, release_rx) = mpsc::channel();
        let (cancelled_tx, cancelled_rx) = mpsc::channel();
        let mut w = Worker::start(move || Held {
            entered: entered_tx,
            cancelled: cancelled_tx,
            release: release_rx,
        })
        .unwrap();
        let (_, work, source) = fixture("fixture.held");
        w.try_submit(work.clone(), source, deadline()).unwrap();
        entered_rx.recv_timeout(Duration::from_secs(2)).unwrap();
        let finish = thread::spawn(move || w.finish());
        cancelled_rx.recv_timeout(Duration::from_secs(2)).unwrap();
        release_tx.send(()).unwrap();
        let result = finish.join().unwrap();
        assert_eq!(result.outstanding, Some(work));
        assert!(result.joined && !result.panicked);
    }
    #[test]
    fn encoded_metadata_and_digest_are_checked_independently() {
        struct Invalid {
            which: u8,
        }
        impl Provider for Invalid {
            fn fetch(
                &mut self,
                _: &Source,
                _: Instant,
                _: &Cancellation,
            ) -> Result<EncodedFile, ProviderError> {
                let mut f = encoded(png(1, 1));
                match self.which {
                    0 => f.digest = [0; 32],
                    1 => f.identity.bytes += 1,
                    2 => f.identity.mode = 0o120644,
                    3 => f.identity.modified_nanos = -1,
                    4 => f.identity.changed_nanos = 1_000_000_000,
                    _ => {
                        f.bytes = vec![1; app_artwork::MAX_ENCODED_BYTES + 1];
                        f.identity.bytes = f.bytes.len() as u64;
                        f.digest = plx_base::sha256::sha256(&f.bytes);
                    }
                }
                Ok(f)
            }
        }
        for which in 0..6 {
            let mut w = Worker::start(move || Invalid { which }).unwrap();
            let (_, work, source) = fixture("fixture.invalid");
            w.try_submit(work, source, deadline()).unwrap();
            assert_eq!(wait(&mut w).status, Status::InvalidEncoded);
            w.finish();
        }
    }
    #[test]
    fn known_digest_skips_decoder_only_after_actual_content_hash_check() {
        let (mut cache, work, source) = fixture("fixture.known");
        let bytes = b"not a PNG: decoder must be skipped".to_vec();
        let digest = plx_base::sha256::sha256(&bytes);
        // Pure cache fixture installs a synthetic known resident, allowing an
        // intentionally undecodable buffer to distinguish skip from decode.
        cache.submit_result(
            work,
            FetchResult::Decoded {
                fingerprint: Fingerprint {
                    content_sha256: digest,
                    file_identity: identity(bytes.len()),
                },
                image: DecodedImage {
                    width: 1,
                    height: 1,
                    rgba: vec![1, 2, 3, 4],
                },
            },
        );
        cache.apply_next(0);
        cache.request_refresh("fixture.known");
        let work = cache.next_work(1).unwrap();
        let (mut w, _) = worker(bytes);
        w.try_submit(work, source, deadline()).unwrap();
        let done = wait(&mut w);
        assert_eq!(done.status, Status::Unchanged);
        assert!(matches!(done.result, FetchResult::Unchanged(_)));
        w.finish();
    }
    #[test]
    fn invalid_or_oversize_image_decode_fails_soft() {
        for bytes in [b"not image bytes".to_vec(), png(513, 1)] {
            let (mut w, _) = worker(bytes);
            let (_, work, source) = fixture("fixture.decode");
            w.try_submit(work, source, deadline()).unwrap();
            let done = wait(&mut w);
            assert_eq!(done.status, Status::DecodeFailed);
            assert!(matches!(done.result, FetchResult::Failed));
            w.finish();
        }
    }
    #[test]
    fn provider_stale_source_error_returns_owned_failed_work() {
        struct Changed;
        impl Provider for Changed {
            fn fetch(
                &mut self,
                _: &Source,
                _: Instant,
                _: &Cancellation,
            ) -> Result<EncodedFile, ProviderError> {
                Err(ProviderError::Changed)
            }
        }
        let mut w = Worker::start(|| Changed).unwrap();
        let (_, work, source) = fixture("fixture.changed");
        w.try_submit(work.clone(), source, deadline()).unwrap();
        let done = wait(&mut w);
        assert_eq!(done.work, work);
        assert_eq!(done.status, Status::ProviderFailed(ProviderError::Changed));
        assert!(matches!(done.result, FetchResult::Failed));
        w.finish();
    }
    #[cfg(panic = "unwind")]
    #[test]
    fn provider_and_initializer_panics_do_not_strand_cache_work() {
        struct Panics;
        impl Provider for Panics {
            fn fetch(
                &mut self,
                _: &Source,
                _: Instant,
                _: &Cancellation,
            ) -> Result<EncodedFile, ProviderError> {
                panic!("synthetic provider panic")
            }
        }
        let mut w = Worker::start(|| Panics).unwrap();
        let (_, work, source) = fixture("fixture.panic");
        w.try_submit(work.clone(), source, deadline()).unwrap();
        let done = wait(&mut w);
        assert_eq!(done.work, work);
        assert_eq!(done.status, Status::ProviderPanicked);
        assert!(matches!(done.result, FetchResult::Failed));
        assert!(!w.finish().panicked);
        let mut w = Worker::start(|| -> Panics { panic!("synthetic initializer panic") }).unwrap();
        let (_, work, source) = fixture("fixture.initpanic");
        w.try_submit(work, source, deadline()).unwrap();
        assert_eq!(wait(&mut w).status, Status::InitPanicked);
        assert!(w.finish().joined);
    }
    #[test]
    fn direct_provider_preflight_cancel_and_deadline_do_not_read() {
        let (_, _, source) = fixture("fixture.direct");
        let cancellation = Cancellation {
            stop: Arc::new(AtomicBool::new(false)),
            job: Arc::new(AtomicBool::new(true)),
        };
        let mut direct = DirectProvider;
        assert_eq!(
            direct
                .fetch(&source, deadline(), &cancellation)
                .unwrap_err(),
            ProviderError::Cancelled
        );
        cancellation.job.store(false, Ordering::Release);
        assert_eq!(
            direct
                .fetch(&source, Instant::now(), &cancellation)
                .unwrap_err(),
            ProviderError::Expired
        );
        if unsafe { libc::geteuid() } == 0 {
            assert_eq!(
                direct
                    .fetch(&source, deadline(), &cancellation)
                    .unwrap_err(),
                ProviderError::Denied
            );
        }
    }
}
