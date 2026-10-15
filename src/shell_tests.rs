//! Behaviour checks for the real focus/press machines, without a GL context or TV actions.
use super::*;
fn fixture() -> Snapshot {
    let apps = (0..12)
        .map(|i| Application {
            id: format!("com.example.app{i:02}"),
            title: format!("Application {i:02}"),
            kind: "native".into(),
            visible: true,
            icon: None,
        })
        .collect::<Vec<_>>();
    Snapshot {
        captured_at: "fixture".into(),
        launch_ids: apps.iter().map(|a| a.id.clone()).collect(),
        applications: apps,
        inputs: vec![Input {
            id: "HDMI_1".into(),
            label: "HDMI 1".into(),
            plugged: true,
            signal: false,
            active: false,
            selected: None,
        }],
        context: Context {
            input: "HDMI_1".into(),
            dynamic_range: "sdr".into(),
            picture_mode: "cinema".into(),
        },
        settings: BTreeMap::new(),
    }
}
fn focus(shell: &Shell) -> u32 {
    shell
        .input
        .engine
        .current(InputOwner::Entry(entry(shell.page())))
        .unwrap()
        .elem
}
fn settle(shell: &mut Shell, ms: u32) {
    for t in (0..=ms).step_by(16) {
        shell.tick(t, 0.016);
    }
}
#[test]
fn navigation_uses_drawer_identity_and_remembers_each_page() {
    let _serial = plx_base::testlock::serial();
    let mut s = Shell::new(fixture(), Preferences::default());
    settle(&mut s, 100);
    let first = focus(&s);
    s.direction(Dir::Right);
    let second = focus(&s);
    assert_ne!(first, second);
    s.direction(Dir::Left);
    assert_eq!(focus(&s), first);
    s.direction(Dir::Left);
    assert_eq!(focus(&s), RAIL_BASE);
    s.direction(Dir::Down);
    s.activate(
        FocusKey {
            entry: entry(Page::Home),
            elem: focus(&s),
        },
        false,
    );
    assert_eq!(s.page(), Page::Applications);
    s.direction(Dir::Right);
    let app_focus = focus(&s);
    s.open(Page::Inputs);
    s.open(Page::Applications);
    assert_eq!(focus(&s), app_focus);
    s.back();
    assert_eq!(s.page(), Page::Home);
    s.back();
    assert!(s.should_exit());
    assert!(s.take_requests().is_empty());
}
#[test]
fn moving_focus_cancels_an_armed_application_launch() {
    let _serial = plx_base::testlock::serial();
    let mut s = Shell::new(fixture(), Preferences::default());
    s.press(true, PressFrom::Key);
    s.direction(Dir::Right);
    s.press(false, PressFrom::Key);
    settle(&mut s, 400);
    assert!(s.take_requests().is_empty());
}
#[test]
fn hold_opens_menu_without_toggling_or_launching_on_release() {
    let _serial = plx_base::testlock::serial();
    let mut s = Shell::new(fixture(), Preferences::default());
    s.take_preferences_changed();
    s.open(Page::Applications);
    let id = s.surface.cards.items[0].id.clone();
    assert!(s.preferences().favourites.contains(&id));
    s.press(true, PressFrom::Key);
    for ms in (0..=800).step_by(16) {
        s.tick(ms, 0.016);
        s.repeat();
    }
    s.press(false, PressFrom::Key);
    for ms in (816..1200).step_by(16) {
        s.tick(ms, 0.016);
    }
    assert!(s.preferences().favourites.contains(&id));
    assert_eq!(s.menu_application(), Some(id.as_str()));
    assert!(s.take_requests().is_empty());
    assert!(!s.take_preferences_changed());
    tap(&mut s);
    assert!(!s.preferences().favourites.contains(&id));
    assert!(s.take_requests().is_empty());
    assert!(s.take_preferences_changed());
}
#[test]
fn explicitly_empty_favourites_stay_empty_after_restart() {
    let _serial = plx_base::testlock::serial();
    let prefs = Preferences {
        initialized: true,
        favourites: BTreeSet::new(),
        favourite_order: Vec::new(),
    };
    let s = Shell::new(fixture(), prefs.clone());
    assert!(s.surface.cards.items.is_empty());
    assert_eq!(focus(&s), 500);
    let roundtrip: Preferences =
        serde_json::from_str(&serde_json::to_string(&prefs).unwrap()).unwrap();
    assert!(Shell::new(fixture(), roundtrip)
        .surface
        .cards
        .items
        .is_empty());
}
#[test]
fn inputs_are_typed_separately_from_application_launches() {
    let _serial = plx_base::testlock::serial();
    let mut s = Shell::new(fixture(), Preferences::default());
    s.enable_input_launches(true);
    s.open(Page::Inputs);
    s.activate(
        FocusKey {
            entry: entry(Page::Inputs),
            elem: 500,
        },
        false,
    );
    assert_eq!(
        s.take_requests(),
        vec![Action::RequestInput("HDMI_1".into())]
    );
    s.activate(
        FocusKey {
            entry: entry(Page::Home),
            elem: 500,
        },
        false,
    );
    assert!(s.take_requests().is_empty());
}
#[test]
fn empty_or_unregistered_catalogue_keeps_navigation_available() {
    let _serial = plx_base::testlock::serial();
    let mut fixture = fixture();
    fixture.launch_ids.clear();
    fixture.inputs.clear();
    let mut s = Shell::new(fixture, Preferences::default());
    assert_eq!(focus(&s), RAIL_BASE);
    s.direction(Dir::Down);
    assert_eq!(focus(&s), RAIL_BASE + 1);
    s.activate(
        FocusKey {
            entry: entry(Page::Home),
            elem: RAIL_BASE + 1,
        },
        false,
    );
    assert_eq!(s.page(), Page::Applications);
    assert_eq!(focus(&s), TOOLBAR_BASE);
    s.direction(Dir::Left);
    assert_eq!(focus(&s), RAIL_BASE + 1);
}

#[test]
fn app_picker_restores_registered_identity_from_a_remembered_rail_without_accepting_arbitrary_targets(
) {
    let _serial = plx_base::testlock::serial();
    let mut s = Shell::new(fixture(), Preferences::default());
    s.open(Page::Applications);
    s.direction(Dir::Left);
    s.direction(Dir::Down);
    s.open(Page::Inputs);
    s.open(Page::Applications);
    assert_eq!(focus(&s), RAIL_BASE + 2);
    assert!(!s.focus_application("com.example.gone"));
    assert_eq!(focus(&s), RAIL_BASE + 2);
    assert!(s.focus_application("com.example.app01"));
    assert_eq!(s.focused_item().as_deref(), Some("com.example.app01"));
    s.open(Page::Inputs);
    assert!(!s.focus_application("HDMI_1"));
    assert!(s.take_requests().is_empty());
}

