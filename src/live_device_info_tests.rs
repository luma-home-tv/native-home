//! The existing collector/merge and actual shared form, with synthetic readings.
use super::*;
use crate::model::{Action, ControlConstraint, SettingsSection};
use plx_ui::form::{Activation, FormTable};
use serde_json::json;

fn full() -> Value {
    let mut raw = tests::global_raw();
    raw["replies"]["system_info"] = json!({"returnValue":true,
        "modelName":"Example OLED 55","firmwareVersion":"24.10.01","sdkVersion":"10.0.0"});
    raw["replies"][crate::locale_reads::LOCALE_VALUES_REPLY] = json!({
        "returnValue":true,"settings":{"localeInfo":{"locales":{
            "UI":"en-GB","AUD":"fr-FR","AUD2":"de_DE","FMT":"en-GB"},
            "clock":"locale","timezone":"Europe/London"}}});
    raw["replies"][crate::locale_reads::REGION_VALUES_REPLY] = json!({
        "returnValue":true,"category":"option",
        "settings":{"country":"GBR","smartServiceCountryCode2":"GB"}});
    raw["replies"]["general_option_values"]["settings"]["pointerSpeed"] = "normal".into();
    let descriptors = raw["replies"]["general_option_desc"]["results"]
        .as_array_mut()
        .unwrap();
    let pointer = descriptors
        .iter_mut()
        .find(|d| d["key"] == "pointerSpeed")
        .unwrap();
    pointer["values"]["arrayExt"] = json!([
        {"value":"slow","active":true,"visible":true},
        {"value":"normal","active":true,"visible":true},
        {"value":"fast","active":true,"visible":true}]);
    raw
}
fn snapshot() -> Snapshot {
    Snapshot {
        captured_at: "synthetic information transitions".into(),
        applications: Vec::new(),
        launch_ids: BTreeSet::new(),
        inputs: Vec::new(),
        context: Context {
            input: "HDMI_1".into(),
            dynamic_range: "sdr".into(),
            picture_mode: "cinema".into(),
        },
        settings: BTreeMap::new(),
    }
}
fn apply(snapshot: &mut Snapshot, fresh: &mut Freshness, raw: &Value, now: u32) {
    merge(snapshot, &parse(raw, "com.example.self", now, false), fresh);
}
fn refuse_legacy(raw: &mut Value) {
    for name in [
        "general_option_values",
        "general_home_values",
        crate::locale_reads::LOCALE_VALUES_REPLY,
        crate::locale_reads::REGION_VALUES_REPLY,
    ] {
        raw["replies"][name] = json!({"returnValue":false});
    }
}
fn identity(snapshot: &Snapshot) {
    let rows = &snapshot.settings["general"];
    assert_eq!(rows.len(), 16);
    assert_eq!(
        rows.iter().map(|r| r.focus_key).collect::<Vec<_>>(),
        (140..=149).chain(153..=155).chain(150..=152).collect::<Vec<_>>()
    );
    for (row, key) in rows[13..].iter().zip(crate::device_info_reads::ALL) {
        assert_eq!(row.id, key.id());
        assert_eq!(row.label, key.label());
        assert!(row.protected && row.constraint.is_none());
    }
    assert!(rows
        .iter()
        .all(|r| r.protected && r.context == snapshot.context));
}

#[test]
fn actual_general_form_keeps_information_inert_and_existing_routes_intact() {
    let _serial = plx_base::testlock::serial();
    let mut snapshot = snapshot();
    let mut fresh = Freshness::default();
    apply(&mut snapshot, &mut fresh, &full(), 100);
    identity(&snapshot);
    let rows = &snapshot.settings["general"];
    assert!(rows.iter().all(|r| r.available));
    for pointer_enabled in [false, true] {
        let mut form = FormTable::new(2000);
        form.set(
            crate::model::controls_form_with_numeric(
                rows,
                &snapshot.context,
                Some(SettingsSection::General),
                pointer_enabled,
                None,
            ),
            None,
        );
        for (index, setting) in rows.iter().enumerate().skip(13) {
            let position = form.index_of(&setting.id).unwrap();
            let binding = form.binding_at(position).unwrap();
            assert_eq!(binding.id, setting.id);
            assert_eq!(binding.action, Action::Inspect(setting.id.clone()));
            assert!(binding.disabled);
            assert_eq!(form.activate(position), None);
            let row = &form.table.sections[1].rows[index];
            assert_eq!(row.value.as_deref(), Some(setting.value.as_str()));
            assert!(!row.dim && row.ticon.is_none());
        }
        assert_eq!(
            form.activate(form.index_of(&"pointerSpeed".into()).unwrap()),
            if pointer_enabled {
                Some(Activation::Action(Action::OpenPointerSpeed))
            } else {
                None
            }
        );
        assert_eq!(
            form.activate(form.index_of(&"lg-settings".into()).unwrap()),
            Some(Activation::Action(Action::OpenLgSettingsSection(
                SettingsSection::General
            )))
        );
    }
}

