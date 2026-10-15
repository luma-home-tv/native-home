//! Optional global Sound readings, independent of audioInput-dimensional soundMode.
//! Two fixed GET operations only. No descriptor or value grants write authority.
use crate::model::{Context, ControlConstraint, Setting};
use serde_json::{json, Value};
use std::collections::BTreeSet;

pub const VALUES_REPLY: &str = "sound_extra_values";
pub const DESCRIPTORS_REPLY: &str = "sound_extra_desc";
const MAX_GROUP_FIELDS: usize = 16;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Key {
    Balance,
    Description,
    DescriptionLevel,
}
impl Key {
    pub const ALL: [Self; 3] = [Self::Balance, Self::Description, Self::DescriptionLevel];
    pub fn id(self) -> &'static str {
        match self {
            Self::Balance => "audioBalance",
            Self::Description => "audioDescription",
            Self::DescriptionLevel => "audioDescriptionVolume",
        }
    }
    pub fn label(self) -> &'static str {
        match self {
            Self::Balance => "Audio balance (LG scale)",
            Self::Description => "Audio description",
            Self::DescriptionLevel => "Audio-description level (LG scale)",
        }
    }
    pub fn focus_key(self) -> u32 {
        match self {
            Self::Balance => 30,
            Self::Description => 31,
            Self::DescriptionLevel => 32,
        }
    }
    fn range(self) -> Option<(i8, i8)> {
        match self {
            Self::Balance => Some((-50, 50)),
            Self::DescriptionLevel => Some((-15, 15)),
            Self::Description => None,
        }
    }
}