fn live_update(ms: u32) -> crate::live::Update {
    use crate::live::{CatalogueReading, PictureReading, Reading, Update};
    let data = fixture();
    Update {
        observed_ms: ms,
        simulated: true,
        controls: BTreeMap::new(),
        catalogue: Reading::Value(CatalogueReading {
            apps: data.applications,
            launch_ids: data.launch_ids,
        }),
        inputs: Reading::Value(data.inputs),
        picture: Reading::Value(PictureReading {
            context: data.context,
            settings: Vec::new(),
        }),
        sound: Reading::Failed("Unavailable".into()),
        network: Reading::Failed("Unavailable".into()),
    }
}
#[test]
fn unchanged_live_refresh_preserves_an_armed_gesture_and_app_identity() {
    let _serial = plx_base::testlock::serial();
    let mut s = Shell::new(fixture(), Preferences::default());
    s.open(Page::Applications);
    s.tick(100, 0.016);
    s.apply_live(&live_update(100), 100);
    s.press(true, PressFrom::Key);
    s.apply_live(&live_update(110), 110);
    assert!(s.input.arm.is_some());
    s.tick(150, 0.016);
    s.press(false, PressFrom::Key);
    for ms in (160..560).step_by(16) {
        s.tick(ms, 0.016);
    }
    assert_eq!(
        s.take_requests(),
        vec![Action::RequestApplication("com.example.app00".into())]
    );
}
#[test]
fn rename_reorder_keeps_focus_and_favourites_but_removal_cancels_the_old_press() {
    let _serial = plx_base::testlock::serial();
    let mut s = Shell::new(fixture(), Preferences::default());
    s.open(Page::Applications);
    s.focus_application("com.example.app01");
    let prefs = s.preferences().clone();
    let mut update = live_update(10);
    let crate::live::Reading::Value(ref mut c) = update.catalogue else {
        unreachable!()
    };
    c.apps.reverse();
    c.apps
        .iter_mut()
        .find(|a| a.id == "com.example.app01")
        .unwrap()
        .title = "Zulu".into();
    s.apply_live(&update, 10);
    assert_eq!(s.focused_item().as_deref(), Some("com.example.app01"));
    assert_eq!(s.preferences(), &prefs);
    s.press(true, PressFrom::Key);
    let crate::live::Reading::Value(ref mut c) = update.catalogue else {
        unreachable!()
    };
    c.apps.retain(|a| a.id != "com.example.app01");
    c.launch_ids.remove("com.example.app01");
    s.apply_live(&update, 20);
    s.press(false, PressFrom::Key);
    settle(&mut s, 400);
    assert_ne!(s.focused_item().as_deref(), Some("com.example.app01"));
    assert!(s.take_requests().is_empty());
}
#[test]
fn failed_or_expired_live_catalogue_keeps_cards_but_cannot_request_launch() {
    let _serial = plx_base::testlock::serial();
    let mut s = Shell::new(fixture(), Preferences::default());
    s.open(Page::Applications);
    let mut update = live_update(0);
    s.apply_live(&update, 0);
    s.age_live(15001);
    let key = focus(&s);
    s.activate(
        FocusKey {
            entry: entry(Page::Applications),
            elem: key,
        },
        false,
    );
    assert!(s.take_requests().is_empty());
    update.catalogue = crate::live::Reading::Failed("No data".into());
    s.apply_live(&update, 15002);
    assert!(s.focused_item().is_some());
    s.activate(
        FocusKey {
            entry: entry(Page::Applications),
            elem: key,
        },
        false,
    );
    assert!(s.take_requests().is_empty());
}

#[test]
fn changed_picture_context_and_failed_audio_never_reuse_fresh_control_values() {
    let _serial = plx_base::testlock::serial();
    let mut data = fixture();
    let ctx = data.context.clone();
    data.settings.insert(
        "sound".into(),
        vec![Setting {
            constraint: None,
            id: "volume".into(),
            focus_key: 20,
            label: "Volume".into(),
            value: "10".into(),
            context: ctx,
            protected: false,
            available: true,
        }],
    );
    let mut s = Shell::new(data, Preferences::default());
    s.open(Page::Sound);
    s.apply_live(&live_update(100), 100);
    assert_eq!(s.surface.table.activate(0), None);
    assert_eq!(
        s.surface.table.table.sections[0].rows[0].value.as_deref(),
        Some("Unavailable")
    );
    let mut update = live_update(110);
    let crate::live::Reading::Value(ref mut picture) = update.picture else {
        unreachable!()
    };
    picture.context.dynamic_range = "hdr".into();
    picture.settings = vec![Setting {
        constraint: None,
        id: "brightness".into(),
        focus_key: 1,
        label: "Brightness".into(),
        value: "70".into(),
        context: picture.context.clone(),
        protected: false,
        available: true,
    }];
    s.open(Page::Picture);
    s.apply_live(&update, 110);
    assert_eq!(s.snapshot.context.dynamic_range, "hdr");
    assert_eq!(
        s.surface.table.table.sections[0].rows[0].value.as_deref(),
        Some("70")
    );
    s.age_live(15111);
    assert_eq!(s.surface.table.activate(0), None);
    assert_eq!(
        s.surface.table.table.sections[0].rows[0].value.as_deref(),
        Some("Unavailable")
    );
}
#[test]
fn sound_and_network_failures_withdraw_rows_and_staleness_rebuilds_the_visible_form() {
    let _serial = plx_base::testlock::serial();
    let mut shell = Shell::new(fixture(), Preferences::default());
    let mut update = live_update(100);
    update.sound = crate::live::Reading::Value(vec![
        ("soundOutput".into(), "external_optical".into()),
        ("volume".into(), "10".into()),
        ("muteStatus".into(), "false".into()),
    ]);
    update.network = crate::live::Reading::Value(vec![
        ("internet".into(), "Available".into()),
        ("wiredState".into(), "Connected".into()),
        ("wifiState".into(), "Disconnected".into()),
    ]);
    shell.apply_live(&update, 100);
    shell.open(Page::Sound);
    assert_eq!(
        shell.surface.table.table.sections[0].rows[1]
            .value
            .as_deref(),
        Some("10")
    );
    assert_eq!(
        shell.surface.table.table.sections[0].rows[3]
            .value
            .as_deref(),
        Some("Unavailable")
    );
    shell.open(Page::Network);
    assert_eq!(
        shell.surface.table.table.sections[0].rows[1]
            .value
            .as_deref(),
        Some("Connected")
    );
    shell.age_live(15101);
    assert_eq!(
        shell.surface.table.table.sections[0].rows[1]
            .value
            .as_deref(),
        Some("Unavailable")
    );
    update.observed_ms = 15200;
    update.network = crate::live::Reading::Failed("Denied".into());
    shell.apply_live(&update, 15200);
    assert!(shell.snapshot.settings["network"]
        .iter()
        .all(|row| !row.available));
}

#[test]
fn explicit_system_routes_use_registered_surface_keys() {
    let _serial = plx_base::testlock::serial();
    let mut s = Shell::new(fixture(), Preferences::default());
    assert!(s.focus_input("HDMI_1"));
    assert!(!s.focus_lg_settings());
    s.open(Page::Inputs);
    assert!(!s.focus_input("HDMI_99"));
    assert!(s.focus_input("HDMI_1"));
    assert_eq!(focus(&s), 500);
    s.open(Page::Picture);
    assert!(s.focus_lg_settings());
    assert_eq!(focus(&s), TABLE_BASE + 1000);
    s.activate(
        FocusKey {
            entry: entry(Page::Picture),
            elem: focus(&s),
        },
        false,
    );
    assert_eq!(s.take_requests(), vec![Action::OpenLgSettings]);
}

