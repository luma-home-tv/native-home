//! Experimental fixed numeric operation, restricted to the isolated proof identity.
//! Never changes audio-description enablement, master volume, sound mode or routing.
use serde::{Deserialize, Serialize};
#[cfg(any(test, target_arch = "arm"))]
use serde_json::json;
use serde_json::Value;

/// webOS24 audio-description level: signed -15..=15, not master volume or percent.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(try_from = "i8", into = "i8")]
pub struct Level(i8);
impl Level {
    pub const MIN: i8 = -15;
    pub const MAX: i8 = 15;
    pub fn new(value: i8) -> Option<Self> {
        (Self::MIN..=Self::MAX)
            .contains(&value)
            .then_some(Self(value))
    }
    pub fn value(self) -> i8 {
        self.0
    }
    pub fn adjacent(self) -> Self {
        Self(if self.0 == Self::MAX {
            self.0 - 1
        } else {
            self.0 + 1
        })
    }
    fn parse(value: &Value) -> Option<Self> {
        let n = if let Some(s) = value.as_str() {
            let n: i8 = s.parse().ok()?;
            if n.to_string() != s {
                return None;
            }
            n
        } else {
            i8::try_from(value.as_i64()?).ok()?
        };
        Self::new(n)
    }
}
impl TryFrom<i8> for Level {
    type Error = &'static str;
    fn try_from(value: i8) -> Result<Self, Self::Error> {
        Self::new(value).ok_or("Audio-description level outside -15..15")
    }
}
impl From<Level> for i8 {
    fn from(value: Level) -> Self {
        value.0
    }
}
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Change {
    pub expected: Level,
    pub value: Level,
}
#[derive(Clone, Debug, Serialize)]
pub struct Outcome {
    pub change: Change,
    pub attempted: bool,
    pub admitted: bool,
    pub verified: bool,
    pub detail: String,
    pub wire_format: Option<Encoding>,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum Encoding {
    Integer,
    DecimalString,
}
impl Encoding {
    #[cfg(any(test, target_arch = "arm"))]
    fn encode(self, level: Level) -> Value {
        match self {
            Self::Integer => Value::from(level.value()),
            Self::DecimalString => Value::from(level.value().to_string()),
        }
    }
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
struct Reading {
    level: Level,
    encoding: Encoding,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Op {
    SystemInfo,
    Foreground,
    Windows,
    Value,
    Descriptor,
    Set(Level, Encoding),
}
pub trait Platform {
    fn call(&mut self, op: Op) -> Result<Value, String>;
    fn wait(&mut self);
}
fn get<P: Platform>(p: &mut P, op: Op) -> Result<Value, String> {
    let v = p.call(op)?;
    if v["returnValue"].as_bool() != Some(true) {
        return Err("Audio-description level operation refused".into());
    }
    Ok(v)
}
fn current(value: &Value) -> Result<Reading, String> {
    if value["category"].as_str() != Some("sound")
        || (!value["dimension"].is_null()
            && !value["dimension"].as_object().is_some_and(|o| o.is_empty()))
        || value["settings"]["audioDescription"].as_str() != Some("off")
    {
        return Err("Audio description must be off in the global sound scope".into());
    }
    let raw = &value["settings"]["audioDescriptionVolume"];
    Ok(Reading {
        level: Level::parse(raw).ok_or("Unknown audio-description level")?,
        encoding: if raw.is_string() {
            Encoding::DecimalString
        } else {
            Encoding::Integer
        },
    })
}
fn descriptor(value: &Value) -> Result<(), String> {
    let rows = value["results"]
        .as_array()
        .filter(|r| r.len() == 2)
        .ok_or("Ambiguous audio-description descriptors")?;
    for key in ["audioDescription", "audioDescriptionVolume"] {
        let matching: Vec<_> = rows
            .iter()
            .filter(|r| r["key"].as_str() == Some(key))
            .collect();
        if matching.len() != 1 {
            return Err("Missing or duplicate audio-description descriptor".into());
        }
        let d = matching[0];
        if d["category"].as_str() != Some("sound")
            || d["dbtype"].as_str() != Some("G")
            || d["account"].as_bool() != Some(false)
            || d["volatile"].as_bool() != Some(false)
            || d["valueCheck"].as_bool() != Some(true)
            || !d["dimension"].as_array().is_some_and(|a| a.is_empty())
            || d["ui"]["visible"].as_bool() != Some(true)
            || d["ui"]["active"].as_bool() != Some(true)
        {
            return Err("Audio-description metadata is not validated".into());
        }
        if key == "audioDescriptionVolume" {
            let r = &d["values"]["range"];
            if d["vtype"].as_str() != Some("Range")
                || r["min"].as_i64() != Some(-15)
                || r["max"].as_i64() != Some(15)
                || r["interval"].as_i64() != Some(1)
            {
                return Err("Audio-description range changed".into());
            }
        } else {
            let choices = d["values"]["arrayExt"]
                .as_array()
                .filter(|a| a.len() == 2)
                .ok_or("Audio-description choices changed")?;
            if d["vtype"].as_str() != Some("ArrayExt")
                || !["on", "off"].iter().all(|v| {
                    choices
                        .iter()
                        .filter(|c| {
                            c["value"].as_str() == Some(v)
                                && c["active"].as_bool() == Some(true)
                                && c["visible"].as_bool() == Some(true)
                        })
                        .count()
                        == 1
                })
            {
                return Err("Audio-description enable metadata changed".into());
            }
        }
    }
    Ok(())
}
fn owner<P: Platform>(p: &mut P, own: &str) -> Result<(), String> {
    if get(p, Op::Foreground)?["appId"].as_str() != Some(own) {
        return Err("Dashboard is not foreground".into());
    }
    let w = get(p, Op::Windows)?;
    let rows = w["windows"]
        .as_array()
        .filter(|r| r.len() <= 256)
        .ok_or("Window context unavailable")?;
    if !rows.iter().any(|r| {
        r["appId"].as_str() == Some(own)
            && r["windowType"].as_str() == Some("_WEBOS_WINDOW_TYPE_CARD")
    }) || rows
        .iter()
        .any(|r| r["windowType"].as_str() == Some("_WEBOS_WINDOW_TYPE_OVERLAY"))
    {
        return Err("Dashboard window is covered or unavailable".into());
    }
    Ok(())
}
pub fn inspect<P: Platform>(p: &mut P, own: &str) -> Result<Level, String> {
    inspect_reading(p, own).map(|r| r.level)
}
fn inspect_reading<P: Platform>(p: &mut P, own: &str) -> Result<Reading, String> {
    if own != crate::identity::PROOF {
        return Err("Numeric trial requires isolated proof identity".into());
    }
    let info = get(p, Op::SystemInfo)?;
    if info["firmwareVersion"].as_str() != Some("23.25.55")
        || info["sdkVersion"].as_str() != Some("9.2.2")
        || !matches!(
            info["modelName"].as_str(),
            Some("OLED55G26LA" | "OLED65C26LD")
        )
    {
        return Err("Audio-description level writes are not validated for this firmware".into());
    }
    owner(p, own)?;
    descriptor(&get(p, Op::Descriptor)?)?;
    current(&get(p, Op::Value)?)
}
pub fn apply<P: Platform>(p: &mut P, own: &str, change: Change, enabled: bool) -> Outcome {
    let mut out = Outcome {
        change,
        attempted: false,
        admitted: false,
        verified: false,
        detail: String::new(),
        wire_format: None,
    };
    let result = (|| {
        if !enabled {
            return Err("Audio-description level write capability is disabled".into());
        }
        let start = std::time::Instant::now();
        let initial = inspect_reading(p, own)?;
        if initial.level != change.expected {
            return Err("Audio-description level changed since selection".into());
        }
        owner(p, own)?;
        if current(&get(p, Op::Value)?)? != initial {
            return Err("Audio-description level changed before commit".into());
        }
        if start.elapsed() > std::time::Duration::from_secs(4) {
            return Err("Audio-description level validation expired".into());
        }
        out.wire_format = Some(initial.encoding);
        if change.value == change.expected {
            out.verified = true;
            return Ok(());
        }
        if (change.value.value() as i16 - change.expected.value() as i16).abs() != 1 {
            return Err("Numeric trial permits one step only".into());
        }
        out.attempted = true;
        let reply = p.call(Op::Set(change.value, initial.encoding))?;
        if reply["returnValue"].as_bool() != Some(true) {
            return Err("Audio-description level setter refused".into());
        }
        out.admitted = true;
        for _ in 0..4 {
            if current(&get(p, Op::Value)?)?
                == (Reading {
                    level: change.value,
                    encoding: initial.encoding,
                })
            {
                owner(p, own)?;
                out.verified = true;
                return Ok(());
            }
            p.wait();
        }
        Err("Setter acknowledged; value readback did not confirm it".into())
    })();
    out.detail = match result {
        Ok(()) => "Audio-description level readback confirmed".into(),
        Err(e) => e,
    };
    out
}
#[cfg(target_arch = "arm")]
pub struct Native {
    bus: plx_platform::webos::ls2::Registration,
}
#[cfg(target_arch = "arm")]
impl Native {
    pub fn new() -> Result<Self, String> {
        plx_platform::webos::ls2::register()
            .map(|bus| Self { bus })
            .map_err(|e| e.to_string())
    }
}
#[cfg(target_arch = "arm")]
impl Platform for Native {
    fn call(&mut self, op: Op) -> Result<Value, String> {
        let (uri, payload) = match op {
            Op::SystemInfo => (
                "luna://com.webos.service.tv.systemproperty/getSystemInfo",
                json!({"keys":["modelName","firmwareVersion","sdkVersion"]}),
            ),
            Op::Foreground => (
                "luna://com.webos.applicationManager/getForegroundAppInfo",
                json!({}),
            ),
            Op::Windows => (
                "luna://com.webos.surfacemanager/getForegroundWindowInfo",
                json!({}),
            ),
            Op::Value => (
                "luna://com.webos.service.settings/getSystemSettings",
                json!({"category":"sound","keys":["audioDescription","audioDescriptionVolume"]}),
            ),
            Op::Descriptor => (
                "luna://com.webos.service.settings/getSystemSettingDesc",
                json!({"category":"sound","keys":["audioDescription","audioDescriptionVolume"]}),
            ),
            Op::Set(value, encoding) => (
                "luna://com.webos.service.settings/setSystemSettings",
                json!({"category":"sound","settings":{"audioDescriptionVolume":encoding.encode(value)}}),
            ),
        };
        let raw = self
            .bus
            .call(
                uri,
                &payload.to_string(),
                std::time::Duration::from_millis(700),
            )
            .map_err(|_| "Audio-description level service call failed".to_owned())?;
        if raw.len() > 64 * 1024 {
            return Err("Audio-description level reply too large".into());
        }
        serde_json::from_str(&raw).map_err(|_| "Malformed audio-description level reply".into())
    }
    fn wait(&mut self) {
        std::thread::sleep(std::time::Duration::from_millis(100));
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    struct Fake {
        level: Level,
        enabled: bool,
        sets: Vec<Level>,
        mutate: bool,
        info: Value,
        desc: Value,
        foreign: bool,
        covered: bool,
        changed_before: bool,
        enabled_before: bool,
        gets: usize,
        encoding: Encoding,
        set_encodings: Vec<Encoding>,
        drift_encoding: bool,
    }
    impl Fake {
        fn new() -> Self {
            let common = json!({"category":"sound","dimension":[],"dbtype":"G","account":false,"volatile":false,"valueCheck":true,"ui":{"visible":true,"active":true}});
            let mut range = common.clone();
            range["key"] = "audioDescriptionVolume".into();
            range["vtype"] = "Range".into();
            range["values"] = json!({"range":{"min":-15,"max":15,"interval":1}});
            let mut enabled = common;
            enabled["key"] = "audioDescription".into();
            enabled["vtype"] = "ArrayExt".into();
            enabled["values"] = json!({"arrayExt":[{"value":"on","visible":true,"active":true},{"value":"off","visible":true,"active":true}]});
            Self {
                level: Level::new(10).unwrap(),
                enabled: false,
                sets: vec![],
                mutate: true,
                info: json!({"returnValue":true,"modelName":"OLED55G26LA","firmwareVersion":"23.25.55","sdkVersion":"9.2.2"}),
                desc: json!({"returnValue":true,"results":[range,enabled]}),
                foreign: false,
                covered: false,
                changed_before: false,
                enabled_before: false,
                gets: 0,
                encoding: Encoding::DecimalString,
                set_encodings: vec![],
                drift_encoding: false,
            }
        }
    }
    impl Platform for Fake {
        fn call(&mut self, op: Op) -> Result<Value, String> {
            Ok(match op {
                Op::SystemInfo => self.info.clone(),
                Op::Descriptor => self.desc.clone(),
                Op::Foreground => {
                    json!({"returnValue":true,"appId":if self.foreign{"other"}else{crate::identity::PROOF}})
                }
                Op::Windows => {
                    json!({"returnValue":true,"windows":[{"appId":crate::identity::PROOF,"windowType":"_WEBOS_WINDOW_TYPE_CARD"},{"appId":"other","windowType":if self.covered{"_WEBOS_WINDOW_TYPE_OVERLAY"}else{"_WEBOS_WINDOW_TYPE_CARD"}}]})
                }
                Op::Value => {
                    self.gets += 1;
                    if self.gets == 2 {
                        if self.drift_encoding {
                            self.encoding = Encoding::Integer;
                        }
                        if self.changed_before {
                            self.level = Level::new(8).unwrap();
                        }
                        if self.enabled_before {
                            self.enabled = true;
                        }
                    }
                    json!({"returnValue":true,"category":"sound","dimension":{},"settings":{"audioDescription":if self.enabled{"on"}else{"off"},"audioDescriptionVolume":self.encoding.encode(self.level)}})
                }
                Op::Set(value, encoding) => {
                    self.set_encodings.push(encoding);
                    self.sets.push(value);
                    if self.mutate {
                        self.level = value;
                        self.encoding = encoding;
                    }
                    json!({"returnValue":true})
                }
            })
        }
        fn wait(&mut self) {}
    }
    fn trial(p: &mut Fake) -> Outcome {
        apply(
            p,
            crate::identity::PROOF,
            Change {
                expected: Level::new(10).unwrap(),
                value: Level::new(11).unwrap(),
            },
            true,
        )
    }
    #[test]
    fn signed_range_accepts_only_canonical_bounded_integers() {
        for n in -15i8..=15 {
            assert_eq!(Level::parse(&json!(n)), Level::new(n));
            assert_eq!(Level::parse(&json!(n.to_string())), Level::new(n));
            let wire = serde_json::to_value(Level::new(n).unwrap()).unwrap();
            assert_eq!(wire, json!(n));
        }
        for v in [
            json!(-16),
            json!(16),
            json!(10.0),
            json!(true),
            json!(null),
            json!("+1"),
            json!("01"),
            json!("-0"),
            json!(" 1"),
            json!("1e0"),
            json!("1.0"),
        ] {
            assert!(Level::parse(&v).is_none(), "{v}");
        }
        for v in [
            json!({"expected":10,"value":16}),
            json!({"expected":10,"value":11,"key":"audioBalance"}),
            json!({"expected":10,"value":11,"uri":"arbitrary"}),
        ] {
            assert!(serde_json::from_value::<Change>(v).is_err());
        }
        assert_eq!(Level::new(15).unwrap().adjacent().value(), 14);
        assert_eq!(Level::new(-15).unwrap().adjacent().value(), -14);
    }
    #[test]
    fn one_step_and_restoration_require_readback_not_acknowledgement() {
        let mut p = Fake::new();
        let r = trial(&mut p);
        assert!(r.attempted && r.admitted && r.verified);
        assert!(
            apply(
                &mut p,
                crate::identity::PROOF,
                Change {
                    expected: Level::new(11).unwrap(),
                    value: Level::new(10).unwrap()
                },
                true
            )
            .verified
        );
        assert_eq!(p.level.value(), 10);
        assert_eq!(
            p.sets.iter().map(|v| v.value()).collect::<Vec<_>>(),
            vec![11, 10]
        );
        let mut p = Fake::new();
        p.mutate = false;
        let r = trial(&mut p);
        assert!(r.attempted && r.admitted && !r.verified);
    }
    #[test]
    fn enabled_description_stale_value_covered_owner_or_schema_drift_refuse_before_set() {
        for case in 0..16 {
            let mut p = Fake::new();
            match case {
                0 => p.enabled = true,
                1 => p.enabled_before = true,
                2 => p.foreign = true,
                3 => p.covered = true,
                4 => p.changed_before = true,
                5 => p.info["firmwareVersion"] = "unknown".into(),
                6 => p.desc["results"][0]["dimension"] = json!(["input"]),
                7 => p.desc["results"][0]["values"]["range"]["max"] = 100.into(),
                8 => p.desc["results"][0]["values"]["range"]["interval"] = 2.into(),
                9 => p.desc["results"][0]["ui"]["active"] = false.into(),
                10 => p.desc["results"][0]["account"] = true.into(),
                11 => p.desc["results"][1]["values"]["arrayExt"][1]["active"] = false.into(),
                12 => p.desc["results"][1]["key"] = "audioDescriptionVolume".into(),
                13 => p.desc["results"][0]["dbtype"] = "other".into(),
                14 => p.desc["returnValue"] = false.into(),
                _ => p.info["modelName"] = "unknown".into(),
            }
            let r = trial(&mut p);
            assert!(!r.attempted && p.sets.is_empty(), "case {case}: {r:?}");
        }
    }
    #[test]
    fn trial_identity_gate_and_one_step_policy_keep_normal_preview_read_only() {
        let c = Change {
            expected: Level::new(10).unwrap(),
            value: Level::new(11).unwrap(),
        };
        for own in [crate::identity::PREVIEW, "other"] {
            let mut p = Fake::new();
            assert!(!apply(&mut p, own, c, true).attempted);
            assert!(p.sets.is_empty());
        }
        let mut p = Fake::new();
        assert!(!apply(&mut p, crate::identity::PROOF, c, false).attempted);
        let mut p = Fake::new();
        assert!(
            !apply(
                &mut p,
                crate::identity::PROOF,
                Change {
                    expected: c.expected,
                    value: Level::new(12).unwrap()
                },
                true
            )
            .attempted
        );
        let mut p = Fake::new();
        assert!(
            apply(
                &mut p,
                crate::identity::PROOF,
                Change {
                    expected: c.expected,
                    value: c.expected
                },
                true
            )
            .verified
        );
        assert!(p.sets.is_empty());
    }
    #[test]
    fn change_and_restore_preserve_the_original_wire_representation() {
        for encoding in [Encoding::DecimalString, Encoding::Integer] {
            let mut p = Fake::new();
            p.encoding = encoding;
            let before = p.encoding.encode(p.level);
            let result = trial(&mut p);
            assert!(result.verified);
            assert_eq!(result.wire_format, Some(encoding));
            let changed = p.encoding.encode(p.level);
            assert_eq!(changed, encoding.encode(Level::new(11).unwrap()));
            let restored = apply(
                &mut p,
                crate::identity::PROOF,
                Change {
                    expected: Level::new(11).unwrap(),
                    value: Level::new(10).unwrap(),
                },
                true,
            );
            assert!(restored.verified);
            assert_eq!(p.encoding.encode(p.level), before);
            assert_eq!(p.set_encodings, vec![encoding, encoding]);
        }
        let mut p = Fake::new();
        p.drift_encoding = true;
        assert!(!trial(&mut p).attempted);
        assert!(p.sets.is_empty());
    }
}
