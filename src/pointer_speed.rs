//! One fixed ordinary-app write. No arbitrary setting, category, service or parameter is accepted.
use serde::{Deserialize, Serialize};
#[cfg(any(test, target_arch = "arm"))]
use serde_json::json;
use serde_json::Value;
use std::collections::BTreeSet;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Speed {
    Slow,
    Normal,
    Fast,
}
impl Speed {
    pub const ALL: [Self; 3] = [Self::Slow, Self::Normal, Self::Fast];
    pub fn wire(self) -> &'static str {
        match self {
            Self::Slow => "slow",
            Self::Normal => "normal",
            Self::Fast => "fast",
        }
    }
    pub fn label(self) -> &'static str {
        match self {
            Self::Slow => "Slow",
            Self::Normal => "Normal",
            Self::Fast => "Fast",
        }
    }
    pub fn parse(value: &Value) -> Option<Self> {
        match value.as_str()? {
            "slow" => Some(Self::Slow),
            "normal" => Some(Self::Normal),
            "fast" => Some(Self::Fast),
            _ => None,
        }
    }
}
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Change {
    pub expected: Speed,
    pub value: Speed,
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
    Set(Speed),
}
pub trait Platform {
    fn call(&mut self, op: Op) -> Result<Value, String>;
    fn wait(&mut self);
}
fn get<P: Platform>(p: &mut P, op: Op) -> Result<Value, String> {
    let v = p.call(op)?;
    if v["returnValue"].as_bool() != Some(true) {
        return Err("Pointer-speed operation refused".into());
    }
    Ok(v)
}
fn current(value: &Value) -> Result<Speed, String> {
    if value["category"].as_str() != Some("option")
        || (!value["dimension"].is_null()
            && !value["dimension"].as_object().is_some_and(|o| o.is_empty()))
    {
        return Err("Pointer-speed scope mismatch".into());
    }
    Speed::parse(&value["settings"]["pointerSpeed"]).ok_or("Unknown pointer-speed value".into())
}
fn descriptor(value: &Value) -> Result<(), String> {
    let rows = value["results"]
        .as_array()
        .filter(|r| r.len() == 1)
        .ok_or("Ambiguous pointer-speed descriptor")?;
    let d = &rows[0];
    if d["key"].as_str() != Some("pointerSpeed")
        || d["category"].as_str() != Some("option")
        || d["vtype"].as_str() != Some("ArrayExt")
        || d["dbtype"].as_str() != Some("G")
        || d["account"].as_bool() != Some(false)
        || d["volatile"].as_bool() != Some(false)
        || d["valueCheck"].as_bool() != Some(true)
        || !d["dimension"].as_array().is_some_and(|a| a.is_empty())
        || d["ui"]["visible"].as_bool() != Some(true)
        || d["ui"]["active"].as_bool() != Some(true)
    {
        return Err("Pointer-speed metadata is not validated".into());
    }
    let choices = d["values"]["arrayExt"]
        .as_array()
        .filter(|r| r.len() == 3)
        .ok_or("Pointer-speed choices changed")?;
    let mut found = BTreeSet::new();
    for c in choices {
        if c["visible"].as_bool() != Some(true) || c["active"].as_bool() != Some(true) {
            return Err("Pointer-speed choice unavailable".into());
        }
        let s = Speed::parse(&c["value"]).ok_or("Unknown pointer-speed choice")?;
        if !found.insert(s.wire()) {
            return Err("Duplicate pointer-speed choice".into());
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
pub fn inspect<P: Platform>(p: &mut P, own: &str) -> Result<Speed, String> {
    if crate::identity::dashboard(own).is_none() {
        return Err("Unexpected dashboard identity".into());
    }
    let info = get(p, Op::SystemInfo)?;
    if info["firmwareVersion"].as_str() != Some("23.25.55")
        || info["sdkVersion"].as_str() != Some("9.2.2")
        || !matches!(
            info["modelName"].as_str(),
            Some("OLED55G26LA" | "OLED65C26LD")
        )
    {
        return Err("Pointer-speed writes are not validated for this firmware".into());
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
    };
    let result = (|| {
        if !enabled {
            return Err("Pointer-speed write capability is disabled".into());
        }
        let start = std::time::Instant::now();
        if inspect(p, own)? != change.expected {
            return Err("Pointer speed changed since selection".into());
        }
        owner(p, own)?;
        if current(&get(p, Op::Value)?)? != change.expected {
            return Err("Pointer speed changed before commit".into());
        }
        if start.elapsed() > std::time::Duration::from_secs(4) {
            return Err("Pointer-speed validation expired".into());
        }
        if change.value == change.expected {
            out.verified = true;
            return Ok(());
        }
        out.attempted = true;
        let reply = p.call(Op::Set(change.value))?;
        if reply["returnValue"].as_bool() != Some(true) {
            return Err("Pointer-speed setter refused".into());
        }
        out.admitted = true;
        for _ in 0..4 {
            if current(&get(p, Op::Value)?)? == change.value {
                owner(p, own)?;
                out.verified = true;
                return Ok(());
            }
            p.wait();
        }
        Err("Setter acknowledged; value readback did not confirm it".into())
    })();
    out.detail = match result {
        Ok(()) => "Pointer-speed readback confirmed".into(),
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
                json!({"category":"option","keys":["pointerSpeed"]}),
            ),
            Op::Descriptor => (
                "luna://com.webos.service.settings/getSystemSettingDesc",
                json!({"category":"option","keys":["pointerSpeed"]}),
            ),
            Op::Set(value) => (
                "luna://com.webos.service.settings/setSystemSettings",
                json!({"category":"option","settings":{"pointerSpeed":value.wire()}}),
            ),
        };
        let raw = self
            .bus
            .call(
                uri,
                &payload.to_string(),
                std::time::Duration::from_millis(700),
            )
            .map_err(|_| "Pointer-speed service call failed".to_owned())?;
        if raw.len() > 64 * 1024 {
            return Err("Pointer-speed reply too large".into());
        }
        serde_json::from_str(&raw).map_err(|_| "Malformed pointer-speed reply".into())
    }
    fn wait(&mut self) {
        std::thread::sleep(std::time::Duration::from_millis(100));
    }
}
/// The GL/input thread only exchanges bounded typed messages with this worker.
pub struct Worker {
    requests: std::sync::mpsc::SyncSender<(std::time::Instant, Change)>,
    replies: std::sync::mpsc::Receiver<Outcome>,
    ready: std::sync::mpsc::Receiver<bool>,
    thread: std::thread::JoinHandle<()>,
}
impl Worker {
    #[cfg(target_arch = "arm")]
    pub fn native(own: String) -> std::io::Result<Self> {
        let (requests, receive) = std::sync::mpsc::sync_channel::<(std::time::Instant, Change)>(1);
        let (send, replies) = std::sync::mpsc::channel();
        let (ready_send, ready) = std::sync::mpsc::channel();
        let thread = std::thread::Builder::new()
            .name("pointer-control".into())
            .spawn(move || {
                let mut p = Native::new().ok();
                let mut supported = false;
                if let Some(p) = p.as_mut() {
                    for _ in 0..10 {
                        if inspect(p, &own).is_ok() {
                            supported = true;
                            break;
                        }
                        p.wait();
                    }
                }
                let _ = ready_send.send(supported);
                while let Ok((queued, change)) = receive.recv() {
                    let result = if queued.elapsed() > std::time::Duration::from_secs(2) {
                        Outcome {
                            change,
                            attempted: false,
                            admitted: false,
                            verified: false,
                            detail: "Pointer-speed gesture expired".into(),
                        }
                    } else {
                        match p.as_mut() {
                            Some(p) => apply(p, &own, change, supported),
                            None => Outcome {
                                change,
                                attempted: false,
                                admitted: false,
                                verified: false,
                                detail: "Pointer-speed service unavailable".into(),
                            },
                        }
                    };
                    if send.send(result).is_err() {
                        break;
                    }
                }
            })?;
        Ok(Self {
            requests,
            replies,
            ready,
            thread,
        })
    }
    pub fn queue(&self, change: Change) -> Result<(), String> {
        self.requests
            .try_send((std::time::Instant::now(), change))
            .map_err(|_| "Pointer-speed change already pending".into())
    }
    pub fn ready(&self) -> Option<bool> {
        self.ready.try_iter().last()
    }
    pub fn poll(&self) -> Vec<Outcome> {
        self.replies.try_iter().collect()
    }
    pub fn finish(self) -> Vec<Outcome> {
        drop(self.requests);
        let _ = self.thread.join();
        self.replies.try_iter().collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    struct Fake {
        speed: Speed,
        sets: Vec<Speed>,
        mutate: bool,
        info: Value,
        desc: Value,
        foreign: bool,
        covered: bool,
        changed_before: bool,
        gets: usize,
    }
    impl Fake {
        fn new() -> Self {
            Self {
                speed: Speed::Normal,
                sets: Vec::new(),
                mutate: true,
                info: json!({"returnValue":true,"modelName":"OLED55G26LA","firmwareVersion":"23.25.55","sdkVersion":"9.2.2"}),
                desc: json!({"returnValue":true,"results":[{"category":"option","key":"pointerSpeed","dimension":[],"vtype":"ArrayExt","dbtype":"G","account":false,"volatile":false,"valueCheck":true,"ui":{"active":true,"visible":true},"values":{"arrayExt":Speed::ALL.map(|s|json!({"value":s.wire(),"active":true,"visible":true}))}}]}),
                foreign: false,
                covered: false,
                changed_before: false,
                gets: 0,
            }
        }
    }
    impl Platform for Fake {
        fn call(&mut self, op: Op) -> Result<Value, String> {
            Ok(match op {
                Op::SystemInfo => self.info.clone(),
                Op::Descriptor => self.desc.clone(),
                Op::Foreground => {
                    json!({"returnValue":true,"appId":if self.foreign{"com.example.other"}else{crate::identity::PROOF}})
                }
                Op::Windows => {
                    json!({"returnValue":true,"windows":[{"appId":crate::identity::PROOF,"windowType":"_WEBOS_WINDOW_TYPE_CARD"},{"appId":"com.example.other","windowType":if self.covered{"_WEBOS_WINDOW_TYPE_OVERLAY"}else{"_WEBOS_WINDOW_TYPE_CARD"}}]})
                }
                Op::Value => {
                    self.gets += 1;
                    if self.changed_before && self.gets == 2 {
                        self.speed = Speed::Fast;
                    }
                    json!({"returnValue":true,"category":"option","settings":{"pointerSpeed":self.speed.wire()}})
                }
                Op::Set(s) => {
                    self.sets.push(s);
                    if self.mutate {
                        self.speed = s;
                    }
                    json!({"returnValue":true})
                }
            })
        }
        fn wait(&mut self) {}
    }
    #[test]
    fn exact_enum_change_and_restoration_require_value_readback() {
        let mut p = Fake::new();
        let c = Change {
            expected: Speed::Normal,
            value: Speed::Slow,
        };
        let r = apply(&mut p, crate::identity::PROOF, c, true);
        assert!(r.attempted && r.admitted && r.verified);
        assert_eq!(p.sets, vec![Speed::Slow]);
        assert!(
            apply(
                &mut p,
                crate::identity::PROOF,
                Change {
                    expected: Speed::Slow,
                    value: Speed::Normal
                },
                true
            )
            .verified
        );
        assert_eq!(p.speed, Speed::Normal);
        let mut p = Fake::new();
        p.mutate = false;
        let r = apply(&mut p, crate::identity::PROOF, c, true);
        assert!(r.attempted && r.admitted && !r.verified);
    }
    #[test]
    fn stale_covered_foreign_or_unknown_metadata_cannot_set_anything() {
        for i in 0..8 {
            let mut p = Fake::new();
            match i {
                0 => p.foreign = true,
                1 => p.covered = true,
                2 => p.changed_before = true,
                3 => p.info["firmwareVersion"] = "unknown".into(),
                4 => p.desc["results"][0]["dimension"] = json!(["input"]),
                5 => p.desc["results"][0]["values"]["arrayExt"][0]["active"] = false.into(),
                6 => p.desc["results"][0]["account"] = true.into(),
                _ => p.speed = Speed::Fast,
            };
            let r = apply(
                &mut p,
                crate::identity::PROOF,
                Change {
                    expected: Speed::Normal,
                    value: Speed::Slow,
                },
                true,
            );
            assert!(!r.attempted && p.sets.is_empty());
        }
        let mut p = Fake::new();
        assert!(
            !apply(
                &mut p,
                crate::identity::PROOF,
                Change {
                    expected: Speed::Normal,
                    value: Speed::Slow
                },
                false
            )
            .attempted
        );
        assert!(
            !apply(
                &mut p,
                "com.example.other",
                Change {
                    expected: Speed::Normal,
                    value: Speed::Slow
                },
                true
            )
            .attempted
        );
    }
    #[test]
    fn idempotence_and_closed_wire_type_prevent_generic_setting_requests() {
        let mut p = Fake::new();
        assert!(
            apply(
                &mut p,
                crate::identity::PROOF,
                Change {
                    expected: Speed::Normal,
                    value: Speed::Normal
                },
                true
            )
            .verified
        );
        assert!(p.sets.is_empty());
        for v in [
            json!({"expected":"normal","value":"extreme"}),
            json!({"expected":"normal","value":"slow","settings":{"screenShift":"off"}}),
            json!({"expected":"normal","value":"slow","uri":"arbitrary"}),
        ] {
            assert!(serde_json::from_value::<Change>(v).is_err());
        }
    }
}
