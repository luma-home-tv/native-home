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
                    values.get(spec.id).and_then(|v| constraint(d, v))
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
}