/// The normal live collector can append these operations without a generic URI API.
pub fn operations() -> [(&'static str, &'static str, Value); 2] {
    let payload = json!({"category":"sound","keys":Key::ALL.map(Key::id)});
    [
        (
            VALUES_REPLY,
            "luna://com.webos.service.settings/getSystemSettings",
            payload.clone(),
        ),
        (
            DESCRIPTORS_REPLY,
            "luna://com.webos.service.settings/getSystemSettingDesc",
            payload,
        ),
    ]
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum NumericEncoding {
    Integer,
    DecimalString,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Description {
    On,
    Off,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ReadValue {
    Number {
        value: i8,
        encoding: NumericEncoding,
    },
    Description(Description),
}
impl ReadValue {
    /// Read-side round trip for diagnostics; not an operation or setter payload.
    pub fn raw_value(self) -> Value {
        match self {
            Self::Number {
                value,
                encoding: NumericEncoding::Integer,
            } => Value::from(value),
            Self::Number {
                value,
                encoding: NumericEncoding::DecimalString,
            } => Value::from(value.to_string()),
            Self::Description(Description::On) => Value::from("on"),
            Self::Description(Description::Off) => Value::from("off"),
        }
    }
    fn display(self) -> String {
        match self {
            Self::Number { value, .. } => value.to_string(),
            Self::Description(Description::On) => "on".into(),
            Self::Description(Description::Off) => "off".into(),
        }
    }
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum RowError {
    GroupUnavailable,
    MissingValue,
    MissingDescriptor,
    DuplicateDescriptor,
    InvalidDescriptor,
    InvalidValue,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum RowState {
    Available { value: ReadValue, active: bool },
    Unavailable(RowError),
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Readout {
    pub key: Key,
    pub state: RowState,
}
impl Readout {
    pub fn as_setting(self) -> Setting {
        let (value, constraint, available) = match self.state {
            RowState::Unavailable(_) => ("Unavailable".into(), None, false),
            RowState::Available { value, active } => {
                let constraint = match self.key.range() {
                    Some((min, max)) => ControlConstraint::Range {
                        minimum: i64::from(min),
                        maximum: i64::from(max),
                        step: 1,
                        active,
                    },
                    None => ControlConstraint::Choices {
                        values: vec!["on".into(), "off".into()],
                        active,
                    },
                };
                (value.display(), Some(constraint), true)
            }
        };
        Setting {
            id: self.key.id().into(),
            label: self.key.label().into(),
            focus_key: self.key.focus_key(),
            value,
            constraint,
            available,
            protected: true,
            context: Context {
                input: "global".into(),
                dynamic_range: "global".into(),
                picture_mode: "global".into(),
            },
        }
    }
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum GroupError {
    Refused,
    WrongCategory,
    UnexpectedDimensions,
    MalformedValues,
    MalformedDescriptors,
    UnexpectedKey,
}

fn descriptor(key: Key, desc: &Value) -> Option<bool> {
    if desc["key"].as_str() != Some(key.id())
        || desc["category"].as_str() != Some("sound")
        || desc["dbtype"].as_str() != Some("G")
        || desc["account"].as_bool() != Some(false)
        || desc["volatile"].as_bool() != Some(false)
        || desc["valueCheck"].as_bool() != Some(true)
        || desc["valueTypeCheck"].as_bool() != Some(false)
        || !desc["dimension"]
            .as_array()
            .is_some_and(|dims| dims.is_empty())
        || desc["ui"]["visible"].as_bool() != Some(true)
    {
        return None;
    }
    let active = desc["ui"]["active"].as_bool()?;
    let values = desc["values"]
        .as_object()
        .filter(|values| values.len() == 1)?;
    if let Some((minimum, maximum)) = key.range() {
        if desc["vtype"].as_str() != Some("Range") {
            return None;
        }
        let range = values
            .get("range")?
            .as_object()
            .filter(|range| range.len() == 3)?;
        if range.get("min")?.as_i64() != Some(i64::from(minimum))
            || range.get("max")?.as_i64() != Some(i64::from(maximum))
            || range.get("interval")?.as_i64() != Some(1)
        {
            return None;
        }
    } else {
        if desc["vtype"].as_str() != Some("ArrayExt") {
            return None;
        }
        let choices = values
            .get("arrayExt")?
            .as_array()
            .filter(|choices| choices.len() == 2)?;
        let mut unique = BTreeSet::new();
        for choice in choices {
            let value = choice["value"].as_str()?;
            if !matches!(value, "on" | "off")
                || !unique.insert(value)
                || choice["active"].as_bool() != Some(true)
                || choice["visible"].as_bool() != Some(true)
            {
                return None;
            }
        }
    }
    Some(active)
}
fn typed_value(key: Key, raw: &Value) -> Option<ReadValue> {
    if let Some((minimum, maximum)) = key.range() {
        let (value, encoding) = if let Some(raw) = raw.as_str() {
            if raw.is_empty() || raw.len() > 3 {
                return None;
            }
            let value: i8 = raw.parse().ok()?;
            if value.to_string() != raw {
                return None;
            }
            (value, NumericEncoding::DecimalString)
        } else {
            (i8::try_from(raw.as_i64()?).ok()?, NumericEncoding::Integer)
        };
        (minimum..=maximum)
            .contains(&value)
            .then_some(ReadValue::Number { value, encoding })
    } else {
        match raw.as_str()? {
            "on" => Some(ReadValue::Description(Description::On)),
            "off" => Some(ReadValue::Description(Description::Off)),
            _ => None,
        }
    }
}

/// Parse a separate optional namespace; never consumes soundMode/audioInput profiles.
/// Bad group envelopes fail closed; missing/invalid requested keys fail independently.
pub fn parse_typed(replies: &Value) -> Result<[Readout; 3], GroupError> {
    let data = &replies[VALUES_REPLY];
    let desc = &replies[DESCRIPTORS_REPLY];
    if data["returnValue"].as_bool() != Some(true) || desc["returnValue"].as_bool() != Some(true) {
        return Err(GroupError::Refused);
    }
    if data["category"].as_str() != Some("sound")
        || (!desc["category"].is_null() && desc["category"].as_str() != Some("sound"))
    {
        return Err(GroupError::WrongCategory);
    }
    let global =
        |value: &Value| value.is_null() || value.as_object().is_some_and(|dims| dims.is_empty());
    if !global(&data["dimension"]) || !global(&desc["dimension"]) {
        return Err(GroupError::UnexpectedDimensions);
    }
    let values = data["settings"]
        .as_object()
        .filter(|values| values.len() <= MAX_GROUP_FIELDS)
        .ok_or(GroupError::MalformedValues)?;
    let descriptors = desc["results"]
        .as_array()
        .filter(|rows| rows.len() <= MAX_GROUP_FIELDS && rows.iter().all(Value::is_object))
        .ok_or(GroupError::MalformedDescriptors)?;
    let known = |id: &str| Key::ALL.iter().any(|key| key.id() == id);
    if values.keys().any(|id| !known(id))
        || descriptors
            .iter()
            .any(|desc| !desc["key"].as_str().is_some_and(known))
    {
        return Err(GroupError::UnexpectedKey);
    }
    Ok(Key::ALL.map(|key| {
        let state = (|| {
            let raw = values.get(key.id()).ok_or(RowError::MissingValue)?;
            let mut matching = descriptors
                .iter()
                .filter(|desc| desc["key"].as_str() == Some(key.id()));
            let desc = matching.next().ok_or(RowError::MissingDescriptor)?;
            if matching.next().is_some() {
                return Err(RowError::DuplicateDescriptor);
            }
            let active = descriptor(key, desc).ok_or(RowError::InvalidDescriptor)?;
            let value = typed_value(key, raw).ok_or(RowError::InvalidValue)?;
            Ok((value, active))
        })()
        .map_or_else(RowState::Unavailable, |(value, active)| {
            RowState::Available { value, active }
        });
        Readout { key, state }
    }))
}

/// Protected display rows only. Preserve baseline Sound independently on Err/expiry.
pub fn parse(replies: &Value) -> Result<Vec<Setting>, GroupError> {
    parse_typed(replies).map(|rows| rows.into_iter().map(Readout::as_setting).collect())
}

/// Withdraw only this optional group's rows after refusal or its own freshness expiry.
pub fn unavailable_rows() -> Vec<Setting> {
    Key::ALL
        .into_iter()
        .map(|key| {
            Readout {
                key,
                state: RowState::Unavailable(RowError::GroupUnavailable),
            }
            .as_setting()
        })
        .collect()
}

#[cfg(test)]
pub(crate) mod tests {
    use super::*;
    pub(crate) fn fixture() -> Value {
        let descriptors = Key::ALL.map(|key| {
            let (kind, values) = match key {
                Key::Balance => ("Range", json!({"range":{"min":-50,"max":50,"interval":1}})),
                Key::DescriptionLevel => ("Range", json!({"range":{"min":-15,"max":15,"interval":1}})),
                Key::Description => ("ArrayExt", json!({"arrayExt":[{"value":"on","active":true,"visible":true},{"value":"off","active":true,"visible":true}]})),
            };
            json!({"key":key.id(),"category":"sound","dimension":[],"dbtype":"G","account":false,"volatile":false,"valueCheck":true,"valueTypeCheck":false,"ui":{"active":true,"visible":true},"vtype":kind,"values":values})
        });
        json!({VALUES_REPLY:{"returnValue":true,"category":"sound","dimension":{},"settings":{"audioBalance":"-4","audioDescription":"on","audioDescriptionVolume":3}},DESCRIPTORS_REPLY:{"returnValue":true,"results":descriptors}})
    }
    fn assert_only_unavailable(replies: &Value, key: Key, reason: RowError) {
        let rows = parse_typed(replies).unwrap();
        for row in rows {
            if row.key == key {
                assert_eq!(row.state, RowState::Unavailable(reason));
            } else {
                assert!(matches!(row.state, RowState::Available { .. }), "{:?}", row);
            }
        }
    }
    #[test]
    fn fixed_getters_and_protected_display_rows_do_not_enable_writes() {
        let operations = operations();
        assert_eq!(operations.len(), 2);
        for (name, uri, payload) in operations {
            assert!(matches!(name, VALUES_REPLY | DESCRIPTORS_REPLY));
            assert!(matches!(
                uri.rsplit('/').next(),
                Some("getSystemSettings" | "getSystemSettingDesc")
            ));
            assert_eq!(
                payload,
                json!({"category":"sound","keys":["audioBalance","audioDescription","audioDescriptionVolume"]})
            );
        }
        let rows = parse(&fixture()).unwrap();
        assert_eq!(
            rows.iter().map(|row| row.focus_key).collect::<Vec<_>>(),
            [30, 31, 32]
        );
        assert!(rows.iter().all(|row| row.protected
            && row.available
            && row.context.input == "global"
            && row.context.dynamic_range == "global"
            && row.context.picture_mode == "global"));
        assert_eq!(
            rows.iter()
                .map(|row| row.value.as_str())
                .collect::<Vec<_>>(),
            ["-4", "on", "3"]
        );
        assert!(matches!(
            rows[0].constraint,
            Some(ControlConstraint::Range {
                minimum: -50,
                maximum: 50,
                step: 1,
                ..
            })
        ));
        assert!(matches!(
            rows[2].constraint,
            Some(ControlConstraint::Range {
                minimum: -15,
                maximum: 15,
                step: 1,
                ..
            })
        ));
    }
    #[test]
    fn canonical_signed_integer_and_string_encodings_round_trip_independently() {
        for key in [Key::Balance, Key::DescriptionLevel] {
            let (minimum, maximum) = key.range().unwrap();
            for number in [minimum, 0, maximum] {
                for raw in [json!(number), json!(number.to_string())] {
                    let mut replies = fixture();
                    replies[VALUES_REPLY]["settings"][key.id()] = raw.clone();
                    let row = parse_typed(&replies)
                        .unwrap()
                        .into_iter()
                        .find(|row| row.key == key)
                        .unwrap();
                    let RowState::Available { value, .. } = row.state else {
                        panic!("canonical value rejected")
                    };
                    assert_eq!(value.raw_value(), raw);
                    assert_eq!(
                        value,
                        ReadValue::Number {
                            value: number,
                            encoding: if raw.is_string() {
                                NumericEncoding::DecimalString
                            } else {
                                NumericEncoding::Integer
                            }
                        }
                    );
                }
            }
        }
        // Enablement is read data, not a prerequisite for seeing its stored level.
        assert!(parse(&fixture()).unwrap().iter().all(|row| row.available));
    }
    #[test]
    fn noncanonical_out_of_range_and_coerced_values_fail_only_their_row() {
        for key in [Key::Balance, Key::DescriptionLevel] {
            for invalid in [
                json!(true),
                json!(1.0),
                json!("01"),
                json!("-0"),
                json!("+1"),
                json!(" 1"),
                json!("1e0"),
                json!(""),
                json!(128),
                json!(i64::MIN),
                Value::Null,
            ] {
                let mut replies = fixture();
                replies[VALUES_REPLY]["settings"][key.id()] = invalid;
                assert_only_unavailable(&replies, key, RowError::InvalidValue);
            }
            let (min, max) = key.range().unwrap();
            for invalid in [i64::from(min) - 1, i64::from(max) + 1] {
                let mut replies = fixture();
                replies[VALUES_REPLY]["settings"][key.id()] = invalid.into();
                assert_only_unavailable(&replies, key, RowError::InvalidValue);
            }
        }
        for invalid in [json!(true), json!(1), json!("enabled"), Value::Null] {
            let mut replies = fixture();
            replies[VALUES_REPLY]["settings"]["audioDescription"] = invalid;
            assert_only_unavailable(&replies, Key::Description, RowError::InvalidValue);
        }
    }
    #[test]
    fn missing_and_duplicate_keys_do_not_destroy_other_optional_rows() {
        let mut replies = fixture();
        replies[VALUES_REPLY]["settings"]
            .as_object_mut()
            .unwrap()
            .remove("audioBalance");
        assert_only_unavailable(&replies, Key::Balance, RowError::MissingValue);
        let mut replies = fixture();
        replies[DESCRIPTORS_REPLY]["results"]
            .as_array_mut()
            .unwrap()
            .remove(0);
        assert_only_unavailable(&replies, Key::Balance, RowError::MissingDescriptor);
        let mut replies = fixture();
        let duplicate = replies[DESCRIPTORS_REPLY]["results"][0].clone();
        replies[DESCRIPTORS_REPLY]["results"]
            .as_array_mut()
            .unwrap()
            .push(duplicate);
        assert_only_unavailable(&replies, Key::Balance, RowError::DuplicateDescriptor);
        let row = parse(&replies).unwrap().remove(0);
        assert!(!row.available && row.protected && row.constraint.is_none());
    }
    #[test]
    fn storage_scope_range_and_mixed_schema_drift_fail_independently() {
        for case in 0..10 {
            let mut replies = fixture();
            let desc = &mut replies[DESCRIPTORS_REPLY]["results"][0];
            match case {
                0 => desc["account"] = true.into(),
                1 => desc["volatile"] = true.into(),
                2 => desc["dbtype"] = "P".into(),
                3 => desc["dimension"] = json!(["audioInput"]),
                4 => desc["values"]["range"]["min"] = (-51).into(),
                5 => desc["values"]["range"]["interval"] = 2.into(),
                6 => desc["values"]["arrayExt"] = json!([]),
                7 => desc["ui"]["visible"] = false.into(),
                8 => desc["valueTypeCheck"] = true.into(),
                _ => desc["values"]["range"]["max"] = "50".into(),
            }
            assert_only_unavailable(&replies, Key::Balance, RowError::InvalidDescriptor);
        }
        let mut replies = fixture();
        replies[DESCRIPTORS_REPLY]["results"][2]["values"]["range"]["max"] = 50.into();
        assert_only_unavailable(&replies, Key::DescriptionLevel, RowError::InvalidDescriptor);
    }
    #[test]
    fn description_is_exactly_the_closed_visible_on_off_enum() {
        for case in 0..5 {
            let mut replies = fixture();
            let desc = &mut replies[DESCRIPTORS_REPLY]["results"][1];
            match case {
                0 => desc["values"]["arrayExt"][0]["value"] = "off".into(),
                1 => desc["values"]["arrayExt"][0]["value"] = "enabled".into(),
                2 => desc["values"]["arrayExt"][0]["active"] = false.into(),
                3 => desc["values"]["arrayExt"][0]["visible"] = false.into(),
                _ => {
                    desc["vtype"] = "Range".into();
                    desc["values"] = json!({"range":{"min":0,"max":1,"interval":1}});
                }
            }
            assert_only_unavailable(&replies, Key::Description, RowError::InvalidDescriptor);
        }
    }
    #[test]
    fn profile_dimensions_denial_unknown_keys_and_malformed_groups_fail_closed() {
        for case in 0..11 {
            let mut replies = fixture();
            match case {
                0 => replies[VALUES_REPLY]["returnValue"] = false.into(),
                1 => replies[DESCRIPTORS_REPLY]["returnValue"] = false.into(),
                2 => replies[VALUES_REPLY]["dimension"] = json!({"audioInput":"hdmi1"}),
                3 => replies[VALUES_REPLY]["category"] = "option".into(),
                4 => replies[VALUES_REPLY]["settings"] = json!([]),
                5 => replies[DESCRIPTORS_REPLY]["results"] = json!([null]),
                6 => replies[VALUES_REPLY]["settings"]["soundMode"] = "synthetic".into(),
                7 => replies[DESCRIPTORS_REPLY]["results"][0]["key"] = "soundMode".into(),
                8 => replies[VALUES_REPLY]["dimension"] = json!([]),
                9 => replies[DESCRIPTORS_REPLY]["dimension"] = json!({"audioInput":"hdmi1"}),
                _ => replies[DESCRIPTORS_REPLY]["category"] = "option".into(),
            }
            assert!(parse(&replies).is_err(), "case {case}");
        }
        assert_eq!(parse_typed(&Value::Null), Err(GroupError::Refused));
    }
    #[test]
    fn inactive_ui_is_readable_but_never_native_editable() {
        let mut replies = fixture();
        replies[DESCRIPTORS_REPLY]["results"][0]["ui"]["active"] = false.into();
        let rows = parse(&replies).unwrap();
        assert!(rows.iter().all(|row| row.available && row.protected));
        assert!(matches!(
            rows[0].constraint,
            Some(ControlConstraint::Range { active: false, .. })
        ));
    }
    #[test]
    fn group_withdrawal_has_only_its_three_fixed_unavailable_rows() {
        let rows = unavailable_rows();
        assert_eq!(
            rows.iter().map(|row| row.id.as_str()).collect::<Vec<_>>(),
            ["audioBalance", "audioDescription", "audioDescriptionVolume"]
        );
        assert_eq!(
            rows.iter().map(|row| row.focus_key).collect::<Vec<_>>(),
            [30, 31, 32]
        );
        assert!(rows.iter().all(|row| row.protected
            && !row.available
            && row.constraint.is_none()
            && row.value == "Unavailable"));
    }
}
