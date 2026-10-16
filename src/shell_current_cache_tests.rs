//! Host-only Shell freshness/ownership integration; Root Runtime stays absent.
//! The worker's host profile is Unsupported, so only packaged fallback is read.
//! Real durable-cache hit files/decoding belong to the separate consumer tests.
use super::*;
use crate::live::{CatalogueReading, Reading};
use serde_json::json;
use std::{
    fs,
    path::PathBuf,
    sync::atomic::{AtomicU64, Ordering},
    time::{Duration, Instant},
};
const ID: &str = "fixture.ordinary.current";
static SERIAL: AtomicU64 = AtomicU64::new(0);
struct Assets(PathBuf);
impl Assets {
    fn new() -> Self {
        let path = std::env::temp_dir().join(format!(
            "shell-current-cache-{}-{}",
            std::process::id(),
            SERIAL.fetch_add(1, Ordering::Relaxed)
        ));
        fs::create_dir(&path).unwrap();
        fs::create_dir(path.join("icons")).unwrap();
        // Real 2x1 transparent/half-alpha PNG, only fake GL handle backend.
        fs::write(
            path.join("icons/app.png"),
            [
                137, 80, 78, 71, 13, 10, 26, 10, 0, 0, 0, 13, 73, 72, 68, 82, 0, 0, 0, 2, 0, 0, 0,
                1, 8, 6, 0, 0, 0, 244, 34, 127, 138, 0, 0, 0, 17, 73, 68, 65, 84, 120, 156, 99,
                224, 18, 145, 99, 208, 48, 178, 105, 0, 0, 3, 199, 1, 83, 206, 227, 180, 69, 0, 0,
                0, 0, 73, 69, 78, 68, 174, 66, 96, 130,
            ],
        )
        .unwrap();
        Self(path)
    }
}
impl Drop for Assets {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.0);
    }
}
fn snapshot(packaged: bool) -> Snapshot {
    Snapshot {
        captured_at: "synthetic ordinary cache".into(),
        applications: vec![Application {
            id: ID.into(),
            title: "Ordinary fixture".into(),
            kind: "native".into(),
            visible: true,
            icon: packaged.then(|| "icons/app.png".into()),
        }],
        launch_ids: BTreeSet::from([ID.into()]),
        inputs: vec![],
        context: Context {
            input: "HDMI_1".into(),
            dynamic_range: "sdr".into(),
            picture_mode: "cinema".into(),
        },
        settings: BTreeMap::new(),
    }
}
fn sources(version: &str) -> crate::app_artwork::SourceMap {
    crate::app_artwork::sources_from_raw(&json!({"replies":{"apps":{"returnValue":true,"apps":[{"id":ID,"title":"Ordinary fixture","type":"native","visible":true,"version":version,"folderPath":format!("/media/developer/apps/usr/palm/applications/{ID}"),"icon":"icon.png"}]},"launch_points":{"returnValue":true,"launchPoints":[{"id":ID}]}}}),crate::identity::PREVIEW).unwrap()
}
fn update(stamp: u32, version: &str) -> Update {
    let s = snapshot(true);
    Update {
        observed_ms: stamp,
        simulated: false,
        system_keyboard: false,
        catalogue: Reading::Value(CatalogueReading {
            apps: s.applications,
            launch_ids: s.launch_ids,
        }),
        artwork: Reading::Value(sources(version)),
        inputs: Reading::Failed("omitted".into()),
        picture: Reading::Failed("omitted".into()),
        sound: Reading::Failed("omitted".into()),
        network: Reading::Failed("omitted".into()),
        controls: BTreeMap::new(),
    }
}
fn shell(assets: &Assets, packaged: bool) -> Shell {
    let mut s = Shell::new(
        snapshot(packaged),
        Preferences {
            initialized: true,
            favourites: BTreeSet::from([ID.into()]),
            favourite_order: vec![ID.into()],
        },
    );
    s.enable_current_icon_cache();
    s.configure_packaged_icons(&assets.0);
    s.open(Page::Applications);
    s
}
fn root_off(s: &mut Shell) {
    assert!(s.artwork.is_none());
    assert!(!s.artwork_ever_enabled);
    assert_eq!(
        s.artwork_transition_readiness(),
        TransitionReadiness::Disabled
    );
    assert!(s.keyboard_transition_ready());
    assert!(s.external_transition_ready());
}
#[test]
fn cache_mode_starts_with_empty_packaged_bindings_and_never_creates_root_runtime() {
    let _serial = plx_base::testlock::serial();
    let assets = Assets::new();
    let mut s = shell(&assets, false);
    assert!(s.packaged_bindings.is_empty());
    assert!(s.packaged_icons.is_some());
    assert_eq!(
        s.packaged_icons.as_ref().unwrap().stats().cache_readiness,
        crate::packaged_icons::CacheReadiness::AwaitingSources
    );
    root_off(&mut s);
    let receipt = s.finish_packaged_icons().unwrap();
    assert_eq!(receipt["worker_joined"], true);
    assert_eq!(receipt["cache_uploads"], 0);
    assert_eq!(receipt["cache_readiness_is_index_not_images"], true);
}
#[test]
fn full_source_map_and_own_clock_refresh_without_replacing_equal_data() {
    let _serial = plx_base::testlock::serial();
    let assets = Assets::new();
    let mut s = shell(&assets, false);
    let u = update(100, "1");
    s.apply_live(&u, 100);
    assert_eq!(s.current_icon_sources.as_ref(), Some(&sources("1")));
    assert_eq!(s.current_icon_sources_observed, Some(100));
    let pointer = s.current_icon_sources.as_ref().unwrap()[ID].app_id.as_ptr();
    let focused = s.focused_item();
    s.apply_live(&update(200, "1"), 200);
    assert_eq!(s.current_icon_sources_observed, Some(200));
    assert_eq!(
        s.current_icon_sources.as_ref().unwrap()[ID].app_id.as_ptr(),
        pointer
    );
    assert_eq!(s.focused_item(), focused);
    assert_eq!(
        s.packaged_icons.as_ref().unwrap().stats().cache_readiness,
        crate::packaged_icons::CacheReadiness::Pending
    );
    s.set_artwork_external_held(true);
    root_off(&mut s);
    s.set_artwork_external_held(false);
}
#[test]
fn partial_failure_or_simulated_catalogue_withdraws_independently_of_other_success() {
    let _serial = plx_base::testlock::serial();
    let assets = Assets::new();
    let mut s = shell(&assets, false);
    for case in 0..3 {
        s.apply_live(&update(10, "1"), 10);
        assert!(s.current_icon_sources.is_some());
        let mut failed = update(20, "1");
        match case {
            0 => failed.artwork = Reading::Failed("artwork refused; catalogue still fresh".into()),
            1 => failed.catalogue = Reading::Failed("catalogue refused; artwork present".into()),
            _ => failed.simulated = true,
        }
        s.apply_live(&failed, 20);
        assert!(s.current_icon_sources.is_none());
        assert_eq!(s.current_icon_sources_observed, None);
        root_off(&mut s);
    }
}
#[test]
fn exact_expiry_uses_own_stamp_and_wrapping_clock_without_root_dependency() {
    let _serial = plx_base::testlock::serial();
    let assets = Assets::new();
    let mut s = shell(&assets, false);
    let stamp = u32::MAX - 500;
    s.apply_live(&update(stamp, "1"), stamp);
    s.age_live(stamp.wrapping_add(15_000));
    assert!(s.current_icon_sources.is_some());
    let expired = stamp.wrapping_add(15_001);
    // A separate fresh general catalogue clock cannot lend freshness to cache data.
    s.live_state.as_mut().unwrap().catalogue = Some(expired);
    s.age_live(expired);
    assert!(s.artwork_catalogue_current());
    assert!(s.current_icon_sources.is_none());
    assert_eq!(s.current_icon_sources_observed, None);
    root_off(&mut s);
    s.apply_live(&update(0, "1"), 16_000);
    assert!(s.current_icon_sources.is_none());
}
#[test]
fn removal_and_complete_source_revision_change_are_forwarded_unfiltered() {
    let _serial = plx_base::testlock::serial();
    let assets = Assets::new();
    let mut s = shell(&assets, false);
    s.artwork_filter = Some(BTreeSet::new()); // Privileged filter must not censor ordinary associations.
    s.apply_live(&update(10, "1"), 10);
    assert_eq!(
        s.current_icon_sources.as_ref().unwrap()[ID]
            .preferred()
            .unwrap()
            .key()
            .version,
        "1"
    );
    s.apply_live(&update(20, "2"), 20);
    assert_eq!(
        s.current_icon_sources.as_ref().unwrap()[ID]
            .preferred()
            .unwrap()
            .key()
            .version,
        "2"
    );
    let mut removed = update(30, "2");
    removed.artwork = Reading::Value(BTreeMap::new());
    s.apply_live(&removed, 30);
    assert_eq!(s.current_icon_sources, Some(BTreeMap::new()));
    assert_eq!(s.current_icon_sources_observed, Some(30));
    root_off(&mut s);
}
#[test]
fn paused_withdrawal_drops_borrowed_pixels_without_poll_upload_delete_or_press_rebuild() {
    let _serial = plx_base::testlock::serial();
    let assets = Assets::new();
    let mut s = shell(&assets, true);
    s.apply_live(&update(0, "1"), 0);
    assert!(s.focus_application(ID));
    let until = Instant::now() + Duration::from_secs(2);
    while !s.textures.contains_key(ID) {
        assert!(Instant::now() < until);
        s.pump_packaged_icons();
        std::thread::yield_now();
    }
    s.press(true, PressFrom::Key);
    let focused = s.focused_item();
    let prefs = s.preferences().clone();
    assert!(s.input.press.is_live());
    s.set_artwork_pause(PauseReason::KeyboardActive, true);
    let before = s.packaged_gl.receipt();
    let mut failed = update(1, "1");
    failed.artwork = Reading::Failed("withdraw while keyboard owns input".into());
    s.apply_live(&failed, 1);
    assert!(!s.textures.contains_key(ID));
    assert!(s
        .surface
        .cards
        .items
        .iter()
        .filter(|c| c.id == ID)
        .all(|c| c.texture.0 == 0));
    let after = s.packaged_gl.receipt();
    assert_eq!(before["uploads"], after["uploads"]);
    assert_eq!(before["deletes"], after["deletes"]);
    assert!(s.input.press.is_live());
    assert_eq!(s.focused_item(), focused);
    assert_eq!(s.preferences(), &prefs);
    root_off(&mut s);
    let done = s.finish_packaged_icons().unwrap();
    assert_eq!(done["worker_joined"], true);
    assert_eq!(done["remaining_textures"], 0);
}
