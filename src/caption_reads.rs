//! One fixed read-only TV caption preference. No setter, descriptor or capability.
//! The documented on/off values are admitted; unknown values are withheld.
//! This does not establish UK DVB, player subtitles, or active caption rendering.
use crate::model::{Context, Setting};
use serde_json::{json, Value};

pub const VALUES_REPLY: &str = "caption_values";
pub const ID: &str = "captionEnable";
pub const FOCUS_KEY: u32 = 107;

pub fn operations() -> [(&'static str, &'static str, Value); 1] {
    [(
        VALUES_REPLY,
        "luna://com.webos.settingsservice/getSystemSettings",
        json!({"category":"caption","keys":[ID],"subscribe":false}),
    )]
}

fn known_value(replies: &Value) -> Option<&str> {
    let reply = replies.get(VALUES_REPLY)?.as_object()?;
    // This closed envelope is the actual isolated ordinary GET shape plus an
    // optional empty dimension. Diagnostic/error mixtures cannot become a value.
    if reply.keys().any(|key| {
        !matches!(
            key.as_str(),
            "returnValue" | "category" | "method" | "subscribed" | "settings" | "dimension"
        )
    }) || reply.get("returnValue")?.as_bool() != Some(true)
        || reply.get("category")?.as_str() != Some("caption")
        || reply.get("method")?.as_str() != Some("getSystemSettings")
        || reply.get("subscribed")?.as_bool() != Some(false)
        || reply
            .get("dimension")
            .is_some_and(|value| !value.as_object().is_some_and(|dims| dims.is_empty()))
    {
        return None;
    }
    let settings = reply.get("settings")?.as_object()?;
    if settings.len() != 1 {
        return None;
    }
    match settings.get(ID)?.as_str()? {
        value @ ("on" | "off") => Some(value),
        _ => None,
    }
}

/// Supply this row every cycle, including missing/refused/malformed replies.
/// `global` is a presentation binding; it does not infer database scope or an
/// active programme. Merge binds it to the current snapshot like other readouts.
pub fn parse(replies: &Value) -> Setting {
    let value = known_value(replies);
    Setting {
        id: ID.into(),
        label: "TV caption preference".into(),
        focus_key: FOCUS_KEY,
        value: value.unwrap_or("Unavailable").into(),
        constraint: None,
        protected: true,
        available: value.is_some(),
        context: Context {
            input: "global".into(),
            dynamic_range: "global".into(),
            picture_mode: "global".into(),
        },
    }
}

#[cfg(test)]
#[path = "caption_reads_tests.rs"]
mod tests;
