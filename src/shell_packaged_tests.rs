//! Actual Shell + packaged worker/reader/decoder; only GL handles use the
//! existing cfg(test) backend. No Window, branding upload, Root provider or TV.
use super::*;
use crate::live::{CatalogueReading, Reading};
use std::{
    fs,
    path::PathBuf,
    sync::atomic::{AtomicU64, Ordering},
    time::{Duration, Instant},
};

static SERIAL: AtomicU64 = AtomicU64::new(0);
const WAIT: Duration = Duration::from_secs(2);
fn id(index: usize) -> String {
    format!("fixture.packaged.app{index:02}")
}
fn png() -> Vec<u8> {
    // Actual 2x1 RGBA PNG; transparent then half-alpha pixel, no GL required.
    vec![
        137, 80, 78, 71, 13, 10, 26, 10, 0, 0, 0, 13, 73, 72, 68, 82, 0, 0, 0, 2, 0, 0, 0, 1, 8, 6,
        0, 0, 0, 244, 34, 127, 138, 0, 0, 0, 17, 73, 68, 65, 84, 120, 156, 99, 224, 18, 145, 99,
        208, 48, 178, 105, 0, 0, 3, 199, 1, 83, 206, 227, 180, 69, 0, 0, 0, 0, 73, 69, 78, 68, 174,
        66, 96, 130,
    ]
}
struct Assets(PathBuf);
impl Assets {
    fn new(count: usize) -> Self {
        let root = std::env::temp_dir().join(format!(
            "shell-packaged-{}-{}",
            std::process::id(),
            SERIAL.fetch_add(1, Ordering::Relaxed)
        ));
        fs::create_dir(&root).unwrap();
        fs::create_dir(root.join("icons")).unwrap();
        for i in 0..count {
            fs::write(root.join(format!("icons/{i:02}.png")), png()).unwrap();
        }
        Self(root)
    }
}
impl Drop for Assets {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.0);
    }
}
fn snapshot(count: usize) -> Snapshot {
    let applications = (0..count)
        .map(|i| Application {
            id: id(i),
            title: format!("Application {i:02}"),
            kind: "native".into(),
            visible: true,
            icon: Some(format!("icons/{i:02}.png")),
        })
        .collect::<Vec<_>>();
    Snapshot {
        captured_at: "synthetic packaged fixture".into(),
        launch_ids: applications.iter().map(|app| app.id.clone()).collect(),
        applications,
        inputs: Vec::new(),
        context: Context {
            input: "HDMI_1".into(),
            dynamic_range: "sdr".into(),
            picture_mode: "cinema".into(),
        },
        settings: BTreeMap::new(),
    }
}
fn shell(assets: &Assets, count: usize) -> Shell {
    let preferences = Preferences {
        initialized: true,
        favourites: (0..count.min(2)).map(id).collect(),
        favourite_order: (0..count.min(2)).map(id).collect(),
    };
    let mut shell = Shell::new(snapshot(count), preferences);
    shell.configure_packaged_icons(&assets.0);
    shell.open(Page::Applications);
    shell
}
fn stats(shell: &Shell) -> crate::packaged_icons::Stats {
    shell
        .packaged_icons
        .as_ref()
        .expect("ordinary controller configured")
        .stats()
}
fn uploads(shell: &Shell) -> u64 {
    shell.packaged_gl.receipt()["uploads"].as_u64().unwrap()
}
fn card<'a>(shell: &'a Shell, id: &str) -> &'a Card {
    shell
        .surface
        .cards
        .items
        .iter()
        .find(|card| card.id == id)
        .expect("current card")
}
fn wait(shell: &mut Shell, mut done: impl FnMut(&Shell) -> bool) {
    let deadline = Instant::now() + WAIT;
    let mut now = shell.now;
    while !done(shell) {
        assert!(
            Instant::now() < deadline,
            "packaged state: {:?}",
            stats(shell)
        );
        now = now.wrapping_add(16);
        shell.tick(now, 0.016);
        std::thread::sleep(Duration::from_millis(1));
    }
}
fn update(count: usize, now: u32) -> Update {
    let snapshot = snapshot(count);
    Update {
        observed_ms: now,
        simulated: false,
        system_keyboard: false,
        catalogue: Reading::Value(CatalogueReading {
            apps: snapshot.applications,
            launch_ids: snapshot.launch_ids,
        }),
        artwork: Reading::Failed("No privileged artwork provider in fixture".into()),
        inputs: Reading::Failed("omitted".into()),
        picture: Reading::Failed("omitted".into()),
        sound: Reading::Failed("omitted".into()),
        network: Reading::Failed("omitted".into()),
        controls: BTreeMap::new(),
    }
}
fn root_off(shell: &mut Shell) {
    assert!(shell.artwork.is_none());
    assert!(!shell.artwork_ever_enabled);
    assert!(!shell.artwork_transition_enabled());
    assert_eq!(
        shell.artwork_transition_readiness(),
        TransitionReadiness::Disabled
    );
    assert!(shell.keyboard_transition_ready());
    assert!(shell.external_transition_ready());
}