#[test]
fn failed_info_cannot_inherit_a_fresh_legacy_general_clock() {
    let mut snapshot = snapshot();
    let mut fresh = Freshness::default();
    apply(&mut snapshot, &mut fresh, &full(), 100);
    let mut failed = full();
    failed["replies"]["system_info"]["returnValue"] = false.into();
    apply(&mut snapshot, &mut fresh, &failed, 200);
    identity(&snapshot);
    assert_eq!(fresh.controls.get(&ControlSection::General), Some(&200));
    assert!(snapshot.settings["general"][..13]
        .iter()
        .all(|r| r.available));
    assert!(snapshot.settings["general"][13..]
        .iter()
        .all(|r| !r.available && r.value == "Unavailable"));
}

#[test]
fn valid_info_cannot_resurrect_a_failed_pointer_or_locale_read() {
    let _serial = plx_base::testlock::serial();
    let mut snapshot = snapshot();
    let mut fresh = Freshness::default();
    apply(&mut snapshot, &mut fresh, &full(), 100);
    let mut partial = full();
    refuse_legacy(&mut partial);
    apply(&mut snapshot, &mut fresh, &partial, 200);
    identity(&snapshot);
    assert!(snapshot.settings["general"][..13]
        .iter()
        .all(|r| !r.available && r.value == "Unavailable"));
    assert!(snapshot.settings["general"][13..]
        .iter()
        .all(|r| r.available));
    assert_eq!(fresh.controls.get(&ControlSection::General), Some(&200));
    let mut form = FormTable::new(2000);
    form.set(
        crate::model::controls_form_with_numeric(
            &snapshot.settings["general"],
            &snapshot.context,
            Some(SettingsSection::General),
            true,
            None,
        ),
        None,
    );
    assert!(form.binding_at(1).unwrap().disabled);
    assert_eq!(
        form.activate(form.index_of(&"pointerSpeed".into()).unwrap()),
        None
    );
}

#[test]
fn total_refusal_expiry_and_partial_recovery_keep_stable_context_and_no_old_values() {
    let mut snapshot = snapshot();
    let mut fresh = Freshness::default();
    apply(&mut snapshot, &mut fresh, &full(), 100);
    let mut failed = full();
    refuse_legacy(&mut failed);
    failed["replies"]["system_info"]["returnValue"] = false.into();
    apply(&mut snapshot, &mut fresh, &failed, 200);
    assert!(!fresh.controls.contains_key(&ControlSection::General));
    assert!(snapshot.settings["general"].iter().all(|r| !r.available));
    snapshot.context = Context {
        input: "HDMI_3".into(),
        dynamic_range: "hdr".into(),
        picture_mode: "new-mode".into(),
    };
    let mut recovered = full();
    refuse_legacy(&mut recovered);
    recovered["replies"]["system_info"]["modelName"] = "New Example Model".into();
    apply(&mut snapshot, &mut fresh, &recovered, 15_300);
    identity(&snapshot);
    assert_eq!(snapshot.settings["general"][13].value, "New Example Model");
    assert!(snapshot.settings["general"][..13]
        .iter()
        .all(|r| !r.available && r.value == "Unavailable"));
    assert!(!needs_expiry(&snapshot, &fresh, 30_300));
    assert!(needs_expiry(&snapshot, &fresh, 30_301));
    expire(&mut snapshot, &fresh, 30_301);
    assert!(snapshot.settings["general"].iter().all(|r| !r.available));
}

#[test]
fn display_validity_never_qualifies_an_unknown_platform_for_keyboard_or_settings() {
    let raw = full();
    let update = parse(&raw, "com.example.self", 100, false);
    assert!(!update.system_keyboard);
    assert!(!crate::keyboard::qualified(&raw["replies"]["system_info"]));
    let Reading::Value(rows) = &update.controls[&ControlSection::General] else {
        panic!("info is displayable")
    };
    assert!(rows[13..]
        .iter()
        .all(|r| r.available && r.protected && r.constraint.is_none()));
    // A readable value supplies no new choice/range or write capability.
    assert!(!rows[13..].iter().any(|r| matches!(
        r.constraint,
        Some(ControlConstraint::Choices { .. } | ControlConstraint::Range { .. })
    )));
}
