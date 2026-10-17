//! Actual-cache/decoder and channel/barrier fixtures. No Root/provider,
//! runtime identity override, host /media/developer, SDL or real GL calls.
use super::*;
use crate::app_artwork::{EncodedFile, FileIdentity as ObservedSource, IconKind};
use std::{
    cell::RefCell,
    fs::{self, File},
    os::unix::fs::{MetadataExt, PermissionsExt},
    sync::{atomic::AtomicU64, Mutex},
    time::{Duration, Instant},
};
const WAIT: Duration = Duration::from_secs(2);
const ID: &str = "com.example.cache";
static SERIAL: AtomicU64 = AtomicU64::new(0);
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Stage {
    Adopt,
    Cached,
    Packaged,
}
#[derive(Default, Debug, Clone, PartialEq, Eq)]
struct Counts {
    opened: usize,
    adopted: usize,
    cached: usize,
    packaged: usize,
}
type Substitution = Arc<Mutex<Option<(Arc<Snapshot>, Source, ProducerVersion)>>>;
struct IoControl {
    hold: Arc<Mutex<Option<Stage>>>,
    stages: Receiver<Stage>,
    release: SyncSender<()>,
    counts: Arc<Mutex<Counts>>,
    events: Arc<Mutex<Vec<&'static str>>>,
    substitute: Substitution,
}
struct RealLoader {
    cache: disk::Cache,
    packaged: PackagedIconReader,
    hold: Arc<Mutex<Option<Stage>>>,
    stages: SyncSender<Stage>,
    release: Receiver<()>,
    counts: Arc<Mutex<Counts>>,
    owner: thread::ThreadId,
    events: Arc<Mutex<Vec<&'static str>>>,
    substitute: Substitution,
}
impl RealLoader {
    fn at(&self, stage: Stage) {
        assert_eq!(
            self.owner,
            thread::current().id(),
            "All open/scan/decode on one worker"
        );
        self.stages.try_send(stage).unwrap();
        let held = *self.hold.lock().unwrap() == Some(stage);
        if held {
            self.release
                .recv_timeout(WAIT)
                .expect("Release held real-file operation");
        }
    }
}
impl Loader for RealLoader {
    fn adopt(&mut self) -> io::Result<Adoption> {
        self.counts.lock().unwrap().adopted += 1;
        self.at(Stage::Adopt);
        self.cache.snapshot().map(|snapshot| Adoption {
            association: None,
            producer: Some(producer()),
            snapshot: Some(Arc::new(snapshot)),
        })
    }
    fn decode(&mut self, path: &RelativeIconPath) -> io::Result<DecodedIcon> {
        self.counts.lock().unwrap().packaged += 1;
        let result = self.packaged.decode(path);
        self.at(Stage::Packaged);
        result
    }
    fn decode_cached(
        &mut self,
        snapshot: &Snapshot,
        demand: disk::Demand,
    ) -> io::Result<CachedIcon> {
        self.counts.lock().unwrap().cached += 1;
        let substitute = self.substitute.lock().unwrap().take();
        let result = if let Some((other, source, producer)) = substitute {
            match other.lookup(&source, &producer) {
                disk::Lookup::Ready(demand) => other.decode(*demand),
                _ => Err(io::Error::other("Substitute fixture missing")),
            }
        } else {
            snapshot.decode(demand)
        };
        // Deliberately hold the decoded opaque result before worker publication,
        // proving late-result guards, not interruptibility of the codec/syscall.
        self.at(Stage::Cached);
        result
    }
}
impl Drop for RealLoader {
    fn drop(&mut self) {
        self.events.lock().unwrap().push("loader_drop");
    }
}
#[derive(Default)]
struct GlState {
    next: u32,
    live: BTreeSet<u32>,
    pixels: Vec<Vec<u8>>,
}
struct Backend {
    state: Rc<RefCell<GlState>>,
    owner: thread::ThreadId,
    events: Arc<Mutex<Vec<&'static str>>>,
}
impl TextureBackend for Backend {
    type Handle = u32;
    type Error = ();
    fn upload(&mut self, _: u32, _: u32, pixels: &[u8]) -> Result<u32, ()> {
        assert_eq!(self.owner, thread::current().id());
        let mut state = self.state.borrow_mut();
        state.next += 1;
        let id = state.next;
        state.live.insert(id);
        state.pixels.push(pixels.to_vec());
        self.events.lock().unwrap().push("upload");
        Ok(id)
    }
    fn delete(&mut self, id: u32) {
        assert_eq!(self.owner, thread::current().id());
        assert!(self.state.borrow_mut().live.remove(&id));
        self.events.lock().unwrap().push("delete");
    }
}
struct Files {
    parent: PathBuf,
    assets: PathBuf,
    writer: disk::Cache,
}
impl Files {
    fn new() -> Self {
        let parent = std::env::temp_dir().join(format!(
            "ordinary-icon-consumer-{}-{}",
            std::process::id(),
            SERIAL.fetch_add(1, Ordering::Relaxed)
        ));
        fs::create_dir(&parent).unwrap();
        fs::set_permissions(&parent, fs::Permissions::from_mode(0o700)).unwrap();
        let assets = parent.join("assets");
        fs::create_dir(&assets).unwrap();
        fs::create_dir(assets.join("icons")).unwrap();
        fs::write(assets.join("icons/fallback.png"), png(9)).unwrap();
        let writer =
            disk::Cache::open_test_directory(File::open(&parent).unwrap(), disk::Config::default())
                .unwrap();
        Self {
            parent,
            assets,
            writer,
        }
    }
    fn publish(&mut self, source: &Source, value: u8) {
        let raw = png(value);
        let path = self.parent.join("observed-source.png");
        fs::write(&path, &raw).unwrap();
        let m = fs::metadata(path).unwrap();
        let encoded = EncodedFile {
            digest: plx_base::sha256::sha256(&raw),
            bytes: raw,
            png_dimensions: Some([2, 1]),
            identity: ObservedSource {
                device: m.dev(),
                inode: m.ino(),
                bytes: m.len(),
                mode: m.mode(),
                uid: m.uid(),
                gid: m.gid(),
                modified_seconds: m.mtime(),
                modified_nanos: m.mtime_nsec(),
                changed_seconds: m.ctime(),
                changed_nanos: m.ctime_nsec(),
            },
        };
        self.writer.publish(source, &encoded, producer()).unwrap();
    }
    fn corrupt_body(&self) {
        for leaf in fs::read_dir(
            self.parent
                .join(".com.ash.nativehome.preview-artwork-cache-v1"),
        )
        .unwrap()
        {
            let path = leaf.unwrap().path();
            if path.extension().is_some_and(|x| x == "icon") {
                let mut raw = fs::read(&path).unwrap();
                *raw.last_mut().unwrap() ^= 1;
                fs::write(path, raw).unwrap();
            }
        }
    }
}
impl Drop for Files {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.parent);
    }
}
fn producer() -> ProducerVersion {
    ProducerVersion::new(&"a".repeat(40), [1; 32], [2; 32]).unwrap()
}
fn sources(version: &str, path: &str, kind: IconKind) -> SourceMap {
    sources_for(ID, version, path, kind)
}
fn sources_for(id: &str, version: &str, path: &str, kind: IconKind) -> SourceMap {
    let mut app = serde_json::json!({"id":id,"title":"Cache fixture","type":"native","visible":true,"version":version,"folderPath":format!("/usr/palm/applications/{id}")});
    app[if kind == IconKind::Icon {
        "icon"
    } else {
        "largeIcon"
    }] = path.into();
    crate::app_artwork::sources_from_raw(&serde_json::json!({"replies":{"apps":{"returnValue":true,"apps":[app]},"launch_points":{"returnValue":true,"launchPoints":[{"id":id}]}}}),crate::identity::PREVIEW).unwrap()
}
fn source(map: &SourceMap) -> &Source {
    map[ID].preferred().unwrap()
}
fn crc(bytes: &[u8]) -> u32 {
    let mut n = !0u32;
    for b in bytes {
        n ^= u32::from(*b);
        for _ in 0..8 {
            n = (n >> 1) ^ if n & 1 != 0 { 0xedb88320 } else { 0 };
        }
    }
    !n
}
fn chunk(out: &mut Vec<u8>, kind: &[u8; 4], bytes: &[u8]) {
    out.extend_from_slice(&(bytes.len() as u32).to_be_bytes());
    let start = out.len();
    out.extend_from_slice(kind);
    out.extend_from_slice(bytes);
    let c = crc(&out[start..]);
    out.extend_from_slice(&c.to_be_bytes());
}
fn png(value: u8) -> Vec<u8> {
    // Real 2x1 RGBA PNG, uncompressed zlib block; alpha/aspect observable.
    let raw = [0, 10, 20, 30, 0, value, 50, 60, 128];
    let mut z = vec![0x78, 0x01, 0x01, 9, 0, 0xf6, 0xff];
    z.extend(raw);
    let (mut a, mut b) = (1u32, 0u32);
    for x in raw {
        a = (a + u32::from(x)) % 65521;
        b = (b + a) % 65521;
    }
    z.extend_from_slice(&((b << 16) | a).to_be_bytes());
    let mut p = b"\x89PNG\r\n\x1a\n".to_vec();
    let mut h = vec![];
    h.extend_from_slice(&2u32.to_be_bytes());
    h.extend_from_slice(&1u32.to_be_bytes());
    h.extend([8, 6, 0, 0, 0]);
    chunk(&mut p, b"IHDR", &h);
    chunk(&mut p, b"IDAT", &z);
    chunk(&mut p, b"IEND", &[]);
    p
}
struct Harness {
    icons: PackagedIcons<Backend>,
    io: IoControl,
    gl: Rc<RefCell<GlState>>,
}
impl Harness {
    fn new(files: &Files, producer: ProducerVersion, packaged: bool) -> Self {
        let hold = Arc::new(Mutex::new(None));
        let counts = Arc::new(Mutex::new(Counts::default()));
        let events = Arc::new(Mutex::new(Vec::new()));
        let (stages_tx, stages) = mpsc::sync_channel(32);
        let (release, release_rx) = mpsc::sync_channel(1);
        let gl = Rc::new(RefCell::new(GlState::default()));
        let backend = Backend {
            state: gl.clone(),
            owner: thread::current().id(),
            events: events.clone(),
        };
        let parent = files.parent.clone();
        let assets = files.assets.clone();
        let worker_hold = hold.clone();
        let worker_counts = counts.clone();
        let worker_events = events.clone();
        let substitute = Arc::new(Mutex::new(None));
        let worker_substitute = substitute.clone();
        let bindings = if packaged {
            [(
                ID.into(),
                RelativeIconPath::parse("icons/fallback.png").unwrap(),
            )]
            .into_iter()
            .collect()
        } else {
            Bindings::new()
        };
        let mut icons = PackagedIcons::start_using(
            bindings,
            backend,
            (MAX_TEXTURES, MAX_TEXTURE_BYTES),
            move || {
                worker_counts.lock().unwrap().opened += 1;
                Ok(RealLoader {
                    cache: disk::Cache::open_test_directory(
                        File::open(parent)?,
                        disk::Config::default(),
                    )?,
                    packaged: PackagedIconReader::from_directory(File::open(assets)?)?,
                    hold: worker_hold,
                    stages: stages_tx,
                    release: release_rx,
                    counts: worker_counts,
                    owner: thread::current().id(),
                    events: worker_events,
                    substitute: worker_substitute,
                })
            },
        )
        .unwrap();
        icons.cache = Some(CacheMode {
            expected: ExpectedPolicy::Supplied(producer),
            producer: None,
            sources: BTreeMap::new(),
            snapshot: None,
            selected: BTreeMap::new(),
            failed: BTreeSet::new(),
            request: 0,
            adopting: None,
            refresh: false,
        });
        Self {
            icons,
            io: IoControl {
                hold,
                stages,
                release,
                counts,
                events,
                substitute,
            },
            gl,
        }
    }
    fn until(&mut self, allowed: bool, predicate: impl Fn(&Self) -> bool) {
        let end = Instant::now() + WAIT;
        loop {
            self.icons.frame(&[ID], allowed);
            if predicate(self) {
                return;
            }
            assert!(Instant::now() < end, "Actual worker outcome missing");
            thread::sleep(Duration::from_millis(1));
        }
    }
    fn stage(&mut self, expected: Stage) {
        self.stage_for(&[ID], expected);
    }
    fn stage_for(&mut self, visible: &[&str], expected: Stage) {
        let end = Instant::now() + WAIT;
        loop {
            self.icons.frame(visible, true);
            while let Ok(stage) = self.io.stages.try_recv() {
                if stage == expected {
                    return;
                }
            }
            assert!(Instant::now() < end);
            thread::sleep(Duration::from_millis(1));
        }
    }
    fn release(&self) {
        *self.io.hold.lock().unwrap() = None;
        self.io.release.send(()).unwrap();
    }
    fn counts(&self) -> Counts {
        self.io.counts.lock().unwrap().clone()
    }
}
#[test]
fn actual_cache_hit_preserves_alpha_aspect_on_existing_worker_without_packaged_read() {
    let mut f = Files::new();
    let map = sources("1", "icon.png", IconKind::Icon);
    f.publish(source(&map), 70);
    let mut h = Harness::new(&f, producer(), false);
    h.icons.update_sources(Some(&map)).unwrap();
    h.until(true, |h| h.icons.texture(ID).is_some());
    let view = h.icons.texture(ID).unwrap();
    assert_eq!((view.width, view.height), (2, 1));
    assert_eq!(h.gl.borrow().pixels[0], [10, 20, 30, 0, 70, 50, 60, 128]);
    assert_eq!(
        h.counts(),
        Counts {
            opened: 1,
            adopted: 1,
            cached: 1,
            packaged: 0
        }
    );
    assert!(h.icons.textures[ID].key.dynamic.is_some());
    let done = h.icons.finish();
    assert!(done.worker_joined && !done.worker_panicked);
    assert_eq!(done.remaining_textures, 0);
    assert!(h.gl.borrow().live.is_empty());
}
#[test]
fn missing_ambiguous_corrupt_or_wrong_producer_falls_back_and_keeps_warming_distinct() {
    for case in 0..4 {
        let mut f = Files::new();
        let map = sources("1", "icon.png", IconKind::Icon);
        if case != 0 {
            f.publish(source(&map), 70);
        }
        if case == 1 {
            f.publish(source(&map), 80);
        }
        if case == 2 {
            f.corrupt_body();
        }
        let expected = if case == 3 {
            ProducerVersion::new(&"b".repeat(40), [1; 32], [2; 32]).unwrap()
        } else {
            producer()
        };
        let mut h = Harness::new(&f, expected, true);
        *h.io.hold.lock().unwrap() = Some(Stage::Packaged);
        h.icons.update_sources(Some(&map)).unwrap();
        h.stage(Stage::Packaged);
        assert!(h.icons.texture(ID).is_none());
        assert!(h.icons.will_load(ID));
        assert!(h.gl.borrow().pixels.is_empty());
        h.release();
        h.until(true, |h| h.icons.texture(ID).is_some());
        assert_eq!(h.gl.borrow().pixels[0], [10, 20, 30, 0, 9, 50, 60, 128]);
        assert!(h.icons.textures[ID].key.dynamic.is_none());
        assert_eq!(h.counts().packaged, 1);
        assert_eq!(h.counts().cached, usize::from(case == 2));
    }
}
#[test]
fn catalogue_withdrawal_and_same_key_aba_discard_held_decoded_result_then_read_fresh_generation() {
    let mut f = Files::new();
    let map = sources("1", "icon.png", IconKind::Icon);
    f.publish(source(&map), 70);
    let mut h = Harness::new(&f, producer(), true);
    *h.io.hold.lock().unwrap() = Some(Stage::Cached);
    h.icons.update_sources(Some(&map)).unwrap();
    h.stage(Stage::Cached);
    let old_epoch = h.icons.epoch;
    h.icons.update_sources(None).unwrap();
    h.icons.frame(&[ID], false);
    h.icons.update_sources(Some(&map)).unwrap();
    assert!(h.icons.epoch > old_epoch);
    assert!(h.gl.borrow().pixels.is_empty());
    assert_eq!(h.counts().adopted, 1);
    h.release();
    h.until(true, |h| h.icons.texture(ID).is_some());
    assert_eq!(h.gl.borrow().pixels.len(), 1);
    assert_eq!(h.counts().adopted, 2);
    assert_eq!(h.counts().cached, 2);
    assert!(h.icons.stats().discarded >= 1);
}
#[test]
fn explicit_same_key_refresh_rejects_late_pixels_and_adopts_new_digest_after_drain() {
    let mut f = Files::new();
    let map = sources("1", "icon.png", IconKind::Icon);
    f.publish(source(&map), 70);
    let old = f.writer.snapshot().unwrap();
    let old_demand = match old.lookup(source(&map), &producer()) {
        disk::Lookup::Ready(d) => *d,
        _ => panic!("published"),
    };
    let mut h = Harness::new(&f, producer(), true);
    *h.io.hold.lock().unwrap() = Some(Stage::Cached);
    h.icons.update_sources(Some(&map)).unwrap();
    h.stage(Stage::Cached);
    // Request withdrawal, release the old decode, and hold the subsequent
    // adoption BEFORE its real scan. The old image job/result is fully drained
    // before exact ordinary entry retirement/publication; no live file is pulled
    // out from underneath an active consumer read.
    h.icons.refresh_cache().unwrap();
    *h.io.hold.lock().unwrap() = Some(Stage::Adopt);
    h.io.release.send(()).unwrap();
    h.stage(Stage::Adopt);
    assert_eq!(h.icons.stats().completed, 1);
    old.generation().withdraw();
    f.writer.retire_after_join(old_demand).unwrap();
    f.publish(source(&map), 90);
    assert!(h.gl.borrow().pixels.is_empty());
    h.release();
    h.until(true, |h| h.icons.texture(ID).is_some());
    assert_eq!(
        h.gl.borrow().pixels.as_slice(),
        [vec![10, 20, 30, 0, 90, 50, 60, 128]]
    );
    assert_eq!(h.counts().adopted, 2);
}
#[test]
fn unchanged_catalogue_poll_does_not_churn_epoch_or_retry_failed_cache_and_package() {
    let mut f = Files::new();
    let map = sources("1", "icon.png", IconKind::Icon);
    f.publish(source(&map), 70);
    f.corrupt_body();
    fs::remove_file(f.assets.join("icons/fallback.png")).unwrap();
    let mut h = Harness::new(&f, producer(), true);
    h.icons.update_sources(Some(&map)).unwrap();
    h.until(true, |h| !h.icons.will_load(ID));
    let counts = h.counts();
    let epoch = h.icons.epoch;
    for _ in 0..20 {
        h.icons.update_sources(Some(&map)).unwrap();
        h.icons.frame(&[ID], true);
    }
    assert_eq!(h.counts(), counts);
    assert_eq!(h.icons.epoch, epoch);
    assert!(h.gl.borrow().pixels.is_empty());
    h.icons.refresh_cache().unwrap();
    h.until(true, |h| !h.icons.will_load(ID));
    assert_eq!(h.counts().cached, counts.cached + 1);
}
#[test]
fn changed_complete_source_or_missing_source_immediately_withdraws_old_texture_even_paused() {
    for changed in [
        sources("2", "icon.png", IconKind::Icon),
        sources("1", "other.png", IconKind::Icon),
        sources("1", "icon.png", IconKind::LargeIcon),
        SourceMap::new(),
    ] {
        let mut f = Files::new();
        let map = sources("1", "icon.png", IconKind::Icon);
        f.publish(source(&map), 70);
        let mut h = Harness::new(&f, producer(), true);
        h.icons.update_sources(Some(&map)).unwrap();
        h.until(true, |h| h.icons.texture(ID).is_some());
        h.icons.update_sources(Some(&changed)).unwrap();
        assert!(h.icons.texture(ID).is_none());
        h.icons.frame(&[ID], false);
        let uploaded = h.gl.borrow().pixels.len();
        h.until(true, |h| h.icons.texture(ID).is_some());
        assert_eq!(
            h.gl.borrow().pixels[uploaded],
            [10, 20, 30, 0, 9, 50, 60, 128]
        );
        assert!(h.icons.textures[ID].key.dynamic.is_none());
    }
}
#[test]
fn pause_drains_without_job_or_upload_and_finish_joins_before_texture_deletion() {
    let mut f = Files::new();
    let mut map = sources("1", "icon.png", IconKind::Icon);
    let second_id = "com.example.cache.second";
    let second = sources_for(second_id, "1", "icon.png", IconKind::Icon);
    f.publish(source(&map), 70);
    f.publish(second[second_id].preferred().unwrap(), 80);
    map.extend(second);
    let mut h = Harness::new(&f, producer(), true);
    h.icons.update_sources(Some(&map)).unwrap();
    for _ in 0..4 {
        h.icons.frame(&[ID], false);
    }
    assert_eq!(h.counts().adopted, 0);
    assert_eq!(h.icons.stats().submitted, 0);
    h.until(true, |h| h.icons.texture(ID).is_some());
    while h.io.stages.try_recv().is_ok() {}
    *h.io.hold.lock().unwrap() = Some(Stage::Cached);
    h.stage_for(&[second_id], Stage::Cached);
    assert_eq!(h.counts().cached, 2);
    assert_eq!(h.gl.borrow().live.len(), 1);
    h.icons.frame(&[second_id], false);
    let submitted = h.icons.stats().submitted;
    let stop = h.icons.worker.as_ref().unwrap().stop.clone();
    let release = h.io.release.clone();
    let release_thread = thread::spawn(move || {
        let end = Instant::now() + WAIT;
        while !stop.load(Ordering::Acquire) {
            assert!(Instant::now() < end);
            thread::yield_now();
        }
        release.send(()).unwrap();
    });
    let done = h.icons.finish();
    release_thread.join().unwrap();
    assert!(done.worker_joined && !done.worker_panicked);
    assert_eq!(done.remaining_textures, 0);
    assert_eq!(h.icons.stats().submitted, submitted);
    assert!(h.gl.borrow().live.is_empty());
    let events = h.io.events.lock().unwrap();
    assert!(
        events.iter().position(|e| *e == "loader_drop").unwrap()
            < events.iter().position(|e| *e == "delete").unwrap()
    );
    assert!(h.icons.cache.as_ref().unwrap().snapshot.is_none());
}