#[test]
fn scrolling_to_application70_loads_beyond_first64_without_changing_selection_or_preferences() {
    let _serial = plx_base::testlock::serial();
    let assets = Assets::new(72);
    let mut shell = shell(&assets, 72);
    let preferences = shell.preferences().clone();
    let target = id(70);
    assert!(shell.focus_application(&target));
    wait(&mut shell, |shell| card(shell, &target).has_icon());
    assert_eq!(shell.focused_item(), Some(target.clone()));
    assert_eq!(shell.preferences(), &preferences);
    assert!(!shell.take_preferences_changed());
    assert!(
        stats(&shell).submitted < 72,
        "demand loading must not require the full catalogue before app70"
    );
    assert_eq!(
        (
            card(&shell, &target).texture.1,
            card(&shell, &target).texture.2
        ),
        (2.0, 1.0)
    );
    assert!(shell.take_requests().is_empty());
    root_off(&mut shell);
}

#[test]
fn actual_worker_arrival_preserves_armed_press_focus_and_saved_preferences() {
    let _serial = plx_base::testlock::serial();
    let assets = Assets::new(1);
    let mut shell = shell(&assets, 1);
    assert!(shell.focus_application(&id(0)));
    shell.tick(100, 0.016); // Dispatch the actual anchored read/decode.
    shell.press(true, PressFrom::Key);
    assert!(shell.input.press.is_live());
    let preferences = shell.preferences().clone();
    let focused = shell.focused_item();
    let deadline = Instant::now() + WAIT;
    while !card(&shell, &id(0)).has_icon() {
        assert!(Instant::now() < deadline);
        // No input-clock advance: this isolates the real completion/presentation
        // change from the press controller's legitimate hold timeout.
        shell.pump_packaged_icons();
        std::thread::sleep(Duration::from_millis(1));
    }
    assert!(
        shell.input.press.is_live(),
        "texture arrival must not rebuild/cancel input"
    );
    assert_eq!(shell.focused_item(), focused);
    assert_eq!(shell.preferences(), &preferences);
    assert!(!shell.take_preferences_changed());
    assert!(shell.take_requests().is_empty());
    shell.cancel_press();
    root_off(&mut shell);
}

#[test]
fn same_frame_keyboard_commit_and_hidden_idle_gates_prevent_new_jobs_and_uploads() {
    let _serial = plx_base::testlock::serial();
    let assets = Assets::new(1);
    let mut shell = shell(&assets, 1);
    shell.system_keyboard_available = true;
    shell.set_artwork_pause(PauseReason::ForegroundLost, true);
    assert!(shell.focus_drawer_control(2));
    shell
        .input_frame(100, 0.016, |shell| {
            shell.press(true, PressFrom::Key);
            Ok::<(), ()>(())
        })
        .unwrap();
    shell
        .input_frame(200, 0.016, |shell| {
            shell.press(false, PressFrom::Key);
            Ok::<(), ()>(())
        })
        .unwrap();
    shell.set_artwork_pause(PauseReason::ForegroundLost, false);
    shell.input_frame(500, 0.016, |_| Ok::<(), ()>(())).unwrap();
    assert!(shell.drawer_state().system_keyboard());
    assert!(shell.artwork_pauses().contains(PauseReason::KeyboardIntent));
    assert_eq!((stats(&shell).submitted, uploads(&shell)), (0, 0));
    root_off(&mut shell);
    shell.back();
    shell.finish_packaged_icons().unwrap();
    for reason in [PauseReason::ForegroundLost, PauseReason::Idle] {
        let mut gated = self::shell(&assets, 1);
        gated.set_artwork_pause(reason, true);
        for frame in 0..10 {
            gated.tick(100 + frame * 16, 0.016);
        }
        assert_eq!((stats(&gated).submitted, uploads(&gated)), (0, 0));
        gated.set_artwork_pause(reason, false);
        wait(&mut gated, |shell| card(shell, &id(0)).has_icon());
        assert!(stats(&gated).submitted > 0 && uploads(&gated) > 0);
        root_off(&mut gated);
    }
}

#[test]
fn removed_app_discards_queued_result_and_reappearance_keeps_original_packaged_binding() {
    let _serial = plx_base::testlock::serial();
    let assets = Assets::new(1);
    let mut shell = shell(&assets, 1);
    shell.tick(100, 0.016);
    assert_eq!(stats(&shell).submitted, 1);
    shell.apply_live(&update(0, 101), 101);
    wait(&mut shell, |shell| stats(shell).completed == 1);
    assert!(shell.surface.cards.items.is_empty());
    assert!(shell.textures.is_empty());
    assert_eq!(uploads(&shell), 0);
    assert!(stats(&shell).discarded >= 1);
    let mut reappeared = update(1, 200);
    if let Reading::Value(catalogue) = &mut reappeared.catalogue {
        catalogue.apps[0].icon = Some("icons/unpackaged-live-path.png".into());
    }
    let expected = crate::packaged_icon_reader::RelativeIconPath::parse("icons/00.png").unwrap();
    shell.apply_live(&reappeared, 200);
    assert_eq!(shell.packaged_bindings.get(&id(0)), Some(&expected));
    assert!(shell.focus_application(&id(0)));
    wait(&mut shell, |shell| card(shell, &id(0)).has_icon());
    assert_eq!(
        (
            card(&shell, &id(0)).texture.1,
            card(&shell, &id(0)).texture.2
        ),
        (2.0, 1.0)
    );
    root_off(&mut shell);
}

