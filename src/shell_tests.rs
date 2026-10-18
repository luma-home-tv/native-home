//! Behaviour checks for the real focus/press machines, without a GL context or TV actions.
use super::*;
fn control_rows(shell: &Shell) -> &[Row] {
    &shell
        .surface
        .table
        .table
        .sections
        .iter()
        .find(|section| section.header == crate::strings::Message::CurrentSettings.text())
        .expect("native readout section")
        .rows
}
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
fn home_hold_keeps_home_and_opens_shared_options_without_release_confirmation() {
    let _serial = plx_base::testlock::serial();
    let mut s = Shell::new(fixture(), Preferences::default());
    s.take_preferences_changed();
    let before = s.preferences().clone();
    let id = s.surface.cards.items[0].id.clone();
    s.press(true, PressFrom::Key);
    for ms in (0..=800).step_by(16) {
        s.tick(ms, 0.016);
        s.repeat();
    }
    s.press(false, PressFrom::Key);
    for ms in (816..1200).step_by(16) {
        s.tick(ms, 0.016);
    }
    assert_eq!(s.page(), Page::Home);
    assert_eq!(s.menu_application(), Some(id.as_str()));
    assert_eq!(s.preferences(), &before);
    assert!(!s.take_preferences_changed());
    assert!(s.take_requests().is_empty());
    assert_eq!(focus(&s), MENU_BASE + MenuAction::ToggleFavourite.key());
    s.direction(Dir::Left);
    assert_eq!(focus(&s), MENU_BASE + MenuAction::ToggleFavourite.key());
    assert!(
        s.surface
            .keys()
            .iter()
            .all(|key| (MENU_BASE..MENU_BASE + 5).contains(key))
    );
}
#[test]
fn home_confirmed_unpin_closes_options_once_and_restores_adjacent_or_apps_link() {
    let _serial = plx_base::testlock::serial();
    for last in [false, true] {
        let preferences = if last {
            Preferences {
                initialized: true,
                favourites: BTreeSet::from(["com.example.app00".into()]),
                favourite_order: Vec::new(),
            }
        } else {
            Preferences::default()
        };
        let mut s = Shell::new(fixture(), preferences);
        s.take_preferences_changed();
        s.activate(
            FocusKey {
                entry: entry(Page::Home),
                elem: focus(&s),
            },
            true,
        );
        tap(&mut s);
        assert_eq!(s.page(), Page::Home);
        assert!(s.menu_application().is_none());
        assert!(!s.preferences().favourites.contains("com.example.app00"));
        assert!(s.take_preferences_changed());
        assert!(!s.take_preferences_changed());
        assert!(!s.should_exit());
        if last {
            assert_eq!(focus(&s), HOME_APPS_LINK);
        } else {
            assert_eq!(s.focused_item().as_deref(), Some("com.example.app01"));
        }
        s.press(false, PressFrom::Key);
        assert!(!s.preferences().favourites.contains("com.example.app00"));
        assert!(!s.take_preferences_changed());
        assert!(s.take_requests().is_empty());
    }
}
#[test]
fn home_options_back_and_close_restore_same_id_without_preferences_or_launch() {
    let _serial = plx_base::testlock::serial();
    for back in [false, true] {
        let mut s = Shell::new(fixture(), Preferences::default());
        s.take_preferences_changed();
        s.focus_application("com.example.app03");
        let before = s.preferences().clone();
        s.activate(
            FocusKey {
                entry: entry(Page::Home),
                elem: focus(&s),
            },
            true,
        );
        if back {
            s.back();
        } else {
            s.focus_menu_action(MenuAction::Close);
            tap(&mut s);
        }
        assert_eq!(s.page(), Page::Home);
        assert!(s.menu_application().is_none());
        assert_eq!(s.focused_item().as_deref(), Some("com.example.app03"));
        assert_eq!(s.preferences(), &before);
        assert!(!s.take_preferences_changed());
        assert!(s.take_requests().is_empty());
        assert!(!s.should_exit());
    }
}
#[test]
fn home_options_reorder_uses_shared_action_and_keeps_home_identity_on_dismiss() {
    let _serial = plx_base::testlock::serial();
    let mut s = Shell::new(fixture(), Preferences::default());
    s.take_preferences_changed();
    s.focus_application("com.example.app01");
    s.activate(
        FocusKey {
            entry: entry(Page::Home),
            elem: focus(&s),
        },
        true,
    );
    s.focus_menu_action(MenuAction::Earlier);
    tap(&mut s);
    assert_eq!(s.page(), Page::Home);
    assert_eq!(s.menu_application(), Some("com.example.app01"));
    assert!(s.take_preferences_changed());
    s.back();
    assert_eq!(s.focused_item().as_deref(), Some("com.example.app01"));
    assert_eq!(s.first_application().as_deref(), Some("com.example.app01"));
    assert!(s.take_requests().is_empty());
}
#[test]
fn home_options_removed_identity_cancels_armed_confirmation_without_unpin() {
    let _serial = plx_base::testlock::serial();
    let mut s = Shell::new(fixture(), Preferences::default());
    s.take_preferences_changed();
    s.focus_application("com.example.app01");
    let before = s.preferences().clone();
    s.activate(
        FocusKey {
            entry: entry(Page::Home),
            elem: focus(&s),
        },
        true,
    );
    s.press(true, PressFrom::Key);
    let snapshot = s.snapshot.clone();
    s.snapshot
        .applications
        .retain(|a| a.id != "com.example.app01");
    s.snapshot.launch_ids.remove("com.example.app01");
    s.catalogue
        .refresh(&s.snapshot.applications, &s.snapshot.launch_ids);
    s.refresh_shown_data(&snapshot);
    s.press(false, PressFrom::Key);
    settle(&mut s, 600);
    assert_eq!(s.page(), Page::Home);
    assert!(s.menu_application().is_none());
    assert_ne!(s.focused_item().as_deref(), Some("com.example.app01"));
    assert!(s.focused_item().is_some());
    assert_eq!(s.preferences(), &before);
    assert!(!s.take_preferences_changed());
    assert!(s.take_requests().is_empty());
}
#[test]
fn home_options_open_preserves_existing_current_catalogue_gate() {
    let _serial = plx_base::testlock::serial();
    for expired in [false, true] {
        let mut s = Shell::new(fixture(), Preferences::default());
        s.take_preferences_changed();
        s.enable_app_launches(true);
        s.apply_live(&live_update(0), 0);
        s.focus_application("com.example.app00");
        s.activate(
            FocusKey {
                entry: entry(Page::Home),
                elem: focus(&s),
            },
            true,
        );
        if expired {
            s.age_live(15_001);
        }
        s.focus_menu_action(MenuAction::Open);
        tap(&mut s);
        if expired {
            assert!(s.take_requests().is_empty());
            assert!(s.menu_application().is_some());
        } else {
            assert_eq!(
                s.take_requests(),
                vec![Action::RequestApplication("com.example.app00".into())]
            );
            assert!(s.menu_application().is_none());
        }
        assert_eq!(s.page(), Page::Home);
        assert!(!s.take_preferences_changed());
    }
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
    assert!(
        Shell::new(fixture(), roundtrip)
            .surface
            .cards
            .items
            .is_empty()
    );
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
fn app_picker_restores_registered_identity_from_a_remembered_rail_without_accepting_arbitrary_targets()
 {
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
        system_keyboard: false,
        observed_ms: ms,
        simulated: true,
        controls: BTreeMap::new(),
        catalogue: Reading::Value(CatalogueReading {
            apps: data.applications,
            launch_ids: data.launch_ids,
        }),
        artwork: Reading::Failed("No fixture artwork".into()),
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
    assert_eq!(
        s.surface
            .table
            .activate(s.surface.table.index_of(&"volume".into()).unwrap()),
        None
    );
    assert_eq!(control_rows(&s)[0].value.as_deref(), Some("Unavailable"));
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
    assert_eq!(control_rows(&s)[0].value.as_deref(), Some("70"));
    s.age_live(15111);
    assert_eq!(
        s.surface
            .table
            .activate(s.surface.table.index_of(&"brightness".into()).unwrap()),
        None
    );
    assert_eq!(control_rows(&s)[0].value.as_deref(), Some("Unavailable"));
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
    assert_eq!(control_rows(&shell)[1].value.as_deref(), Some("10"));
    assert_eq!(
        control_rows(&shell)[3].value.as_deref(),
        Some("Unavailable")
    );
    shell.open(Page::Network);
    assert_eq!(control_rows(&shell)[1].value.as_deref(), Some("Connected"));
    shell.age_live(15101);
    assert_eq!(
        control_rows(&shell)[1].value.as_deref(),
        Some("Unavailable")
    );
    update.observed_ms = 15200;
    update.network = crate::live::Reading::Failed("Denied".into());
    shell.apply_live(&update, 15200);
    assert!(
        shell.snapshot.settings["network"]
            .iter()
            .all(|row| !row.available)
    );
}

#[test]
fn optional_sound_rows_remain_read_only_and_legible_across_independent_failures() {
    let _serial = plx_base::testlock::serial();
    let mut s = Shell::new(fixture(), Preferences::default());
    let mut data = crate::live::tests::global_raw();
    data["replies"].as_object_mut().unwrap().extend(
        crate::sound_reads::tests::fixture()
            .as_object()
            .unwrap()
            .clone(),
    );
    let mut update = crate::live::parse(&data, "self", 100, false);
    update.sound = live::Reading::Value(vec![
        ("soundOutput".into(), "external_optical".into()),
        ("volume".into(), "10".into()),
        ("muteStatus".into(), "false".into()),
        ("soundMode".into(), "standard".into()),
        ("audioInput".into(), "hdmi4".into()),
    ]);
    s.apply_live(&update, 100);
    s.open(Page::Sound);
    assert_eq!(
        s.control_readout_proof()["sound"],
        serde_json::to_value(&s.snapshot.settings["sound"]).unwrap()
    );
    s.direction(Dir::Right);
    let prefs = s.preferences().clone();
    let mut now = 100;
    for (index, key, id, expected) in [
        (5, 30, "audioBalance", "-4"),
        (6, 31, "audioDescription", "On"),
        (7, 32, "audioDescriptionVolume", "3"),
    ] {
        for _ in 0..10 {
            if s.focused_item().as_deref()
                == Some(&format!(
                    "control-row-{}",
                    s.surface.table.index_of(&id.to_owned()).unwrap()
                ))
            {
                break;
            }
            s.direction(Dir::Down);
        }
        for _ in 0..45 {
            now += 17;
            s.tick(now, 0.017);
        }
        let row = s.control_row_layout(id).unwrap();
        assert_eq!(row["key"], key);
        assert_eq!(row["focused"], true);
        assert_eq!(row["disabled"], true);
        assert_eq!(row["fully_visible"], true);
        assert_eq!(control_rows(&s)[index].value.as_deref(), Some(expected));
        assert!(
            s.surface
                .table
                .activate(s.surface.table.index_of(&id.to_owned()).unwrap())
                .is_none()
        );
        tap(&mut s);
        assert!(s.take_requests().is_empty());
        assert_eq!(s.page(), Page::Sound);
    }
    let owner = InputOwner::Entry(entry(s.page()));
    let focus_before = s.input.engine.current(owner);
    update.observed_ms = now;
    update.sound = live::Reading::Failed("Denied".into());
    s.apply_live(&update, now);
    assert_eq!(s.input.engine.current(owner), focus_before);
    assert!(s.reading_label().contains("some readings unavailable"));
    assert_eq!(control_rows(&s)[1].value.as_deref(), Some("Unavailable"));
    assert_eq!(control_rows(&s)[5].value.as_deref(), Some("-4"));
    update.observed_ms = now + 1;
    update.sound = live::Reading::Value(vec![("volume".into(), "12".into())]);
    update.controls.insert(
        crate::control_reads::Section::Sound,
        live::Reading::Failed("Denied".into()),
    );
    s.apply_live(&update, now + 1);
    assert_eq!(s.input.engine.current(owner), focus_before);
    assert_eq!(control_rows(&s)[1].value.as_deref(), Some("12"));
    assert!(
        control_rows(&s)[5..8]
            .iter()
            .all(|r| r.value.as_deref() == Some("Unavailable"))
    );
    assert!(s.reading_label().contains("some readings unavailable"));
    assert!(s.focus_lg_settings());
    tap(&mut s);
    assert_eq!(
        s.take_requests(),
        vec![Action::OpenLgSettingsSection(SettingsSection::Sound)]
    );
    assert_eq!(s.preferences(), &prefs);
}

#[test]
fn sound_status_is_unavailable_when_fresh_optional_envelopes_contain_no_readings() {
    let _serial = plx_base::testlock::serial();
    for simulated in [false, true] {
        let mut shell = Shell::new(fixture(), Preferences::default());
        let mut data = crate::live::tests::global_raw();
        data["replies"].as_object_mut().unwrap().extend(
            crate::sound_reads::tests::fixture()
                .as_object()
                .unwrap()
                .clone(),
        );
        let mut first = crate::live::parse(&data, "self", 100, simulated);
        first.sound = live::Reading::Value(vec![("volume".into(), "10".into())]);
        shell.apply_live(&first, 100);
        shell.open(Page::Sound);
        assert!(shell.reading_label().contains("some readings unavailable"));
        data["replies"][crate::sound_reads::VALUES_REPLY]["settings"] = serde_json::json!({});
        data["replies"][crate::sound_reads::DESCRIPTORS_REPLY]["results"] = serde_json::json!([]);
        let denied = crate::live::parse(&data, "self", 200, simulated);
        assert!(matches!(denied.sound, live::Reading::Failed(_)));
        assert!(matches!(
            denied.controls[&crate::control_reads::Section::Sound],
            live::Reading::Value(_)
        ));
        shell.apply_live(&denied, 200);
        let fresh = shell.live_state.as_ref().unwrap();
        assert_eq!(fresh.sound, None);
        assert_eq!(
            fresh.controls.get(&crate::control_reads::Section::Sound),
            Some(&200)
        );
        assert!(
            shell.snapshot.settings["sound"]
                .iter()
                .all(|row| !row.available)
        );
        assert!(
            control_rows(&shell)
                .iter()
                .all(|row| row.value.as_deref() == Some("Unavailable"))
        );
        assert_eq!(
            shell.reading_label(),
            "Reading unavailable • last state kept"
        );
        assert!(shell.take_requests().is_empty());
        data["replies"].as_object_mut().unwrap().extend(
            crate::sound_reads::tests::fixture()
                .as_object()
                .unwrap()
                .clone(),
        );
        shell.apply_live(&crate::live::parse(&data, "self", 300, simulated), 300);
        assert!(shell.reading_label().contains("some readings unavailable"));
        assert_eq!(control_rows(&shell)[5].value.as_deref(), Some("-4"));
        assert!(shell.take_requests().is_empty());
    }
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
    assert_eq!(s.drawer.query(), "APP01");
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
    assert!(s.drawer.query().is_empty());
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
    assert!(s.drawer.query().is_empty());
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
    assert_eq!(
        s.surface.table.key_at(0),
        Some(RowKey(SettingsSection::Sound.focus_key()))
    );
    s.direction(Dir::Right);
    assert_eq!(focus(&s), TABLE_BASE + SettingsSection::Sound.focus_key());
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
    assert_eq!(control_rows(&s)[1].value.as_deref(), Some("Unavailable"));
    assert_eq!(
        s.surface
            .table
            .activate(s.surface.table.index_of(&"wiredState".into()).unwrap()),
        None
    );
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
    assert_eq!(focus(&s), TABLE_BASE + SettingsSection::Picture.focus_key());
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
        let first = control_rows(&s)[0].value.clone();
        assert_eq!(first.as_deref(), Some("On"));
        assert!(!control_rows(&s)[0].dim);
        assert!(s.surface.table.activate(1).is_none());
        let owner = InputOwner::Entry(entry(s.page()));
        let before = s.input.engine.current(owner);
        s.age_live(15_101);
        assert_eq!(control_rows(&s)[0].value.as_deref(), Some("Unavailable"));
        assert_eq!(s.input.engine.current(owner), before);
        assert!(control_rows(&s)[0].dim);
        assert!(s.take_requests().is_empty());
        s.apply_live(
            &crate::live::parse(&data, "com.example.self", 15_200, false),
            15_200,
        );
    }
}