#[test]
fn cache_root_replacement_is_not_hot_reopened_and_invalid_catalogue_cannot_keep_pixels() {
    let mut f = Files::new();
    let map = sources("1", "icon.png", IconKind::Icon);
    f.publish(source(&map), 70);
    let mut h = Harness::new(&f, producer(), true);
    h.icons.update_sources(Some(&map)).unwrap();
    h.until(true, |h| h.icons.texture(ID).is_some());
    let root = f
        .parent
        .join(".com.ash.nativehome.preview-artwork-cache-v1");
    fs::rename(&root, f.parent.join("old-root")).unwrap();
    let _replacement =
        disk::Cache::open_test_directory(File::open(&f.parent).unwrap(), disk::Config::default())
            .unwrap();
    h.icons.refresh_cache().unwrap();
    h.until(true, |h| h.icons.texture(ID).is_some());
    assert_eq!(h.counts().opened, 1);
    assert!(h.icons.textures[ID].key.dynamic.is_none());
    let mut malformed = map.clone();
    let item = malformed.remove(ID).unwrap();
    malformed.insert("wrong.map.id".into(), item);
    assert_eq!(
        h.icons.update_sources(Some(&malformed)),
        Err(ConfigureError::InvalidBindings)
    );
    assert!(h.icons.cache.as_ref().unwrap().sources.is_empty());
    assert!(h.icons.texture(ID).is_none());
}