#[test]
fn home_shelves_cross_by_geometry_and_keep_input_requests_out_of_favourites() {
    let _serial = plx_base::testlock::serial();
    let mut data = fixture();
    data.inputs = (1..=4)
        .map(|i| Input {
            id: format!("HDMI_{i}"),
            label: format!("HDMI {i}"),
            plugged: false,
            signal: false,
            active: false,
            selected: None,
        })
        .collect();
    let mut s = Shell::new(data, Preferences::default());
    s.enable_input_launches(true);
    let first = s.focused_item();
    s.direction(Dir::Right);
    s.direction(Dir::Right);
    let chosen = s.focused_item();
    assert_ne!(chosen, first);
    s.direction(Dir::Down);
    assert_eq!(s.focused_item().as_deref(), Some("HDMI_3"));
    let favourites = s.preferences().clone();
    s.press(true, PressFrom::Key);
    for ms in (0..800).step_by(16) {
        s.tick(ms, 0.016);
        s.repeat();
    }
    assert_eq!(s.preferences(), &favourites);
    s.press(false, PressFrom::Key);
    for ms in (816..1100).step_by(16) {
        s.tick(ms, 0.016);
    }
    assert_eq!(
        s.take_requests(),
        vec![Action::RequestInput("HDMI_3".into())]
    );
    s.direction(Dir::Up);
    assert_eq!(s.focused_item().as_deref(), Some("View all inputs"));
    s.direction(Dir::Up);
    // Leaving the right-aligned section action lands on the nearest favourite card.
    assert_eq!(s.focused_item().as_deref(), Some("com.example.app03"));
    s.direction(Dir::Left);
    s.direction(Dir::Left);
    s.direction(Dir::Left);
    assert_eq!(s.focused_item(), first);
}

#[test]
fn home_updates_inputs_by_identity_without_turning_old_input_readings_into_actions() {
    let _serial = plx_base::testlock::serial();
    let mut s = Shell::new(fixture(), Preferences::default());
    assert!(s.focus_input("HDMI_1"));
    let mut update = Update {
        observed_ms: 100,
        inputs: crate::live::Reading::Value(vec![Input {
            id: "HDMI_1".into(),
            label: "Console".into(),
            plugged: true,
            signal: true,
            active: false,
            selected: None,
        }]),
        ..live_update(100)
    };
    s.apply_live(&update, 100);
    assert_eq!(s.focused_item().as_deref(), Some("HDMI_1"));
    assert_eq!(
        s.surface.input_cards.items[0].title.to_str().unwrap(),
        "Console"
    );
    s.press(true, PressFrom::Key);
    update.observed_ms = 200;
    update.inputs = crate::live::Reading::Failed("Unavailable".into());
    s.apply_live(&update, 200);
    s.press(false, PressFrom::Key);
    settle(&mut s, 500);
    assert!(s.take_requests().is_empty());
    s.press(true, PressFrom::Key);
    s.press(false, PressFrom::Key);
    settle(&mut s, 500);
    assert!(s.take_requests().is_empty());
}

#[test]
fn home_focus_springs_report_motion_then_go_quiet_at_rest() {
    let _serial = plx_base::testlock::serial();
    let previous = plx_machine::idle::enabled();
    plx_machine::idle::set_enabled(true);
    let mut s = Shell::new(fixture(), Preferences::default());
    settle(&mut s, 2000);
    s.direction(Dir::Right);
    let (_, moving) = plx_machine::idle::scoped_motion(|| s.tick(2016, 0.016));
    assert!(moving, "deliberate shelf focus must keep animating");
    for ms in (2032..6000).step_by(16) {
        s.tick(ms, 0.016);
    }
    let (_, moving) = plx_machine::idle::scoped_motion(|| s.tick(6016, 0.016));
    assert!(!moving, "settled shelves must stop reporting spring motion");
    plx_machine::idle::set_enabled(previous);
}

fn tap(s: &mut Shell) {
    let start = s.now;
    s.press(true, PressFrom::Key);
    s.tick(start + 50, 0.05);
    s.press(false, PressFrom::Key);
    for ms in (start + 66..start + 500).step_by(16) {
        s.tick(ms, 0.016);
    }
}
#[test]
fn drawer_favourites_filter_is_local_and_unpin_keeps_valid_focus() {
    let _serial = plx_base::testlock::serial();
    let mut s = Shell::new(fixture(), Preferences::default());
    s.open(Page::Applications);
    assert!(s.focus_drawer_control(1));
    tap(&mut s);
    assert!(s.drawer.favourites_only);
    assert_eq!(s.surface.cards.len(), 8);
    assert!(s.focus_application("com.example.app00"));
    s.activate(
        FocusKey {
            entry: entry(Page::Applications),
            elem: focus(&s),
        },
        true,
    );
    assert_eq!(s.menu_application(), Some("com.example.app00"));
    tap(&mut s);
    s.back();
    assert_eq!(s.surface.cards.len(), 7);
    assert_ne!(s.focused_item().as_deref(), Some("com.example.app00"));
    assert!(s.take_requests().is_empty());
    assert!(s.focus_drawer_control(0));
    tap(&mut s);
    assert_eq!(s.surface.cards.len(), 12);
}
#[test]
fn native_search_keyboard_commits_once_and_back_restores_search_control() {
    let _serial = plx_base::testlock::serial();
    let mut s = Shell::new(fixture(), Preferences::default());
    s.open(Page::Applications);
    assert!(s.focus_drawer_control(2));
    tap(&mut s);
    assert!(s.drawer.editing);
    assert!(!s.focus_application("com.example.app00"));
    for label in ["A", "P", "P", "0", "1"] {
        assert!(s.focus_search_key(KEYS.iter().position(|k| *k == label).unwrap()));
        tap(&mut s);
    }
    assert_eq!(s.drawer.query, "APP01");
    assert_eq!(s.surface.cards.len(), 0); // word matching, not arbitrary application ID matching
    assert!(s.take_requests().is_empty());
    s.back();
    assert!(!s.drawer.editing);
    assert_eq!(s.focused_item().as_deref(), Some("Search apps"));
    assert_eq!(s.page(), Page::Applications);
    assert!(s.focus_drawer_control(2));
    tap(&mut s);
    assert!(s.focus_search_key(38));
    tap(&mut s);
    assert!(s.drawer.query.is_empty());
    assert_eq!(s.surface.cards.len(), 12);
    s.back();
    s.back();
    assert_eq!(s.page(), Page::Home);
}
#[test]
fn search_key_movement_cancels_an_armed_character_and_live_refresh_keeps_keyboard_identity() {
    let _serial = plx_base::testlock::serial();
    let mut s = Shell::new(fixture(), Preferences::default());
    s.open(Page::Applications);
    s.focus_drawer_control(2);
    tap(&mut s);
    s.focus_search_key(0);
    s.press(true, PressFrom::Key);
    s.direction(Dir::Right);
    s.press(false, PressFrom::Key);
    let now = s.now;
    for ms in (now + 16..now + 500).step_by(16) {
        s.tick(ms, 0.016);
    }
    assert!(s.drawer.query.is_empty());
    let before = s.focused_item();
    s.apply_live(&live_update(100), 100);
    assert!(s.drawer.editing);
    assert_eq!(s.focused_item(), before);
    assert!(s.take_requests().is_empty());
}

