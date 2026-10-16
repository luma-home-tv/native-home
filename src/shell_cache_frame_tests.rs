//! Pure metadata/geometry fixtures, never an actual cached-pixel or native claim.
//! The actual controller origin getter is independently exercised with real files.
use super::*;
use crate::live::{CatalogueReading, Reading};
use serde_json::json;
fn id(i: usize) -> String {
    format!("fixture.session.app{i:03}")
}
fn fixture(count: usize) -> (Shell, crate::app_artwork::SourceMap) {
    let apps = (0..count)
        .map(|i| Application {
            id: id(i),
            title: format!("Fixture {i:03}"),
            kind: "native".into(),
            visible: true,
            icon: None,
        })
        .collect::<Vec<_>>();
    let launch_ids = apps.iter().map(|a| a.id.clone()).collect();
    let snapshot = Snapshot {
        captured_at: "synthetic frame metadata".into(),
        applications: apps.clone(),
        launch_ids,
        inputs: vec![],
        context: Context {
            input: "HDMI_1".into(),
            dynamic_range: "sdr".into(),
            picture_mode: "cinema".into(),
        },
        settings: BTreeMap::new(),
    };
    let rows=(0..count).map(|i|json!({"id":id(i),"title":format!("Fixture {i:03}"),"type":"native","visible":true,"version":"1","folderPath":format!("/media/developer/apps/usr/palm/applications/{}",id(i)),"icon":"icon.png"})).collect::<Vec<_>>();
    let points = (0..count).map(|i| json!({"id":id(i)})).collect::<Vec<_>>();
    let map=crate::app_artwork::sources_from_raw(&json!({"replies":{"apps":{"returnValue":true,"apps":rows},"launch_points":{"returnValue":true,"launchPoints":points}}}),crate::identity::PREVIEW).unwrap();
    let mut shell = Shell::new(
        snapshot.clone(),
        Preferences {
            initialized: true,
            favourites: (0..count.min(12)).map(id).collect(),
            favourite_order: (0..count.min(12)).map(id).collect(),
        },
    );
    shell.enable_current_icon_cache();
    shell.open(Page::Applications);
    let update = Update {
        observed_ms: 100,
        simulated: false,
        system_keyboard: false,
        catalogue: Reading::Value(CatalogueReading {
            apps,
            launch_ids: snapshot.launch_ids,
        }),
        artwork: Reading::Value(map.clone()),
        inputs: Reading::Failed("omitted".into()),
        picture: Reading::Failed("omitted".into()),
        sound: Reading::Failed("omitted".into()),
        network: Reading::Failed("omitted".into()),
        controls: BTreeMap::new(),
    };
    shell.apply_live(&update, 100);
    shell.tick(100, 0.016);
    for card in &mut shell.surface.cards.items {
        card.texture = (77, 2.0, 1.0);
    }
    (shell, map)
}
fn stats() -> crate::packaged_icons::Stats {
    crate::packaged_icons::Stats {
        cache_readiness: crate::packaged_icons::CacheReadiness::Ready,
        cache_uploads: 1,
        cache_adoptions: 1,
        texture_entries: 1,
        ..Default::default()
    }
}
fn origin(
    map: &crate::app_artwork::SourceMap,
    id: &str,
) -> Option<(
    crate::packaged_icons::TextureView<u32>,
    crate::app_artwork::SourceKey,
)> {
    Some((
        crate::packaged_icons::TextureView {
            handle: 77,
            width: 2,
            height: 1,
        },
        map.get(id)?.preferred()?.key().clone(),
    ))
}
#[test]
fn eligible_metadata_preserves_old_schema_geometry_focus_and_all_authority_false() {
    let _serial = plx_base::testlock::serial();
    let (s, map) = fixture(1);
    let old = s.icon_presentation_cards();
    let focus = s.focused_item();
    let prefs = s.preferences().clone();
    let (page, value) = s
        .session_cache_frame_with(stats(), |id| origin(&map, id))
        .unwrap();
    assert_eq!(page, Page::Applications);
    let cards = value["cached_cards"].as_array().unwrap();
    assert_eq!(cards.len(), 1);
    assert_eq!(
        cards[0]["source_key"],
        json!(map[&id(0)].preferred().unwrap().key())
    );
    assert_eq!(cards[0]["rect"], old[0]["rect"]);
    assert_eq!(value["source_age_ms"], 0);
    for key in [
        "Root_authority_qualified",
        "positive_Root_producer_qualified",
        "native_foreground_scene_qualified",
        "physical_panel_qualified",
        "cleanup_qualified",
        "performance_qualified",
    ] {
        assert_eq!(value[key], false);
    }
    assert_eq!(s.icon_presentation_cards(), old);
    assert_eq!(s.focused_item(), focus);
    assert_eq!(s.preferences(), &prefs);
    assert!(s.artwork.is_none());
    assert!(s.session_cache_frame().is_none()); // No actual controller, synthetic fixture cannot grant a production receipt.
}
#[test]
fn fallback_handle_dimension_source_and_missing_map_mismatches_never_qualify() {
    let _serial = plx_base::testlock::serial();
    let (mut s, map) = fixture(1);
    assert!(s.session_cache_frame_with(stats(), |_| None).is_none());
    for bad in 0..3 {
        assert!(s
            .session_cache_frame_with(stats(), |id| {
                let (mut view, mut key) = origin(&map, id)?;
                match bad {
                    0 => view.handle += 1,
                    1 => view.width += 1,
                    _ => key.version = "2".into(),
                };
                Some((view, key))
            })
            .is_none());
    }
    assert!(s
        .session_cache_frame_with(crate::packaged_icons::Stats::default(), |id| origin(
            &map, id
        ))
        .is_none());
    s.current_icon_sources = None;
    assert!(s
        .session_cache_frame_with(stats(), |id| origin(&map, id))
        .is_none());
}
#[test]
fn disabled_stale_paused_keyboard_menu_and_wrong_page_are_observation_barriers() {
    let _serial = plx_base::testlock::serial();
    let (mut s, map) = fixture(1);
    s.current_icon_cache = false;
    assert!(s
        .session_cache_frame_with(stats(), |id| origin(&map, id))
        .is_none());
    s.current_icon_cache = true;
    s.live_now = 15_101;
    assert!(s
        .session_cache_frame_with(stats(), |id| origin(&map, id))
        .is_none());
    s.live_now = 100;
    for reason in [
        PauseReason::ForegroundLost,
        PauseReason::Idle,
        PauseReason::KeyboardIntent,
        PauseReason::KeyboardActive,
        PauseReason::ExternalIntent,
    ] {
        s.artwork_pause.set(reason, true);
        assert!(s
            .session_cache_frame_with(stats(), |id| origin(&map, id))
            .is_none());
        s.artwork_pause.set(reason, false);
    }
    s.drawer.editing = true;
    assert!(s
        .session_cache_frame_with(stats(), |id| origin(&map, id))
        .is_none());
    s.drawer.editing = false;
    s.menu = Some(AppMenu {
        id: id(0),
        origin: s.surface.cards.items[0].key,
        index: 0,
    });
    assert!(s
        .session_cache_frame_with(stats(), |id| origin(&map, id))
        .is_none());
    s.menu = None;
    s.open(Page::General);
    assert!(s
        .session_cache_frame_with(stats(), |id| origin(&map, id))
        .is_none());
}
#[test]
fn a_hundred_sources_emit_only_actual_drawn_clipped_cards_with_fixed_row_budget() {
    let _serial = plx_base::testlock::serial();
    let (s, map) = fixture(100);
    let visible = s.packaged_visible().into_iter().collect::<BTreeSet<_>>();
    let mut observed = stats();
    observed.texture_entries = crate::packaged_icons::MAX_TEXTURES;
    let (_, value) = s
        .session_cache_frame_with(observed, |id| origin(&map, id))
        .unwrap();
    let rows = value["cached_cards"].as_array().unwrap();
    assert!(!rows.is_empty());
    assert!(rows.len() <= crate::packaged_icons::MAX_TEXTURES);
    assert!(rows.len() < 100);
    for row in rows {
        assert!(visible.contains(row["id"].as_str().unwrap()));
        assert_ne!(row["id"], id(99));
    }
}
