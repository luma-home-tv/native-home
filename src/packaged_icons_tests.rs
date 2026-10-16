use super::*;
use crate::packaged_icon_reader::FileIdentity;
use std::{
    cell::RefCell,
    sync::Mutex,
    time::{Duration, Instant},
};
const WAIT: Duration = Duration::from_secs(2);
#[derive(Default)]
struct BackendState {
    next: u32,
    live: BTreeSet<u32>,
    deleted: Vec<u32>,
    pixels: Vec<Vec<u8>>,
    force_next: Option<u32>,
    fail_next: bool,
}
struct Backend {
    state: Rc<RefCell<BackendState>>,
    owner: thread::ThreadId,
    events: Arc<Mutex<Vec<&'static str>>>,
}
impl TextureBackend for Backend {
    type Handle = u32;
    type Error = ();
    fn upload(&mut self, _: u32, _: u32, rgba: &[u8]) -> Result<u32, ()> {
        assert_eq!(thread::current().id(), self.owner);
        let mut s = self.state.borrow_mut();
        if std::mem::take(&mut s.fail_next) {
            return Err(());
        }
        let handle = if let Some(handle) = s.force_next.take() {
            handle
        } else {
            s.next += 1;
            s.next
        };
        s.live.insert(handle);
        s.pixels.push(rgba.to_vec());
        self.events.lock().unwrap().push("upload");
        Ok(handle)
    }
    fn delete(&mut self, handle: u32) {
        assert_eq!(thread::current().id(), self.owner);
        let mut s = self.state.borrow_mut();
        assert!(
            s.live.remove(&handle),
            "Delete exactly an owned live allocation"
        );
        s.deleted.push(handle);
        self.events.lock().unwrap().push("delete");
    }
}
enum Answer {
    Image(DecodedIcon),
    Fail,
    Panic,
}
struct Controlled {
    calls: SyncSender<RelativeIconPath>,
    answers: Receiver<Answer>,
    owner: thread::ThreadId,
    events: Arc<Mutex<Vec<&'static str>>>,
}
impl Loader for Controlled {
    fn decode(&mut self, path: &RelativeIconPath) -> io::Result<DecodedIcon> {
        assert_eq!(thread::current().id(), self.owner);
        self.calls
            .try_send(path.clone())
            .map_err(|_| io::Error::other("Unconsumed test call"))?;
        match self.answers.recv_timeout(WAIT) {
            Ok(Answer::Image(image)) => Ok(image),
            Ok(Answer::Fail) => Err(io::Error::other("Controlled unreadable source")),
            Ok(Answer::Panic) => panic!("Controlled worker panic"),
            Err(_) => Err(io::Error::new(
                io::ErrorKind::TimedOut,
                "Test release absent",
            )),
        }
    }
}
impl Drop for Controlled {
    fn drop(&mut self) {
        self.events.lock().unwrap().push("loader_drop");
    }
}
fn bindings(ids: &[String]) -> Bindings {
    ids.iter()
        .map(|id| {
            (
                id.clone(),
                RelativeIconPath::parse(&format!("icons/{id}.png")).unwrap(),
            )
        })
        .collect()
}
fn image(value: u8) -> DecodedIcon {
    DecodedIcon {
        width: 1,
        height: 1,
        rgba: vec![value, 2, 3, 127],
        encoded_identity: FileIdentity {
            device: 1,
            inode: 2,
            uid: 501,
            gid: 20,
            mode: 0o100644,
            links: 1,
            bytes: 20,
            modified_seconds: 0,
            modified_nanos: 0,
            changed_seconds: 0,
            changed_nanos: 0,
        },
    }
}
struct Harness {
    icons: PackagedIcons<Backend>,
    calls: Receiver<RelativeIconPath>,
    answers: SyncSender<Answer>,
    state: Rc<RefCell<BackendState>>,
    events: Arc<Mutex<Vec<&'static str>>>,
}
impl Harness {
    fn new(ids: &[&str], limits: (usize, usize)) -> Self {
        let ids = ids.iter().map(|s| (*s).into()).collect::<Vec<_>>();
        Self::with_bindings(bindings(&ids), limits)
    }
    fn with_bindings(bindings: Bindings, limits: (usize, usize)) -> Self {
        let state = Rc::new(RefCell::new(BackendState::default()));
        let events = Arc::new(Mutex::new(Vec::new()));
        let backend = Backend {
            state: state.clone(),
            owner: thread::current().id(),
            events: events.clone(),
        };
        let (call_tx, calls) = mpsc::sync_channel(1);
        let (answers, answer_rx) = mpsc::sync_channel(1);
        let child_events = events.clone();
        let icons = PackagedIcons::start_using(bindings, backend, limits, move || {
            Ok(Controlled {
                calls: call_tx,
                answers: answer_rx,
                owner: thread::current().id(),
                events: child_events,
            })
        })
        .unwrap();
        Self {
            icons,
            calls,
            answers,
            state,
            events,
        }
    }
    fn until(
        &mut self,
        visible: &[&str],
        allowed: bool,
        predicate: impl Fn(&PackagedIcons<Backend>, &FrameUpdate) -> bool,
    ) -> FrameUpdate {
        let deadline = Instant::now() + WAIT;
        loop {
            let update = self.icons.frame(visible, allowed);
            if predicate(&self.icons, &update) {
                return update;
            }
            assert!(Instant::now() < deadline, "Controlled result not observed");
            thread::yield_now();
        }
    }
    fn call(&self) -> RelativeIconPath {
        self.calls.recv_timeout(WAIT).unwrap()
    }
    fn load(&mut self, id: &str, value: u8) -> FrameUpdate {
        let before = self.icons.stats().completed;
        self.icons.frame(&[id], true);
        self.call();
        self.answers.send(Answer::Image(image(value))).unwrap();
        self.until(&[id], true, |icons, _| icons.stats().completed > before)
    }
}
#[test]
fn all_one_hundred_sources_can_load_as_view_scrolls_beyond_old_sixty_four_cap() {
    let ids = (0..100).map(|n| format!("app{n:03}")).collect::<Vec<_>>();
    let mut h = Harness::with_bindings(bindings(&ids), (MAX_TEXTURES, MAX_TEXTURE_BYTES));
    for id in &ids {
        h.load(id, 1);
        assert!(h.icons.texture(id).is_some());
        assert!(h.icons.stats().texture_entries <= 64);
    }
    assert_eq!(h.icons.stats().submitted, 100);
    assert_eq!(h.icons.stats().completed, 100);
    assert_eq!(h.icons.stats().texture_entries, 64);
    assert_eq!(h.state.borrow().deleted.len(), 36);
    assert!(h.icons.texture("app099").is_some());
    let done = h.icons.finish();
    assert!(done.worker_joined && !done.worker_panicked);
    assert!(h.state.borrow().live.is_empty());
}
#[test]
fn epoch_and_path_drift_discard_real_held_old_result_including_aba() {
    for final_old_path in [false, true] {
        let mut h = Harness::new(&["a"], (64, MAX_TEXTURE_BYTES));
        h.icons.frame(&["a"], true);
        let old = h.call();
        let mut next = h.icons.bindings.clone();
        next.insert(
            "a".into(),
            RelativeIconPath::parse("icons/new.png").unwrap(),
        );
        h.icons.reconfigure(next).unwrap();
        if final_old_path {
            let mut next = h.icons.bindings.clone();
            next.insert("a".into(), old.clone());
            h.icons.reconfigure(next).unwrap();
        }
        h.icons.frame(&["a"], true);
        assert_eq!(h.calls.try_recv(), Err(TryRecvError::Empty));
        h.answers.send(Answer::Image(image(1))).unwrap();
        h.until(&["a"], true, |icons, _| icons.stats().completed == 1);
        let expected = h.icons.bindings["a"].clone();
        assert_eq!(h.call(), expected);
        assert!(h.state.borrow().pixels.is_empty());
        h.answers.send(Answer::Image(image(9))).unwrap();
        h.until(&["a"], true, |icons, _| icons.texture("a").is_some());
        assert_eq!(h.state.borrow().pixels[0][0], 9);
        assert_eq!(h.icons.stats().discarded, 1);
    }
}
#[test]
fn hidden_or_keyboard_gate_is_nonblocking_and_revokes_late_pixels_without_root_readiness() {
    let mut h = Harness::new(&["a"], (64, MAX_TEXTURE_BYTES));
    h.icons.frame(&["a"], true);
    h.call();
    // This returns while the real worker is held. It is no keyboard/action wait.
    let update = h.icons.frame(&["a"], false);
    assert!(!update.visible_pixels_changed);
    assert_eq!(h.icons.stats().submitted, 1);
    h.answers.send(Answer::Image(image(1))).unwrap();
    h.until(&["a"], false, |icons, _| icons.stats().completed == 1);
    for _ in 0..10 {
        assert!(!h.icons.frame(&["a"], false).visible_pixels_changed);
    }
    assert!(h.state.borrow().pixels.is_empty());
    assert_eq!(h.icons.stats().unavailable_sources, 0);
    h.icons.frame(&["a"], true);
    h.call();
    h.answers.send(Answer::Image(image(2))).unwrap();
    h.until(&["a"], true, |icons, _| icons.texture("a").is_some());
    assert_eq!(h.state.borrow().pixels[0][0], 2);
}
#[test]
fn nonvisible_result_does_not_upload_or_invalidate_a_new_view() {
    let mut h = Harness::new(&["a"], (64, MAX_TEXTURE_BYTES));
    h.icons.frame(&["a"], true);
    h.call();
    h.icons.frame(&[], true);
    h.answers.send(Answer::Image(image(1))).unwrap();
    let update = h.until(&[], true, |icons, _| icons.stats().completed == 1);
    assert!(!update.visible_pixels_changed);
    assert!(h.state.borrow().pixels.is_empty());
    assert_eq!(h.icons.stats().submitted, 1);
}
#[test]
fn failed_source_is_negative_cached_until_explicit_configuration_epoch() {
    let mut h = Harness::new(&["a"], (64, MAX_TEXTURE_BYTES));
    h.icons.frame(&["a"], true);
    h.call();
    h.answers.send(Answer::Fail).unwrap();
    h.until(&["a"], true, |icons, _| icons.stats().completed == 1);
    for _ in 0..20 {
        h.icons.frame(&["a"], true);
    }
    assert_eq!(h.icons.stats().submitted, 1);
    assert_eq!(h.icons.stats().unavailable_sources, 1);
    h.icons.reconfigure(h.icons.bindings.clone()).unwrap();
    h.load("a", 3);
    assert_eq!(h.icons.stats().unavailable_sources, 0);
    assert!(h.icons.texture("a").is_some());
}
#[test]
fn visible_capacity_is_deferred_without_eviction_retry_or_false_source_failure() {
    let mut h = Harness::new(&["a", "b"], (1, 4));
    h.load("a", 1);
    h.icons.frame(&["a", "b"], true);
    h.call();
    h.answers.send(Answer::Image(image(2))).unwrap();
    let update = h.until(&["a", "b"], true, |icons, _| icons.stats().completed == 2);
    assert!(update.capacity_deferred);
    assert!(update.evicted.is_empty());
    for _ in 0..20 {
        let update = h.icons.frame(&["a", "b"], true);
        assert!(update.capacity_deferred);
        assert!(!update.visible_pixels_changed);
    }
    assert_eq!(h.icons.stats().submitted, 2);
    assert_eq!(h.icons.stats().unavailable_sources, 0);
    assert_eq!(h.state.borrow().pixels.len(), 1);
    let update = h.icons.frame(&["b"], true);
    assert_eq!(update.evicted, vec!["a"]);
    assert!(update.visible_pixels_changed);
    assert!(h.icons.texture("b").is_some());
    assert_eq!(h.icons.stats().texture_bytes_estimate, 4);
}
#[test]
fn capacity_ready_is_discarded_on_pause_or_view_withdrawal_without_churn() {
    for pause in [false, true] {
        let mut h = Harness::new(&["a", "b"], (1, 4));
        h.load("a", 1);
        h.icons.frame(&["a", "b"], true);
        h.call();
        h.answers.send(Answer::Image(image(2))).unwrap();
        h.until(&["a", "b"], true, |icons, _| icons.stats().completed == 2);
        let update = h.icons.frame(&["a"], !pause);
        assert!(!update.visible_pixels_changed);
        assert_eq!(h.icons.stats().submitted, 2);
        assert_eq!(h.state.borrow().pixels.len(), 1);
        h.icons.frame(&["b"], true);
        h.call();
        h.answers.send(Answer::Image(image(3))).unwrap();
        h.until(&["b"], true, |icons, _| icons.texture("b").is_some());
        assert_eq!(h.state.borrow().pixels.last().unwrap()[0], 3);
        assert_eq!(h.icons.stats().unavailable_sources, 0);
    }
}
#[test]
fn idle_worker_is_woken_by_sender_drop_and_joined_without_a_job() {
    let mut h = Harness::new(&["a"], (64, MAX_TEXTURE_BYTES));
    let done = h.icons.finish();
    assert!(done.worker_joined && !done.worker_panicked);
    assert_eq!(h.icons.stats().submitted, 0);
    assert!(h.state.borrow().pixels.is_empty());
    assert!(h.state.borrow().deleted.is_empty());
    assert_eq!(h.icons.finish(), done);
}
#[test]
fn backend_alias_or_upload_error_never_deletes_an_owned_other_texture() {
    let mut h = Harness::new(&["a", "b", "c"], (64, MAX_TEXTURE_BYTES));
    h.load("a", 1);
    let a = h.icons.texture("a").unwrap().handle;
    h.state.borrow_mut().force_next = Some(a);
    h.icons.frame(&["a", "b"], true);
    h.call();
    h.answers.send(Answer::Image(image(2))).unwrap();
    h.until(&["a", "b"], true, |icons, _| icons.stats().completed == 2);
    assert_eq!(h.icons.stats().aliased_handles, 1);
    assert!(h.icons.texture("b").is_none());
    assert_eq!(h.icons.texture("a").unwrap().handle, a);
    assert!(h.state.borrow().deleted.is_empty());
    h.state.borrow_mut().fail_next = true;
    h.icons.frame(&["a", "c"], true);
    h.call();
    h.answers.send(Answer::Image(image(3))).unwrap();
    h.until(&["a", "c"], true, |icons, _| icons.stats().completed == 3);
    assert_eq!(h.icons.stats().upload_failures, 1);
    assert_eq!(h.icons.stats().unavailable_sources, 2);
    h.icons.finish();
    assert_eq!(h.state.borrow().deleted, vec![a]);
}
#[test]
fn recycled_numeric_handle_and_dimensions_still_report_changed_visible_content() {
    let mut h = Harness::new(&["a"], (64, MAX_TEXTURE_BYTES));
    h.load("a", 1);
    let old = h.icons.texture("a").unwrap();
    h.icons.reconfigure(h.icons.bindings.clone()).unwrap();
    let reset = h.icons.frame(&["a"], true);
    assert!(reset.visible_pixels_changed);
    h.call();
    h.state.borrow_mut().force_next = Some(old.handle);
    h.answers.send(Answer::Image(image(8))).unwrap();
    let update = h.until(&["a"], true, |icons, _| icons.stats().completed == 2);
    assert!(update.visible_pixels_changed);
    assert_eq!(h.icons.texture("a").unwrap(), old);
    assert_eq!(h.state.borrow().pixels.last().unwrap(), &vec![8, 2, 3, 127]);
    h.icons.finish();
    assert_eq!(h.state.borrow().deleted, vec![old.handle, old.handle]);
}
#[test]
fn removing_binding_invalidates_visible_pixels_and_leaves_no_stale_view() {
    let mut h = Harness::new(&["a"], (64, MAX_TEXTURE_BYTES));
    h.load("a", 1);
    h.icons.reconfigure(Bindings::new()).unwrap();
    assert!(h.icons.texture("a").is_none());
    let update = h.icons.frame(&["a"], true);
    assert!(update.visible_pixels_changed);
    assert_eq!(update.evicted, vec!["a"]);
    assert_eq!(h.icons.stats().texture_entries, 0);
}
#[test]
fn invalid_decoded_shape_and_reserved_capacity_never_reach_gpu() {
    for large_capacity in [false, true] {
        let mut h = Harness::new(&["a"], (64, MAX_TEXTURE_BYTES));
        h.icons.frame(&["a"], true);
        h.call();
        let mut bad = image(1);
        if large_capacity {
            bad.rgba = Vec::with_capacity(MAX_RGBA_BYTES + 1);
            bad.rgba.extend_from_slice(&[1, 2, 3, 4]);
        } else {
            bad.width = 513;
        }
        h.answers.send(Answer::Image(bad)).unwrap();
        h.until(&["a"], true, |icons, _| icons.stats().completed == 1);
        assert!(h.state.borrow().pixels.is_empty());
        assert_eq!(h.icons.stats().unavailable_sources, 1);
    }
}
#[test]
fn shutdown_joins_real_held_worker_before_deleting_gl_and_is_idempotent() {
    let mut h = Harness::new(&["a", "b"], (64, MAX_TEXTURE_BYTES));
    h.load("a", 1);
    h.icons.frame(&["a", "b"], true);
    h.call();
    h.events.lock().unwrap().clear();
    let stopped = h.icons.worker.as_ref().unwrap().stop.clone();
    let release = h.answers.clone();
    let helper = thread::spawn(move || {
        let end = Instant::now() + WAIT;
        while !stopped.load(Ordering::Acquire) {
            assert!(Instant::now() < end);
            thread::yield_now();
        }
        let _ = release.send(Answer::Image(image(2)));
    });
    let done = h.icons.finish();
    helper.join().unwrap();
    assert!(done.worker_joined && !done.worker_panicked);
    let events = h.events.lock().unwrap().clone();
    assert!(
        events.iter().position(|e| *e == "loader_drop").unwrap()
            < events.iter().position(|e| *e == "delete").unwrap()
    );
    assert_eq!(h.icons.finish(), done);
    assert_eq!(h.state.borrow().deleted.len(), 1);
    assert!(h.state.borrow().live.is_empty());
    assert_eq!(h.state.borrow().pixels.len(), 1);
}
#[test]
fn worker_panic_is_ordinary_unavailable_and_does_not_erase_cleanup_or_repeat() {
    let mut h = Harness::new(&["a"], (64, MAX_TEXTURE_BYTES));
    h.icons.frame(&["a"], true);
    h.call();
    h.answers.send(Answer::Panic).unwrap();
    h.until(&["a"], true, |icons, _| icons.stats().worker_closed);
    for _ in 0..10 {
        h.icons.frame(&["a"], true);
    }
    assert_eq!(h.icons.stats().submitted, 1);
    let done = h.icons.finish();
    assert!(done.worker_joined && done.worker_panicked);
    assert_eq!(h.icons.finish(), done);
    assert!(h.state.borrow().live.is_empty());
}