#[test]
fn menu_is_modal_cancels_press_on_back_and_restores_app_identity() {
    let _serial = plx_base::testlock::serial();
    let mut s = Shell::new(fixture(), Preferences::default());
    s.take_preferences_changed();
    s.open(Page::Applications);
    assert!(s.focus_application("com.example.app03"));
    let origin = focus(&s);
    s.activate(
        FocusKey {
            entry: entry(s.page()),
            elem: origin,
        },
        true,
    );
    for _ in 0..8 {
        s.direction(Dir::Left);
    }
    assert_eq!(focus(&s), MENU_BASE + 1);
    assert!(!s.focus_application("com.example.app04"));
    assert!(!s.focus_drawer_control(2));
    s.press(true, PressFrom::Key);
    s.back();
    s.press(false, PressFrom::Key);
    settle(&mut s, 600);
    assert_eq!(focus(&s), origin);
    assert!(s.take_requests().is_empty());
    assert!(!s.take_preferences_changed());
}
#[test]
fn ordering_survives_serialization_and_changes_only_favourite_views() {
    let _serial = plx_base::testlock::serial();
    let mut s = Shell::new(fixture(), Preferences::default());
    s.open(Page::Applications);
    assert!(s.focus_application("com.example.app01"));
    s.activate(
        FocusKey {
            entry: entry(s.page()),
            elem: focus(&s),
        },
        true,
    );
    assert!(s.focus_menu_action(MenuAction::Earlier));
    tap(&mut s);
    s.back();
    assert_eq!(s.first_application().as_deref(), Some("com.example.app00"));
    assert_eq!(s.focused_item().as_deref(), Some("com.example.app01"));
    assert!(s.take_requests().is_empty());
    let saved = serde_json::from_str(&serde_json::to_string(s.preferences()).unwrap()).unwrap();
    let mut restarted = Shell::new(fixture(), saved);
    assert_eq!(
        restarted.first_application().as_deref(),
        Some("com.example.app01")
    );
    restarted.open(Page::Applications);
    restarted.focus_drawer_control(1);
    tap(&mut restarted);
    assert_eq!(
        restarted.first_application().as_deref(),
        Some("com.example.app01")
    );
    assert_eq!(
        restarted.preferences().favourites,
        s.preferences().favourites
    );
}
#[test]
fn menu_refresh_renames_target_but_removal_cancels_armed_action() {
    let _serial = plx_base::testlock::serial();
    let mut s = Shell::new(fixture(), Preferences::default());
    s.take_preferences_changed();
    s.open(Page::Applications);
    s.activate(
        FocusKey {
            entry: entry(s.page()),
            elem: focus(&s),
        },
        true,
    );
    let id = s.menu_application().unwrap().to_owned();
    let before = s.snapshot.clone();
    s.snapshot
        .applications
        .iter_mut()
        .find(|a| a.id == id)
        .unwrap()
        .title = "Renamed app".into();
    s.catalogue
        .refresh(&s.snapshot.applications, &s.snapshot.launch_ids);
    s.refresh_shown_data(&before);
    assert_eq!(s.menu_application(), Some(id.as_str()));
    assert_eq!(focus(&s), MENU_BASE + 1);
    s.press(true, PressFrom::Key);
    let before = s.snapshot.clone();
    s.snapshot.applications.retain(|a| a.id != id);
    s.snapshot.launch_ids.remove(&id);
    s.catalogue
        .refresh(&s.snapshot.applications, &s.snapshot.launch_ids);
    s.refresh_shown_data(&before);
    s.press(false, PressFrom::Key);
    settle(&mut s, 600);
    assert!(s.menu_application().is_none());
    assert!(s.take_requests().is_empty());
    assert!(!s.take_preferences_changed());
    assert!(s.preferences().favourites.contains(&id));
}
#[test]
fn disabled_menu_actions_and_order_bounds_make_no_request_or_write() {
    let _serial = plx_base::testlock::serial();
    let mut s = Shell::new(fixture(), Preferences::default());
    s.take_preferences_changed();
    s.open(Page::Applications);
    s.activate(
        FocusKey {
            entry: entry(s.page()),
            elem: focus(&s),
        },
        true,
    );
    for action in [MenuAction::Open, MenuAction::Earlier] {
        s.focus_menu_action(action);
        tap(&mut s);
    }
    assert!(s.take_requests().is_empty());
    assert!(!s.take_preferences_changed());
    assert!(s.preferences().favourite_order.is_empty());
    s.back();
    s.focus_application("com.example.app11");
    s.activate(
        FocusKey {
            entry: entry(s.page()),
            elem: focus(&s),
        },
        true,
    );
    s.focus_menu_action(MenuAction::Later);
    tap(&mut s);
    assert!(!s.take_preferences_changed());
}

#[test]
fn menu_open_uses_registered_identity_and_rejects_stale_catalogue() {
    let _serial = plx_base::testlock::serial();
    let mut s = Shell::new(fixture(), Preferences::default());
    s.enable_app_launches(true);
    s.open(Page::Applications);
    s.activate(
        FocusKey {
            entry: entry(s.page()),
            elem: focus(&s),
        },
        true,
    );
    s.focus_menu_action(MenuAction::Open);
    tap(&mut s);
    assert_eq!(
        s.take_requests(),
        vec![Action::RequestApplication("com.example.app00".into())]
    );
    assert!(s.menu_application().is_none());
    s.activate(
        FocusKey {
            entry: entry(s.page()),
            elem: focus(&s),
        },
        true,
    );
    s.live_state = Some(Freshness::default());
    s.live_now = 16_000;
    s.focus_menu_action(MenuAction::Open);
    tap(&mut s);
    assert!(s.take_requests().is_empty());
    assert!(s.menu_application().is_some());
}