#[test]
fn same_metadata_from_a_different_adoption_seal_is_not_a_valid_gl_result() {
    let mut f = Files::new();
    let map = sources("1", "icon.png", IconKind::Icon);
    f.publish(source(&map), 70);
    let unrelated_adoption = Arc::new(f.writer.snapshot().unwrap());
    let mut h = Harness::new(&f, producer(), true);
    *h.io.substitute.lock().unwrap() = Some((unrelated_adoption, source(&map).clone(), producer()));
    h.icons.update_sources(Some(&map)).unwrap();
    h.until(true, |h| h.icons.texture(ID).is_some());
    assert_eq!(
        h.gl.borrow().pixels.as_slice(),
        [vec![10, 20, 30, 0, 9, 50, 60, 128]]
    );
    assert_eq!(h.counts().cached, 1);
    assert_eq!(h.counts().packaged, 1);
    assert!(h.icons.stats().discarded >= 1);
    assert!(h.icons.textures[ID].key.dynamic.is_none());
}

// Fixed-profile initializer seam: actual sidecar parser and real temporary cache,
// all executed by the existing worker; actual native self measurement is not
// imitated or claimed by this host-only supplied-hash boundary.
fn current_harness(
    files: &Files,
    packaged: bool,
    fail_cache: bool,
) -> (Harness, Arc<AtomicU64>, Arc<AtomicU64>) {
    profile_harness(files, packaged, fail_cache, false)
}
fn profile_harness(
    files: &Files,
    packaged: bool,
    fail_cache: bool,
    accepted_consumer: bool,
) -> (Harness, Arc<AtomicU64>, Arc<AtomicU64>) {
    let ui_thread = thread::current().id();
    let gl = Rc::new(RefCell::new(GlState::default()));
    let events = Arc::new(Mutex::new(Vec::new()));
    let backend = Backend {
        state: gl.clone(),
        owner: ui_thread,
        events: events.clone(),
    };
    let loaded = Arc::new(AtomicU64::new(0));
    let opened = Arc::new(AtomicU64::new(0));
    let load_count = loaded.clone();
    let open_count = opened.clone();
    let profile_parent = files.parent.clone();
    let cache_parent = files.parent.clone();
    let bindings = if packaged {
        [(
            ID.into(),
            RelativeIconPath::parse("icons/fallback.png").unwrap(),
        )]
        .into_iter()
        .collect()
    } else {
        Bindings::new()
    };
    let icons = PackagedIcons::start_cache_using(
        files.assets.clone(),
        bindings,
        backend,
        disk::Config::default(),
        ExpectedPolicy::WorkerProfile,
        move || {
            assert_ne!(thread::current().id(), ui_thread);
            load_count.fetch_add(1, Ordering::Relaxed);
            if accepted_consumer {
                crate::cache_consumer_profile::load_for_test(
                    &File::open(profile_parent)?,
                    &"03".repeat(32),
                )
            } else {
                crate::cache_producer_profile::ExpectedProducer::load_for_test(
                    &File::open(profile_parent)?,
                    &"01".repeat(32),
                )
                .map(|profile| {
                    profile.map(|p| {
                        crate::cache_consumer_profile::ReadExpectation::supplied(p.version())
                    })
                })
            }
        },
        move |config| {
            assert_ne!(thread::current().id(), ui_thread);
            open_count.fetch_add(1, Ordering::Relaxed);
            if fail_cache {
                Err(io::Error::new(
                    io::ErrorKind::PermissionDenied,
                    "Controlled cache-open refusal",
                ))
            } else {
                disk::Cache::open_test_directory(File::open(cache_parent)?, config)
            }
        },
    )
    .unwrap();
    let (_, stages) = mpsc::sync_channel(1);
    let (release, _) = mpsc::sync_channel(1);
    let io = IoControl {
        hold: Arc::new(Mutex::new(None)),
        stages,
        release,
        counts: Arc::new(Mutex::new(Counts::default())),
        events,
        substitute: Arc::new(Mutex::new(None)),
    };
    (Harness { icons, io, gl }, loaded, opened)
}
fn write_profile(files: &Files) {
    let path = files.parent.join("artwork-cache-producer.json");
    fs::write(&path, serde_json::to_vec(&serde_json::json!({"schema":1,
        "app_id":crate::identity::PREVIEW,"producer_kind":"joined-single-positive-v1",
        "source_revision":"a".repeat(40),"ui_sha256":"01".repeat(32),"broker_sha256":"02".repeat(32)})).unwrap()).unwrap();
    fs::set_permissions(path, fs::Permissions::from_mode(0o644)).unwrap();
}

