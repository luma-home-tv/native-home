//! Synthetic observations exercise the real merge, expiry and shared native form.
use super::*;
use crate::model::SettingsSection;
use plx_ui::form::FormTable;
use serde_json::json;

fn source() -> Value {
    let mut raw = tests::global_raw();
    raw["replies"][crate::caption_reads::VALUES_REPLY] = json!({"returnValue":true,
        "subscribed":false,"category":"caption","method":"getSystemSettings",
        "settings":{"captionEnable":"on"}});
    raw
}
fn snapshot() -> Snapshot {
    Snapshot {
        captured_at: "synthetic caption lifecycle".into(),
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
fn caption(snapshot: &Snapshot) -> &Setting {
    let rows = &snapshot.settings["accessibility"];
    assert_eq!(rows.len(), 8);
    assert_eq!(
        rows.iter().map(|r| r.focus_key).collect::<Vec<_>>(),
        (100..=107).collect::<Vec<_>>()
    );
    &rows[7]
}
#[test]
fn fresh_caption_does_not_reuse_failed_legacy_readings_and_legacy_cannot_revive_caption() {
    let mut raw = source();
    let mut snapshot = snapshot();
    let mut fresh = Freshness::default();
    apply(&mut snapshot, &mut fresh, &raw, 100);
    assert!(snapshot.settings["accessibility"]
        .iter()
        .all(|r| r.available));
    raw["replies"]["accessibility_values"] = json!({"returnValue":false});
    apply(&mut snapshot, &mut fresh, &raw, 200);
    assert!(snapshot.settings["accessibility"][..7]
        .iter()
        .all(|r| !r.available));
    assert!(caption(&snapshot).available);
    assert_eq!(
        fresh.controls.get(&ControlSection::Accessibility),
        Some(&200)
    );
    raw = source();
    raw["replies"][crate::caption_reads::VALUES_REPLY] = json!({"timeout":true});
    apply(&mut snapshot, &mut fresh, &raw, 300);
    assert!(snapshot.settings["accessibility"][..7]
        .iter()
        .all(|r| r.available));
    assert!(!caption(&snapshot).available);
    assert_eq!(caption(&snapshot).value, "Unavailable");
    assert_eq!(
        fresh.controls.get(&ControlSection::Accessibility),
        Some(&300)
    );
}
#[test]
fn all_failed_still_supplies_visible_row_and_withdraws_group_freshness() {
    let mut snapshot = snapshot();
    let mut fresh = Freshness::default();
    apply(&mut snapshot, &mut fresh, &json!({"replies":{}}), 100);
    assert!(!caption(&snapshot).available);
    assert!(snapshot.settings["accessibility"]
        .iter()
        .all(|r| !r.available && r.protected));
    assert!(!fresh.controls.contains_key(&ControlSection::Accessibility));
    apply(&mut snapshot, &mut fresh, &source(), 200);
    assert!(caption(&snapshot).available);
    apply(
        &mut snapshot,
        &mut fresh,
        &json!({"replies":{"system_info":{"returnValue":true,"captionEnable":"on"}}}),
        300,
    );
    assert!(!caption(&snapshot).available);
    assert!(!fresh.controls.contains_key(&ControlSection::Accessibility));
}
#[test]
fn expiry_and_unrelated_refresh_cannot_restore_a_cached_caption() {
    let mut snapshot = snapshot();
    let mut fresh = Freshness::default();
    let mut raw = source();
    apply(&mut snapshot, &mut fresh, &raw, 100);
    expire(&mut snapshot, &fresh, 15_100);
    assert!(caption(&snapshot).available);
    expire(&mut snapshot, &fresh, 15_101);
    assert!(!caption(&snapshot).available);
    raw["replies"]
        .as_object_mut()
        .unwrap()
        .remove(crate::caption_reads::VALUES_REPLY);
    apply(&mut snapshot, &mut fresh, &raw, 15_200);
    assert!(!caption(&snapshot).available);
    assert_eq!(caption(&snapshot).value, "Unavailable");
    raw["replies"][crate::caption_reads::VALUES_REPLY] = json!({"returnValue":false});
    apply(&mut snapshot, &mut fresh, &raw, 15_300);
    assert!(!caption(&snapshot).available);
}
#[test]
fn actual_accessibility_form_displays_value_without_activation_or_write_constraint() {
    let mut snapshot = snapshot();
    let mut fresh = Freshness::default();
    apply(&mut snapshot, &mut fresh, &source(), 100);
    let current = caption(&snapshot);
    assert!(current.protected && current.constraint.is_none());
    let mut table = FormTable::new(2000);
    table.set(
        crate::model::controls_form_for_section(
            &snapshot.settings["accessibility"],
            &snapshot.context,
            Some(SettingsSection::Accessibility),
        ),
        None,
    );
    assert_eq!(
        table.table.sections[1].rows[7].label,
        "TV caption preference"
    );
    assert_eq!(table.table.sections[1].rows[7].value.as_deref(), Some("On"));
    assert_eq!(
        table.activate(table.index_of(&"captionEnable".into()).unwrap()),
        None
    );
    let mut other = snapshot.context.clone();
    other.input = "HDMI_2".into();
    table.set(
        crate::model::controls_form_for_section(
            &snapshot.settings["accessibility"],
            &other,
            Some(SettingsSection::Accessibility),
        ),
        None,
    );
    assert_eq!(
        table.table.sections[1].rows[7].value.as_deref(),
        Some("Unavailable")
    );
    assert_eq!(
        table.activate(table.index_of(&"captionEnable".into()).unwrap()),
        None
    );
    expire(&mut snapshot, &fresh, 15_101);
    table.set(
        crate::model::controls_form_for_section(
            &snapshot.settings["accessibility"],
            &snapshot.context,
            Some(SettingsSection::Accessibility),
        ),
        None,
    );
    assert_eq!(
        table.table.sections[1].rows[7].value.as_deref(),
        Some("Unavailable")
    );
    assert_eq!(
        table.activate(table.index_of(&"captionEnable".into()).unwrap()),
        None
    );
}
