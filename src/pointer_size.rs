//! Unconnected, proof-only pointer-size candidate. Descriptor proof is not write proof.
//! No UI, worker, normal-preview enablement or generic setting parameters are exposed.
use serde::{Deserialize, Serialize};
#[cfg(any(test, target_arch = "arm"))]
use serde_json::json;
use serde_json::Value;
use std::collections::BTreeSet;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Size {
    Small,
    Medium,
    Large,
}
impl Size {
    pub const ALL: [Self; 3] = [Self::Small, Self::Medium, Self::Large];
    pub fn wire(self) -> &'static str {
        match self {
            Self::Small => "small",
            Self::Medium => "medium",
            Self::Large => "large",
        }
    }
    pub fn parse(value: &Value) -> Option<Self> {
        Self::ALL
            .into_iter()
            .find(|size| value.as_str() == Some(size.wire()))
    }
}
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Change {
    pub expected: Size,
    pub value: Size,
}
#[derive(Clone, Debug, Serialize)]
pub struct Outcome {
    pub change: Change,
    pub attempted: bool,
    pub admitted: bool,
    pub verified: bool,
    pub detail: String,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Op {
    SystemInfo,
    Foreground,
    Windows,
    Value,
    Descriptor,
    Set(Size),
}
/// Implementations must bound calls and supply monotonic milliseconds from one clock.
pub trait Platform {
    fn call(&mut self, op: Op) -> Result<Value, String>;
    fn wait(&mut self);
    fn now_ms(&self) -> u64;
}
fn get<P: Platform>(p: &mut P, op: Op) -> Result<Value, String> {
    let value = p.call(op)?;
    if value["returnValue"].as_bool() != Some(true) {
        return Err("Pointer-size operation refused".into());
    }
    Ok(value)
}
fn within(now: u64, then: u64, limit: u64) -> bool {
    now.checked_sub(then).is_some_and(|age| age <= limit)
}
fn current(value: &Value) -> Result<Size, String> {
    if value["category"].as_str() != Some("option")
        || (!value["dimension"].is_null()
            && !value["dimension"].as_object().is_some_and(|v| v.is_empty()))
    {
        return Err("Pointer-size scope mismatch".into());
    }
    Size::parse(&value["settings"]["pointerSize"])
        .ok_or_else(|| "Unknown pointer-size representation".into())
}
fn descriptor(value: &Value) -> Result<(), String> {
    let rows = value["results"]
        .as_array()
        .filter(|rows| rows.len() == 1)
        .ok_or("Ambiguous pointer-size descriptor")?;
    let d = &rows[0];
    if d["key"].as_str() != Some("pointerSize")
        || d["category"].as_str() != Some("option")
        || d["vtype"].as_str() != Some("ArrayExt")
        || d["dbtype"].as_str() != Some("G")
        || d["account"].as_bool() != Some(false)
        || d["volatile"].as_bool() != Some(false)
        || d["valueCheck"].as_bool() != Some(true)
        || d["valueTypeCheck"].as_bool() != Some(false)
        || !d["dimension"].as_array().is_some_and(|v| v.is_empty())
        || d["ui"]["visible"].as_bool() != Some(true)
        || d["ui"]["active"].as_bool() != Some(true)
    {
        return Err("Pointer-size metadata is not admitted by the candidate".into());
    }
    let choices = d["values"]["arrayExt"]
        .as_array()
        .filter(|rows| rows.len() == 3)
        .ok_or("Pointer-size choices changed")?;
    let mut found = BTreeSet::new();
    for choice in choices {
        if choice["visible"].as_bool() != Some(true) || choice["active"].as_bool() != Some(true) {
            return Err("Pointer-size choice unavailable".into());
        }
        let size = Size::parse(&choice["value"]).ok_or("Unknown pointer-size choice")?;
        if !found.insert(size.wire()) {
            return Err("Duplicate pointer-size choice".into());
        }
    }
    Ok(())
}
fn owner<P: Platform>(p: &mut P, own: &str) -> Result<(), String> {
    if get(p, Op::Foreground)?["appId"].as_str() != Some(own) {
        return Err("Proof app is not foreground".into());
    }
    let value = get(p, Op::Windows)?;
    let rows = value["windows"]
        .as_array()
        .filter(|rows| rows.len() <= 256)
        .ok_or("Window context unavailable")?;
    // Proof-only admission is deliberately narrower than the normal settings path.
    // A keyboard can cover the card while the foreground appId remains ours.
    if rows.is_empty()
        || !rows.iter().all(|row| {
            row["appId"].as_str() == Some(own)
                && row["windowType"].as_str() == Some("_WEBOS_WINDOW_TYPE_CARD")
        })
    {
        return Err("Proof card is covered or unavailable".into());
    }
    Ok(())
}
/// Candidate inspection only; success does not establish setter admission or effect.
pub fn inspect<P: Platform>(p: &mut P, own: &str) -> Result<Size, String> {
    if own != crate::identity::PROOF {
        return Err("Pointer-size candidate is proof-only".into());
    }
    let info = get(p, Op::SystemInfo)?;
    if info["modelName"].as_str() != Some("OLED55G26LA")
        || info["firmwareVersion"].as_str() != Some("23.25.55")
        || info["sdkVersion"].as_str() != Some("9.2.2")
    {
        return Err("Pointer-size candidate is not admitted for this platform".into());
    }
    owner(p, own)?;
    descriptor(&get(p, Op::Descriptor)?)?;
    current(&get(p, Op::Value)?)
}
/// Only a separately authorised isolated trial may pass `enabled=true`.
/// Restore via a reverse Change with a freshly checked expected value, never blindly.
pub fn apply<P: Platform>(
    p: &mut P,
    own: &str,
    change: Change,
    enabled: bool,
    queued_at_ms: u64,
) -> Outcome {
    let mut out = Outcome {
        change,
        attempted: false,
        admitted: false,
        verified: false,
        detail: String::new(),
    };
    let result = (|| {
        if !enabled {
            return Err("Pointer-size proof trial is disabled".into());
        }
        let started = p.now_ms();
        if !within(started, queued_at_ms, 2000) {
            return Err("Pointer-size gesture expired".into());
        }
        if inspect(p, own)? != change.expected {
            return Err("Pointer size changed since selection".into());
        }
        owner(p, own)?;
        if current(&get(p, Op::Value)?)? != change.expected {
            return Err("Pointer size changed before commit".into());
        }
        if !within(p.now_ms(), started, 4000) {
            return Err("Pointer-size validation expired".into());
        }
        if change.value == change.expected {
            out.verified = true;
            return Ok(());
        }
        out.attempted = true;
        if p.call(Op::Set(change.value))?["returnValue"].as_bool() != Some(true) {
            return Err("Pointer-size setter refused".into());
        }
        out.admitted = true;
        let readback_started = p.now_ms();
        for _ in 0..4 {
            if !within(p.now_ms(), readback_started, 3000) {
                break;
            }
            if current(&get(p, Op::Value)?)? == change.value {
                owner(p, own)?;
                if within(p.now_ms(), readback_started, 3000) {
                    out.verified = true;
                    return Ok(());
                }
                break;
            }
            p.wait();
        }
        Err("Setter acknowledged; bounded value readback did not confirm it".into())
    })();
    out.detail = match result {
        Ok(()) => "Pointer-size stored-value readback confirmed".into(),
        Err(error) => error,
    };
    out
}
/// One fixed payload mapping; no arbitrary category, key, URI or flags.
#[cfg(any(test, target_arch = "arm"))]
fn request(op: Op) -> (&'static str, Value) {
    match op {
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
            json!({"category":"option","keys":["pointerSize"]}),
        ),
        Op::Descriptor => (
            "luna://com.webos.service.settings/getSystemSettingDesc",
            json!({"category":"option","keys":["pointerSize"]}),
        ),
        Op::Set(size) => (
            "luna://com.webos.service.settings/setSystemSettings",
            json!({"category":"option","settings":{"pointerSize":size.wire()}}),
        ),
    }
}
#[cfg(target_arch = "arm")]
pub struct Native {
    bus: plx_platform::webos::ls2::Registration,
    clock: std::time::Instant,
}
#[cfg(target_arch = "arm")]
impl Native {
    pub fn new() -> Result<Self, String> {
        if plx_base::paths::app_id() != crate::identity::PROOF {
            return Err("Pointer-size native transport is proof-only".into());
        }
        plx_platform::webos::ls2::register()
            .map(|bus| Self {
                bus,
                clock: std::time::Instant::now(),
            })
            .map_err(|error| error.to_string())
    }
}
#[cfg(target_arch = "arm")]
impl Platform for Native {
    fn call(&mut self, op: Op) -> Result<Value, String> {
        let (uri, payload) = request(op);
        let raw = self
            .bus
            .call(
                uri,
                &payload.to_string(),
                std::time::Duration::from_millis(700),
            )
            .map_err(|_| "Pointer-size service call failed".to_owned())?;
        if raw.len() > 64 * 1024 {
            return Err("Pointer-size reply too large".into());
        }
        serde_json::from_str(&raw).map_err(|_| "Malformed pointer-size reply".into())
    }
    fn wait(&mut self) {
        std::thread::sleep(std::time::Duration::from_millis(100));
    }
    fn now_ms(&self) -> u64 {
        self.clock
            .elapsed()
            .as_millis()
            .try_into()
            .unwrap_or(u64::MAX)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    struct Fake {
        size: Size,
        info: Value,
        desc: Value,
        dimension: Value,
        calls: Vec<Op>,
        clock: u64,
        per_call: u64,
        gets: usize,
        foregrounds: usize,
        change_before: bool,
        lose_owner_at: usize,
        windows: Value,
        keyboard_on_recheck: bool,
        accept: bool,
        mutate: bool,
        set_error_after_mutation: Option<&'static str>,
        value_error_at: Option<usize>,
        external_change_at_readback: bool,
        external_change_at_owner_confirmation: bool,
    }
    impl Fake {
        fn new() -> Self {
            Self {
                size: Size::Medium,
                info: json!({"returnValue":true,"modelName":"OLED55G26LA","firmwareVersion":"23.25.55","sdkVersion":"9.2.2"}),
                desc: json!({"returnValue":true,"results":[{"key":"pointerSize","category":"option","dimension":[],"vtype":"ArrayExt","dbtype":"G","account":false,"volatile":false,"valueCheck":true,"valueTypeCheck":false,"ui":{"active":true,"visible":true},"values":{"arrayExt":Size::ALL.map(|size|json!({"value":size.wire(),"active":true,"visible":true}))}}]}),
                dimension: Value::Null,
                calls: Vec::new(),
                clock: 100,
                per_call: 0,
                gets: 0,
                foregrounds: 0,
                change_before: false,
                lose_owner_at: usize::MAX,
                windows: json!([{"appId":crate::identity::PROOF,"windowType":"_WEBOS_WINDOW_TYPE_CARD"}]),
                keyboard_on_recheck: false,
                accept: true,
                mutate: true,
                set_error_after_mutation: None,
                value_error_at: None,
                external_change_at_readback: false,
                external_change_at_owner_confirmation: false,
            }
        }
        fn sets(&self) -> Vec<Size> {
            self.calls
                .iter()
                .filter_map(|op| match op {
                    Op::Set(size) => Some(*size),
                    _ => None,
                })
                .collect()
        }
    }
    impl Platform for Fake {
        fn call(&mut self, op: Op) -> Result<Value, String> {
            self.calls.push(op);
            self.clock += self.per_call;
            Ok(match op {
                Op::SystemInfo => self.info.clone(),
                Op::Descriptor => self.desc.clone(),
                Op::Foreground => {
                    self.foregrounds += 1;
                    if self.external_change_at_owner_confirmation && self.foregrounds == 3 {
                        self.size = Size::Large;
                    }
                    json!({"returnValue":true,"appId":if self.foregrounds >= self.lose_owner_at {"com.example.other"} else {crate::identity::PROOF}})
                }
                Op::Windows => {
                    let mut windows = self.windows.clone();
                    if self.keyboard_on_recheck && self.foregrounds >= 2 {
                        windows.as_array_mut().unwrap().push(json!({"appId":"com.example.keyboard","windowType":"_WEBOS_WINDOW_TYPE_KEYBOARD"}));
                    }
                    json!({"returnValue":true,"windows":windows})
                }
                Op::Value => {
                    self.gets += 1;
                    if self.value_error_at == Some(self.gets) {
                        return Err("Transient pointer-size readback failure".into());
                    }
                    if self.change_before && self.gets == 2 {
                        self.size = Size::Large;
                    }
                    if self.external_change_at_readback && self.gets == 3 {
                        self.size = Size::Large;
                    }
                    json!({"returnValue":true,"category":"option","dimension":self.dimension,"settings":{"pointerSize":self.size.wire()}})
                }
                Op::Set(size) => {
                    if self.accept && self.mutate {
                        self.size = size;
                    }
                    if let Some(error) = self.set_error_after_mutation {
                        return Err(error.into());
                    }
                    json!({"returnValue":self.accept})
                }
            })
        }
        fn wait(&mut self) {
            self.clock += 100;
        }
        fn now_ms(&self) -> u64 {
            self.clock
        }
    }
    fn change() -> Change {
        Change {
            expected: Size::Medium,
            value: Size::Small,
        }
    }
    fn run(p: &mut Fake) -> Outcome {
        apply(p, crate::identity::PROOF, change(), true, 100)
    }
    #[test]
    fn change_and_reverse_restore_need_separate_stored_value_readback() {
        let mut p = Fake::new();
        let outcome = run(&mut p);
        assert!(outcome.attempted && outcome.admitted && outcome.verified);
        let restore = apply(
            &mut p,
            crate::identity::PROOF,
            Change {
                expected: Size::Small,
                value: Size::Medium,
            },
            true,
            100,
        );
        assert!(restore.verified);
        assert_eq!(p.size, Size::Medium);
        assert_eq!(p.sets(), [Size::Small, Size::Medium]);
        p.size = Size::Large;
        let failed_restore = apply(
            &mut p,
            crate::identity::PROOF,
            Change {
                expected: Size::Small,
                value: Size::Medium,
            },
            true,
            100,
        );
        assert!(!failed_restore.attempted);
        assert_eq!(p.size, Size::Large);
    }
    #[test]
    fn metadata_drift_and_context_changes_refuse_before_any_setter() {
        for case in 0..14 {
            let mut p = Fake::new();
            match case {
                0 => p.info["modelName"] = "OLED65C26LD".into(),
                1 => p.info["firmwareVersion"] = "unknown".into(),
                2 => p.desc["results"][0]["account"] = true.into(),
                3 => p.desc["results"][0]["volatile"] = true.into(),
                4 => p.desc["results"][0]["dimension"] = json!(["input"]),
                5 => p.desc["results"][0]["dbtype"] = "P".into(),
                6 => p.desc["results"][0]["values"]["arrayExt"][0]["value"] = "medium".into(),
                7 => p.desc["results"][0]["values"]["arrayExt"][0]["visible"] = false.into(),
                8 => p.desc["results"][0]["ui"]["active"] = false.into(),
                9 => p.dimension = json!({"input":"hdmi1"}),
                10 => p.windows.as_array_mut().unwrap().push(
                    json!({"appId":"com.example.other","windowType":"_WEBOS_WINDOW_TYPE_OVERLAY"}),
                ),
                11 => p.change_before = true,
                12 => p.lose_owner_at = 2,
                _ => p.desc["results"][0]["valueTypeCheck"] = true.into(),
            }
            assert!(!run(&mut p).attempted, "case {case}");
            assert!(p.sets().is_empty(), "case {case}");
        }
    }
    #[test]
    fn queue_and_validation_expiry_are_tested_with_a_fake_clock() {
        let mut p = Fake::new();
        p.clock = 2101;
        assert!(!run(&mut p).attempted);
        assert!(p.calls.is_empty());
        p.clock = 99; // An impossible future queue timestamp also fails closed.
        assert!(!run(&mut p).attempted);
        assert!(p.calls.is_empty());
        let mut p = Fake::new();
        p.per_call = 501;
        assert!(!run(&mut p).attempted);
        assert!(p.sets().is_empty());
    }
    #[test]
    fn keyboards_foreign_cards_and_unknown_windows_refuse_before_any_setter() {
        for row in [
            json!({"appId":"com.example.keyboard","windowType":"_WEBOS_WINDOW_TYPE_KEYBOARD"}),
            json!({"appId":crate::identity::PROOF,"windowType":"_WEBOS_WINDOW_TYPE_KEYBOARD"}),
            json!({"appId":"com.example.other","windowType":"_WEBOS_WINDOW_TYPE_CARD"}),
            json!({"appId":"com.example.other","windowType":"_WEBOS_WINDOW_TYPE_UNKNOWN"}),
            json!({"appId":crate::identity::PROOF,"windowType":"_WEBOS_WINDOW_TYPE_UNKNOWN"}),
            json!({"appId":crate::identity::PROOF}),
        ] {
            let mut p = Fake::new();
            p.windows.as_array_mut().unwrap().push(row);
            assert!(!run(&mut p).attempted);
            assert!(p.sets().is_empty());
        }
        let mut p = Fake::new();
        p.windows = json!([]);
        assert!(!run(&mut p).attempted);
        assert!(p.sets().is_empty());

        // Inspection was clear, but LG opened its keyboard before the final check.
        // The appId remains ours, so foreground-app equality alone is insufficient.
        let mut p = Fake::new();
        p.keyboard_on_recheck = true;
        assert!(!run(&mut p).attempted);
        assert_eq!(p.foregrounds, 2);
        assert!(p.sets().is_empty());
    }
    #[test]
    fn acknowledgement_is_distinct_from_readback_and_owner_confirmation() {
        let mut p = Fake::new();
        p.mutate = false;
        let result = run(&mut p);
        assert!(result.attempted && result.admitted && !result.verified);
        assert_eq!(p.gets, 6); // Two expected-value reads plus four bounded readbacks.
        let mut p = Fake::new();
        p.accept = false;
        let result = run(&mut p);
        assert!(result.attempted && !result.admitted && !result.verified);
        let mut p = Fake::new();
        p.lose_owner_at = 3;
        let result = run(&mut p);
        assert!(result.attempted && result.admitted && !result.verified);
    }
    #[test]
    fn mutating_setter_timeout_is_ambiguous_and_needs_a_fresh_guarded_recovery() {
        let mut p = Fake::new();
        p.set_error_after_mutation = Some("Pointer-size setter timed out after dispatch");
        let result = run(&mut p);
        assert!(result.attempted && !result.admitted && !result.verified);
        assert_eq!(p.size, Size::Small); // No acknowledgement does not imply no mutation.
        assert_eq!(p.sets(), [Size::Small]);

        p.set_error_after_mutation = None;
        let actual = inspect(&mut p, crate::identity::PROOF).unwrap();
        assert_eq!(actual, change().value);
        let queued = p.now_ms();
        let restored = apply(
            &mut p,
            crate::identity::PROOF,
            Change {
                expected: actual,
                value: change().expected,
            },
            true,
            queued,
        );
        assert!(restored.verified);
        assert_eq!(p.size, Size::Medium);
        assert_eq!(p.sets(), [Size::Small, Size::Medium]);
    }
    #[test]
    fn acknowledged_setter_then_transient_read_error_can_already_have_applied() {
        let mut p = Fake::new();
        p.value_error_at = Some(3); // First post-set read; earlier expectation checks passed.
        let result = run(&mut p);
        assert!(result.attempted && result.admitted && !result.verified);
        assert_eq!(p.size, Size::Small);
        assert_eq!(p.sets(), [Size::Small]);
        assert_eq!(p.gets, 3);
        // The caller re-inspects; apply did not silently retry or restore after the error.
        assert_eq!(
            inspect(&mut p, crate::identity::PROOF).unwrap(),
            Size::Small
        );
        assert_eq!(p.sets(), [Size::Small]);
    }
    #[test]
    fn external_changes_during_readback_or_owner_confirmation_do_not_have_cas_semantics() {
        let reverse = Change {
            expected: Size::Small,
            value: Size::Medium,
        };
        let mut p = Fake::new();
        p.external_change_at_readback = true;
        let result = run(&mut p);
        assert!(result.attempted && result.admitted && !result.verified);
        assert_eq!(p.size, Size::Large);
        assert_eq!(
            inspect(&mut p, crate::identity::PROOF).unwrap(),
            Size::Large
        );
        assert!(!apply(&mut p, crate::identity::PROOF, reverse, true, 100).attempted);
        assert_eq!(p.sets(), [Size::Small]); // Never overwrite the observed external change.

        let mut p = Fake::new();
        p.external_change_at_owner_confirmation = true;
        let result = run(&mut p);
        assert!(result.attempted && result.admitted && result.verified);
        // Verification means the target was sampled, followed by valid ownership.
        // Another writer can change it between calls; verified is not atomic exclusion
        // or a guarantee that the sampled value remains current on return.
        assert_eq!(p.size, Size::Large);
        assert_eq!(
            inspect(&mut p, crate::identity::PROOF).unwrap(),
            Size::Large
        );
        assert!(!apply(&mut p, crate::identity::PROOF, reverse, true, 100).attempted);
        assert_eq!(p.sets(), [Size::Small]);
    }
    #[test]
    fn closed_contract_rejects_preview_identity_extra_fields_and_arbitrary_payloads() {
        let mut p = Fake::new();
        assert!(!apply(&mut p, crate::identity::PREVIEW, change(), true, 100).attempted);
        assert!(p.calls.is_empty());
        assert!(!apply(&mut p, crate::identity::PROOF, change(), false, 100).attempted);
        assert!(p.calls.is_empty());
        for value in [
            json!({"expected":"medium","value":"huge"}),
            json!({"expected":"medium","value":"small","key":"screenShift"}),
        ] {
            assert!(serde_json::from_value::<Change>(value).is_err());
        }
        let (uri, payload) = request(Op::Set(Size::Small));
        assert_eq!(uri, "luna://com.webos.service.settings/setSystemSettings");
        assert_eq!(
            payload,
            json!({"category":"option","settings":{"pointerSize":"small"}})
        );
        assert!(current(&json!({"category":"option","settings":{"pointerSize":1}})).is_err());
        let mut p = Fake::new();
        assert!(
            apply(
                &mut p,
                crate::identity::PROOF,
                Change {
                    expected: Size::Medium,
                    value: Size::Medium
                },
                true,
                100
            )
            .verified
        );
        assert!(p.sets().is_empty());
    }
}
