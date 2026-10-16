//! Three read-only TV information rows from the already-collected system_info reply.
//! No request adapter, capability, derived marketing version or setting operation.
use crate::model::{Context, Setting};
use serde_json::Value;

const MAX_VALUE_BYTES: usize = 128;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Key {
    Model,
    SoftwareVersion,
    SdkVersion,
}
impl Key {
    pub const ALL: [Self; 3] = [Self::Model, Self::SoftwareVersion, Self::SdkVersion];

    pub fn id(self) -> &'static str {
        match self {
            Self::Model => "modelName",
            Self::SoftwareVersion => "firmwareVersion",
            Self::SdkVersion => "sdkVersion",
        }
    }
    pub fn label(self) -> &'static str {
        match self {
            Self::Model => "TV model",
            Self::SoftwareVersion => "Software version",
            Self::SdkVersion => "webOS SDK version",
        }
    }
    pub fn focus_key(self) -> u32 {
        match self {
            Self::Model => 150,
            Self::SoftwareVersion => 151,
            Self::SdkVersion => 152,
        }
    }
}
pub const ALL: [Key; 3] = Key::ALL;

fn printable(value: &Value) -> Option<&str> {
    let text = value.as_str()?;
    // Model and version spellings are displayed verbatim, never interpreted.
    // ASCII printable bytes exclude controls, line separators and invisible text.
    (!text.is_empty()
        && text.len() <= MAX_VALUE_BYTES
        && text.bytes().all(|byte| (b' '..=b'~').contains(&byte))
        && text.bytes().any(|byte| byte != b' '))
    .then_some(text)
}

/// `replies` is the existing collector's reply map, not the outer capture.
/// Missing/refused envelopes withhold all three rows. A bad individual value
/// withholds only its own row; unknown reply fields are neither copied nor inferred.
pub fn parse(replies: &Value) -> Vec<Setting> {
    let envelope = replies
        .get("system_info")
        .filter(|value| value.is_object())
        .filter(|value| value.get("returnValue").and_then(Value::as_bool) == Some(true));
    ALL.into_iter()
        .map(|key| {
            let value = envelope
                .and_then(|reply| reply.get(key.id()))
                .and_then(printable);
            Setting {
                constraint: None,
                id: key.id().into(),
                focus_key: key.focus_key(),
                label: key.label().into(),
                value: value.unwrap_or("Unavailable").into(),
                context: Context {
                    input: "global".into(),
                    dynamic_range: "global".into(),
                    picture_mode: "global".into(),
                },
                protected: true,
                available: value.is_some(),
            }
        })
        .collect()
}

#[cfg(test)]
#[path = "device_info_reads_tests.rs"]
mod tests;