#[test]
fn accepted_prior_profile_reads_original_header_on_worker_and_reports_both_identities() {
    let mut files = Files::new();
    let map = sources("1", "icon.png", IconKind::Icon);
    files.publish(source(&map), 70);
    let cache = files
        .parent
        .join(".com.ash.nativehome.preview-artwork-cache-v1");
    let held = || {
        fs::read_dir(&cache)
            .unwrap()
            .map(|entry| {
                let path = entry.unwrap().path();
                let m = fs::metadata(&path).unwrap();
                (
                    path.file_name().unwrap().to_owned(),
                    (
                        fs::read(path).unwrap(),
                        (
                            m.dev(),
                            m.ino(),
                            m.uid(),
                            m.gid(),
                            m.mode(),
                            m.nlink(),
                            m.len(),
                            m.mtime(),
                            m.mtime_nsec(),
                            m.ctime(),
                            m.ctime_nsec(),
                        ),
                    ),
                )
            })
            .collect::<BTreeMap<_, _>>()
    };
    let before = held();
    let selector = crate::cache_consumer_profile::record_for_test(
        &"03".repeat(32),
        serde_json::json!({
        "schema":1,"app_id":crate::identity::PREVIEW,"producer_kind":"joined-single-positive-v1",
        "source_revision":"a".repeat(40),"ui_sha256":"01".repeat(32),"broker_sha256":"02".repeat(32)}),
    );
    let path = files.parent.join(crate::cache_consumer_profile::NAME);
    fs::write(&path, serde_json::to_vec(&selector).unwrap()).unwrap();
    fs::set_permissions(&path, fs::Permissions::from_mode(0o644)).unwrap();
    let (mut h, loaded, opened) = profile_harness(&files, false, false, true);
    assert!(h.icons.cache_association_observation().is_none());
    h.icons.update_sources(Some(&map)).unwrap();
    h.until(true, |h| h.icons.texture(ID).is_some());
    assert_eq!(h.gl.borrow().pixels[0], [10, 20, 30, 0, 70, 50, 60, 128]);
    assert_eq!(h.icons.stats().cache_uploads, 1);
    assert_eq!(h.icons.stats().packaged_uploads, 0);
    assert_eq!(loaded.load(Ordering::Relaxed), 1);
    assert_eq!(opened.load(Ordering::Relaxed), 1);
    let receipt = serde_json::to_value(h.icons.cache_association_observation().unwrap()).unwrap();
    assert_eq!(receipt["consumer_ui_sha256"], "03".repeat(32));
    assert_eq!(
        receipt["producer"],
        serde_json::to_value(producer()).unwrap()
    );
    assert_eq!(receipt["selection"], "accepted_producer");
    assert_eq!(receipt["association_only"], true);
    assert_eq!(before, held()); // Same original full header/body and file identities.
    h.icons.update_sources(None).unwrap();
    assert!(h.icons.current_cached_source(ID).is_none());
    // This remains a historical worker-loaded association, never current coverage.
    assert!(h.icons.cache_association_observation().is_some());
    let done = h.icons.finish();
    assert!(done.worker_joined && !done.worker_panicked);
    assert!(h.gl.borrow().live.is_empty());
}

