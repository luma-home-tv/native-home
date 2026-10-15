//! Fixed, read-only language and region readings. No setter or generic key API.
//! Optional descriptors are evidence for diagnostics; they never gate typed GET
//! values or grant editing authority. Retail firmware and ordinary UID access
//! still require separate device qualification.
use crate::model::{Context, Setting};
use serde_json::{json, Value};

pub const LOCALE_VALUES_REPLY: &str = "locale_values";
pub const LOCALE_DESCRIPTORS_REPLY: &str = "locale_desc";
pub const REGION_VALUES_REPLY: &str = "region_values";
pub const REGION_DESCRIPTORS_REPLY: &str = "region_desc";
const MAX_OBJECT_FIELDS: usize = 16;
const MAX_TAG_BYTES: usize = 64;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Key {
    MenuLanguage,
    AudioLanguage,
    SecondaryAudioLanguage,
    Country,
    ServiceCountry,
}
impl Key {
    pub const ALL: [Self; 5] = [
        Self::MenuLanguage,
        Self::AudioLanguage,
        Self::SecondaryAudioLanguage,
        Self::Country,
        Self::ServiceCountry,
    ];
    pub fn id(self) -> &'static str {
        match self {
            Self::MenuLanguage => "localeInfo.locales.UI",
            Self::AudioLanguage => "localeInfo.locales.AUD",
            Self::SecondaryAudioLanguage => "localeInfo.locales.AUD2",
            Self::Country => "country",
            Self::ServiceCountry => "smartServiceCountryCode2",
        }
    }
    pub fn label(self) -> &'static str {
        match self {
            Self::MenuLanguage => "Menu language",
            Self::AudioLanguage => "Preferred audio language",
            Self::SecondaryAudioLanguage => "Secondary audio language",
            Self::Country => "TV country",
            Self::ServiceCountry => "LG services country",
        }
    }
    pub fn focus_key(self) -> u32 {
        match self {
            Self::MenuLanguage => 145,
            Self::AudioLanguage => 146,
            Self::SecondaryAudioLanguage => 147,
            Self::Country => 148,
            Self::ServiceCountry => 149,
        }
    }
    fn locale_leaf(self) -> Option<&'static str> {
        match self {
            Self::MenuLanguage => Some("UI"),
            Self::AudioLanguage => Some("AUD"),
            Self::SecondaryAudioLanguage => Some("AUD2"),
            Self::Country | Self::ServiceCountry => None,
        }
    }
}