fn controls_fixture() -> Snapshot {
    let mut data = fixture();
    for (domain, keys) in [
        ("picture", vec![("pictureMode", 1), ("backlight", 2)]),
        ("sound", vec![("soundOutput", 20), ("volume", 21)]),
        (
            "network",
            vec![("internet", 60), ("wiredState", 61), ("wifiState", 62)],
        ),
    ] {
        let context = data.context.clone();
        data.settings.insert(
            domain.into(),
            keys.into_iter()
                .map(|(id, key)| Setting {
                    constraint: None,
                    id: id.into(),
                    focus_key: key,
                    label: id.into(),
                    value: "fixture".into(),
                    context: context.clone(),
                    available: true,
                    protected: false,
                })
                .collect(),
        );
    }
    data
}
#[test]
fn controls_master_detail_uses_category_identity_and_back_hierarchy() {
    let _serial = plx_base::testlock::serial();
    let mut s = Shell::new(controls_fixture(), Preferences::default());
    s.open(Page::Controls);
    assert_eq!(focus(&s), CATEGORY_BASE);
    assert_eq!(s.control_category(), Some(Page::Picture));
    s.direction(Dir::Down);
    assert_eq!(focus(&s), CATEGORY_BASE + 1);
    assert_eq!(s.control_category(), Some(Page::Sound));
    assert_eq!(s.surface.table.key_at(0), Some(RowKey(20)));
    s.direction(Dir::Right);
    assert_eq!(focus(&s), TABLE_BASE + 20);
    s.back();
    assert_eq!(focus(&s), CATEGORY_BASE + 1);
    assert_eq!(s.control_category(), Some(Page::Sound));
    s.direction(Dir::Left);
    assert_eq!(focus(&s), RAIL_BASE + 3);
    s.direction(Dir::Right);
    assert_eq!(focus(&s), CATEGORY_BASE + 1);
    s.back();
    assert_eq!(s.page(), Page::Home);
    assert!(s.take_requests().is_empty());
}
#[test]
fn category_movement_cancels_armed_picture_and_routes_each_category_to_its_typed_section() {
    let _serial = plx_base::testlock::serial();
    let mut s = Shell::new(controls_fixture(), Preferences::default());
    s.open(Page::Picture);
    s.focus_lg_settings();
    s.press(true, PressFrom::Key);
    s.direction(Dir::Left);
    s.direction(Dir::Down);
    s.press(false, PressFrom::Key);
    settle(&mut s, 600);
    assert_eq!(s.control_category(), Some(Page::Sound));
    assert!(s.take_requests().is_empty());
    for (index, (page, _)) in CATEGORIES.iter().enumerate().skip(1) {
        s.focus_control_category(index);
        assert!(s.focus_lg_settings());
        tap(&mut s);
        assert_eq!(
            s.take_requests(),
            vec![Action::OpenLgSettingsSection(
                SettingsSection::from_page(*page).unwrap()
            )]
        );
    }
}
#[test]
fn empty_native_categories_offer_only_the_lg_managed_route_and_keep_navigation() {
    let _serial = plx_base::testlock::serial();
    let mut s = Shell::new(controls_fixture(), Preferences::default());
    s.open(Page::Controls);
    s.focus_control_category(5);
    let before = s.preferences().clone();
    s.direction(Dir::Down);
    assert_eq!(focus(&s), CATEGORY_BASE + 5);
    s.direction(Dir::Right);
    assert_eq!(
        focus(&s),
        TABLE_BASE + SettingsSection::PanelCare.focus_key()
    );
    tap(&mut s);
    assert_eq!(
        focus(&s),
        TABLE_BASE + SettingsSection::PanelCare.focus_key()
    );
    assert_eq!(s.control_category(), Some(Page::PanelCare));
    assert_eq!(
        s.take_requests(),
        vec![Action::OpenLgSettingsSection(SettingsSection::PanelCare)]
    );
    assert_eq!(s.preferences(), &before);
    s.back();
    assert_eq!(focus(&s), CATEGORY_BASE + 5);
    s.back();
    assert_eq!(s.page(), Page::Home);
}
#[test]
fn controls_root_rebuilds_live_selected_detail_without_reusing_stale_values() {
    let _serial = plx_base::testlock::serial();
    let mut s = Shell::new(controls_fixture(), Preferences::default());
    s.open(Page::Controls);
    s.focus_control_category(2);
    let mut update = crate::live::Update {
        observed_ms: 100,
        network: crate::live::Reading::Value(vec![
            ("internet".into(), "Available".into()),
            ("wiredState".into(), "Connected".into()),
            ("wifiState".into(), "Disconnected".into()),
        ]),
        ..live_update(100)
    };
    s.apply_live(&update, 100);
    s.direction(Dir::Right);
    s.direction(Dir::Down);
    let selected = focus(&s);
    assert_eq!(s.control_category(), Some(Page::Network));
    assert_eq!(s.page(), Page::Controls);
    s.age_live(15_101);
    assert_eq!(focus(&s), selected);
    assert_eq!(
        s.surface.table.table.sections[0].rows[1].value.as_deref(),
        Some("Unavailable")
    );
    assert_eq!(s.surface.table.activate(1), None);
    update.observed_ms = 15_200;
    update.network = crate::live::Reading::Failed("Denied".into());
    s.apply_live(&update, 15_200);
    assert_eq!(s.control_category(), Some(Page::Network));
    assert_eq!(focus(&s), selected);
}

#[test]
fn picture_breadcrumb_expires_even_if_no_setting_row_can_trigger_a_rebuild() {
    let _serial = plx_base::testlock::serial();
    let mut s = Shell::new(fixture(), Preferences::default());
    s.open(Page::Controls);
    let mut update = live_update(100);
    update.picture = live::Reading::Value(live::PictureReading {
        context: s.snapshot.context.clone(),
        settings: vec![],
    });
    s.apply_live(&update, 100);
    assert!(s.control_breadcrumb.to_str().unwrap().contains("HDMI_1"));
    s.age_live(15101);
    assert_eq!(
        s.control_breadcrumb.to_str().unwrap(),
        "Picture context unavailable"
    );
}

#[test]
fn stale_pointer_frame_cannot_leave_focus_on_a_removed_detail_row_or_escape_a_new_menu() {
    let _serial = plx_base::testlock::serial();
    let mut s = Shell::new(controls_fixture(), Preferences::default());
    s.open(Page::Controls);
    s.focus_control_category(2);
    let stale = FocusKey {
        entry: entry(s.page()),
        elem: TABLE_BASE + 61,
    };
    let rect = Rect::new(1000.0, 650.0, 300.0, 60.0);
    s.input.hit.fill(vec![Stop {
        key: stale,
        rect,
        rest_rect: rect,
        clip: DETAIL_ROWS,
        hover: Hover::Focus,
        activate: Activate::Press,
    }]);
    s.input.hit.swap();
    s.focus_control_category(0);
    s.pointer(1100.0, 660.0, true);
    assert_eq!(focus(&s), CATEGORY_BASE);
    s.direction(Dir::Right);
    assert_eq!(focus(&s), TABLE_BASE + 1);
    assert!(s.take_requests().is_empty());
    s.open(Page::Applications);
    let app = FocusKey {
        entry: entry(s.page()),
        elem: focus(&s),
    };
    s.input.hit.fill(vec![Stop {
        key: app,
        rect,
        rest_rect: rect,
        clip: CONTENT,
        hover: Hover::Focus,
        activate: Activate::Press,
    }]);
    s.input.hit.swap();
    s.activate(app, true);
    s.pointer(1100.0, 660.0, true);
    assert_eq!(focus(&s), MENU_BASE + 1);
    assert!(s.menu_application().is_some());
    assert!(s.take_requests().is_empty());
}

