//! Fixed read-only control registry. Descriptors validate readouts; they never grant write authority.
use crate::model::{Context, ControlConstraint, Setting};
use serde_json::{json, Value};
use std::collections::BTreeSet;

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum Section {
    Accessibility,
    General,
    PanelCare,
}
impl Section {
    pub fn name(self) -> &'static str {
        match self {
            Self::Accessibility => "accessibility",
            Self::General => "general",
            Self::PanelCare => "panel_care",
        }
    }
}
#[derive(Clone, Copy)]
pub struct Spec {
    pub id: &'static str,
    pub label: &'static str,
    pub key: u32,
}
pub const PICTURE: &[Spec] = &[
    Spec {
        id: "color",
        label: "Colour depth",
        key: 5,
    },
    Spec {
        id: "sharpness",
        label: "Sharpness",
        key: 6,
    },
    Spec {
        id: "tint",
        label: "Tint",
        key: 7,
    },
    Spec {
        id: "colorTemperature",
        label: "Colour temperature (LG scale)",
        key: 8,
    },
    Spec {
        id: "colorGamut",
        label: "Colour gamut",
        key: 9,
    },
    Spec {
        id: "gamma",
        label: "Gamma profile",
        key: 10,
    },
    Spec {
        id: "dynamicContrast",
        label: "Dynamic contrast",
        key: 11,
    },
    Spec {
        id: "peakBrightness",
        label: "Peak brightness",
        key: 12,
    },
];
pub const ACCESSIBILITY: &[Spec] = &[
    Spec {
        id: "audioGuidance",
        label: "Audio guidance",
        key: 100,
    },
    Spec {
        id: "highContrast",
        label: "High contrast",
        key: 101,
    },
    Spec {
        id: "screenMagnification",
        label: "Screen magnification",
        key: 102,
    },
    Spec {
        id: "menuTransparency",
        label: "Menu transparency",
        key: 103,
    },
    Spec {
        id: "audioGuidanceVolume",
        label: "Audio guidance volume",
        key: 104,
    },
    Spec {
        id: "audioGuidancePitch",
        label: "Audio guidance pitch",
        key: 105,
    },
    Spec {
        id: "audioGuidanceSpeed",
        label: "Audio guidance speed",
        key: 106,
    },
];
pub const GENERAL_OPTION: &[Spec] = &[
    Spec {
        id: "quickStartMode",
        label: "Quick Start+",
        key: 140,
    },
    Spec {
        id: "pointerSpeed",
        label: "Pointer speed",
        key: 141,
    },
    Spec {
        id: "pointerSize",
        label: "Pointer size",
        key: 142,
    },
];
pub const GENERAL_HOME: &[Spec] = &[
    Spec {
        id: "homeAutoLaunch",
        label: "LG Home auto launch",
        key: 143,
    },
    Spec {
        id: "homePromotion",
        label: "LG Home promotions",
        key: 144,
    },
];
pub const PANEL_CARE: &[Spec] = &[
    Spec {
        id: "screenShift",
        label: "Screen Move",
        key: 180,
    },
    Spec {
        id: "logoLuminanceAdjust",
        label: "Logo brightness adjustment",
        key: 181,
    },
    Spec {
        id: "energySaving",
        label: "Energy saving",
        key: 182,
    },
];
pub fn operations() -> Vec<(String, &'static str, Value)> {
    let mut out = Vec::new();
    for (name, category, specs) in [
        ("picture_extra", "picture", PICTURE),
        ("accessibility", "option", ACCESSIBILITY),
        ("general_option", "option", GENERAL_OPTION),
        ("general_home", "general", GENERAL_HOME),
        ("panel_care", "picture", PANEL_CARE),
    ] {
        let payload =
            json!({"category":category,"keys":specs.iter().map(|s|s.id).collect::<Vec<_>>()});
        out.push((
            format!("{name}_values"),
            "luna://com.webos.service.settings/getSystemSettings",
            payload.clone(),
        ));
        out.push((
            format!("{name}_desc"),
            "luna://com.webos.service.settings/getSystemSettingDesc",
            payload,
        ));
    }
    out
}
fn string(value: &Value) -> Option<String> {
    value
        .as_str()
        .filter(|s| !s.is_empty() && s.len() <= 64 && !s.chars().any(char::is_control))
        .map(str::to_owned)
}
fn integer(value: &Value) -> Option<i64> {
    value.as_i64().or_else(|| {
        value
            .as_str()
            .filter(|s| {
                s.len() <= 6
                    && !s.is_empty()
                    && s.bytes()
                        .enumerate()
                        .all(|(i, c)| c.is_ascii_digit() || (i == 0 && c == b'-'))
            })
            .and_then(|s| s.parse().ok())
    })
}
fn constraint(desc: &Value, current: &Value) -> Option<(ControlConstraint, String)> {
    let active = desc["ui"]["active"].as_bool()?;
    if desc["ui"]["visible"].as_bool() != Some(true) {
        return None;
    }
    match desc["vtype"].as_str()? {
        "Range" => {
            let r = &desc["values"]["range"];
            let minimum = r["min"].as_i64()?;
            let maximum = r["max"].as_i64()?;
            let step = r["interval"].as_i64()?;
            let value = integer(current)?;
            if minimum < -10_000
                || maximum > 10_000
                || minimum >= maximum
                || !(1..=1000).contains(&step)
                || !(minimum..=maximum).contains(&value)
                || (value - minimum) % step != 0
            {
                return None;
            }
            Some((
                ControlConstraint::Range {
                    minimum,
                    maximum,
                    step,
                    active,
                },
                value.to_string(),
            ))
        }
        "ArrayExt" => {
            let rows = desc["values"]["arrayExt"].as_array()?;
            if rows.is_empty() || rows.len() > 32 {
                return None;
            }
            let mut unique = BTreeSet::new();
            let mut values = Vec::new();
            for row in rows {
                let value = string(&row["value"])?;
                if !unique.insert(value.clone()) {
                    return None;
                }
                let visible = row["visible"].as_bool()?;
                let enabled = row["active"].as_bool()?;
                if visible && enabled {
                    values.push(value);
                }
            }
            let value = string(current)?;
            if !values.contains(&value) {
                return None;
            }
            Some((ControlConstraint::Choices { values, active }, value))
        }
        _ => None,
    }
}
/// These three global preferences are evidenced string enums, never numeric sliders.
/// Keep this validation local to their fixed identities; other registry schemas are unchanged.
fn registry_constraint(
    spec: &Spec,
    desc: &Value,
    current: &Value,
) -> Option<(ControlConstraint, String)> {
    let expected: Option<&[&str]> = match spec.id {
        "audioGuidanceVolume" | "audioGuidancePitch" => {
            Some(&["veryLow", "low", "medium", "high", "veryHigh"])
        }
        "audioGuidanceSpeed" => Some(&["veryFast", "fast", "medium", "slow", "verySlow"]),
        _ => None,
    };
    if let Some(expected) = expected {
        if desc["category"].as_str() != Some("option")
            || desc["vtype"].as_str() != Some("ArrayExt")
            || !desc["dimension"]
                .as_array()
                .is_some_and(|dims| dims.is_empty())
        {
            return None;
        }
        let values = desc["values"].as_object()?;
        if values.len() != 1 {
            return None;
        }
        let choices = values.get("arrayExt")?.as_array()?;
        if choices.len() != expected.len() {
            return None;
        }
        let mut seen = BTreeSet::new();
        for choice in choices {
            let value = choice["value"].as_str()?;
            if !expected.contains(&value)
                || !seen.insert(value)
                || choice["visible"].as_bool() != Some(true)
                || choice["active"].as_bool() != Some(true)
            {
                return None;
            }
        }
    }
    constraint(desc, current)
}
/// Picture extra reads bind to the current profile; these global domains require empty dimensions.
pub fn read(
    replies: &Value,
    name: &str,
    category: &str,
    specs: &[Spec],
    context: Option<&Context>,
) -> Result<Vec<Setting>, String> {
    let data = &replies[format!("{name}_values")];
    let desc = &replies[format!("{name}_desc")];
    if data["returnValue"].as_bool() != Some(true)
        || desc["returnValue"].as_bool() != Some(true)
        || data["category"].as_str() != Some(category)
    {
        return Err(format!("{name}: values/descriptors unavailable"));
    }
    let dims = &data["dimension"];
    let expected = if let Some(context) = context {
        let fields = dims.as_object().ok_or("Missing picture dimensions")?;
        if fields.len() < 2
            || fields.len() > 3
            || dims["input"].as_str() != Some(context.input.as_str())
            || dims["pictureMode"].as_str() != Some(context.picture_mode.as_str())
            || fields
                .keys()
                .any(|key| !matches!(key.as_str(), "input" | "pictureMode" | "dynamicRange"))
            || (fields.contains_key("dynamicRange")
                && dims["dynamicRange"].as_str() != Some(context.dynamic_range.as_str()))
        {
            return Err("Picture extra context mismatch".into());
        }
        BTreeSet::from(["input", "pictureMode"])
    } else {
        if !dims.is_null() && !dims.as_object().is_some_and(|v| v.is_empty()) {
            return Err("Unexpected global settings dimensions".into());
        }
        BTreeSet::new()
    };
    let values = data["settings"]
        .as_object()
        .filter(|v| v.len() <= 64)
        .ok_or("Invalid control values")?;
    let defs = desc["results"]
        .as_array()
        .filter(|v| v.len() <= 64)
        .ok_or("Invalid control descriptors")?;
    Ok(specs
        .iter()
        .map(|spec| {
            let matches = defs
                .iter()
                .filter(|d| d["key"].as_str() == Some(spec.id))
                .collect::<Vec<_>>();
            let checked = if matches.len() == 1 {
                let d = matches[0];
                let dimensions = d["dimension"]
                    .as_array()
                    .and_then(|a| a.iter().map(Value::as_str).collect::<Option<Vec<_>>>());
                if d["category"].as_str() == Some(category)
                    && dimensions.as_ref().is_some_and(|a| {
                        a.len() == expected.len()
                            && a.iter().copied().collect::<BTreeSet<_>>() == expected
                    })
                {
                    values
                        .get(spec.id)
                        .and_then(|v| registry_constraint(spec, d, v))
                } else {
                    None
                }
            } else {
                None
            };
            let available = checked.is_some();
            let (constraint, value) =
                checked.map_or((None, "Unavailable".into()), |(c, v)| (Some(c), v));
            Setting {
                id: spec.id.into(),
                label: spec.label.into(),
                focus_key: spec.key,
                value,
                context: context.cloned().unwrap_or(Context {
                    input: "global".into(),
                    dynamic_range: "global".into(),
                    picture_mode: "global".into(),
                }),
                protected: true,
                available,
                constraint,
            }
        })
        .collect())
}
#[cfg(test)]
mod tests {
    use super::*;
    fn range() -> Value {
        json!({"category":"picture","key":"tint","dimension":["input","pictureMode"],"vtype":"Range","ui":{"active":true,"visible":true},"values":{"range":{"min":-50,"max":50,"interval":1}}})
    }
    #[test]
    fn fixed_registry_only_reads_named_settings_and_descriptors() {
        let ops = operations();
        assert_eq!(ops.len(), 10);
        let mut names = BTreeSet::new();
        for (name, uri, payload) in ops {
            assert!(names.insert(name));
            assert!(matches!(
                uri.rsplit('/').next(),
                Some("getSystemSettings" | "getSystemSettingDesc")
            ));
            assert!(matches!(
                payload["category"].as_str(),
                Some("picture" | "option" | "general")
            ));
            assert!(payload["subscribe"].is_null());
            assert!(payload["keys"].as_array().is_some_and(|a| a.len() <= 8));
        }
    }
    #[test]
    fn signed_ranges_reject_bad_bounds_steps_types_and_overflow() {
        assert_eq!(constraint(&range(), &json!("-50")).unwrap().1, "-50");
        assert_eq!(constraint(&range(), &json!(0)).unwrap().1, "0");
        for v in [
            json!(-51),
            json!(51),
            json!(1.5),
            json!(true),
            json!("+2"),
            json!("1;cmd"),
            json!("1000000"),
        ] {
            assert!(constraint(&range(), &v).is_none());
        }
        let mut d = range();
        d["values"]["range"]["interval"] = 2.into();
        assert!(constraint(&d, &json!(1)).is_none());
        d["values"]["range"]["min"] = 100.into();
        assert!(constraint(&d, &json!(100)).is_none());
    }
    #[test]
    fn enum_validation_rejects_hidden_inactive_duplicate_and_unknown_options() {
        let mut d = json!({"vtype":"ArrayExt","ui":{"active":true,"visible":true},"values":{"arrayExt":[{"value":"on","visible":true,"active":true},{"value":"off","visible":true,"active":true}]}});
        assert!(constraint(&d, &json!("on")).is_some());
        assert!(constraint(&d, &json!("unknown")).is_none());
        d["values"]["arrayExt"][0]["visible"] = false.into();
        assert!(constraint(&d, &json!("on")).is_none());
        d["values"]["arrayExt"][0]["value"] = "off".into();
        assert!(constraint(&d, &json!("off")).is_none());
        d["values"]["arrayExt"][0]["value"] = "on".into();
        d["values"]["arrayExt"][1]["active"] = false.into();
        assert!(constraint(&d, &json!("off")).is_none());
    }
    #[test]
    fn profile_and_descriptor_dimensions_must_match_and_schema_cannot_grant_write_authority() {
        let context = Context {
            input: "default".into(),
            dynamic_range: "sdr".into(),
            picture_mode: "game".into(),
        };
        let specs = [Spec {
            id: "tint",
            label: "Tint",
            key: 7,
        }];
        let mut replies = json!({"picture_extra_values":{"returnValue":true,"category":"picture","dimension":{"input":"default","pictureMode":"game"},"settings":{"tint":-25}},"picture_extra_desc":{"returnValue":true,"results":[range()]}});
        let rows = read(&replies, "picture_extra", "picture", &specs, Some(&context)).unwrap();
        assert!(rows[0].available && rows[0].protected);
        assert_eq!(rows[0].value, "-25");
        replies["picture_extra_values"]["dimension"]["pictureMode"] = "cinema".into();
        assert!(read(&replies, "picture_extra", "picture", &specs, Some(&context)).is_err());
        replies["picture_extra_values"]["dimension"]["pictureMode"] = "game".into();
        replies["picture_extra_desc"]["results"][0]["dimension"] = json!(["dynamicRange"]);
        assert!(
            !read(&replies, "picture_extra", "picture", &specs, Some(&context)).unwrap()[0]
                .available
        );
    }
    fn accessibility_fixture() -> Value {
        let mut values = serde_json::Map::new();
        let descriptors = ACCESSIBILITY.iter().map(|spec| {
            let choices: &[&str] = match spec.id {
                "audioGuidanceVolume" | "audioGuidancePitch" => &["veryLow","low","medium","high","veryHigh"],
                "audioGuidanceSpeed" => &["veryFast","fast","medium","slow","verySlow"],
                _ => &["on","off"],
            };
            values.insert(spec.id.into(), if choices.len() == 5 { "medium" } else { "off" }.into());
            json!({"key":spec.id,"category":"option","dimension":[],"vtype":"ArrayExt","ui":{"visible":true,"active":true},"values":{"arrayExt":choices.iter().map(|value|json!({"value":value,"visible":true,"active":true})).collect::<Vec<_>>()}})
        }).collect::<Vec<_>>();
        json!({"accessibility_values":{"returnValue":true,"category":"option","settings":values},"accessibility_desc":{"returnValue":true,"results":descriptors}})
    }
    fn accessibility_rows(replies: &Value) -> Result<Vec<Setting>, String> {
        read(replies, "accessibility", "option", ACCESSIBILITY, None)
    }
    fn only_guidance_volume_unavailable(replies: &Value) {
        let rows = accessibility_rows(replies).unwrap();
        for row in rows {
            assert!(row.protected);
            if row.id == "audioGuidanceVolume" {
                assert!(!row.available);
                assert!(row.constraint.is_none());
                assert_eq!(row.value, "Unavailable");
            } else {
                assert!(row.available, "{} unexpectedly withdrawn", row.id);
            }
        }
    }
    #[test]
    fn guidance_readouts_have_stable_unique_keys_and_keep_the_existing_bounded_calls() {
        let mut keys = BTreeSet::new();
        for specs in [
            PICTURE,
            ACCESSIBILITY,
            GENERAL_OPTION,
            GENERAL_HOME,
            PANEL_CARE,
        ] {
            for spec in specs {
                assert!(keys.insert(spec.key));
            }
        }
        assert_eq!(
            ACCESSIBILITY.iter().map(|s| s.key).collect::<Vec<_>>(),
            (100..=106).collect::<Vec<_>>()
        );
        let operations = operations();
        assert_eq!(operations.len(), 10);
        let access = operations
            .iter()
            .filter(|(name, _, _)| name.starts_with("accessibility_"))
            .collect::<Vec<_>>();
        assert_eq!(access.len(), 2);
        assert!(access.iter().all(|(_, uri, payload)| matches!(
            uri.rsplit('/').next(),
            Some("getSystemSettings" | "getSystemSettingDesc")
        ) && payload["keys"]
            .as_array()
            .unwrap()
            .len()
            == 7));
        let rows = accessibility_rows(&accessibility_fixture()).unwrap();
        assert!(rows.iter().all(|r| r.available && r.protected));
        for row in &rows[4..] {
            assert_eq!(row.value, "medium");
            assert!(
                matches!(&row.constraint,Some(ControlConstraint::Choices{values,..}) if values.len()==5)
            );
        }
    }
    #[test]
    fn partial_missing_guidance_value_or_descriptor_does_not_withdraw_valid_rows() {
        let mut replies = accessibility_fixture();
        replies["accessibility_values"]["settings"]
            .as_object_mut()
            .unwrap()
            .remove("audioGuidanceVolume");
        only_guidance_volume_unavailable(&replies);
        let mut replies = accessibility_fixture();
        replies["accessibility_desc"]["results"]
            .as_array_mut()
            .unwrap()
            .remove(4);
        only_guidance_volume_unavailable(&replies);
        let mut replies = accessibility_fixture();
        let duplicate = replies["accessibility_desc"]["results"][4].clone();
        replies["accessibility_desc"]["results"]
            .as_array_mut()
            .unwrap()
            .push(duplicate);
        only_guidance_volume_unavailable(&replies);
    }
    #[test]
    fn inactive_guidance_ui_can_retain_a_valid_protected_reading() {
        let mut replies = accessibility_fixture();
        replies["accessibility_desc"]["results"][4]["ui"]["active"] = false.into();
        let rows = accessibility_rows(&replies).unwrap();
        let row = &rows[4];
        assert!(row.available && row.protected);
        assert_eq!(row.value, "medium");
        assert!(
            matches!(&row.constraint,Some(ControlConstraint::Choices{active:false,values}) if values.len()==5)
        );
        assert!(rows.iter().all(|row| row.available && row.protected));
    }
    #[test]
    fn guidance_numeric_mixed_unknown_and_duplicate_schemas_are_not_inferred_as_ranges() {
        for case in 0..9 {
            let mut replies = accessibility_fixture();
            match case {
                0 => {
                    let d = &mut replies["accessibility_desc"]["results"][4];
                    d["vtype"] = "Range".into();
                    d["values"] = json!({"range":{"min":0,"max":4,"interval":1}});
                    replies["accessibility_values"]["settings"]["audioGuidanceVolume"] = 2.into();
                }
                1 => replies["accessibility_values"]["settings"]["audioGuidanceVolume"] = 2.into(),
                2 => {
                    replies["accessibility_desc"]["results"][4]["values"]["arrayExt"][0]["value"] =
                        "extraLow".into()
                }
                3 => {
                    replies["accessibility_desc"]["results"][4]["values"]["arrayExt"][0]["value"] =
                        "low".into()
                }
                4 => {
                    replies["accessibility_desc"]["results"][4]["values"]["range"] =
                        json!({"min":0,"max":4,"interval":1})
                }
                5 => {
                    replies["accessibility_desc"]["results"][4]["values"]["arrayExt"][0]["active"] =
                        false.into()
                }
                6 => {
                    replies["accessibility_values"]["settings"]["audioGuidanceVolume"] = "on".into()
                }
                7 => {
                    replies["accessibility_desc"]["results"][4]["values"]["arrayExt"]
                        .as_array_mut()
                        .unwrap()
                        .remove(0);
                }
                _ => replies["accessibility_desc"]["results"][4]["category"] = "sound".into(),
            }
            only_guidance_volume_unavailable(&replies);
        }
        // Pitch and speed each require their own exact enum, not merely five strings.
        for index in [5, 6] {
            let mut replies = accessibility_fixture();
            replies["accessibility_desc"]["results"][index]["values"]["arrayExt"][0]["value"] =
                "unknown".into();
            let rows = accessibility_rows(&replies).unwrap();
            assert!(!rows[index].available);
            assert!(rows
                .iter()
                .enumerate()
                .all(|(i, row)| i == index || row.available));
        }
    }
    #[test]
    fn guidance_global_dimensions_and_reply_integrity_are_required() {
        let mut replies = accessibility_fixture();
        replies["accessibility_desc"]["results"][4]["dimension"] = json!(["input"]);
        only_guidance_volume_unavailable(&replies);
        let mut replies = accessibility_fixture();
        replies["accessibility_values"]["dimension"] = json!({"input":"HDMI_1"});
        assert!(accessibility_rows(&replies).is_err());
        let mut replies = accessibility_fixture();
        let context = Context {
            input: "HDMI_1".into(),
            picture_mode: "game".into(),
            dynamic_range: "sdr".into(),
        };
        replies["accessibility_values"]["dimension"] =
            json!({"input":"HDMI_1","pictureMode":"game"});
        for desc in replies["accessibility_desc"]["results"]
            .as_array_mut()
            .unwrap()
        {
            desc["dimension"] = json!(["input", "pictureMode"]);
        }
        let rows = read(
            &replies,
            "accessibility",
            "option",
            ACCESSIBILITY,
            Some(&context),
        )
        .unwrap();
        assert!(rows[4..].iter().all(|r| !r.available && r.protected));
        for case in 0..5 {
            let mut replies = accessibility_fixture();
            match case {
                0 => replies["accessibility_values"]["returnValue"] = false.into(),
                1 => replies["accessibility_desc"]["returnValue"] = false.into(),
                2 => replies["accessibility_values"]["settings"] = json!([]),
                3 => replies["accessibility_desc"]["results"] = "malformed".into(),
                _ => replies["accessibility_values"]["category"] = "sound".into(),
            }
            assert!(accessibility_rows(&replies).is_err(), "case {case}");
        }
    }
}
