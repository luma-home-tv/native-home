use super::*;
use serde_json::json;

fn replies() -> Value {
    json!({"system_info": {
        "returnValue": true,
        "modelName": "Example OLED 55",
        "firmwareVersion": "24.10.01",
        "sdkVersion": "10.0.0"
    }})
}

#[test]
fn wire_fields_have_stable_read_only_identities_without_version_inference() {
    let rows = parse(&replies());
    assert_eq!(ALL, Key::ALL);
    assert_eq!(rows.len(), 3);
    assert_eq!(
        rows.iter()
            .map(|row| (
                row.id.as_str(),
                row.label.as_str(),
                row.focus_key,
                row.value.as_str()
            ))
            .collect::<Vec<_>>(),
        vec![
            ("modelName", "TV model", 150, "Example OLED 55"),
            ("firmwareVersion", "Software version", 151, "24.10.01"),
            ("sdkVersion", "webOS SDK version", 152, "10.0.0"),
        ]
    );
    assert!(rows.iter().all(|row| row.available
        && row.protected
        && row.constraint.is_none()
        && row.context
            == Context {
                input: "global".into(),
                dynamic_range: "global".into(),
                picture_mode: "global".into()
            }));
}

#[test]
fn missing_or_unsuccessful_envelopes_never_turn_values_into_available_rows() {
    let examples = [
        Value::Null,
        json!([]),
        json!({}),
        json!({"system_info": null}),
        json!({"system_info": []}),
        json!({"system_info": {"modelName":"Example OLED 55"}}),
        json!({"system_info": {"returnValue":false,"modelName":"Example OLED 55","firmwareVersion":"24.10.01","sdkVersion":"10.0.0"}}),
        json!({"system_info": {"returnValue":1,"modelName":"Example OLED 55"}}),
        json!({"system_info": {"returnValue":"true","modelName":"Example OLED 55"}}),
        json!({"replies":replies()}),
    ];
    for raw in examples {
        let rows = parse(&raw);
        assert_eq!(rows.len(), 3);
        assert!(rows.iter().all(|row| !row.available
            && row.value == "Unavailable"
            && row.protected
            && row.constraint.is_none()));
    }
}

#[test]
fn malformed_or_missing_individual_field_does_not_poison_its_peers() {
    let bad = [
        Value::Null,
        json!(true),
        json!(123),
        json!([]),
        json!({}),
        json!(""),
        json!("   "),
    ];
    for key in ALL {
        for value in &bad {
            let mut raw = replies();
            raw["system_info"][key.id()] = value.clone();
            let rows = parse(&raw);
            for row in rows {
                if row.id == key.id() {
                    assert!(!row.available);
                    assert_eq!(row.value, "Unavailable");
                } else {
                    assert!(row.available);
                }
            }
        }
        let mut raw = replies();
        raw["system_info"].as_object_mut().unwrap().remove(key.id());
        let rows = parse(&raw);
        assert_eq!(rows.iter().filter(|row| row.available).count(), 2);
        assert!(
            !rows
                .iter()
                .find(|row| row.id == key.id())
                .unwrap()
                .available
        );
    }
}

#[test]
fn encoded_byte_bound_and_nonprinting_values_are_checked_independently() {
    for key in ALL {
        let mut raw = replies();
        raw["system_info"][key.id()] = "A".repeat(MAX_VALUE_BYTES).into();
        assert!(
            parse(&raw)
                .iter()
                .find(|row| row.id == key.id())
                .unwrap()
                .available
        );
        for text in [
            "A".repeat(MAX_VALUE_BYTES + 1),
            "one\ntwo".into(),
            "tab\tvalue".into(),
            "\0".into(),
            "\u{7f}".into(),
            "\u{1b}[31m".into(),
            "\u{2028}".into(),
            "\u{200b}".into(),
            "modèle".into(),
        ] {
            raw["system_info"][key.id()] = text.into();
            let rows = parse(&raw);
            assert_eq!(rows.iter().filter(|row| row.available).count(), 2);
            assert_eq!(
                rows.iter().find(|row| row.id == key.id()).unwrap().value,
                "Unavailable"
            );
        }
    }
}

#[test]
fn unknown_fields_and_other_reply_domains_are_not_displayed_or_used_as_fallbacks() {
    let mut raw = replies();
    raw["system_info"]["marketingVersion"] = "webOS 24".into();
    raw["system_info"]["cpu"] = "invented CPU".into();
    raw["system_info"]["ram"] = "999 GB".into();
    raw["system_info"]["unrelated"] = json!({"large":"X".repeat(4096)});
    raw["other"] = json!({"modelName":"different model","sdkVersion":"99"});
    assert_eq!(parse(&raw), parse(&replies()));
    raw["system_info"]
        .as_object_mut()
        .unwrap()
        .remove("sdkVersion");
    let rows = parse(&raw);
    assert!(!rows[2].available);
    assert_eq!(rows[2].value, "Unavailable");
    assert_eq!(rows.len(), 3);
}

#[test]
fn supplied_strings_remain_verbatim_with_no_model_or_version_allowlist() {
    let raw = json!({"system_info":{"returnValue":true,"modelName":"Different Model X-1","firmwareVersion":"05.00.02 build-A","sdkVersion":"new-preview.1"}});
    let rows = parse(&raw);
    assert_eq!(
        rows.iter()
            .map(|row| row.value.as_str())
            .collect::<Vec<_>>(),
        vec!["Different Model X-1", "05.00.02 build-A", "new-preview.1"]
    );
    assert!(rows
        .iter()
        .all(|row| row.available && row.protected && row.constraint.is_none()));
}