#[test]
fn a_previous_category_settings_button_cannot_invoke_the_new_category_route() {
    let _serial = plx_base::testlock::serial();
    let mut s = Shell::new(controls_fixture(), Preferences::default());
    s.open(Page::Controls);
    let rect = Rect::new(1000.0, 650.0, 300.0, 60.0);
    s.input.hit.fill(vec![Stop {
        key: FocusKey {
            entry: entry(Page::Controls),
            elem: TABLE_BASE + 1000,
        },
        rect,
        rest_rect: rect,
        clip: DETAIL_ROWS,
        hover: Hover::Focus,
        activate: Activate::Press,
    }]);
    s.input.hit.swap();
    s.focus_control_category(1);
    s.pointer(1100.0, 660.0, true);
    assert_eq!(focus(&s), CATEGORY_BASE + 1);
    assert!(s.take_requests().is_empty());
}

#[test]
fn two_column_inputs_preserve_port_identity_and_withdraw_selected_status_when_stale() {
    let _serial = plx_base::testlock::serial();
    let mut data = fixture();
    data.inputs = (1..=4)
        .map(|port| Input {
            id: format!("HDMI_{port}"),
            label: format!("Port {port}"),
            plugged: false,
            signal: false,
            active: false,
            selected: Some(port == 4),
        })
        .collect();
    let mut s = Shell::new(data.clone(), Preferences::default());
    s.open(Page::Inputs);
    s.direction(Dir::Right);
    assert_eq!(s.focused_item().as_deref(), Some("HDMI_2"));
    s.direction(Dir::Down);
    assert_eq!(s.focused_item().as_deref(), Some("HDMI_4"));
    let update = Update {
        inputs: live::Reading::Value(data.inputs),
        ..live_update(100)
    };
    s.apply_live(&update, 100);
    s.age_live(15101);
    assert!(s.snapshot.inputs.iter().all(|p| p.selected.is_none()));
    assert_eq!(s.focused_item().as_deref(), Some("HDMI_4"));
}

#[test]
fn lg_home_rail_is_reachable_and_uses_a_distinct_gated_request_without_changing_preferences() {
    let _serial = plx_base::testlock::serial();
    let mut s = Shell::new(fixture(), Preferences::default());
    let before = s.preferences().clone();
    assert!(s.focus_lg_home());
    assert_eq!(focus(&s), LG_HOME_KEY);
    let key = FocusKey {
        entry: entry(s.page()),
        elem: LG_HOME_KEY,
    };
    s.activate(key, false);
    assert!(s.take_requests().is_empty());
    s.direction(Dir::Up);
    assert_eq!(focus(&s), RAIL_BASE + 3);
    s.direction(Dir::Down);
    assert_eq!(focus(&s), LG_HOME_KEY);
    s.enable_home_launches(true);
    s.press(true, PressFrom::Key);
    s.tick(50, 0.05);
    s.press(false, PressFrom::Key);
    settle(&mut s, 300);
    assert_eq!(s.take_requests(), vec![Action::OpenLgHome]);
    assert_eq!(s.preferences(), &before);
    assert_eq!(s.page(), Page::Home);
    assert!(!s.should_exit());
}
#[test]
fn armed_lg_home_return_is_cancelled_by_navigation_and_hidden_behind_search_or_menu() {
    let _serial = plx_base::testlock::serial();
    let mut s = Shell::new(fixture(), Preferences::default());
    s.enable_home_launches(true);
    s.focus_lg_home();
    s.press(true, PressFrom::Key);
    s.direction(Dir::Up);
    s.press(false, PressFrom::Key);
    settle(&mut s, 300);
    assert!(s.take_requests().is_empty());
    s.open(Page::Applications);
    let key = FocusKey {
        entry: entry(s.page()),
        elem: LG_HOME_KEY,
    };
    s.drawer.editing = true;
    s.rebuild(Page::Applications);
    assert!(!s.focus_lg_home());
    s.activate(key, false);
    assert!(s.take_requests().is_empty());
    s.back();
    s.focus_application("com.example.app00");
    let app = FocusKey {
        entry: entry(s.page()),
        elem: focus(&s),
    };
    s.activate(app, true);
    assert!(s.menu_application().is_some());
    assert!(!s.focus_lg_home());
    s.activate(key, false);
    assert!(s.take_requests().is_empty());
    assert!(s.menu_application().is_some());
}

#[test]
fn new_global_native_pages_rebuild_on_updates_and_expiry_without_emitting_a_setting_write() {
    let _serial = plx_base::testlock::serial();
    let mut s = Shell::new(fixture(), Preferences::default());
    let data = crate::live::tests::global_raw();
    s.apply_live(
        &crate::live::parse(&data, "com.example.self", 100, false),
        100,
    );
    for page in [Page::Accessibility, Page::General, Page::PanelCare] {
        s.open(page);
        let first = s.surface.table.table.sections[0].rows[0].value.clone();
        assert_eq!(first.as_deref(), Some("On"));
        assert!(!s.surface.table.table.sections[0].rows[0].dim);
        assert!(s.surface.table.activate(0).is_none());
        let owner = InputOwner::Entry(entry(s.page()));
        let before = s.input.engine.current(owner);
        s.age_live(15_101);
        assert_eq!(
            s.surface.table.table.sections[0].rows[0].value.as_deref(),
            Some("Unavailable")
        );
        assert_eq!(s.input.engine.current(owner), before);
        assert!(s.surface.table.table.sections[0].rows[0].dim);
        assert!(s.take_requests().is_empty());
        s.apply_live(
            &crate::live::parse(&data, "com.example.self", 15_200, false),
            15_200,
        );
    }
}

