//! Actual asynchronous Provider/decoder/cache/runtime plus Shell integration.
//! The GL backend is its cfg(test) fake; no GL context, filesystem or TV calls.
use super::*;
use crate::app_artwork::{sources_from_raw, EncodedFile, FileIdentity, Source, SourceKey};
use crate::artwork_cache::Config;
use crate::artwork_runtime::Runtime;
use crate::artwork_worker::{Cancellation, Provider, ProviderError, Worker};
use crate::live::{CatalogueReading, Reading, Update};
use serde_json::{json, Value};
use std::sync::{
    atomic::{AtomicUsize, Ordering},
    mpsc, Arc,
};
use std::time::{Duration, Instant};

const A: &str = "fixture.artwork.a";
const B: &str = "fixture.artwork.b";
const FALLBACK: (u32, f32, f32) = (0, 80.0, 80.0);

fn crc(bytes: &[u8]) -> u32 {
    let mut value = u32::MAX;
    for byte in bytes {
        value ^= u32::from(*byte);
        for _ in 0..8 {
            value = (value >> 1) ^ if value & 1 != 0 { 0xedb8_8320 } else { 0 };
        }
    }
    !value
}
fn chunk(output: &mut Vec<u8>, kind: &[u8; 4], body: &[u8]) {
    output.extend_from_slice(&(body.len() as u32).to_be_bytes());
    output.extend_from_slice(kind);
    output.extend_from_slice(body);
    let mut checked = kind.to_vec();
    checked.extend_from_slice(body);
    output.extend_from_slice(&crc(&checked).to_be_bytes());
}
fn png(red: u8) -> Vec<u8> {
    // Valid 2x1 RGBA PNG, stored DEFLATE, exact CRC and Adler. Different red
    // samples distinguish actual v1/v2 decoder results rather than fake pixels.
    let mut raw = vec![0];
    raw.extend_from_slice(&[red, 20, 30, 255].repeat(2));
    let size = raw.len() as u16;
    let mut compressed = vec![0x78, 0x01, 1];
    compressed.extend_from_slice(&size.to_le_bytes());
    compressed.extend_from_slice(&(!size).to_le_bytes());
    compressed.extend_from_slice(&raw);
    let (mut a, mut b) = (1_u32, 0_u32);
    for byte in raw {
        a = (a + u32::from(byte)) % 65521;
        b = (b + a) % 65521;
    }
    compressed.extend_from_slice(&((b << 16) | a).to_be_bytes());
    let mut output = b"\x89PNG\r\n\x1a\n".to_vec();
    let mut header = 2_u32.to_be_bytes().to_vec();
    header.extend_from_slice(&1_u32.to_be_bytes());
    header.extend_from_slice(&[8, 6, 0, 0, 0]);
    chunk(&mut output, b"IHDR", &header);
    chunk(&mut output, b"IDAT", &compressed);
    chunk(&mut output, b"IEND", &[]);
    output
}
struct Permit {
    red: u8,
    inode: u64,
}
struct Gate {
    entered: mpsc::Sender<SourceKey>,
    permits: mpsc::Receiver<Permit>,
    dropped: mpsc::Sender<()>,
    cancelled: Arc<AtomicUsize>,
}
impl Provider for Gate {
    fn fetch(
        &mut self,
        source: &Source,
        deadline: Instant,
        cancellation: &Cancellation,
    ) -> Result<EncodedFile, ProviderError> {
        self.entered
            .send(source.key().clone())
            .map_err(|_| ProviderError::Unavailable)?;
        loop {
            if cancellation.is_cancelled() {
                self.cancelled.fetch_add(1, Ordering::SeqCst);
                return Err(ProviderError::Cancelled);
            }
            let left = deadline.saturating_duration_since(Instant::now());
            if left.is_zero() {
                return Err(ProviderError::Expired);
            }
            match self
                .permits
                .recv_timeout(left.min(Duration::from_millis(5)))
            {
                Ok(permit) => {
                    let bytes = png(permit.red);
                    return Ok(EncodedFile {
                        identity: FileIdentity {
                            device: 1,
                            inode: permit.inode,
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
                    });
                }
                Err(mpsc::RecvTimeoutError::Timeout) => (),
                Err(mpsc::RecvTimeoutError::Disconnected) => {
                    return Err(ProviderError::Unavailable)
                }
            }
        }
    }
}
impl Drop for Gate {
    fn drop(&mut self) {
        let _ = self.dropped.send(());
    }
}
struct Controls {
    entered: mpsc::Receiver<SourceKey>,
    permits: mpsc::Sender<Permit>,
    dropped: mpsc::Receiver<()>,
    cancelled: Arc<AtomicUsize>,
}
impl Controls {
    fn started(&self) -> SourceKey {
        self.entered
            .recv_timeout(Duration::from_secs(2))
            .expect("actual controlled fetch starts")
    }
    fn release(&self, red: u8, inode: u64) {
        self.permits
            .send(Permit { red, inode })
            .expect("actual provider remains available");
    }
}
fn runtime() -> (Runtime, Controls) {
    let (entered_tx, entered_rx) = mpsc::channel();
    let (permit_tx, permit_rx) = mpsc::channel();
    let (drop_tx, drop_rx) = mpsc::channel();
    let cancelled = Arc::new(AtomicUsize::new(0));
    let observed = cancelled.clone();
    let worker = Worker::start(move || Gate {
        entered: entered_tx,
        permits: permit_rx,
        dropped: drop_tx,
        cancelled: observed,
    })
    .unwrap();
    (
        Runtime::new(worker, Config::default()).unwrap(),
        Controls {
            entered: entered_rx,
            permits: permit_tx,
            dropped: drop_rx,
            cancelled,
        },
    )
}
fn snapshot(ids: &[&str]) -> Snapshot {
    Snapshot {
        captured_at: "synthetic artwork fixture".into(),
        applications: ids
            .iter()
            .map(|id| Application {
                id: (*id).into(),
                title: (*id).into(),
                kind: "native".into(),
                visible: true,
                icon: None,
            })
            .collect(),
        launch_ids: ids.iter().map(|id| (*id).into()).collect(),
        inputs: Vec::new(),
        context: Context {
            input: "HDMI_1".into(),
            dynamic_range: "sdr".into(),
            picture_mode: "cinema".into(),
        },
        settings: BTreeMap::new(),
    }
}
fn update(ids: &[&str], version: &str, icon: &str, observed_ms: u32) -> Update {
    let apps = ids
        .iter()
        .map(|id| {
            json!({"id":id,"title":id,"visible":true,"type":"native","version":version,
        "folderPath":format!("/usr/palm/applications/{id}"),"icon":icon})
        })
        .collect::<Vec<_>>();
    let points = ids.iter().map(|id| json!({"id":id})).collect::<Vec<_>>();
    let sources = sources_from_raw(&json!({"replies":{"apps":{"returnValue":true,"apps":apps},"launch_points":{"returnValue":true,"launchPoints":points}}}), crate::identity::PROOF).unwrap();
    let snapshot = snapshot(ids);
    Update {
        system_keyboard: false,
        observed_ms,
        simulated: false,
        catalogue: Reading::Value(CatalogueReading {
            apps: snapshot.applications,
            launch_ids: snapshot.launch_ids,
        }),
        artwork: Reading::Value(sources),
        inputs: Reading::Failed("fixture omitted".into()),
        picture: Reading::Failed("fixture omitted".into()),
        sound: Reading::Failed("fixture omitted".into()),
        network: Reading::Failed("fixture omitted".into()),
        controls: BTreeMap::new(),
    }
}
fn shell(ids: &[&str]) -> (Shell, Controls) {
    let (runtime, controls) = runtime();
    let mut shell = Shell::new(snapshot(ids), Preferences::default());
    for id in ids {
        shell.textures.insert((*id).into(), FALLBACK);
    }
    shell.enable_live_artwork(runtime, None);
    shell.apply_live(&update(ids, "1", "icon.png", 100), 100);
    shell.open(Page::Applications);
    shell.tick(100, 0.016);
    shell.take_preferences_changed();
    (shell, controls)
}
fn value(shell: &Shell, path: &str) -> u64 {
    shell
        .live_artwork_receipt()
        .pointer(path)
        .and_then(Value::as_u64)
        .expect("typed artwork counter")
}
fn drive_until(shell: &mut Shell, ms: u32, ready: impl Fn(&mut Shell) -> bool) {
    let deadline = Instant::now() + Duration::from_secs(2);
    loop {
        let before = value(shell, "/gl/uploads");
        shell.tick(ms, 0.016);
        assert!(
            value(shell, "/gl/uploads") - before <= 1,
            "one actual fake-backend upload per tick"
        );
        if ready(shell) {
            return;
        }
        assert!(
            Instant::now() < deadline,
            "controlled worker did not reach expected Shell state"
        );
        std::thread::yield_now();
    }
}
fn texture(shell: &Shell, id: &str) -> (u32, f32, f32) {
    shell
        .surface
        .cards
        .items
        .iter()
        .find(|card| card.id == id)
        .unwrap()
        .texture
}
fn pixels(red: u8) -> String {
    plx_base::sha256::sha256(&[red, 20, 30, 255].repeat(2))
        .iter()
        .map(|b| format!("{b:02x}"))
        .collect()
}

#[test]
fn replaced_visible_pixels_invalidate_even_when_backend_reuses_the_same_handle() {
    let _serial = plx_base::testlock::serial();
    let (mut shell, controls) = shell(&[A]);
    assert_eq!(controls.started().app_id, A);
    controls.release(40, 1);
    let deadline = Instant::now() + Duration::from_secs(2);
    while value(&shell, "/gl/uploads") < 1 {
        shell.pump_artwork();
        assert!(Instant::now() < deadline);
        std::thread::yield_now();
    }
    let old = texture(&shell, A);
    assert!(old.0 > 0);
    shell.artwork_gl.reuse_test_handle(old.0);
    shell.artwork.as_mut().unwrap().request_refresh(A);
    shell.pump_artwork();
    assert_eq!(controls.started().app_id, A);
    let _ = plx_machine::idle::take_page_damage();
    controls.release(200, 2);
    while value(&shell, "/gl/uploads") < 2 {
        shell.pump_artwork();
        assert!(Instant::now() < deadline);
        std::thread::yield_now();
    }
    assert_eq!(
        texture(&shell, A),
        old,
        "a reused numeric handle cannot signal changed pixels"
    );
    assert_eq!(
        shell.artwork_target(A, true).unwrap()["rgba_sha256"],
        pixels(200)
    );
    assert!(
        plx_machine::idle::take_page_damage(),
        "new visible pixels must schedule a frame"
    );
    let done = shell.finish_live_artwork().unwrap();
    assert_eq!(done["gpu_cleanup_qualified"], true);
}

#[test]
fn asynchronous_pixels_preserve_armed_ok_focus_scroll_and_preferences_with_one_upload_per_tick() {
    let _serial = plx_base::testlock::serial();
    let (mut shell, controls) = shell(&[A, B]);
    assert_eq!(controls.started().app_id, A);
    shell.enable_app_launches(true);
    assert!(shell.focus_application(A));
    shell.drawer_scroll.jump(0.0);
    let scroll = shell.drawer_scroll.pos;
    let focus = shell.focused_item();
    let preferences = shell.preferences().clone();
    shell.press(true, PressFrom::Key);
    controls.release(10, 1);
    drive_until(&mut shell, 116, |s| s.artwork_target(A, true).is_some());
    assert_eq!(shell.focused_item(), focus);
    assert_eq!(shell.drawer_scroll.pos, scroll);
    assert_eq!(shell.preferences(), &preferences);
    assert!(!shell.take_preferences_changed());
    assert!(shell.take_requests().is_empty());
    assert_ne!(texture(&shell, A).0, 0);
    assert_eq!(controls.started().app_id, B);
    controls.release(20, 2);
    drive_until(&mut shell, 132, |s| s.artwork_target(B, true).is_some());
    assert_eq!(shell.focused_item(), focus);
    assert_eq!(shell.drawer_scroll.pos, scroll);
    shell.tick(150, 0.016);
    shell.press(false, PressFrom::Key);
    for ms in (166..=500).step_by(16) {
        shell.tick(ms, 0.016);
    }
    assert_eq!(
        shell.take_requests(),
        vec![Action::RequestApplication(A.into())]
    );
    assert!(shell.menu_application().is_none());
    assert_eq!(shell.preferences(), &preferences);
    assert!(!shell.take_preferences_changed());
    assert_eq!(
        shell.artwork_target(A, true).unwrap()["rgba_sha256"],
        pixels(10)
    );
}

#[test]
fn old_worker_pixels_after_metadata_replacement_or_removal_never_reenter_shell_textures() {
    let _serial = plx_base::testlock::serial();
    let (mut shell, controls) = shell(&[A, B]);
    assert_eq!(controls.started().version, "1");
    assert!(shell.focus_application(A));
    let focus = shell.focused_item();
    let prefs = shell.preferences().clone();
    shell.apply_live(&update(&[A, B], "2", "replacement.png", 116), 116);
    controls.release(10, 10);
    drive_until(&mut shell, 116, |s| {
        value(s, "/runtime/completed") == 1 && value(s, "/runtime/submitted") == 2
    });
    assert_eq!(value(&shell, "/gl/uploads"), 0);
    assert_eq!(value(&shell, "/runtime/stale_results"), 1);
    assert_eq!(texture(&shell, A), FALLBACK);
    let fresh = controls.started();
    assert_eq!(fresh.app_id, A);
    assert_eq!(fresh.version, "2");
    assert_eq!(fresh.relative, vec!["replacement.png"]);
    controls.release(30, 30);
    drive_until(&mut shell, 132, |s| s.artwork_target(A, true).is_some());
    let target = shell.artwork_target(A, true).unwrap();
    assert_eq!(target["source_key"]["version"], "2");
    assert_eq!(target["rgba_sha256"], pixels(30));
    assert_eq!(shell.focused_item(), focus);
    assert_eq!(shell.preferences(), &prefs);
    assert_eq!(controls.started().app_id, B);
    shell.apply_live(&update(&[], "2", "replacement.png", 148), 148);
    controls.release(40, 40);
    drive_until(&mut shell, 148, |s| value(s, "/runtime/completed") == 3);
    assert!(shell.artwork_target_absent(A));
    assert!(shell.artwork_target_absent(B));
    assert_eq!(value(&shell, "/gl/uploads"), 1);
    assert_eq!(value(&shell, "/gl/deletes"), 1);
    assert_eq!(value(&shell, "/textures/entries"), 0);
    assert_eq!(shell.preferences(), &prefs);
}

#[test]
fn suspended_completion_and_failed_or_aged_catalogue_restore_only_packaged_fallback() {
    let _serial = plx_base::testlock::serial();
    for expired in [false, true] {
        let (mut shell, controls) = shell(&[A]);
        controls.started();
        controls.release(10, 1);
        drive_until(&mut shell, 116, |s| s.artwork_target(A, false).is_some());
        shell.artwork.as_mut().unwrap().request_refresh(A);
        shell.tick(132, 0.016);
        controls.started();
        shell.set_artwork_active(false);
        controls.release(20, 2);
        drive_until(&mut shell, 148, |s| value(s, "/runtime/completed") == 2);
        assert_eq!(value(&shell, "/runtime/pending_results"), 1);
        assert_eq!(value(&shell, "/gl/uploads"), 1);
        if expired {
            shell.age_live(15_101);
        } else {
            let mut bad = update(&[A], "1", "icon.png", 148);
            bad.catalogue = Reading::Failed("actual fixture refusal".into());
            bad.artwork = Reading::Failed("actual fixture refusal".into());
            shell.apply_live(&bad, 148);
        }
        shell.tick(164, 0.016);
        assert_eq!(texture(&shell, A), FALLBACK);
        assert!(shell.artwork_target(A, false).is_none());
        assert_eq!(value(&shell, "/textures/entries"), 0);
        assert_eq!(value(&shell, "/gl/deletes"), 1);
        assert_eq!(value(&shell, "/runtime/pending_results"), 0);
        shell.set_artwork_active(true);
        shell.tick(180, 0.016);
        assert_eq!(value(&shell, "/runtime/submitted"), 2);
        assert!(controls.entered.try_recv().is_err());
        assert_eq!(shell.textures.get(A), Some(&FALLBACK));
        shell.apply_live(&update(&[A], "1", "icon.png", 200), 200);
        shell.tick(216, 0.016);
        assert_eq!(controls.started().app_id, A);
        controls.release(30, 3);
        drive_until(&mut shell, 216, |s| s.artwork_target(A, true).is_some());
        assert_eq!(
            shell.artwork_target(A, true).unwrap()["rgba_sha256"],
            pixels(30)
        );
        assert_eq!(value(&shell, "/gl/uploads"), 2);
    }
}

#[test]
fn finish_cancels_and_joins_actual_worker_before_releasing_only_live_texture_ownership() {
    let _serial = plx_base::testlock::serial();
    let (mut shell, controls) = shell(&[A, B]);
    controls.started();
    controls.release(10, 1);
    drive_until(&mut shell, 116, |s| s.artwork_target(A, false).is_some());
    assert_eq!(controls.started().app_id, B);
    let prefs = shell.preferences().clone();
    let done = shell.finish_live_artwork().unwrap();
    assert_eq!(done["joined"], true);
    assert_eq!(done["panicked"], false);
    assert_eq!(done["outstanding_resolved"], true);
    assert_eq!(done["gpu_cleanup_qualified"], true);
    assert_eq!(done["remaining_live_textures"], 0);
    assert_eq!(done["runtime"]["closed"], true);
    assert_eq!(done["runtime"]["in_flight"], 0);
    assert_eq!(done["runtime"]["pending_results"], 0);
    assert_eq!(done["runtime"]["resident_capacity_bytes"], 0);
    assert_eq!(done["gl"]["uploads"], 1);
    assert_eq!(done["gl"]["deletes"], 1);
    assert!(controls.cancelled.load(Ordering::SeqCst) >= 1);
    assert!(
        controls.dropped.try_recv().is_ok(),
        "provider dropped before join returned"
    );
    assert_eq!(texture(&shell, A), FALLBACK);
    assert_eq!(shell.textures.get(A), Some(&FALLBACK));
    assert_eq!(shell.preferences(), &prefs);
    assert!(shell.finish_live_artwork().is_none());
    assert_eq!(value(&shell, "/gl/deletes"), 1);
}