#[test]
fn invalid_consumer_selector_keeps_packaged_fallback_without_cache_open_or_origin() {
    let mut files = Files::new();
    let map = sources("1", "icon.png", IconKind::Icon);
    files.publish(source(&map), 70);
    let selector = crate::cache_consumer_profile::record_for_test(
        &"04".repeat(32),
        serde_json::json!({
        "schema":1,"app_id":crate::identity::PREVIEW,"producer_kind":"joined-single-positive-v1",
        "source_revision":"a".repeat(40),"ui_sha256":"01".repeat(32),"broker_sha256":"02".repeat(32)}),
    );
    let path = files.parent.join(crate::cache_consumer_profile::NAME);
    fs::write(&path, serde_json::to_vec(&selector).unwrap()).unwrap();
    fs::set_permissions(path, fs::Permissions::from_mode(0o644)).unwrap();
    let (mut h, loaded, opened) = profile_harness(&files, true, false, true);
    h.icons.update_sources(Some(&map)).unwrap();
    h.until(true, |h| h.icons.texture(ID).is_some());
    assert_eq!(h.icons.stats().cache_uploads, 0);
    assert_eq!(h.icons.stats().packaged_uploads, 1);
    assert_eq!(loaded.load(Ordering::Relaxed), 1);
    assert_eq!(opened.load(Ordering::Relaxed), 0);
    assert!(h.icons.cache_association_observation().is_none());
    assert!(h.icons.finish().worker_joined);
}
#[test]
fn current_profile_is_loaded_once_on_same_worker_and_empty_package_can_later_use_cache() {
    let mut f = Files::new();
    let map = sources("1", "icon.png", IconKind::Icon);
    f.publish(source(&map), 70);
    write_profile(&f);
    let (mut h, loaded, opened) = current_harness(&f, false, false);
    assert_eq!(
        h.icons.stats().cache_readiness,
        CacheReadiness::AwaitingSources
    );
    h.icons.update_sources(Some(&map)).unwrap();
    assert_eq!(h.icons.stats().cache_readiness, CacheReadiness::Pending);
    h.until(true, |h| h.icons.texture(ID).is_some());
    let stats = h.icons.stats();
    assert_eq!(stats.cache_readiness, CacheReadiness::Ready);
    assert_eq!(
        (
            stats.cache_adoptions,
            stats.cache_uploads,
            stats.packaged_uploads
        ),
        (1, 1, 0)
    );
    assert_eq!(
        (
            loaded.load(Ordering::Relaxed),
            opened.load(Ordering::Relaxed)
        ),
        (1, 1)
    );
    let epoch = h.icons.epoch;
    for _ in 0..4 {
        h.icons.update_sources(Some(&map)).unwrap();
        h.icons.frame(&[ID], true);
    }
    assert_eq!(h.icons.epoch, epoch);
    assert_eq!(loaded.load(Ordering::Relaxed), 1);
    h.icons.refresh_cache().unwrap();
    h.until(true, |h| h.icons.texture(ID).is_some());
    assert_eq!(loaded.load(Ordering::Relaxed), 1);
    assert_eq!(opened.load(Ordering::Relaxed), 1);
    h.icons.finish();
    assert_eq!(h.icons.stats().cache_readiness, CacheReadiness::Disabled);
}
#[test]
fn absent_malformed_profile_or_cache_open_error_keeps_packaged_art_usable() {
    for case in 0..3 {
        let mut f = Files::new();
        let map = sources("1", "icon.png", IconKind::Icon);
        f.publish(source(&map), 70);
        if case == 1 {
            let path = f.parent.join("artwork-cache-producer.json");
            fs::write(&path, b"not JSON").unwrap();
            fs::set_permissions(path, fs::Permissions::from_mode(0o644)).unwrap();
        }
        if case == 2 {
            write_profile(&f);
        }
        let (mut h, loaded, opened) = current_harness(&f, true, case == 2);
        h.icons.update_sources(Some(&map)).unwrap();
        h.until(true, |h| h.icons.texture(ID).is_some());
        let stats = h.icons.stats();
        assert_eq!(stats.cache_readiness, CacheReadiness::Unavailable);
        assert_eq!(
            (
                stats.cache_uploads,
                stats.packaged_uploads,
                stats.cache_adoptions
            ),
            (0, 1, 0)
        );
        assert_eq!(
            h.gl.borrow().pixels.as_slice(),
            [vec![10, 20, 30, 0, 9, 50, 60, 128]]
        );
        assert_eq!(loaded.load(Ordering::Relaxed), 1);
        assert_eq!(opened.load(Ordering::Relaxed), u64::from(case == 2));
        for _ in 0..4 {
            h.icons.update_sources(Some(&map)).unwrap();
            h.icons.frame(&[ID], true);
        }
        assert_eq!(loaded.load(Ordering::Relaxed), 1);
    }
}
#[test]
fn supplied_producer_mismatch_and_stale_worker_profile_ack_never_become_ready() {
    let mut f = Files::new();
    let map = sources("1", "icon.png", IconKind::Icon);
    f.publish(source(&map), 70);
    let other = ProducerVersion::new(&"b".repeat(40), [1; 32], [2; 32]).unwrap();
    let mut h = Harness::new(&f, other, true);
    h.icons.update_sources(Some(&map)).unwrap();
    h.until(true, |h| h.icons.texture(ID).is_some());
    assert_eq!(h.icons.stats().cache_readiness, CacheReadiness::Unavailable);
    assert_eq!(
        (
            h.icons.stats().cache_adoptions,
            h.icons.stats().cache_uploads,
            h.icons.stats().packaged_uploads
        ),
        (0, 0, 1)
    );
    let mut h = Harness::new(&f, producer(), true);
    h.icons.cache.as_mut().unwrap().expected = ExpectedPolicy::WorkerProfile;
    *h.io.hold.lock().unwrap() = Some(Stage::Adopt);
    h.icons.update_sources(Some(&map)).unwrap();
    h.stage(Stage::Adopt);
    h.icons.update_sources(None).unwrap();
    h.release();
    h.until(false, |h| {
        h.icons.cache.as_ref().unwrap().adopting.is_none()
    });
    assert_eq!(
        h.icons.stats().cache_readiness,
        CacheReadiness::AwaitingSources
    );
    assert_eq!(h.icons.stats().cache_adoptions, 0);
    assert_eq!(h.icons.stats().cache_uploads, 0);
    assert!(h.icons.cache.as_ref().unwrap().producer.is_none());
    assert!(h.gl.borrow().pixels.is_empty());
}
#[test]
fn capacity_deferred_cached_pixels_are_counted_only_when_actual_owned_upload_occurs() {
    let mut f = Files::new();
    let mut map = sources("1", "icon.png", IconKind::Icon);
    let second_id = "com.example.cache.second";
    let second = sources_for(second_id, "1", "icon.png", IconKind::Icon);
    f.publish(source(&map), 70);
    f.publish(second[second_id].preferred().unwrap(), 80);
    map.extend(second);
    let mut h = Harness::new(&f, producer(), true);
    h.icons.limits = (1, MAX_TEXTURE_BYTES);
    h.icons.update_sources(Some(&map)).unwrap();
    h.until(true, |h| h.icons.texture(ID).is_some());
    assert_eq!(h.icons.stats().cache_uploads, 1);
    let deadline = Instant::now() + WAIT;
    loop {
        let update = h.icons.frame(&[ID, second_id], true);
        if update.capacity_deferred {
            break;
        }
        assert!(Instant::now() < deadline);
        thread::sleep(Duration::from_millis(1));
    }
    assert_eq!(h.icons.stats().cache_uploads, 1);
    assert!(h.icons.texture(second_id).is_none());
    h.icons.frame(&[second_id], true);
    assert!(h.icons.texture(second_id).is_some());
    assert_eq!(h.icons.stats().cache_uploads, 2);
    assert_eq!(h.icons.stats().packaged_uploads, 0);
}