fn pointer_data() -> serde_json::Value {
    let mut raw = crate::live::tests::global_raw();
    raw["replies"]["general_option_values"]["settings"]["pointerSpeed"] = "normal".into();
    for d in raw["replies"]["general_option_desc"]["results"]
        .as_array_mut()
        .unwrap()
    {
        if d["key"] == "pointerSpeed" {
            d["values"]["arrayExt"] = serde_json::json!(crate::pointer_speed::Speed::ALL
                .map(|s| serde_json::json!({"value":s.wire(),"visible":true,"active":true})));
        }
    }
    raw
}
fn pointer_shell() -> Shell {
    let mut s = Shell::new(fixture(), Preferences::default());
    s.apply_live(
        &crate::live::parse(&pointer_data(), "com.example.self", 100, false),
        100,
    );
    s.enable_pointer_editor(true);
    s.open(Page::General);
    assert!(s.focus_pointer_setting());
    s
}
#[test]
fn native_pointer_picker_only_commits_on_ok_and_back_or_movement_cancel_old_gestures() {
    use crate::pointer_speed::{Change, Speed};
    let _serial = plx_base::testlock::serial();
    let mut s = pointer_shell();
    let original = s.preferences().clone();
    s.activate(
        FocusKey {
            entry: entry(s.page()),
            elem: focus(&s),
        },
        false,
    );
    assert_eq!(s.page(), Page::PointerSpeed);
    assert_eq!(focus(&s), TABLE_BASE + 1501);
    assert!(s.surface.table.table.sections[0].rows[1].checked);
    s.focus_pointer_choice(Speed::Slow);
    assert!(s.take_requests().is_empty());
    s.press(true, PressFrom::Key);
    s.direction(Dir::Down);
    s.press(false, PressFrom::Key);
    settle(&mut s, 300);
    assert!(s.take_requests().is_empty());
    s.back();
    assert_eq!(s.page(), Page::General);
    assert_eq!(focus(&s), TABLE_BASE + 141);
    assert_eq!(s.preferences(), &original);
    s.activate(
        FocusKey {
            entry: entry(s.page()),
            elem: focus(&s),
        },
        false,
    );
    s.focus_pointer_choice(Speed::Slow);
    s.activate(
        FocusKey {
            entry: entry(s.page()),
            elem: focus(&s),
        },
        false,
    );
    assert_eq!(
        s.take_requests(),
        vec![Action::SetPointerSpeed(Change {
            expected: Speed::Normal,
            value: Speed::Slow
        })]
    );
    assert!(s.pointer_pending);
    s.back();
    s.pointer(200., 280., true);
    assert_eq!(s.page(), Page::PointerSpeed);
    assert!(s.take_requests().is_empty());
    s.pointer_reply(
        &crate::pointer_speed::Outcome {
            change: Change {
                expected: Speed::Normal,
                value: Speed::Slow,
            },
            attempted: true,
            admitted: true,
            verified: true,
            detail: String::new(),
        },
        350,
    );
    assert_eq!(s.page(), Page::General);
    assert_eq!(s.pointer_value(), Some(Speed::Slow));
    assert_eq!(s.preferences(), &original);
}
#[test]
fn native_pointer_picker_withdraws_on_stale_metadata_and_rejects_previous_page_hits() {
    use crate::pointer_speed::Speed;
    let _serial = plx_base::testlock::serial();
    let mut s = pointer_shell();
    s.activate(
        FocusKey {
            entry: entry(s.page()),
            elem: focus(&s),
        },
        false,
    );
    s.focus_pointer_choice(Speed::Slow);
    let stale = FocusKey {
        entry: entry(s.page()),
        elem: focus(&s),
    };
    s.press(true, PressFrom::Key);
    s.age_live(15_101);
    s.press(false, PressFrom::Key);
    settle(&mut s, 300);
    assert_eq!(s.page(), Page::General);
    s.activate(stale, false);
    assert!(s.take_requests().is_empty());
    assert!(s.pointer_value().is_none());
}
#[test]
fn confirmed_pointer_value_cannot_be_rewound_by_a_read_cycle_started_before_the_commit() {
    use crate::pointer_speed::{Change, Outcome, Speed};
    let _serial = plx_base::testlock::serial();
    let mut s = pointer_shell();
    s.pointer_reply(
        &Outcome {
            change: Change {
                expected: Speed::Normal,
                value: Speed::Slow,
            },
            attempted: true,
            admitted: true,
            verified: true,
            detail: String::new(),
        },
        200,
    );
    s.apply_live(
        &crate::live::parse(&pointer_data(), "com.example.self", 150, false),
        220,
    );
    assert_eq!(s.pointer_value(), Some(Speed::Slow));
    let mut changed = pointer_data();
    changed["replies"]["general_option_values"]["settings"]["pointerSpeed"] = "fast".into();
    s.apply_live(
        &crate::live::parse(&changed, "com.example.self", 250, false),
        260,
    );
    assert_eq!(s.pointer_value(), Some(Speed::Fast));
}

#[test]
fn input_capability_is_required_and_stale_input_state_cannot_be_activated() {
    let _serial = plx_base::testlock::serial();
    let mut s = Shell::new(fixture(), Preferences::default());
    s.open(Page::Inputs);
    s.focus_input("HDMI_1");
    let key = FocusKey {
        entry: entry(s.page()),
        elem: focus(&s),
    };
    s.activate(key, false);
    assert!(s.take_requests().is_empty());
    s.enable_input_launches(true);
    s.apply_live(&live_update(100), 100);
    s.age_live(15_101);
    s.activate(key, false);
    assert!(s.take_requests().is_empty());
}

#[test]
fn home_view_all_routes_clear_hidden_filters_and_return_to_their_own_shortcut() {
    let _serial = plx_base::testlock::serial();
    let mut s = Shell::new(fixture(), Preferences::default());
    let prefs = s.preferences().clone();
    s.drawer.query = "unmatched".into();
    s.drawer.favourites_only = true;
    assert!(s.focus_home_link(0));
    s.activate(
        FocusKey {
            entry: entry(s.page()),
            elem: focus(&s),
        },
        false,
    );
    assert_eq!(s.page(), Page::Applications);
    assert_eq!(s.application_count(), 12);
    assert!(!s.drawer.favourites_only && s.drawer.query.is_empty());
    s.focus_drawer_control(2);
    s.activate(
        FocusKey {
            entry: entry(s.page()),
            elem: focus(&s),
        },
        false,
    );
    assert!(s.drawer.editing);
    s.back();
    assert_eq!(s.page(), Page::Applications);
    s.back();
    assert_eq!(s.page(), Page::Home);
    assert_eq!(focus(&s), HOME_APPS_LINK);
    s.focus_home_link(1);
    s.activate(
        FocusKey {
            entry: entry(s.page()),
            elem: focus(&s),
        },
        false,
    );
    assert_eq!(s.page(), Page::Inputs);
    s.back();
    assert_eq!(focus(&s), HOME_INPUTS_LINK);
    assert_eq!(s.preferences(), &prefs);
    assert!(s.take_requests().is_empty());
}
#[test]
fn empty_home_and_previous_page_pointer_keys_cannot_turn_a_shortcut_into_a_tv_action() {
    let _serial = plx_base::testlock::serial();
    let mut s = Shell::new(
        fixture(),
        Preferences {
            initialized: true,
            ..Default::default()
        },
    );
    assert!(s.surface.cards.items.is_empty());
    s.focus_input("HDMI_1");
    s.direction(Dir::Up);
    assert_eq!(focus(&s), HOME_INPUTS_LINK);
    s.direction(Dir::Up);
    assert_eq!(focus(&s), HOME_APPS_LINK);
    let stale = FocusKey {
        entry: entry(Page::Home),
        elem: HOME_INPUTS_LINK,
    };
    s.activate(
        FocusKey {
            entry: entry(s.page()),
            elem: focus(&s),
        },
        false,
    );
    s.activate(stale, false);
    assert_eq!(s.page(), Page::Applications);
    assert!(s.take_requests().is_empty());
}
#[test]
fn drawer_scroll_reveals_full_focused_caption_and_keeps_menu_anchor_in_the_same_viewport() {
    let _serial = plx_base::testlock::serial();
    let mut data = fixture();
    for i in 12..70 {
        let id = format!("com.example.app{i:02}");
        data.launch_ids.insert(id.clone());
        data.applications.push(Application {
            id,
            title: format!("Application {i:02} with a deliberately long title"),
            kind: "native".into(),
            visible: true,
            icon: None,
        });
    }
    let mut s = Shell::new(data, Preferences::default());
    s.open(Page::Applications);
    for id in [
        "com.example.app00",
        "com.example.app07",
        "com.example.app29",
        "com.example.app69",
    ] {
        assert!(s.focus_application(id));
        settle(&mut s, 1200);
        let (r, clip) = s.drawer_focus_bounds().unwrap();
        assert!(r.y >= clip.y - 1.0, "top {id}: {} < {}", r.y, clip.y);
        assert!(
            r.y + r.h <= clip.y + clip.h + 1.0,
            "bottom {id}: {} > {}",
            r.y + r.h,
            clip.y + clip.h
        );
    }
    s.activate(
        FocusKey {
            entry: entry(s.page()),
            elem: focus(&s),
        },
        true,
    );
    settle(&mut s, 1200);
    let (r, clip) = s.drawer_focus_bounds().unwrap();
    assert!(r.y >= clip.y - 1.0 && r.y + r.h <= clip.y + clip.h + 1.0);
    assert!(s.menu_application().is_some());
    s.back();
    settle(&mut s, 1200);
    let (r, clip) = s.drawer_focus_bounds().unwrap();
    assert!(r.y >= clip.y - 1.0 && r.y + r.h <= clip.y + clip.h + 1.0);
    assert!(
        r.y + r.h >= clip.y + clip.h - 24.0,
        "menu return overscrolled past the document end"
    );
    assert_eq!(s.focused_item().as_deref(), Some("com.example.app69"));
    assert!(s.take_requests().is_empty());
}