#[test]
fn guidance_rows_scroll_into_read_only_focus_and_return_without_actions() {
    let _serial = plx_base::testlock::serial();
    let mut s = Shell::new(fixture(), Preferences::default());
    let data = crate::live::tests::global_raw();
    s.apply_live(
        &crate::live::parse(&data, "com.example.self", 100, false),
        100,
    );
    let preferences = s.preferences().clone();
    assert!(s.control_row_layout("audioGuidanceVolume").is_none());
    s.open(Page::Accessibility);
    s.direction(Dir::Right);
    let mut now = 100;
    let mut previous_scroll = 0.0;
    for (index, id) in [
        (4, "audioGuidanceVolume"),
        (5, "audioGuidancePitch"),
        (6, "audioGuidanceSpeed"),
    ] {
        for _ in 0..9 {
            if s.focused_item().as_deref()
                == Some(&format!(
                    "control-row-{}",
                    s.surface.table.index_of(&id.to_owned()).unwrap()
                ))
            {
                break;
            }
            s.direction(Dir::Down);
        }
        for _ in 0..45 {
            now += 17;
            s.tick(now, 0.017);
        }
        let row = s.control_row_layout(id).unwrap();
        assert_eq!(
            row["index"],
            s.surface.table.index_of(&id.to_owned()).unwrap()
        );
        assert_eq!(row["key"], 100 + index);
        assert_eq!(row["focused"], true);
        assert_eq!(row["disabled"], true);
        assert_eq!(row["fully_visible"], true);
        let scroll = row["scroll"].as_f64().unwrap();
        assert!(scroll > previous_scroll);
        previous_scroll = scroll;
        s.press(true, PressFrom::Key);
        s.press(false, PressFrom::Key);
        for _ in 0..30 {
            now += 17;
            s.tick(now, 0.017);
        }
        assert!(s.take_requests().is_empty());
        assert_eq!(s.page(), Page::Accessibility);
    }
    for _ in 0..2 {
        s.direction(Dir::Up);
    }
    for _ in 0..45 {
        now += 17;
        s.tick(now, 0.017);
    }
    let row = s.control_row_layout("audioGuidanceVolume").unwrap();
    assert_eq!(row["focused"], true);
    assert_eq!(row["fully_visible"], true);
    assert!(row["scroll"].as_f64().unwrap() < previous_scroll);
    s.back();
    assert_eq!(s.focused_item().as_deref(), Some("category-Accessibility"));
    assert_eq!(s.preferences(), &preferences);
    assert!(s.take_requests().is_empty());
}