#[test]
fn cached_origin_exists_only_after_actual_owned_upload_not_held_decode() {
    let mut files = Files::new();
    let map = sources("1", "icon.png", IconKind::Icon);
    files.publish(source(&map), 70);
    let mut h = Harness::new(&files, producer(), false);
    *h.io.hold.lock().unwrap() = Some(Stage::Cached);
    h.icons.update_sources(Some(&map)).unwrap();
    h.stage(Stage::Cached);
    assert!(h.icons.current_cached_source(ID).is_none());
    assert!(h.icons.texture(ID).is_none());
    h.release();
    h.until(true, |h| h.icons.texture(ID).is_some());
    assert_eq!(h.icons.current_cached_source(ID), Some(source(&map).key()));
    assert_eq!(h.icons.stats().cache_uploads, 1);
    h.icons.finish();
    assert!(h.icons.current_cached_source(ID).is_none());
}
#[test]
fn real_packaged_fallback_never_reports_cached_origin() {
    let files = Files::new();
    let map = sources("1", "icon.png", IconKind::Icon);
    let mut h = Harness::new(&files, producer(), true);
    h.icons.update_sources(Some(&map)).unwrap();
    h.until(true, |h| h.icons.texture(ID).is_some());
    assert!(h.icons.current_cached_source(ID).is_none());
    assert_eq!(h.icons.stats().packaged_uploads, 1);
    assert_eq!(h.icons.stats().cache_uploads, 0);
}
#[test]
fn deferred_evicted_and_withdrawn_cache_entries_have_no_origin_receipt() {
    let mut files = Files::new();
    let mut map = sources("1", "icon.png", IconKind::Icon);
    let second_id = "com.example.cache.second";
    let second = sources_for(second_id, "1", "icon.png", IconKind::Icon);
    files.publish(source(&map), 70);
    files.publish(second[second_id].preferred().unwrap(), 80);
    map.extend(second);
    let mut h = Harness::new(&files, producer(), false);
    h.icons.limits = (1, MAX_TEXTURE_BYTES);
    h.icons.update_sources(Some(&map)).unwrap();
    h.until(true, |h| h.icons.texture(ID).is_some());
    let until = Instant::now() + WAIT;
    loop {
        if h.icons.frame(&[ID, second_id], true).capacity_deferred {
            break;
        }
        assert!(Instant::now() < until);
        thread::yield_now();
    }
    assert_eq!(h.icons.current_cached_source(ID), Some(source(&map).key()));
    assert!(h.icons.current_cached_source(second_id).is_none());
    h.icons.frame(&[second_id], true);
    assert!(h.icons.current_cached_source(ID).is_none());
    assert_eq!(
        h.icons.current_cached_source(second_id),
        Some(map[second_id].preferred().unwrap().key())
    );
    h.icons.update_sources(None).unwrap();
    assert!(h.icons.current_cached_source(second_id).is_none()); // Before any GL cleanup/poll.
}