#[test]
fn actual_drawn_partial_unfocused_tile_is_ineligible_but_focus_keeps_clipped_image_interest() {
    let _serial = plx_base::testlock::serial();
    let mut shell = Shell::new(snapshot(16), Preferences::default());
    shell.open(Page::Applications);
    assert!(shell.focus_drawer_control(0));
    shell.drawer_scroll.jump(100.0);
    shell.surface.grid.set_page(340.0, 100.0);
    let partial = {
        let cx = shell.context(&LegacyMeasure);
        let clip = shell.surface.grid_area();
        shell
            .surface
            .cards
            .items
            .iter()
            .find_map(|card| {
                let r = shell
                    .surface
                    .grid
                    .place(&cx, &shell.surface.cards, &card.key, At::Drawn)?
                    .rect;
                let intersects = r.x < clip.x + clip.w
                    && r.x + r.w > clip.x
                    && r.y < clip.y + clip.h
                    && r.y + r.h > clip.y;
                let whole = r.x >= clip.x
                    && r.y >= clip.y
                    && r.x + r.w <= clip.x + clip.w
                    && r.y + r.h <= clip.y + clip.h;
                (intersects && !whole).then(|| card.id.clone())
            })
            .expect("actual grid has a partially clipped row at this scroll")
    };
    assert!(!shell.packaged_visible().contains(&partial));
    assert!(shell.focus_application(&partial));
    assert!(shell.packaged_visible().contains(&partial));
    root_off(&mut shell);
}

#[test]
fn known_pending_icon_is_blank_icon_art_and_actual_read_failure_restores_missing_art_fallback() {
    let _serial = plx_base::testlock::serial();
    let assets = Assets::new(0);
    let mut shell = shell(&assets, 1);
    assert!(card(&shell, &id(0)).loading_icon);
    assert!(
        matches!(shell.surface.cards.art(0), Art::Icon { id: 0, .. }),
        "warming known icon must not acquire the fallback matte"
    );
    wait(&mut shell, |shell| stats(shell).unavailable_sources == 1);
    assert!(!card(&shell, &id(0)).loading_icon);
    assert!(!card(&shell, &id(0)).has_icon());
    assert!(matches!(
        shell.surface.cards.art(0),
        Art::Texture { id: 0, .. }
    ));
    assert_eq!(uploads(&shell), 0);
    root_off(&mut shell);
}

#[test]
fn finish_joins_deletes_all_owned_handles_and_clears_borrowed_cards_without_root_wait() {
    let _serial = plx_base::testlock::serial();
    let assets = Assets::new(8);
    let mut shell = shell(&assets, 8);
    wait(&mut shell, |shell| uploads(shell) >= 2);
    let uploaded = uploads(&shell);
    assert!(shell.surface.cards.items.iter().any(Card::has_icon));
    root_off(&mut shell);
    let receipt = shell.finish_packaged_icons().unwrap();
    assert_eq!(receipt["worker_joined"], true);
    assert_eq!(receipt["worker_panicked"], false);
    assert_eq!(receipt["remaining_textures"], 0);
    assert_eq!(receipt["gl"]["uploads"].as_u64(), Some(uploaded));
    assert_eq!(receipt["gl"]["deletes"].as_u64(), Some(uploaded));
    assert!(shell.packaged_icons.is_none() && shell.textures.is_empty());
    assert!(shell
        .surface
        .cards
        .items
        .iter()
        .all(|card| card.texture.0 == 0 && !card.loading_icon));
    assert!(shell.finish_packaged_icons().is_none());
    root_off(&mut shell);
    assert!(shell.take_requests().is_empty());
}

#[test]
fn window_close_batch_skips_packaged_poll_upload_and_new_dispatch() {
    let _serial = plx_base::testlock::serial();
    let assets = Assets::new(1);
    let mut shell = shell(&assets, 1);
    shell.tick(100, 0.016);
    assert_eq!(stats(&shell).submitted, 1);
    assert_eq!(stats(&shell).completed, 0);
    shell
        .input_frame(116, 0.016, |shell| {
            shell.request_exit();
            Ok::<(), ()>(())
        })
        .unwrap();
    assert!(shell.should_exit());
    // The batch must skip tick altogether, independently of whether the real
    // worker has already queued its completion. No result is polled or uploaded.
    assert_eq!(stats(&shell).submitted, 1);
    assert_eq!(stats(&shell).completed, 0);
    assert_eq!(uploads(&shell), 0);
    let finish = shell.finish_packaged_icons().unwrap();
    assert_eq!(finish["worker_joined"], true);
    assert_eq!(finish["remaining_textures"], 0);
}