fn pointer_data() -> serde_json::Value {
    let mut raw = crate::live::tests::global_raw();
    raw["replies"]["general_option_values"]["settings"]["pointerSpeed"] = "normal".into();
    for d in raw["replies"]["general_option_desc"]["results"]
        .as_array_mut()
        .unwrap()
    {
        if d["key"] == "pointerSpeed" {
            d["values"]["arrayExt"] = serde_json::json!(
                crate::pointer_speed::Speed::ALL
                    .map(|s| serde_json::json!({"value":s.wire(),"visible":true,"active":true}))
            );
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
    s.drawer.set_query("unmatched");
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
    assert!(!s.drawer.favourites_only && s.drawer.query().is_empty());
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

#[test]
fn range_back_restores_the_actual_parent_instead_of_always_returning_to_controls() {
    let _serial = plx_base::testlock::serial();
    let mut s = Shell::new(fixture(), Preferences::default());
    s.open_range_editor("Example", RangeScale::new(-15, 15, 1).unwrap(), 0);
    s.direction(Dir::Right);
    s.back();
    assert_eq!(s.page(), Page::Home);
    assert!(s.range_state().is_none());
    assert!(s.take_range_request().is_none());
}
fn numeric_shell() -> Shell {
    let mut s = Shell::new(fixture(), Preferences::default());
    s.description_sample(
        Some(crate::description_level::Level::new(10).unwrap()),
        100,
        100,
    );
    assert!(s.focus_description_setting());
    tap(&mut s);
    assert_eq!(s.page(), Page::RangeEditor);
    s
}
#[test]
fn accessibility_help_tracks_the_actual_editor_without_changing_its_admission() {
    let _serial = plx_base::testlock::serial();
    for page in [Page::Accessibility, Page::Controls] {
        let mut shell = Shell::new(fixture(), Preferences::default());
        shell.open(Page::Accessibility);
        shell.open(page);
        let preferences = shell.preferences().clone();
        assert!(shell.status.to_str().unwrap().contains("View only"));
        assert!(
            shell
                .control_breadcrumb
                .to_str()
                .unwrap()
                .contains("LG-managed")
        );
        shell.description_sample(
            Some(crate::description_level::Level::new(10).unwrap()),
            100,
            100,
        );
        assert!(
            shell
                .status
                .to_str()
                .unwrap()
                .contains("Audio-description level can be adjusted")
        );
        assert_eq!(
            shell.control_breadcrumb.to_str().unwrap(),
            "Audio-description level can be adjusted here"
        );
        let row = shell
            .surface
            .table
            .index_of(&"audioDescriptionVolume".into())
            .unwrap();
        assert_eq!(
            shell.surface.table.activate(row),
            Some(plx_ui::form::Activation::Action(
                Action::OpenDescriptionLevel
            ))
        );
        shell.age_description(8_101);
        assert!(shell.status.to_str().unwrap().contains("View only"));
        assert!(
            shell
                .control_breadcrumb
                .to_str()
                .unwrap()
                .contains("LG-managed")
        );
        let row = shell
            .surface
            .table
            .index_of(&"audioDescriptionVolume".into())
            .unwrap();
        assert!(shell.surface.table.activate(row).is_none());
        assert_eq!(shell.preferences(), &preferences);
        assert!(shell.take_requests().is_empty());
    }
}

#[test]
fn lg_overview_activation_reports_opening_and_keeps_the_fixed_route() {
    let _serial = plx_base::testlock::serial();
    for section in [
        SettingsSection::Picture,
        SettingsSection::Sound,
        SettingsSection::Network,
        SettingsSection::Accessibility,
        SettingsSection::General,
        SettingsSection::PanelCare,
    ] {
        let mut shell = Shell::new(fixture(), Preferences::default());
        shell.open(section.page());
        assert!(shell.focus_lg_settings());
        tap(&mut shell);
        assert_eq!(shell.status.to_str().unwrap(), "Opening LG Settings…");
        assert_eq!(
            shell.take_requests(),
            vec![if section == SettingsSection::Picture {
                Action::OpenLgSettings
            } else {
                Action::OpenLgSettingsSection(section)
            }]
        );
    }
}

#[test]
fn general_reading_header_distinguishes_partial_failed_and_expired_groups() {
    let _serial = plx_base::testlock::serial();
    for simulated in [false, true] {
        let mut shell = Shell::new(fixture(), Preferences::default());
        let mut raw = crate::live::tests::global_raw();
        shell.apply_live(
            &crate::live::parse(&raw, "com.example.self", 100, simulated),
            100,
        );
        shell.open(Page::General);
        assert!(
            shell.snapshot.settings["general"]
                .iter()
                .any(|row| row.available)
        );
        assert!(
            shell.snapshot.settings["general"]
                .iter()
                .any(|row| !row.available)
        );
        assert_eq!(
            shell.reading_label(),
            if simulated {
                "Simulated update • some readings unavailable"
            } else {
                "Live TV state • some readings unavailable"
            }
        );
        shell.age_live(15_101);
        assert_eq!(
            shell.reading_label(),
            "Reading unavailable • last state kept"
        );
        for group in ["general_option_values", "general_home_values"] {
            raw["replies"][group]["returnValue"] = false.into();
        }
        shell.apply_live(
            &crate::live::parse(&raw, "com.example.self", 15_200, simulated),
            15_200,
        );
        assert!(
            shell.snapshot.settings["general"]
                .iter()
                .all(|row| !row.available)
        );
        assert_eq!(
            shell.reading_label(),
            "Reading unavailable • last state kept"
        );
        assert!(shell.take_requests().is_empty());
    }
}
#[test]
fn accessibility_reading_header_withdraws_failed_optional_caption_and_legacy_groups() {
    let _serial = plx_base::testlock::serial();
    for simulated in [false, true] {
        let mut shell = Shell::new(fixture(), Preferences::default());
        let mut raw = crate::live::tests::global_raw();
        shell.apply_live(
            &crate::live::parse(&raw, "com.example.self", 100, simulated),
            100,
        );
        shell.open(Page::Accessibility);
        assert!(
            shell.snapshot.settings["accessibility"]
                .iter()
                .any(|row| row.available)
        );
        assert!(
            !shell.snapshot.settings["accessibility"]
                .last()
                .unwrap()
                .available
        );
        assert!(shell.reading_label().contains("some readings unavailable"));
        raw["replies"]["accessibility_values"]["returnValue"] = false.into();
        shell.apply_live(
            &crate::live::parse(&raw, "com.example.self", 200, simulated),
            200,
        );
        assert!(
            shell.snapshot.settings["accessibility"]
                .iter()
                .all(|row| !row.available)
        );
        assert_eq!(
            shell.reading_label(),
            "Reading unavailable • last state kept"
        );
        assert!(shell.take_requests().is_empty());
    }
}

#[test]
fn numeric_editor_withdraws_stale_or_changed_samples_and_never_turns_staging_into_a_write() {
    let _serial = plx_base::testlock::serial();
    let mut s = numeric_shell();
    s.direction(Dir::Right);
    assert!(s.take_description_request().is_none());
    s.age_description(8_101);
    assert_ne!(s.page(), Page::RangeEditor);
    assert!(s.description_value().is_none());
    assert!(s.take_description_request().is_none());
    let mut s = numeric_shell();
    s.description_sample(
        Some(crate::description_level::Level::new(9).unwrap()),
        200,
        200,
    );
    assert_ne!(s.page(), Page::RangeEditor);
    assert_eq!(s.description_value().unwrap().value(), 9);
    assert!(s.take_requests().is_empty());
}
#[test]
fn numeric_worker_reply_advances_saved_and_ignores_read_cycles_started_before_confirmation() {
    use crate::description_level::{Change, Level, Outcome};
    let _serial = plx_base::testlock::serial();
    let mut s = numeric_shell();
    s.direction(Dir::Right);
    s.direction(Dir::Down);
    tap(&mut s);
    let change = s.take_description_request().unwrap();
    assert_eq!(
        change,
        Change {
            expected: Level::new(10).unwrap(),
            value: Level::new(11).unwrap()
        }
    );
    s.description_reply(
        &Outcome {
            change,
            attempted: true,
            admitted: true,
            verified: true,
            detail: "confirmed".into(),
            wire_format: None,
        },
        300,
    );
    assert_eq!(s.range_state().unwrap().saved(), 11);
    s.description_sample(Some(Level::new(10).unwrap()), 250, 350);
    assert_eq!(s.description_value().unwrap().value(), 11);
    s.description_sample(Some(Level::new(9).unwrap()), 400, 450);
    assert_ne!(s.page(), Page::RangeEditor);
    assert_eq!(s.description_value().unwrap().value(), 9);
}

#[test]
fn text_ingress_is_owned_by_the_editing_drawer_and_composition_never_filters_early() {
    let _serial = plx_base::testlock::serial();
    let mut s = Shell::new(fixture(), Preferences::default());
    let prefs = s.preferences().clone();
    assert!(!s.text_input("hidden", false));
    s.open(Page::Applications);
    s.focus_drawer_control(2);
    tap(&mut s);
    assert!(s.text_input("app", false));
    assert!(s.text_input("li", false));
    assert_eq!(s.drawer_state().query(), "appli");
    let count = s.application_count();
    assert!(s.text_preedit("é"));
    assert_eq!(s.application_count(), count);
    assert_eq!(s.drawer_state().query(), "appli");
    s.text_edit(&plx_machine::machine::TextEdit::Clear);
    assert!(s.text_input("e\u{301}", false));
    s.text_edit(&plx_machine::machine::TextEdit::Backspace);
    assert!(s.drawer_state().query().is_empty());
    s.back();
    assert!(!s.text_input("late", false));
    assert!(!s.text_preedit("late"));
    assert!(s.take_requests().is_empty());
    assert_eq!(s.preferences(), &prefs);
}
#[test]
fn typed_text_cancels_an_armed_native_key_instead_of_appending_that_key_on_release() {
    let _serial = plx_base::testlock::serial();
    let mut s = Shell::new(fixture(), Preferences::default());
    s.open(Page::Applications);
    s.focus_drawer_control(2);
    tap(&mut s);
    s.focus_search_key(0);
    s.press(true, PressFrom::Key);
    assert!(s.text_input("Écran", false));
    s.press(false, PressFrom::Key);
    settle(&mut s, 300);
    assert_eq!(s.drawer_state().query(), "Écran");
    assert!(s.take_requests().is_empty());
}

#[test]
fn system_keyboard_owns_search_and_leaving_ends_editing_without_another_keypad() {
    let _serial = plx_base::testlock::serial();
    let mut s = Shell::new(fixture(), Preferences::default());
    s.open(Page::Applications);
    s.focus_drawer_control(2);
    tap(&mut s);
    s.use_system_keyboard(true);
    assert!(s.drawer_state().system_keyboard());
    assert!(!s.focus_search_key(0));
    s.press(true, PressFrom::Key);
    s.press(false, PressFrom::Key);
    assert!(s.drawer_state().query().is_empty());
    assert!(s.text_input("luma", false));
    assert!(s.drawer_state().system_keyboard()); // a live drawer rebuild must retain ownership
    s.back();
    assert!(!s.drawer_state().editing);
    assert!(!s.drawer_state().system_keyboard());
    assert_eq!(s.drawer_state().query(), "luma");
    s.focus_drawer_control(2);
    tap(&mut s);
    s.use_system_keyboard(true);
    s.open(Page::Home);
    s.open(Page::Applications);
    assert!(!s.drawer_state().editing);
    assert!(!s.drawer_state().system_keyboard());
    assert!(s.take_requests().is_empty());
}

#[test]
fn qualified_search_and_explicit_keypad_are_separate_routes_with_no_implicit_switch() {
    let _serial = plx_base::testlock::serial();
    let mut s = Shell::new(fixture(), Preferences::default());
    let mut update = live_update(100);
    update.system_keyboard = true;
    s.apply_live(&update, 100);
    s.open(Page::Applications);
    assert!(s.focus_drawer_control(2));
    tap(&mut s);
    assert!(s.drawer_state().system_keyboard());
    assert!(!s.focus_search_key(0));
    s.text_input("luma", false);
    s.back();
    assert!(!s.drawer_state().editing);
    assert_eq!(s.focused_item().as_deref(), Some("Search apps"));
    assert!(s.focus_drawer_control(3));
    assert_eq!(s.focused_item().as_deref(), Some("Search keypad"));
    tap(&mut s);
    assert!(s.drawer_state().editing);
    assert!(!s.drawer_state().system_keyboard());
    assert!(s.focus_search_key(0));
    tap(&mut s);
    assert_eq!(s.drawer_state().query(), "lumaA");
    s.back();
    update.system_keyboard = false;
    s.apply_live(&update, 200);
    s.focus_drawer_control(2);
    tap(&mut s);
    assert!(s.drawer_state().editing);
    assert!(!s.drawer_state().system_keyboard());
    assert!(s.take_requests().is_empty());
}

#[test]
fn fallback_observation_counts_effective_registered_targets_not_an_editing_constant() {
    let _serial = plx_base::testlock::serial();
    let mut s = Shell::new(fixture(), Preferences::default());
    s.open(Page::Applications);
    s.focus_drawer_control(3);
    tap(&mut s);
    assert_eq!(s.effective_fallback_targets(), 0);
    let target = |index, clip| Stop {
        key: FocusKey {
            entry: entry(Page::Applications),
            elem: KEYBOARD_BASE + index,
        },
        rect: keyboard_rect(index as usize),
        rest_rect: keyboard_rect(index as usize),
        clip,
        hover: Hover::Focus,
        activate: Activate::Press,
    };
    s.input.hit.fill(vec![
        target(0, KEYBOARD),
        target(37, KEYBOARD),
        target(1, Rect::new(0.0, 0.0, 0.0, 0.0)),
    ]);
    s.input.hit.swap();
    assert_eq!(s.effective_fallback_targets(), 2);
    s.use_system_keyboard(true);
    assert_eq!(s.effective_fallback_targets(), 0);
    s.back();
    assert_eq!(s.effective_fallback_targets(), 0);
}

struct RestoreDrawerIdle(bool, bool);
impl RestoreDrawerIdle {
    fn start() -> Self {
        let saved = Self(
            plx_machine::idle::enabled(),
            plx_machine::idle::video_plane_bound(),
        );
        plx_machine::idle::set_enabled(true);
        plx_machine::idle::note(plx_machine::present::PresentEvent::VideoPlane(false));
        saved
    }
}
impl Drop for RestoreDrawerIdle {
    fn drop(&mut self) {
        use plx_machine::idle;
        idle::frame_begin(1.0 / 60.0);
        let now = idle::now_ms();
        if idle::should_present(now) {
            idle::note_present(now);
        }
        let _ = idle::take_page_damage();
        idle::note(plx_machine::present::PresentEvent::VideoPlane(self.1));
        idle::set_enabled(self.0);
    }
}
fn drawer_gate_fixture() -> Shell {
    let mut data = fixture();
    for i in 12..70 {
        let id = format!("com.example.app{i:02}");
        data.launch_ids.insert(id.clone());
        data.applications.push(Application {
            id,
            title: format!("Application {i:02}"),
            kind: "native".into(),
            visible: true,
            icon: None,
        });
    }
    let mut shell = Shell::new(data, Preferences::default());
    shell.open(Page::Applications);
    shell
}
fn drawer_gate_tick(shell: &mut Shell) -> (bool, bool) {
    use plx_machine::idle;
    idle::frame_begin(1.0 / 60.0);
    let now = idle::now_ms();
    shell.tick(now, 1.0 / 60.0);
    let moving = idle::present_moving();
    let present = idle::should_present(now);
    if present {
        idle::note_present(now);
    }
    (present, moving)
}

#[test]
fn apps_six_arrow_replay_stops_subvisible_scroll_tail_presentations() {
    use plx_machine::idle;
    let _serial = plx_base::testlock::serial();
    let _restore = RestoreDrawerIdle::start();
    let mut shell = drawer_gate_fixture();
    const DT: f32 = 1.0 / 60.0;
    const WAIT_FRAMES: usize = 120;
    let arrows = [
        Dir::Right,
        Dir::Right,
        Dir::Down,
        Dir::Left,
        Dir::Up,
        Dir::Left,
    ];
    let mut trace = Vec::new();
    let mut last_second_presents = 0;
    let mut last_second_settled_damage = 0;
    let mut observed_nonzero_target = false;
    // Two seconds of warm-up followed by six discrete arrows, each with two
    // seconds of real Shell updates. No draw/GL context, live worker or clock
    // refresh is introduced: this isolates the update -> actual idle-gate path.
    for frame in 0..WAIT_FRAMES * (1 + arrows.len()) {
        idle::frame_begin(DT);
        let now = idle::now_ms();
        let key = if frame >= WAIT_FRAMES && frame % WAIT_FRAMES == 0 {
            let index = frame / WAIT_FRAMES - 1;
            shell.direction(arrows[index]);
            Some(index)
        } else {
            None
        };
        let _ = idle::take_page_damage();
        let before = shell.drawer_scroll.pos;
        shell.tick(now, DT);
        let damage = idle::take_page_damage();
        let pos = shell.drawer_scroll.pos;
        let vel = shell.drawer_scroll.vel;
        let target = shell.drawer_scroll_target;
        let settled = idle::settled(pos, target, vel);
        let moving = idle::present_moving();
        let present = idle::should_present(now);
        let dirty = idle::present_dirty();
        if present {
            // A selected frame was presented in the gate replay; no actual GL
            // submission or compositor/performance claim follows from this.
            idle::note_present(now);
        }
        observed_nonzero_target |= target > 0.0;
        if frame >= WAIT_FRAMES * arrows.len() + 60 {
            last_second_presents += usize::from(present);
            last_second_settled_damage +=
                usize::from(settled && !moving && damage && dirty && present && pos != before);
        }
        trace.push(serde_json::json!({"frame":frame,"ms":now,"key_index":key,
            "scroll_pos":pos,"scroll_vel":vel,"scroll_target":target,
            "pos_changed":pos!=before,"idle_settled":settled,"moving":moving,
            "tick_page_damage":damage,"dirty":dirty,"present":present}));
    }
    println!(
        "LUMA_APPS_GATE_REPLAY {}",
        serde_json::json!({
        "dt":DT,"app_count":70,"arrows":["Right","Right","Down","Left","Up","Left"],
        "frames_per_key_wait":WAIT_FRAMES,"actual_idle_gate":true,"draw_or_gl_invoked":false,
        "observed_nonzero_scroll_target":observed_nonzero_target,
        "last_second_presents":last_second_presents,
        "last_second_settled_numeric_scroll_damage":last_second_settled_damage,
        "frames":trace})
    );
    assert!(
        observed_nonzero_target,
        "replay did not exercise drawer scrolling"
    );
    assert!(
        shell.take_requests().is_empty(),
        "arrow-only replay emitted a TV action"
    );
    assert!(
        last_second_presents <= 2,
        "settled Apps drawer kept selecting {last_second_presents}/60 frames in the last second; \
         {last_second_settled_damage} had sub-visible numeric scroll motion, discrete damage, \
         no visible spring motion, and an actual present decision (keepalive is only every 2 s)"
    );
}

#[test]
fn drawer_exact_rest_handoff_presents_final_target_then_goes_quiet() {
    let _serial = plx_base::testlock::serial();
    let _restore = RestoreDrawerIdle::start();
    let mut shell = drawer_gate_fixture();
    for _ in 0..360 {
        drawer_gate_tick(&mut shell);
    }
    assert_eq!(shell.drawer_scroll_target, 0.0);
    shell.drawer_scroll.pos = 0.0005;
    shell.drawer_scroll.vel = 0.0;
    let (present, moving) = drawer_gate_tick(&mut shell);
    assert!(present, "the exact final target must get a selected frame");
    assert!(
        plx_machine::idle::present_dirty(),
        "the final frame must carry damage, not only a keepalive"
    );
    assert!(!moving);
    assert_eq!(shell.drawer_scroll.pos, 0.0);
    assert_eq!(shell.drawer_scroll.vel, 0.0);
    assert_eq!(drawer_gate_tick(&mut shell), (false, false));
    assert!(shell.take_requests().is_empty());
}

#[test]
fn drawer_rest_handoff_does_not_discard_velocity_or_prevent_retargeting() {
    let _serial = plx_base::testlock::serial();
    let _restore = RestoreDrawerIdle::start();
    let mut shell = drawer_gate_fixture();
    for _ in 0..360 {
        drawer_gate_tick(&mut shell);
    }
    // The analytic step lands near its target with substantial velocity. A
    // position-only rest check would incorrectly discard that momentum.
    let dt = 1.0 / 60.0;
    let velocity = 200.0;
    shell.drawer_scroll.pos = -velocity * dt / (1.0 + plx_ui::consts::K_SCROLL.sqrt() * dt);
    shell.drawer_scroll.vel = velocity;
    shell.surface.grid.set_page(340.0, shell.drawer_scroll.pos);
    let (present, moving) = drawer_gate_tick(&mut shell);
    assert!(present && moving);
    assert!(shell.drawer_scroll.pos.abs() < 0.001);
    assert!(!plx_machine::idle::settled(
        shell.drawer_scroll.pos,
        shell.drawer_scroll_target,
        shell.drawer_scroll.vel
    ));
    assert_ne!(shell.drawer_scroll.vel, 0.0);
    assert!(shell.focus_application("com.example.app69"));
    for _ in 0..20 {
        drawer_gate_tick(&mut shell);
    }
    assert!(shell.drawer_scroll_target > 0.0 && shell.drawer_scroll.pos > 0.0);
    let old_target = shell.drawer_scroll_target;
    assert!(shell.focus_application("com.example.app00"));
    for _ in 0..360 {
        drawer_gate_tick(&mut shell);
    }
    assert!(shell.drawer_scroll_target < old_target);
    assert_eq!(shell.drawer_scroll.pos, shell.drawer_scroll_target);
    assert_eq!(shell.drawer_scroll.vel, 0.0);
    assert!(shell.take_requests().is_empty());
}

#[test]
fn drawer_nonzero_rest_preserves_end_visibility_and_stops_repeated_damage() {
    let _serial = plx_base::testlock::serial();
    let _restore = RestoreDrawerIdle::start();
    let mut shell = drawer_gate_fixture();
    assert!(shell.focus_application("com.example.app69"));
    for _ in 0..360 {
        drawer_gate_tick(&mut shell);
    }
    let target = shell.drawer_scroll_target;
    assert!(target > 0.0);
    assert_eq!(shell.drawer_scroll.pos, target);
    assert_eq!(shell.drawer_scroll.vel, 0.0);
    let (rect, clip) = shell.drawer_focus_bounds().unwrap();
    assert!(rect.y >= clip.y - 1.0 && rect.y + rect.h <= clip.y + clip.h + 1.0);
    let frames = (0..60).filter(|_| drawer_gate_tick(&mut shell).0).count();
    assert!(frames <= 1, "resting nonzero target kept selecting frames");
    assert_eq!(
        shell.drawer_scroll.pos, target,
        "content-end target drifted"
    );
    assert_eq!(shell.drawer_scroll.vel, 0.0);
    assert!(shell.take_requests().is_empty());
}

#[test]
fn pointer_click_replaces_a_same_card_keyboard_press_without_a_second_launch() {
    let _serial = plx_base::testlock::serial();
    let mut shell = Shell::new(fixture(), Preferences::default());
    shell.enable_app_launches(true);
    shell.open(Page::Applications);
    assert!(shell.focus_application("com.example.app00"));
    let key = FocusKey {
        entry: entry(Page::Applications),
        elem: focus(&shell),
    };
    let rect = Rect::new(700.0, 400.0, 200.0, 180.0);
    shell.input.hit.fill(vec![Stop {
        key,
        rect,
        rest_rect: rect,
        clip: CONTENT,
        hover: Hover::Focus,
        activate: Activate::Press,
    }]);
    shell.input.hit.swap();
    shell.tick(100, 0.016);
    shell.press(true, PressFrom::Key);
    shell.tick(150, 0.05);
    shell.pointer(rect.cx(), rect.cy(), true);
    assert_eq!(
        focus(&shell),
        key.elem,
        "click must exercise the already-focused card"
    );
    shell.press(false, PressFrom::Key);
    for ms in (166..=950).step_by(16) {
        shell.tick(ms, 0.016);
    }
    assert_eq!(
        shell.take_requests(),
        vec![Action::RequestApplication("com.example.app00".into())],
        "pointer release owns this activation; later keyboard release must not launch again"
    );
    assert!(
        shell.menu_application().is_none(),
        "the replaced keyboard arm must not become a hold"
    );
}

#[test]
fn pointer_click_miss_cancels_keyboard_press_but_same_card_hover_keeps_it() {
    let _serial = plx_base::testlock::serial();
    for click in [false, true] {
        let mut shell = Shell::new(fixture(), Preferences::default());
        shell.enable_app_launches(true);
        shell.open(Page::Applications);
        assert!(shell.focus_application("com.example.app00"));
        let key = FocusKey {
            entry: entry(Page::Applications),
            elem: focus(&shell),
        };
        let rect = Rect::new(700.0, 400.0, 200.0, 180.0);
        shell.input.hit.fill(vec![Stop {
            key,
            rect,
            rest_rect: rect,
            clip: CONTENT,
            hover: Hover::Focus,
            activate: Activate::Press,
        }]);
        shell.input.hit.swap();
        shell.tick(100, 0.016);
        shell.press(true, PressFrom::Key);
        shell.tick(150, 0.05);
        if click {
            shell.pointer(5.0, 5.0, true);
        } else {
            shell.pointer(rect.cx(), rect.cy(), false);
        }
        shell.press(false, PressFrom::Key);
        for ms in (166..=950).step_by(16) {
            shell.tick(ms, 0.016);
        }
        let expected = if click {
            vec![]
        } else {
            vec![Action::RequestApplication("com.example.app00".into())]
        };
        assert_eq!(
            shell.take_requests(),
            expected,
            "click miss must cancel; hover over the same card must retain a keyboard tap"
        );
        assert_eq!(focus(&shell), key.elem);
        assert!(shell.menu_application().is_none());
    }
}

#[test]
fn home_back_blocks_a_later_pointer_activation_in_the_same_input_batch() {
    let _serial = plx_base::testlock::serial();
    let mut shell = Shell::new(fixture(), Preferences::default());
    shell.enable_app_launches(true);
    let key = FocusKey {
        entry: entry(Page::Home),
        elem: focus(&shell),
    };
    assert_eq!(shell.focused_item().as_deref(), Some("com.example.app00"));
    let rect = Rect::new(700.0, 400.0, 200.0, 180.0);
    shell.input.hit.fill(vec![Stop {
        key,
        rect,
        rest_rect: rect,
        clip: CONTENT,
        hover: Hover::Focus,
        activate: Activate::Press,
    }]);
    shell.input.hit.swap();
    let before = shell.preferences().clone();
    shell
        .input_frame(400, 0.016, |shell| {
            shell.back();
            assert!(shell.should_exit());
            shell.pointer(rect.cx(), rect.cy(), true);
            Ok::<(), ()>(())
        })
        .unwrap();
    assert!(shell.should_exit());
    assert_eq!(shell.page(), Page::Home);
    assert_eq!(shell.preferences(), &before);
    assert!(
        shell.take_requests().is_empty(),
        "Back from Home is terminal; a later queued click must not launch an application"
    );
}

#[test]
fn home_focus_bookmark_restores_deliberate_favourite_after_restart_without_replaying_input() {
    let _serial = plx_base::testlock::serial();
    let mut first = Shell::new(fixture(), Preferences::default());
    first.take_preferences_changed();
    assert!(first.take_home_focus_changed().is_none());
    first.direction(Dir::Right);
    first.direction(Dir::Right);
    let id = first.take_home_focus_changed().unwrap();
    assert_eq!(id, "com.example.app02");
    assert!(first.take_home_focus_changed().is_none());
    let saved = serde_json::to_string(first.preferences()).unwrap();
    let mut restarted = Shell::new(fixture(), serde_json::from_str(&saved).unwrap());
    assert!(restarted.restore_home_focus(&id));
    assert_eq!(restarted.focused_item().as_deref(), Some(id.as_str()));
    assert!(restarted.take_home_focus_changed().is_none());
    assert!(!restarted.take_preferences_changed());
    assert!(restarted.take_requests().is_empty());
    assert!(restarted.menu_application().is_none());
    assert!(!restarted.drawer_state().editing);
    assert!(!restarted.should_exit());
}

#[test]
fn home_focus_bookmark_missing_unpinned_and_explicit_empty_use_default_without_rewriting() {
    let _serial = plx_base::testlock::serial();
    for missing in ["com.example.gone", "com.example.app11"] {
        let mut s = Shell::new(fixture(), Preferences::default());
        s.take_preferences_changed();
        assert!(!s.restore_home_focus(missing));
        assert_eq!(s.focused_item().as_deref(), Some("com.example.app00"));
        assert!(s.take_home_focus_changed().is_none());
        assert!(!s.take_preferences_changed());
        assert!(s.take_requests().is_empty());
    }
    let mut s = Shell::new(
        fixture(),
        Preferences {
            initialized: true,
            ..Preferences::default()
        },
    );
    assert!(!s.restore_home_focus("com.example.app00"));
    assert_eq!(focus(&s), HOME_APPS_LINK);
    assert!(s.preferences().favourites.is_empty());
    assert!(s.take_home_focus_changed().is_none());
    assert!(!s.take_preferences_changed());
}

#[test]
fn home_focus_bookmark_rename_reorder_and_automatic_reconciliation_never_dirty_it() {
    let _serial = plx_base::testlock::serial();
    let mut s = Shell::new(fixture(), Preferences::default());
    assert!(s.restore_home_focus("com.example.app03"));
    let before = s.snapshot.clone();
    s.snapshot.applications.reverse();
    s.snapshot
        .applications
        .iter_mut()
        .find(|a| a.id == "com.example.app03")
        .unwrap()
        .title = "ZZ renamed".into();
    s.catalogue
        .refresh(&s.snapshot.applications, &s.snapshot.launch_ids);
    s.refresh_shown_data(&before);
    assert_eq!(s.focused_item().as_deref(), Some("com.example.app03"));
    s.focus_application("com.example.app02");
    s.open(Page::Applications);
    s.direction(Dir::Right);
    s.open(Page::Home);
    assert!(s.take_home_focus_changed().is_none());
    let before = s.snapshot.clone();
    s.snapshot
        .applications
        .retain(|a| a.id != "com.example.app02");
    s.snapshot.launch_ids.remove("com.example.app02");
    s.catalogue
        .refresh(&s.snapshot.applications, &s.snapshot.launch_ids);
    s.refresh_shown_data(&before);
    assert!(s.take_home_focus_changed().is_none());
}

#[test]
fn home_focus_bookmark_pointer_hover_is_quiet_but_click_is_deliberate_once() {
    let _serial = plx_base::testlock::serial();
    let mut s = Shell::new(fixture(), Preferences::default());
    s.take_preferences_changed();
    let card = &s.surface.cards.items[3];
    let key = FocusKey {
        entry: entry(Page::Home),
        elem: card.key,
    };
    let rect = Rect::new(700.0, 400.0, 200.0, 180.0);
    s.input.hit.fill(vec![Stop {
        key,
        rect,
        rest_rect: rect,
        clip: CONTENT,
        hover: Hover::Focus,
        activate: Activate::Press,
    }]);
    s.input.hit.swap();
    for _ in 0..10 {
        s.pointer(rect.cx(), rect.cy(), false);
    }
    assert_eq!(s.focused_item().as_deref(), Some("com.example.app03"));
    assert!(s.take_home_focus_changed().is_none());
    s.pointer(rect.cx(), rect.cy(), true);
    assert_eq!(
        s.take_home_focus_changed().as_deref(),
        Some("com.example.app03")
    );
    s.pointer(rect.cx(), rect.cy(), true);
    assert!(s.take_home_focus_changed().is_none());
    assert!(!s.take_preferences_changed());
}

#[test]
fn home_focus_bookmark_hold_release_and_menu_do_not_duplicate_or_save_menu_ids() {
    let _serial = plx_base::testlock::serial();
    let mut s = Shell::new(fixture(), Preferences::default());
    s.take_preferences_changed();
    assert!(s.focus_application("com.example.app03"));
    assert!(s.take_home_focus_changed().is_none());
    s.press(true, PressFrom::Key);
    for ms in (0..=800).step_by(16) {
        s.tick(ms, 0.016);
        s.repeat();
    }
    assert_eq!(
        s.take_home_focus_changed().as_deref(),
        Some("com.example.app03")
    );
    s.press(false, PressFrom::Key);
    for ms in (816..1200).step_by(16) {
        s.tick(ms, 0.016);
    }
    s.direction(Dir::Down);
    s.back();
    assert!(s.take_home_focus_changed().is_none());
    assert!(!s.take_preferences_changed());
    assert!(s.take_requests().is_empty());
}

fn late_home_focus_shell() -> Shell {
    let mut packaged = fixture();
    packaged
        .applications
        .retain(|a| a.id != "com.example.app09");
    packaged.launch_ids.remove("com.example.app09");
    let prefs = Preferences {
        initialized: true,
        favourites: [
            "com.example.app00".into(),
            "com.example.app01".into(),
            "com.example.app09".into(),
        ]
        .into(),
        favourite_order: Vec::new(),
    };
    let mut s = Shell::new(packaged, prefs);
    assert!(!s.restore_home_focus("com.example.app09"));
    assert!(s.home_focus_restore_pending());
    s
}
#[test]
fn home_focus_first_fresh_real_catalogue_restores_post_packaging_favourite_once() {
    let _serial = plx_base::testlock::serial();
    let mut s = late_home_focus_shell();
    let mut update = live_update(100);
    update.simulated = false;
    s.apply_live(&update, 100);
    assert_eq!(s.focused_item().as_deref(), Some("com.example.app09"));
    assert!(s.home_focus_restored_from_live());
    assert!(!s.home_focus_restore_pending());
    assert!(s.take_home_focus_changed().is_none());
    assert!(!s.take_preferences_changed());
    assert!(s.take_requests().is_empty());
    s.focus_application("com.example.app00");
    s.apply_live(&update, 100);
    assert_eq!(s.focused_item().as_deref(), Some("com.example.app00"));
}
#[test]
fn home_focus_nonqualifying_catalogues_do_not_consume_pending_restore() {
    let _serial = plx_base::testlock::serial();
    for variant in 0..4 {
        let mut s = late_home_focus_shell();
        let mut update = live_update(100);
        update.simulated = false;
        let now = match variant {
            0 => {
                update.simulated = true;
                100
            }
            1 => 15_101,
            2 => {
                update.catalogue = crate::live::Reading::Failed("unavailable".into());
                100
            }
            _ => {
                update.observed_ms = 101;
                100
            }
        };
        s.apply_live(&update, now);
        assert!(s.home_focus_restore_pending());
        assert!(!s.home_focus_restored_from_live());
        let mut fresh = live_update(20_000);
        fresh.simulated = false;
        s.apply_live(&fresh, 20_000);
        assert_eq!(s.focused_item().as_deref(), Some("com.example.app09"));
        assert!(!s.home_focus_restore_pending());
        assert!(s.take_home_focus_changed().is_none());
    }
}
#[test]
fn home_focus_first_fresh_absence_consumes_restore_and_later_install_cannot_steal_focus() {
    let _serial = plx_base::testlock::serial();
    let mut s = late_home_focus_shell();
    let mut absent = live_update(100);
    absent.simulated = false;
    if let crate::live::Reading::Value(c) = &mut absent.catalogue {
        c.apps.retain(|a| a.id != "com.example.app09");
        c.launch_ids.remove("com.example.app09");
    }
    s.apply_live(&absent, 100);
    assert!(!s.home_focus_restore_pending());
    assert!(!s.home_focus_restored_from_live());
    let mut present = live_update(200);
    present.simulated = false;
    s.apply_live(&present, 200);
    assert_eq!(s.focused_item().as_deref(), Some("com.example.app00"));
    assert!(s.take_home_focus_changed().is_none());
}
#[test]
fn home_focus_explicit_input_page_change_and_close_cancel_delayed_restore() {
    let _serial = plx_base::testlock::serial();
    for variant in 0..6 {
        let mut s = late_home_focus_shell();
        match variant {
            0 => s.direction(Dir::Right),
            1 => {
                s.press(true, PressFrom::Key);
                s.cancel_press();
            }
            2 => s.back(),
            3 => {
                s.open(Page::Applications);
                s.back();
            }
            4 => s.pointer(5.0, 5.0, true),
            _ => s.request_exit(),
        }
        assert!(!s.home_focus_restore_pending());
        let mut update = live_update(100);
        update.simulated = false;
        s.apply_live(&update, 100);
        assert_ne!(s.focused_item().as_deref(), Some("com.example.app09"));
        assert!(!s.home_focus_restored_from_live());
    }
}
#[test]
fn home_focus_valid_hover_change_cancels_restore_without_saving_but_same_hover_does_not() {
    let _serial = plx_base::testlock::serial();
    for changed in [false, true] {
        let mut s = late_home_focus_shell();
        let target = if changed {
            "com.example.app01"
        } else {
            "com.example.app00"
        };
        let card = s
            .surface
            .cards
            .items
            .iter()
            .find(|c| c.id == target)
            .unwrap();
        let key = FocusKey {
            entry: entry(Page::Home),
            elem: card.key,
        };
        let rect = Rect::new(700.0, 400.0, 200.0, 180.0);
        s.input.hit.fill(vec![Stop {
            key,
            rect,
            rest_rect: rect,
            clip: CONTENT,
            hover: Hover::Focus,
            activate: Activate::Press,
        }]);
        s.input.hit.swap();
        s.pointer(rect.cx(), rect.cy(), false);
        assert_eq!(s.home_focus_restore_pending(), !changed);
        assert!(s.take_home_focus_changed().is_none());
        let mut update = live_update(100);
        update.simulated = false;
        s.apply_live(&update, 100);
        assert_eq!(
            s.focused_item().as_deref(),
            Some(if changed { target } else { "com.example.app09" })
        );
        assert!(s.take_home_focus_changed().is_none());
        assert!(s.take_requests().is_empty());
    }
}

#[test]
fn home_focus_empty_packaged_shelf_then_fresh_absence_selects_first_eligible_favourite() {
    let _serial = plx_base::testlock::serial();
    for order in [
        Vec::new(),
        vec![
            "com.example.app11".to_owned(),
            "com.example.app10".to_owned(),
            "com.example.app09".to_owned(),
        ],
    ] {
        let mut packaged = fixture();
        packaged.applications.retain(|a| {
            !matches!(
                a.id.as_str(),
                "com.example.app09" | "com.example.app10" | "com.example.app11"
            )
        });
        packaged.launch_ids.retain(|id| {
            !matches!(
                id.as_str(),
                "com.example.app09" | "com.example.app10" | "com.example.app11"
            )
        });
        let prefs = Preferences {
            initialized: true,
            favourites: [
                "com.example.app09".into(),
                "com.example.app10".into(),
                "com.example.app11".into(),
            ]
            .into(),
            favourite_order: order.clone(),
        };
        let mut s = Shell::new(packaged, prefs);
        assert!(!s.restore_home_focus("com.example.app09"));
        assert_eq!(focus(&s), HOME_APPS_LINK);
        let mut fresh = live_update(100);
        fresh.simulated = false;
        if let crate::live::Reading::Value(c) = &mut fresh.catalogue {
            c.apps.retain(|a| a.id != "com.example.app09");
            c.launch_ids.remove("com.example.app09");
        }
        s.apply_live(&fresh, 100);
        assert_eq!(
            s.focused_item().as_deref(),
            Some(if order.is_empty() {
                "com.example.app10"
            } else {
                "com.example.app11"
            })
        );
        assert!(!s.home_focus_restore_pending());
        assert!(!s.home_focus_restored_from_live());
        assert!(s.take_home_focus_changed().is_none());
        assert!(!s.take_preferences_changed());
        assert!(s.take_requests().is_empty());
    }
}
#[test]
fn home_focus_initial_fallback_is_explicit_first_favourite_not_existing_programmatic_focus() {
    let _serial = plx_base::testlock::serial();
    let mut s = Shell::new(fixture(), Preferences::default());
    s.take_preferences_changed();
    assert!(s.focus_application("com.example.app03"));
    assert!(!s.restore_home_focus("com.example.gone"));
    assert_eq!(s.focused_item().as_deref(), Some("com.example.app00"));
    assert!(s.take_home_focus_changed().is_none());
    assert!(!s.take_preferences_changed());
    assert!(s.take_requests().is_empty());
}
#[test]
fn home_focus_fresh_hidden_unregistered_and_unpinned_saved_id_use_eligible_fallback() {
    let _serial = plx_base::testlock::serial();
    for variant in 0..3 {
        let mut packaged = fixture();
        packaged.applications.clear();
        packaged.launch_ids.clear();
        let prefs = Preferences {
            initialized: true,
            favourites: if variant == 2 {
                ["com.example.app10".into()].into()
            } else {
                ["com.example.app09".into(), "com.example.app10".into()].into()
            },
            favourite_order: Vec::new(),
        };
        let mut s = Shell::new(packaged, prefs);
        assert!(!s.restore_home_focus("com.example.app09"));
        let mut fresh = live_update(100);
        fresh.simulated = false;
        if let crate::live::Reading::Value(c) = &mut fresh.catalogue {
            if variant == 0 {
                c.apps
                    .iter_mut()
                    .find(|a| a.id == "com.example.app09")
                    .unwrap()
                    .visible = false;
            }
            if variant == 1 {
                c.launch_ids.remove("com.example.app09");
            }
        }
        s.apply_live(&fresh, 100);
        assert_eq!(s.focused_item().as_deref(), Some("com.example.app10"));
        assert!(!s.home_focus_restore_pending());
        assert!(!s.home_focus_restored_from_live());
        assert!(s.take_home_focus_changed().is_none());
        assert!(s.take_requests().is_empty());
    }
}
#[test]
fn home_focus_fresh_catalogue_with_explicit_empty_favourites_stays_on_view_all_without_seeding() {
    let _serial = plx_base::testlock::serial();
    let mut s = Shell::new(
        fixture(),
        Preferences {
            initialized: true,
            ..Preferences::default()
        },
    );
    assert!(!s.restore_home_focus("com.example.app09"));
    let mut fresh = live_update(100);
    fresh.simulated = false;
    s.apply_live(&fresh, 100);
    assert_eq!(focus(&s), HOME_APPS_LINK);
    assert!(s.preferences().favourites.is_empty());
    assert!(!s.home_focus_restore_pending());
    assert!(s.take_home_focus_changed().is_none());
    assert!(!s.take_preferences_changed());
    assert!(s.take_requests().is_empty());
}

#[test]
fn named_lg_task_links_queue_only_the_existing_general_or_sound_overview() {
    let _serial = plx_base::testlock::serial();
    for (page, keys, route) in [
        (
            Page::General,
            vec![1700, 1701, 1702, 1703],
            SettingsSection::General,
        ),
        (Page::Sound, vec![1706], SettingsSection::Sound),
    ] {
        for key in keys {
            let mut s = Shell::new(fixture(), Preferences::default());
            s.take_preferences_changed();
            s.open(page);
            s.focus_special(TABLE_BASE + key, GroupId(1));
            tap(&mut s);
            assert_eq!(
                s.take_requests(),
                vec![Action::OpenLgSettingsSection(route)]
            );
            assert!(!s.take_preferences_changed());
            assert!(!s.should_exit());
            assert_eq!(s.page(), page);
        }
    }
}
#[test]
fn named_app_and_memory_guides_are_focusable_read_only_and_back_keeps_category_hierarchy() {
    let _serial = plx_base::testlock::serial();
    for key in [1704, 1705] {
        let mut s = Shell::new(fixture(), Preferences::default());
        s.take_preferences_changed();
        s.open(Page::General);
        s.focus_special(TABLE_BASE + key, GroupId(1));
        assert_eq!(focus(&s), TABLE_BASE + key);
        tap(&mut s);
        assert!(s.take_requests().is_empty());
        assert!(!s.take_preferences_changed());
        s.back();
        assert_eq!(s.control_category(), Some(Page::General));
        assert_eq!(s.focused_item().as_deref(), Some("category-General"));
        s.back();
        assert_eq!(s.page(), Page::Home);
        assert!(!s.should_exit());
    }
}
#[test]
fn named_task_focus_survives_new_native_readout_and_never_becomes_an_editor() {
    let _serial = plx_base::testlock::serial();
    let mut s = Shell::new(fixture(), Preferences::default());
    s.take_preferences_changed();
    s.open(Page::General);
    s.focus_special(TABLE_BASE + 1702, GroupId(1));
    let before = s.snapshot.clone();
    s.snapshot
        .settings
        .entry("general".into())
        .or_default()
        .push(Setting {
            id: "fixture.reading".into(),
            focus_key: 1599,
            label: "Fixture reading".into(),
            value: "On".into(),
            context: s.snapshot.context.clone(),
            protected: true,
            available: true,
            constraint: None,
        });
    s.refresh_shown_data(&before);
    assert_eq!(focus(&s), TABLE_BASE + 1702);
    tap(&mut s);
    assert_eq!(
        s.take_requests(),
        vec![Action::OpenLgSettingsSection(SettingsSection::General)]
    );
    assert!(!s.take_preferences_changed());
}

#[test]
fn observed_and_noop_renderer_have_identical_painter_and_hit_state_trace() {
    use crate::draw_diagnostics::{Clock, Phase, Probe, Timed};
    use std::{cell::Cell, rc::Rc};
    let _serial = plx_base::testlock::serial();
    struct FakeClock(Rc<Cell<u64>>);
    impl Clock for FakeClock {
        fn micros(&mut self) -> u64 {
            let n = self.0.get();
            self.0.set(n + 1);
            n
        }
    }
    struct Trace<P> {
        inner: P,
        cuts: Vec<Phase>,
    }
    impl<P: Probe> Probe for Trace<P> {
        fn cut(&mut self, p: Phase) {
            self.cuts.push(p);
            self.inner.cut(p);
        }
    }
    #[derive(Clone, Copy, Debug)]
    enum Scene {
        Page(Page),
        HomeMenu,
        RestoredBookmark,
        GuideForm,
    }
    for scene in [
        Scene::Page(Page::Home),
        Scene::Page(Page::Applications),
        Scene::Page(Page::Inputs),
        Scene::Page(Page::Controls),
        Scene::HomeMenu,
        Scene::RestoredBookmark,
        Scene::GuideForm,
    ] {
        let mut normal = Shell::new(fixture(), Preferences::default());
        let mut observed = Shell::new(fixture(), Preferences::default());
        for shell in [&mut normal, &mut observed] {
            // Discard only the constructor's expected initial seed notification.
            shell.take_preferences_changed();
            match scene {
                Scene::Page(page) => shell.rebuild(page),
                Scene::HomeMenu => {
                    assert!(shell.focus_application("com.example.app03"));
                    shell.press(true, PressFrom::Key);
                    for ms in (0..=800).step_by(16) {
                        shell.tick(ms, 0.016);
                        shell.repeat();
                    }
                    shell.press(false, PressFrom::Key);
                    for ms in (816..1200).step_by(16) {
                        shell.tick(ms, 0.016);
                    }
                    assert_eq!(shell.page(), Page::Home);
                    assert_eq!(shell.menu_application(), Some("com.example.app03"));
                    // The deliberate hold records the card once; rendering must not.
                    assert_eq!(
                        shell.take_home_focus_changed().as_deref(),
                        Some("com.example.app03")
                    );
                }
                Scene::RestoredBookmark => {
                    assert!(shell.restore_home_focus("com.example.app03"));
                    let mut fresh = live_update(100);
                    fresh.simulated = false;
                    shell.apply_live(&fresh, 100);
                    assert!(shell.home_focus_restored_from_live());
                    assert_eq!(shell.page(), Page::Home);
                    assert_eq!(shell.focused_item().as_deref(), Some("com.example.app03"));
                    assert!(!shell.home_focus_restore_pending());
                }
                Scene::GuideForm => {
                    shell.open(Page::General);
                    shell.focus_special(TABLE_BASE + 1704, GroupId(1));
                    tap(shell);
                    assert_eq!(shell.control_category(), Some(Page::General));
                    assert_eq!(focus(shell), TABLE_BASE + 1704);
                }
            }
            if !matches!(scene, Scene::HomeMenu) {
                settle(shell, 100);
            }
            assert!(
                !shell.take_preferences_changed(),
                "setup changed favourites: {scene:?}"
            );
            assert!(shell.take_home_focus_changed().is_none());
            assert!(shell.take_requests().is_empty());
        }
        let preferences = normal.preferences().clone();
        let focused = normal.focused_item();
        let menu = normal.menu_application().map(str::to_owned);
        let before = normal.presentation_probe();
        assert_eq!(before, observed.presentation_probe());
        let plain = plx_ui::draw_census::capture(|| {
            plx_gfx::gfx::without_frame_clear(|| {
                plx_ui::screen::record_stops_while_recording(|| {
                    plx_ui::record_walk(|| normal.draw())
                })
            })
        });
        let reads = Rc::new(Cell::new(0));
        let mut probe = Trace {
            inner: Timed::new(FakeClock(reads.clone())),
            cuts: Vec::new(),
        };
        let seen = plx_ui::draw_census::capture(|| {
            plx_gfx::gfx::without_frame_clear(|| {
                plx_ui::screen::record_stops_while_recording(|| {
                    plx_ui::record_walk(|| observed.draw_observed(&mut probe))
                })
            })
        });
        assert_eq!(plain, seen);
        assert!(!plain.is_empty());
        assert_eq!(normal.presentation_probe(), observed.presentation_probe());
        let hit_state = |shell: &Shell| {
            shell
                .input
                .hit
                .front()
                .iter()
                .map(|stop| {
                    let rect = stop.rect;
                    let rest = stop.rest_rect;
                    let clip = stop.clip;
                    (
                        stop.key,
                        [
                            rect.x, rect.y, rect.w, rect.h, rest.x, rest.y, rest.w, rest.h, clip.x,
                            clip.y, clip.w, clip.h,
                        ]
                        .map(f32::to_bits),
                        stop.hover,
                        stop.activate,
                    )
                })
                .collect::<Vec<_>>()
        };
        let normal_hits = hit_state(&normal);
        assert!(
            !normal_hits.is_empty(),
            "recording omitted hit stops: {scene:?}"
        );
        assert_eq!(
            normal_hits,
            hit_state(&observed),
            "draw changed ordered hit stops: {scene:?}"
        );
        assert_eq!(
            probe.cuts,
            [Phase::Clear, Phase::Chrome, Phase::Content, Phase::Finalize]
        );
        assert_eq!(reads.get(), 5);
        let _ = probe.inner.finish();
        for shell in [&mut normal, &mut observed] {
            assert_eq!(
                shell.preferences(),
                &preferences,
                "draw changed favourites: {scene:?}"
            );
            assert_eq!(
                shell.focused_item(),
                focused,
                "draw changed focus: {scene:?}"
            );
            assert_eq!(shell.menu_application(), menu.as_deref());
            assert!(
                !shell.take_preferences_changed(),
                "draw queued favourites: {scene:?}"
            );
            assert!(
                shell.take_home_focus_changed().is_none(),
                "draw queued bookmark: {scene:?}"
            );
            assert!(
                shell.take_requests().is_empty(),
                "draw queued action: {scene:?}"
            );
            assert!(!shell.should_exit());
        }
    }
}