#[test]
fn drawer_owned_scroll_reports_motion_and_becomes_quiet_after_settling() {
    let _serial = plx_base::testlock::serial();
    let old = plx_machine::idle::enabled();
    plx_machine::idle::set_enabled(true);
    let mut s = Shell::new(fixture(), Preferences::default());
    s.open(Page::Applications);
    settle(&mut s, 2000);
    s.focus_application("com.example.app11");
    let (_, moving) = plx_machine::idle::scoped_motion(|| s.tick(2016, 0.016));
    assert!(moving);
    for ms in (2032..6500).step_by(16) {
        s.tick(ms, 0.016);
    }
    let (_, moving) = plx_machine::idle::scoped_motion(|| s.tick(6512, 0.016));
    assert!(!moving);
    plx_machine::idle::set_enabled(old);
}

fn range_shell() -> Shell {
    let mut s = Shell::new(fixture(), Preferences::default());
    s.open(Page::Controls);
    s.focus_control_category(3);
    assert!(s.open_range_editor("Example level", RangeScale::new(-15, 15, 1).unwrap(), 0));
    s
}
#[test]
fn range_staging_cancel_and_unchanged_apply_cannot_emit_a_tv_operation() {
    let _serial = plx_base::testlock::serial();
    let mut s = range_shell();
    let before = s.preferences().clone();
    assert_eq!(s.focused_item().as_deref(), Some("range-value"));
    s.direction(Dir::Down);
    assert_eq!(s.focused_item().as_deref(), Some("range-apply"));
    tap(&mut s);
    assert!(s.take_range_request().is_none());
    s.direction(Dir::Up);
    for _ in 0..40 {
        s.direction(Dir::Left);
    }
    assert_eq!(s.range_state().unwrap().staged(), -15);
    assert_eq!(s.range_state().unwrap().saved(), 0);
    assert!(s.take_range_request().is_none());
    assert!(s.take_requests().is_empty());
    s.back();
    assert_eq!(s.page(), Page::Controls);
    assert!(s.range_state().is_none());
    assert_eq!(&before, s.preferences());
    assert!(s.take_range_request().is_none());
}
#[test]
fn range_apply_is_single_flight_and_only_matching_verified_readback_moves_saved_marker() {
    let _serial = plx_base::testlock::serial();
    let mut s = range_shell();
    s.direction(Dir::Right);
    s.direction(Dir::Down);
    tap(&mut s);
    assert_eq!(s.take_range_request(), Some((0, 1)));
    assert!(s.take_range_request().is_none());
    s.back();
    s.direction(Dir::Up);
    s.press(true, PressFrom::Key);
    s.press(false, PressFrom::Key);
    assert_eq!(s.page(), Page::RangeEditor);
    assert_eq!(s.range_state().unwrap().saved(), 0);
    assert!(!s.range_reply(0, 2, true));
    assert!(!s.open_range_editor("Other", RangeScale::new(0, 100, 1).unwrap(), 50));
    s.open(Page::Home);
    assert_eq!(s.page(), Page::RangeEditor);
    assert!(s.range_reply(0, 1, true));
    assert_eq!(s.range_state().unwrap().saved(), 1);
    assert!(!s.range_reply(0, 1, true));
    assert!(s.take_requests().is_empty());
    s.direction(Dir::Up);
    s.direction(Dir::Left);
    s.direction(Dir::Down);
    tap(&mut s);
    assert_eq!(s.take_range_request(), Some((1, 0)));
    assert!(!s.range_reply(1, 0, false));
    assert_eq!(s.range_state().unwrap().saved(), 1);
    s.back();
    assert_eq!(s.page(), Page::Controls);
}
#[test]
fn range_direction_cancels_armed_apply_and_old_page_keys_cannot_commit_the_new_editor() {
    let _serial = plx_base::testlock::serial();
    let mut s = range_shell();
    s.direction(Dir::Right);
    s.direction(Dir::Down);
    s.press(true, PressFrom::Key);
    s.direction(Dir::Up);
    s.direction(Dir::Right);
    s.press(false, PressFrom::Key);
    settle(&mut s, 300);
    assert!(s.take_range_request().is_none());
    assert_eq!(s.range_state().unwrap().staged(), 2);
    s.activate(
        FocusKey {
            entry: entry(Page::Accessibility),
            elem: RANGE_BASE + 1,
        },
        false,
    );
    assert!(s.take_range_request().is_none());
    let cx = s.context(&LegacyMeasure);
    assert!(s.surface.group_of(&RAIL_BASE, &cx).is_none());
    assert!(s.surface.group_of(&CATEGORY_BASE, &cx).is_none());
    assert!(!s.focus_control_category(0));
}
#[test]
fn range_track_click_stages_the_painted_value_while_hover_and_external_refresh_preserve_it() {
    let _serial = plx_base::testlock::serial();
    let mut s = range_shell();
    let layout = RangeLayout::new(RANGE_FRAME, s.range_state().unwrap()).unwrap();
    s.pointer(layout.track.x, layout.track.cy(), false);
    assert_eq!(s.range_state().unwrap().staged(), 0);
    s.pointer(layout.track.x, layout.track.cy(), true);
    assert_eq!(s.range_state().unwrap().staged(), -15);
    assert!(s.take_range_request().is_none());
    let mut changed = s.snapshot.clone();
    changed.context.input = "HDMI_2".into();
    let before = std::mem::replace(&mut s.snapshot, changed);
    s.refresh_shown_data(&before);
    assert_eq!(s.range_state().unwrap().staged(), -15);
    assert_eq!(s.range_state().unwrap().saved(), 0);
    s.open(Page::Home);
    assert!(s.range_state().is_none());
    assert!(s.take_requests().is_empty());
}