/// Only these named requests are exposed. In particular, localeInfo has no
/// category; a combined option/locale request would address the wrong namespace.
pub fn operations() -> [(&'static str, &'static str, Value); 4] {
    let locale = json!({"keys":["localeInfo"],"subscribe":false});
    let region = json!({"category":"option","keys":["country","smartServiceCountryCode2"],"subscribe":false});
    [
        (
            LOCALE_VALUES_REPLY,
            "luna://com.webos.settingsservice/getSystemSettings",
            locale.clone(),
        ),
        (
            LOCALE_DESCRIPTORS_REPLY,
            "luna://com.webos.service.settings/getSystemSettingDesc",
            locale,
        ),
        (
            REGION_VALUES_REPLY,
            "luna://com.webos.settingsservice/getSystemSettings",
            region.clone(),
        ),
        (
            REGION_DESCRIPTORS_REPLY,
            "luna://com.webos.service.settings/getSystemSettingDesc",
            region,
        ),
    ]
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum ReadValue {
    LocaleTag(String),
    CountryCode(String),
    ServiceCountryCode(String),
}
impl ReadValue {
    pub fn as_str(&self) -> &str {
        match self {
            Self::LocaleTag(value) | Self::CountryCode(value) | Self::ServiceCountryCode(value) => {
                value
            }
        }
    }
    /// Preserve the actual string wire encoding, including tag case/separator.
    pub fn raw_value(&self) -> Value {
        Value::String(self.as_str().into())
    }
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum RowError {
    Refused,
    WrongCategory,
    UnexpectedDimensions,
    MalformedValues,
    UnexpectedKey,
    MissingValue,
    InvalidContainer,
    InvalidValue,
}
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum RowState {
    Available(ReadValue),
    Unavailable(RowError),
}
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Readout {
    pub key: Key,
    pub state: RowState,
}
impl Readout {
    pub fn as_setting(&self) -> Setting {
        let (value, available) = match &self.state {
            RowState::Available(value) => (value.as_str().into(), true),
            RowState::Unavailable(_) => ("Unavailable".into(), false),
        };
        Setting {
            id: self.key.id().into(),
            label: self.key.label().into(),
            focus_key: self.key.focus_key(),
            value,
            constraint: None,
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

fn envelope<'a>(
    data: &'a Value,
    category: Option<&str>,
    keys: &[&str],
) -> Result<&'a serde_json::Map<String, Value>, RowError> {
    if data["returnValue"].as_bool() != Some(true) {
        return Err(RowError::Refused);
    }
    let correct_category = match category {
        Some(category) => data["category"].as_str() == Some(category),
        None => data.get("category").is_none() || data["category"].as_str() == Some(""),
    };
    if !correct_category {
        return Err(RowError::WrongCategory);
    }
    if data
        .get("dimension")
        .is_some_and(|dimension| !dimension.as_object().is_some_and(|dims| dims.is_empty()))
    {
        return Err(RowError::UnexpectedDimensions);
    }
    let values = data["settings"]
        .as_object()
        .filter(|values| values.len() <= keys.len())
        .ok_or(RowError::MalformedValues)?;
    if values.keys().any(|id| !keys.contains(&id.as_str())) {
        return Err(RowError::UnexpectedKey);
    }
    Ok(values)
}

/// A bounded language-tag spelling, not a claim of registry membership or a
/// complete BCP 47 validator. Both documented hyphens and underscores survive.
fn language_tag(value: &str) -> bool {
    if value.is_empty() || value.len() > MAX_TAG_BYTES || !value.is_ascii() {
        return false;
    }
    let mut parts = value.split(['-', '_']);
    let Some(language) = parts.next() else {
        return false;
    };
    (2..=8).contains(&language.len())
        && language.bytes().all(|c| c.is_ascii_alphabetic())
        && parts.all(|part| {
            (1..=8).contains(&part.len()) && part.bytes().all(|c| c.is_ascii_alphanumeric())
        })
}
fn typed_value(key: Key, raw: &Value) -> Result<ReadValue, RowError> {
    let value = raw.as_str().ok_or(RowError::InvalidValue)?;
    let valid = match key {
        Key::MenuLanguage | Key::AudioLanguage | Key::SecondaryAudioLanguage => language_tag(value),
        Key::Country => value.len() == 3 && value.bytes().all(|c| c.is_ascii_uppercase()),
        Key::ServiceCountry => value.len() == 2 && value.bytes().all(|c| c.is_ascii_uppercase()),
    };
    if !valid {
        return Err(RowError::InvalidValue);
    }
    Ok(match key {
        Key::MenuLanguage | Key::AudioLanguage | Key::SecondaryAudioLanguage => {
            ReadValue::LocaleTag(value.into())
        }
        Key::Country => ReadValue::CountryCode(value.into()),
        Key::ServiceCountry => ReadValue::ServiceCountryCode(value.into()),
    })
}
fn locale_value(values: &serde_json::Map<String, Value>, key: Key) -> Result<ReadValue, RowError> {
    let info = values
        .get("localeInfo")
        .ok_or(RowError::MissingValue)?
        .as_object()
        .filter(|info| info.len() <= MAX_OBJECT_FIELDS)
        .ok_or(RowError::InvalidContainer)?;
    let locales = info
        .get("locales")
        .ok_or(RowError::MissingValue)?
        .as_object()
        .filter(|locales| locales.len() <= MAX_OBJECT_FIELDS)
        .ok_or(RowError::InvalidContainer)?;
    let raw = locales
        .get(key.locale_leaf().ok_or(RowError::UnexpectedKey)?)
        .ok_or(RowError::MissingValue)?;
    typed_value(key, raw)
}

/// Independent groups and leaf errors: no missing descriptor, country or audio
/// preference can fabricate a value or erase an otherwise valid sibling row.
pub fn parse_typed(replies: &Value) -> [Readout; 5] {
    let locale = envelope(&replies[LOCALE_VALUES_REPLY], None, &["localeInfo"]);
    let region = envelope(
        &replies[REGION_VALUES_REPLY],
        Some("option"),
        &["country", "smartServiceCountryCode2"],
    );
    Key::ALL.map(|key| {
        let result = if key.locale_leaf().is_some() {
            locale
                .as_ref()
                .map_err(|error| *error)
                .and_then(|values| locale_value(values, key))
        } else {
            region.as_ref().map_err(|error| *error).and_then(|values| {
                typed_value(key, values.get(key.id()).ok_or(RowError::MissingValue)?)
            })
        };
        Readout {
            key,
            state: result.map_or_else(RowState::Unavailable, RowState::Available),
        }
    })
}
pub fn parse(replies: &Value) -> Vec<Setting> {
    parse_typed(replies)
        .iter()
        .map(Readout::as_setting)
        .collect()
}
pub fn unavailable_rows() -> Vec<Setting> {
    Key::ALL
        .map(|key| Readout {
            key,
            state: RowState::Unavailable(RowError::Refused),
        })
        .iter()
        .map(Readout::as_setting)
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    fn fixture() -> Value {
        json!({
            LOCALE_VALUES_REPLY:{"returnValue":true,"settings":{"localeInfo":{"locales":{"UI":"en-GB","AUD":"fr-FR","AUD2":"de_DE"},"clock":"locale","keyboards":["en"],"timezone":""}}},
            REGION_VALUES_REPLY:{"returnValue":true,"category":"option","settings":{"country":"GBR","smartServiceCountryCode2":"GB"}}
        })
    }
    #[test]
    fn fixed_non_subscribing_requests_keep_the_category_boundary() {
        let operations = operations();
        assert_eq!(operations.len(), 4);
        for (name, uri, payload) in &operations {
            assert!(uri.ends_with("/getSystemSettings") || uri.ends_with("/getSystemSettingDesc"));
            assert_eq!(payload["subscribe"], false);
            if *name == LOCALE_VALUES_REPLY || *name == LOCALE_DESCRIPTORS_REPLY {
                assert!(payload.get("category").is_none());
                assert_eq!(payload["keys"], json!(["localeInfo"]));
            } else {
                assert_eq!(payload["category"], "option");
                assert_eq!(
                    payload["keys"],
                    json!(["country", "smartServiceCountryCode2"])
                );
            }
        }
    }
    #[test]
    fn five_protected_rows_round_trip_real_string_encoding_without_descriptors() {
        let readouts = parse_typed(&fixture());
        for (row, raw) in readouts
            .iter()
            .zip(["en-GB", "fr-FR", "de_DE", "GBR", "GB"])
        {
            let RowState::Available(value) = &row.state else {
                panic!("Expected typed value")
            };
            assert_eq!(value.raw_value(), Value::String(raw.into()));
            let setting = row.as_setting();
            assert!(setting.available && setting.protected);
            assert!(setting.constraint.is_none());
        }
        assert_eq!(
            readouts.map(|row| row.key.focus_key()),
            [145, 146, 147, 148, 149]
        );
    }
    #[test]
    fn absent_or_refused_descriptor_does_not_erase_read_only_values() {
        let mut value = fixture();
        value[LOCALE_DESCRIPTORS_REPLY] = json!({"returnValue":false});
        value[REGION_DESCRIPTORS_REPLY] = json!({"results":"unavailable"});
        assert!(parse(&value)
            .iter()
            .all(|row| row.available && row.protected));
    }
    #[test]
    fn refused_or_wrong_namespace_only_invalidates_its_group() {
        for bad in [
            json!({"returnValue":false}),
            json!({"returnValue":true,"category":"option","settings":{"localeInfo":{}}}),
        ] {
            let mut value = fixture();
            value[LOCALE_VALUES_REPLY] = bad;
            let rows = parse(&value);
            assert!(rows[..3].iter().all(|row| !row.available));
            assert!(rows[3..].iter().all(|row| row.available));
        }
        let mut value = fixture();
        value[REGION_VALUES_REPLY]["category"] = "general".into();
        let rows = parse(&value);
        assert!(rows[..3].iter().all(|row| row.available));
        assert!(rows[3..].iter().all(|row| !row.available));
    }
    #[test]
    fn missing_or_invalid_leaf_only_invalidates_that_row() {
        let mut value = fixture();
        value[LOCALE_VALUES_REPLY]["settings"]["localeInfo"]["locales"]
            .as_object_mut()
            .unwrap()
            .remove("AUD");
        let rows = parse(&value);
        assert!(!rows[1].available);
        assert!(rows
            .iter()
            .enumerate()
            .all(|(index, row)| index == 1 || row.available));
        let mut value = fixture();
        value[REGION_VALUES_REPLY]["settings"]["country"] = json!(true);
        let rows = parse(&value);
        assert!(!rows[3].available);
        assert!(rows
            .iter()
            .enumerate()
            .all(|(index, row)| index == 3 || row.available));
    }
    #[test]
    fn bad_wire_types_and_unbounded_or_rendering_control_strings_are_unavailable() {
        for bad in [
            Value::Null,
            json!(true),
            json!(12),
            json!([]),
            json!({}),
            json!(""),
            json!("en GB"),
            json!("en\nGB"),
            json!("en--GB"),
            json!("-en"),
            json!("en-"),
            json!("é-GB"),
            json!("x".repeat(65)),
        ] {
            let mut value = fixture();
            value[LOCALE_VALUES_REPLY]["settings"]["localeInfo"]["locales"]["UI"] = bad;
            let rows = parse(&value);
            assert!(!rows[0].available);
            assert!(rows[1..].iter().all(|row| row.available));
        }
        for bad in ["gb", "GBR", "other", "", "G1"] {
            let mut value = fixture();
            value[REGION_VALUES_REPLY]["settings"]["smartServiceCountryCode2"] = bad.into();
            let rows = parse(&value);
            assert!(!rows[4].available);
            assert!(rows[..4].iter().all(|row| row.available));
        }
    }
    #[test]
    fn unrelated_locale_fields_are_not_interpreted_as_preferences_or_capabilities() {
        let mut value = fixture();
        value[LOCALE_VALUES_REPLY]["settings"]["localeInfo"]["clock"] =
            json!({"unexpected":"shape"});
        value[LOCALE_VALUES_REPLY]["settings"]["localeInfo"]["locales"]["STT"] = true.into();
        assert!(parse(&value)
            .iter()
            .all(|row| row.available && row.protected && row.constraint.is_none()));
    }
    #[test]
    fn malformed_containers_dimensions_and_unknown_top_level_keys_fail_closed() {
        for bad in [json!(null), json!([]), json!("en-GB")] {
            let mut value = fixture();
            value[LOCALE_VALUES_REPLY]["settings"]["localeInfo"] = bad;
            let rows = parse(&value);
            assert!(rows[..3].iter().all(|row| !row.available));
            assert!(rows[3..].iter().all(|row| row.available));
        }
        let mut value = fixture();
        value[LOCALE_VALUES_REPLY]["dimension"] = json!({"input":"hdmi1"});
        assert!(parse(&value)[..3].iter().all(|row| !row.available));
        let mut value = fixture();
        value[REGION_VALUES_REPLY]["settings"]["otherKey"] = true.into();
        assert!(parse(&value)[3..].iter().all(|row| !row.available));
    }
    #[test]
    fn expired_group_has_five_unavailable_protected_rows() {
        let rows = unavailable_rows();
        assert_eq!(rows.len(), 5);
        assert!(rows
            .iter()
            .all(|row| !row.available && row.protected && row.constraint.is_none()));
    }
}
