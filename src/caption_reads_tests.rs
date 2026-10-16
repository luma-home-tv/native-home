use super::*;
use serde_json::json;

fn reply(value: Value) -> Value {
    json!({(VALUES_REPLY):{"subscribed":false,"category":"caption","method":"getSystemSettings",
        "returnValue":true,"settings":{"captionEnable":value}}})
}
#[test]
fn ordinary_observed_on_and_documented_off_have_stable_read_only_identity() {
    for value in ["on", "off"] {
        let row = parse(&reply(json!(value)));
        assert_eq!(
            (row.id.as_str(), row.label.as_str(), row.focus_key),
            ("captionEnable", "TV caption preference", 107)
        );
        assert_eq!(row.value, value);
        assert!(row.available && row.protected && row.constraint.is_none());
    }
    let (_, uri, payload) = operations().into_iter().next().unwrap();
    assert_eq!(uri, "luna://com.webos.settingsservice/getSystemSettings");
    assert_eq!(
        payload,
        json!({"category":"caption","keys":["captionEnable"],"subscribe":false})
    );
}
#[test]
fn missing_and_collector_failure_replies_never_use_another_domain() {
    for bad in [
        Value::Null,
        json!({}),
        json!({"returnValue":false}),
        json!({"oversize":true}),
        json!({"parse_error":true}),
        json!({"timeout":true}),
        json!({"setup_error":"call"}),
    ] {
        let raw = json!({(VALUES_REPLY):bad,"accessibility_values":{"settings":{"captionEnable":"on"}},
            "system_info":{"captionEnable":"on"}});
        let row = parse(&raw);
        assert!(!row.available);
        assert_eq!(row.value, "Unavailable");
    }
}
#[test]
fn malformed_envelopes_and_mixed_errors_cannot_admit_a_caption_value() {
    for (key, value) in [
        ("returnValue", json!("true")),
        ("category", json!("option")),
        ("method", json!("getSystemSettingDesc")),
        ("subscribed", json!(true)),
        ("dimension", json!({"input":"HDMI_1"})),
        ("dimension", Value::Null),
        ("dimension", json!([])),
        ("oversize", json!(true)),
        ("errorCode", json!(0)),
        (
            "settings",
            json!({"captionEnable":"on","captionSize":"small"}),
        ),
    ] {
        let mut raw = reply(json!("on"));
        raw[VALUES_REPLY][key] = value;
        assert!(!parse(&raw).available, "unexpected admission for {key}");
    }
    for key in [
        "returnValue",
        "category",
        "method",
        "subscribed",
        "settings",
    ] {
        let mut raw = reply(json!("on"));
        raw[VALUES_REPLY].as_object_mut().unwrap().remove(key);
        assert!(!parse(&raw).available, "missing {key}");
    }
    let mut raw = reply(json!("on"));
    raw[VALUES_REPLY]["dimension"] = json!({});
    assert!(parse(&raw).available);
}
#[test]
fn unknown_or_wrong_typed_values_remain_unavailable_without_enum_guessing() {
    for value in [
        json!(true),
        json!(1),
        Value::Null,
        json!([]),
        json!({}),
        json!(""),
        json!("ON"),
        json!("Off"),
        json!("auto"),
        json!("on\n"),
        json!("x".repeat(2 * 1024 * 1024 + 1)),
    ] {
        let row = parse(&reply(value));
        assert!(!row.available);
        assert_eq!(row.value, "Unavailable");
    }
}
