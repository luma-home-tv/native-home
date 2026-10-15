//! Typed, bounded read-only updates. Raw service replies do not become fresh state merely
//! because a call was admitted. Independent domains can fail without inventing replacement values.
use crate::control_reads::{self, Section as ControlSection};
use crate::{
    model::{valid_app_id, Application, Context, Setting},
    shell::{Input, Snapshot},
};
use serde_json::Value;
use std::{
    collections::{BTreeMap, BTreeSet},
    sync::{mpsc, Arc, Mutex},
    thread,
    time::{Duration, Instant},
};
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Reading<T> {
    Value(T),
    Failed(String),
}
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct CatalogueReading {
    pub apps: Vec<Application>,
    pub launch_ids: BTreeSet<String>,
}
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct PictureReading {
    pub context: Context,
    pub settings: Vec<Setting>,
}
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Update {
    pub system_keyboard: bool,
    pub observed_ms: u32,
    pub simulated: bool,
    pub catalogue: Reading<CatalogueReading>,
    pub artwork: Reading<crate::app_artwork::SourceMap>,
    pub inputs: Reading<Vec<Input>>,
    pub picture: Reading<PictureReading>,
    pub sound: Reading<Vec<(String, String)>>,
    pub network: Reading<Vec<(String, String)>>,
    pub controls: BTreeMap<ControlSection, Reading<Vec<Setting>>>,
}
fn checked<'a>(replies: &'a Value, name: &str) -> Result<&'a Value, String> {
    let value = replies
        .get(name)
        .ok_or_else(|| format!("{name}: missing reply"))?;
    if value.get("returnValue").and_then(Value::as_bool) != Some(true) {
        return Err(format!("{name}: unavailable"));
    }
    Ok(value)
}
fn text(value: &Value, key: &str, limit: usize) -> Result<String, String> {
    let text = value
        .get(key)
        .and_then(Value::as_str)
        .ok_or_else(|| format!("missing {key}"))?;
    if text.is_empty() || text.len() > limit || text.chars().any(char::is_control) {
        return Err(format!("invalid {key}"));
    }
    Ok(text.to_owned())
}
fn array<'a>(value: &'a Value, key: &str, limit: usize) -> Result<&'a Vec<Value>, String> {
    let list = value
        .get(key)
        .and_then(Value::as_array)
        .ok_or_else(|| format!("missing {key}"))?;
    if list.len() > limit {
        return Err(format!("too many {key}"));
    }
    Ok(list)
}
fn scalar(value: &Value) -> Option<String> {
    match value {
        Value::String(v) if v.len() <= 128 && !v.chars().any(char::is_control) => Some(v.clone()),
        Value::Number(v) => Some(v.to_string()),
        Value::Bool(v) => Some(v.to_string()),
        _ => None,
    }
}
fn reading<T>(result: Result<T, String>) -> Reading<T> {
    match result {
        Ok(value) => Reading::Value(value),
        Err(error) => Reading::Failed(error),
    }
}
fn unavailable(specs: &[control_reads::Spec], context: &Context) -> Vec<Setting> {
    specs
        .iter()
        .map(|s| Setting {
            constraint: None,
            id: s.id.into(),
            label: s.label.into(),
            focus_key: s.key,
            value: "Unavailable".into(),
            context: context.clone(),
            protected: true,
            available: false,
        })
        .collect()
}
pub fn parse(raw: &Value, own_id: &str, observed_ms: u32, simulated: bool) -> Update {
    let replies = &raw["replies"];
    let catalogue = reading((|| {
        let apps = checked(replies, "apps")?;
        let points = checked(replies, "launch_points")?;
        let launch_ids = array(points, "launchPoints", 512)?
            .iter()
            .filter_map(|p| p.get("id").and_then(Value::as_str))
            .filter(|id| valid_app_id(id) && *id != own_id)
            .map(str::to_owned)
            .collect::<BTreeSet<_>>();
        let mut unique = BTreeMap::new();
        for app in array(apps, "apps", 512)? {
            let Some(id) = app.get("id").and_then(Value::as_str) else {
                continue;
            };
            let Some(kind) = app.get("type").and_then(Value::as_str) else {
                continue;
            };
            if !valid_app_id(id)
                || id == own_id
                || !launch_ids.contains(id)
                || app.get("visible").and_then(Value::as_bool) != Some(true)
                || !matches!(kind, "native" | "native_builtin" | "web")
            {
                continue;
            }
            let Ok(title) = text(app, "title", 512) else {
                continue;
            };
            unique.entry(id.to_owned()).or_insert(Application {
                id: id.to_owned(),
                title,
                kind: kind.to_owned(),
                visible: true,
                icon: None,
            });
        }
        Ok(CatalogueReading {
            apps: unique.into_values().collect(),
            launch_ids,
        })
    })());
    let inputs = reading((|| {
        let data = checked(replies, "inputs")?;
        let mut ports = BTreeMap::new();
        let devices = array(data, "devices", 16)?;
        let current_selection = checked(replies, "current_input")
            .and_then(|data| text(data, "mainInputSourceId", 64))
            .ok()
            .filter(|current| {
                devices
                    .iter()
                    .any(|device| device["id"].as_str() == Some(current.as_str()))
            });
        for input in devices {
            let id = text(input, "id", 16)?;
            if !matches!(id.as_str(), "HDMI_1" | "HDMI_2" | "HDMI_3" | "HDMI_4") {
                continue;
            }
            let flag = |key| {
                input
                    .get(key)
                    .and_then(Value::as_bool)
                    .ok_or_else(|| format!("missing {key}"))
            };
            let selected = current_selection.as_ref().map(|current| current == &id);
            let row = Input {
                id: id.clone(),
                label: text(input, "label", 128)?,
                plugged: flag("hdmiPlugIn")?,
                signal: flag("hdmiSignalExist")?,
                active: flag("activate")?,
                selected,
            };
            if ports.insert(id, row).is_some() {
                return Err("Duplicate HDMI identity".into());
            }
        }
        if ports.len() != 4 {
            return Err("Incomplete HDMI status".into());
        }
        Ok(ports.into_values().collect())
    })());
    let picture = reading((|| {
        let data = checked(replies, "picture")?;
        let dimension = &data["dimension"];
        let context = Context {
            input: text(dimension, "input", 64)?,
            dynamic_range: text(dimension, "dynamicRange", 64)?,
            picture_mode: text(dimension, "pictureMode", 64)?,
        };
        let values = data
            .get("settings")
            .and_then(Value::as_object)
            .ok_or("Missing picture values")?;
        if values.get("pictureMode").and_then(Value::as_str) != Some(context.picture_mode.as_str())
        {
            return Err("Picture mode/context mismatch".into());
        }
        let mut settings = Vec::new();
        for (i, (id, label)) in [
            ("pictureMode", "Picture mode"),
            ("backlight", "OLED pixel brightness"),
            ("contrast", "Contrast"),
            ("brightness", "Black level"),
        ]
        .into_iter()
        .enumerate()
        {
            let mut value = values
                .get(id)
                .and_then(scalar)
                .ok_or_else(|| format!("Missing {id}"))?;
            if id != "pictureMode" {
                let number = values[id]
                    .as_u64()
                    .or_else(|| {
                        values[id]
                            .as_str()
                            .filter(|v| {
                                !v.is_empty()
                                    && v.len() <= 3
                                    && v.bytes().all(|c| c.is_ascii_digit())
                            })
                            .and_then(|v| v.parse::<u64>().ok())
                    })
                    .filter(|v| *v <= 100)
                    .ok_or_else(|| format!("Invalid numeric {id}"))?;
                value = number.to_string();
            }
            settings.push(Setting {
                constraint: None,
                id: id.into(),
                focus_key: i as u32 + 1,
                label: label.into(),
                value,
                context: context.clone(),
                protected: false,
                available: true,
            });
        }
        if replies.get("picture_extra_values").is_some() {
            let after = checked(replies, "picture_after")?;
            if after["dimension"] != data["dimension"] || after["settings"] != data["settings"] {
                return Err("Picture changed during control reads".into());
            }
            let extra = control_reads::read(
                replies,
                "picture_extra",
                "picture",
                control_reads::PICTURE,
                Some(&context),
            )
            .unwrap_or_else(|_| unavailable(control_reads::PICTURE, &context));
            settings.extend(extra);
        }
        Ok(PictureReading { context, settings })
    })());
    let sound = reading((|| {
        let data = checked(replies, "audio")?;
        let before = text(checked(replies, "audio_output_before")?, "soundOutput", 128)?;
        let output = text(checked(replies, "audio_output")?, "soundOutput", 128)?;
        if before != output {
            return Err("Sound output changed during volume read".into());
        }
        let volume = data["volume"]
            .as_u64()
            .filter(|v| *v <= 100)
            .ok_or("Invalid TV volume")?;
        let muted = data["muteStatus"].as_bool().ok_or("Invalid mute status")?;
        text(data, "scenario", 128)?;
        let mut values = vec![
            ("soundOutput".into(), output.clone()),
            ("volume".into(), volume.to_string()),
            ("muteStatus".into(), muted.to_string()),
        ];
        if let Ok(settings) = checked(replies, "sound_settings") {
            if settings["category"].as_str() == Some("sound")
                && settings["settings"]["soundOutput"].as_str() == Some(output.as_str())
            {
                if let (Ok(mode), Ok(input)) = (
                    text(&settings["settings"], "soundMode", 128),
                    text(&settings["dimension"], "audioInput", 64),
                ) {
                    values.push(("soundMode".into(), mode));
                    values.push(("audioInput".into(), input));
                }
            }
        }
        Ok(values)
    })());
    let network = reading((|| {
        let data = checked(replies, "network")?;
        let internet = data["isInternetConnectionAvailable"]
            .as_bool()
            .ok_or("Invalid internet availability")?;
        let mut values = vec![(
            "internet".into(),
            if internet { "Available" } else { "Unavailable" }.into(),
        )];
        for (kind, label) in [("wired", "Wired"), ("wifi", "Wi-Fi")] {
            let state = text(&data[kind], "state", 32)?;
            if !matches!(state.as_str(), "connected" | "disconnected") {
                return Err(format!("Invalid {label} state"));
            }
            values.push((
                format!("{kind}State"),
                if state == "connected" {
                    "Connected"
                } else {
                    "Disconnected"
                }
                .into(),
            ));
            if state == "connected" {
                if let Ok(ip) = text(&data[kind], "ipAddress", 64) {
                    if ip.parse::<std::net::IpAddr>().is_ok() {
                        values.push((format!("{kind}Address"), ip));
                    }
                }
                if kind == "wifi" {
                    if let Ok(ssid) = text(&data[kind], "ssid", 128) {
                        values.push(("wifiName".into(), ssid));
                    }
                }
            }
        }
        Ok(values)
    })());
    let global = Context {
        input: "global".into(),
        dynamic_range: "global".into(),
        picture_mode: "global".into(),
    };
    let option = control_reads::read(
        replies,
        "general_option",
        "option",
        control_reads::GENERAL_OPTION,
        None,
    );
    let home = control_reads::read(
        replies,
        "general_home",
        "general",
        control_reads::GENERAL_HOME,
        None,
    );
    let locale = crate::locale_reads::parse(replies);
    let general = match (option, home) {
        (Err(a), Err(_)) if !locale.iter().any(|row| row.available) => Reading::Failed(a),
        (a, b) => {
            let mut rows =
                a.unwrap_or_else(|_| unavailable(control_reads::GENERAL_OPTION, &global));
            rows.extend(b.unwrap_or_else(|_| unavailable(control_reads::GENERAL_HOME, &global)));
            // Each cycle supplies all five rows, including per-group failures.
            // Old locale values cannot inherit another General group's freshness.
            rows.extend(locale);
            Reading::Value(rows)
        }
    };
    let controls = BTreeMap::from([
        (
            ControlSection::Sound,
            reading(
                crate::sound_reads::parse(replies)
                    .map_err(|error| format!("Optional Sound read unavailable: {error:?}")),
            ),
        ),
        (
            ControlSection::Accessibility,
            reading(control_reads::read(
                replies,
                "accessibility",
                "option",
                control_reads::ACCESSIBILITY,
                None,
            )),
        ),
        (ControlSection::General, general),
        (
            ControlSection::PanelCare,
            reading(control_reads::read(
                replies,
                "panel_care",
                "picture",
                control_reads::PANEL_CARE,
                None,
            )),
        ),
    ]);
    Update {
        system_keyboard: !simulated && crate::keyboard::qualified(&replies["system_info"]),
        controls,
        observed_ms,
        simulated,
        catalogue,
        artwork: reading(crate::app_artwork::sources_from_raw(raw, own_id)
            .map_err(|error| format!("Artwork metadata unavailable: {error:?}"))),
        inputs,
        picture,
        sound,
        network,
    }
}
#[derive(Clone, Debug, Default)]
pub struct Freshness {
    pub catalogue: Option<u32>,
    pub inputs: Option<u32>,
    pub picture: Option<u32>,
    pub sound: Option<u32>,
    pub network: Option<u32>,
    pub simulated: bool,
    pub controls: BTreeMap<ControlSection, u32>,
}
impl Freshness {
    pub fn current(stamp: Option<u32>, now: u32) -> bool {
        stamp.is_some_and(|ms| now.wrapping_sub(ms) <= 15_000)
    }
}
pub struct Worker {
    latest: Arc<Mutex<Option<Update>>>,
    stop: mpsc::Sender<()>,
    thread: thread::JoinHandle<()>,
}
impl Worker {
    pub fn fixture(
        values: Vec<Value>,
        own_id: String,
        origin: Instant,
        period: Duration,
    ) -> std::io::Result<Self> {
        if values.is_empty() {
            return Err(std::io::Error::new(
                std::io::ErrorKind::InvalidInput,
                "Empty live fixture",
            ));
        }
        let mut index = 0;
        Self::start(
            move || {
                move || {
                    let data = values[index.min(values.len() - 1)].clone();
                    index = index.saturating_add(1);
                    parse(&data, &own_id, origin.elapsed().as_millis() as u32, true)
                }
            },
            period,
        )
    }
    fn start<Init, F>(make_reader: Init, period: Duration) -> std::io::Result<Self>
    where
        Init: FnOnce() -> F + Send + 'static,
        F: FnMut() -> Update + 'static,
    {
        let latest = Arc::new(Mutex::new(None));
        let output = latest.clone();
        let (stop, receive) = mpsc::channel();
        let thread = thread::Builder::new()
            .name("home-live-read".into())
            .spawn(move || {
                let mut read = make_reader();
                loop {
                    *output.lock().unwrap_or_else(|e| e.into_inner()) = Some(read());
                    match receive.recv_timeout(period) {
                        Err(mpsc::RecvTimeoutError::Timeout) => {}
                        _ => break,
                    }
                }
            })?;
        Ok(Self {
            latest,
            stop,
            thread,
        })
    }
    #[cfg(target_arch = "arm")]
    pub fn native(own_id: String, origin: Instant) -> std::io::Result<Self> {
        // Registration lives on this worker. Its private GLib context never crosses threads.
        Self::start(
            move || {
                let mut reader = None;
                move || {
                    use plx_platform::webos::ls2;
                    use serde_json::json;
                    if reader.is_none() {
                        reader = ls2::register().ok();
                    }
                    let observed_ms = origin.elapsed().as_millis() as u32;
                    let raw = reader
                        .as_ref()
                        .map_or_else(|| json!({}), crate::native_probe::read);
                    parse(&raw, &own_id, observed_ms, false)
                }
            },
            Duration::from_secs(2),
        )
    }
    pub fn poll(&self) -> Option<Update> {
        self.latest.lock().unwrap_or_else(|e| e.into_inner()).take()
    }
    pub fn finish(self) {
        let Self {
            latest: _,
            stop,
            thread,
        } = self;
        let _ = stop.send(());
        let _ = thread.join();
    }
}
/// Keep the last image/value for inspection when unavailable, but withdraw its freshness.
pub fn merge(snapshot: &mut Snapshot, update: &Update, fresh: &mut Freshness) {
    fresh.simulated = update.simulated;
    match &update.catalogue {
        Reading::Value(value) => {
            let icons = snapshot
                .applications
                .iter()
                .filter_map(|a| a.icon.as_ref().map(|icon| (a.id.clone(), icon.clone())))
                .collect::<BTreeMap<_, _>>();
            snapshot.applications = value.apps.clone();
            for app in &mut snapshot.applications {
                app.icon = icons.get(&app.id).cloned();
            }
            snapshot.launch_ids = value.launch_ids.clone();
            fresh.catalogue = Some(update.observed_ms);
        }
        Reading::Failed(_) => fresh.catalogue = None,
    }
    match &update.inputs {
        Reading::Value(value) => {
            snapshot.inputs = value.clone();
            fresh.inputs = Some(update.observed_ms);
        }
        Reading::Failed(_) => {
            fresh.inputs = None;
            for input in &mut snapshot.inputs {
                input.selected = None;
            }
        }
    }
    match &update.picture {
        Reading::Value(value) => {
            snapshot.context = value.context.clone();
            snapshot
                .settings
                .insert("picture".into(), value.settings.clone());
            fresh.picture = Some(update.observed_ms);
        }
        Reading::Failed(_) => {
            fresh.picture = None;
        }
    }
    match &update.sound {
        Reading::Value(value) => {
            let rows = [
                ("soundOutput", "Sound output"),
                ("volume", "TV volume"),
                ("muteStatus", "Muted"),
                ("soundMode", "Sound mode"),
                ("audioInput", "Sound profile input"),
            ]
            .into_iter()
            .enumerate()
            .map(|(i, (id, label))| {
                let field = value
                    .iter()
                    .find(|(key, _)| key == id)
                    .map(|(_, v)| v.clone());
                Setting {
                    constraint: None,
                    id: id.into(),
                    focus_key: 20 + i as u32,
                    label: label.into(),
                    value: field.clone().unwrap_or_else(|| "Unavailable".into()),
                    context: snapshot.context.clone(),
                    protected: false,
                    available: field.is_some(),
                }
            })
            .collect::<Vec<_>>();
            let sound = snapshot.settings.entry("sound".into()).or_default();
            sound.retain(optional_sound_row);
            sound.splice(0..0, rows);
            fresh.sound = Some(update.observed_ms);
        }
        Reading::Failed(_) => fresh.sound = None,
    }
    match &update.network {
        Reading::Value(value) => {
            let rows = [
                ("internet", "Internet"),
                ("wiredState", "Wired connection"),
                ("wifiState", "Wi-Fi connection"),
                ("wiredAddress", "Wired address"),
                ("wifiAddress", "Wi-Fi address"),
                ("wifiName", "Wi-Fi network"),
            ]
            .into_iter()
            .enumerate()
            .map(|(i, (id, label))| {
                let field = value
                    .iter()
                    .find(|(key, _)| key == id)
                    .map(|(_, v)| v.clone());
                Setting {
                    constraint: None,
                    id: id.into(),
                    focus_key: 60 + i as u32,
                    label: label.into(),
                    value: field.clone().unwrap_or_else(|| "Unavailable".into()),
                    context: snapshot.context.clone(),
                    protected: false,
                    available: field.is_some(),
                }
            })
            .collect();
            snapshot.settings.insert("network".into(), rows);
            fresh.network = Some(update.observed_ms);
        }
        Reading::Failed(_) => fresh.network = None,
    }
    let mut optional_sound = match update.controls.get(&ControlSection::Sound) {
        Some(Reading::Value(rows)) => {
            fresh
                .controls
                .insert(ControlSection::Sound, update.observed_ms);
            rows.clone()
        }
        _ => {
            fresh.controls.remove(&ControlSection::Sound);
            crate::sound_reads::unavailable_rows()
        }
    };
    // Global validation stays in the parser; bind read-only rows to this presentation
    // snapshot, matching the existing global groups and exact-context form model.
    for row in &mut optional_sound {
        row.context = snapshot.context.clone();
        row.protected = true;
    }
    let sound = snapshot.settings.entry("sound".into()).or_default();
    sound.retain(|row| !optional_sound_row(row));
    sound.extend(optional_sound);
    for section in [
        ControlSection::Accessibility,
        ControlSection::General,
        ControlSection::PanelCare,
    ] {
        match update.controls.get(&section) {
            Some(Reading::Value(rows)) => {
                let mut rows = rows.clone();
                for row in &mut rows {
                    row.context = snapshot.context.clone();
                    row.protected = true;
                }
                snapshot.settings.insert(section.name().into(), rows);
                fresh.controls.insert(section, update.observed_ms);
            }
            _ => {
                fresh.controls.remove(&section);
            }
        }
    }
    expire(snapshot, fresh, update.observed_ms);
}
fn optional_sound_row(row: &Setting) -> bool {
    crate::sound_reads::Key::ALL
        .iter()
        .any(|key| row.id == key.id() && row.focus_key == key.focus_key())
}
/// The same per-row predicate drives withdrawal and the shell's redraw decision.
/// Optional global Sound and core routed audio have independent freshness clocks.
fn row_expired(section: &str, row: &Setting, fresh: &Freshness, now: u32) -> bool {
    let stamp = match section {
        "picture" => fresh.picture,
        "sound" if optional_sound_row(row) => fresh.controls.get(&ControlSection::Sound).copied(),
        "sound" => fresh.sound,
        "network" => fresh.network,
        "accessibility" => fresh.controls.get(&ControlSection::Accessibility).copied(),
        "general" => fresh.controls.get(&ControlSection::General).copied(),
        "panel_care" => fresh.controls.get(&ControlSection::PanelCare).copied(),
        _ => return false,
    };
    row.available && !Freshness::current(stamp, now)
}
pub fn needs_expiry(snapshot: &Snapshot, fresh: &Freshness, now: u32) -> bool {
    (!Freshness::current(fresh.inputs, now)
        && snapshot.inputs.iter().any(|input| input.selected.is_some()))
        || snapshot
            .settings
            .iter()
            .any(|(section, rows)| rows.iter().any(|row| row_expired(section, row, fresh, now)))
}
pub fn expire(snapshot: &mut Snapshot, fresh: &Freshness, now: u32) {
    if !Freshness::current(fresh.inputs, now) {
        for input in &mut snapshot.inputs {
            input.selected = None;
        }
    }
    for (section, rows) in &mut snapshot.settings {
        for row in rows {
            if row_expired(section, row, fresh, now) {
                row.available = false;
            }
        }
    }
}

#[cfg(test)]
pub(crate) mod tests {
    use super::*;
    use serde_json::json;
    pub(crate) fn raw() -> Value {
        let devices=(1..=4).map(|port|json!({"id":format!("HDMI_{port}"),"label":format!("HDMI {port}"),"hdmiPlugIn":false,"hdmiSignalExist":false,"activate":false})).collect::<Vec<_>>();
        json!({"replies":{
            "apps":{"returnValue":true,"apps":[{"id":"com.example.a","title":"Alpha","type":"native","visible":true},{"id":"com.example.b","title":"Bravo","type":"web","visible":true},{"id":"com.example.self","title":"Dashboard","type":"native","visible":true},{"id":"com.example.stub","title":"Store stub","type":"stub","visible":true}]},
            "launch_points":{"returnValue":true,"launchPoints":[{"id":"com.example.a"},{"id":"com.example.b"},{"id":"com.example.self"},{"id":"com.example.stub"}]},
            "inputs":{"returnValue":true,"devices":devices},
            "picture":{"returnValue":true,"dimension":{"input":"default","dynamicRange":"sdr","pictureMode":"game"},"settings":{"pictureMode":"game","backlight":90,"contrast":85,"brightness":50}},
            "audio":{"returnValue":false,"errorText":"Unavailable"}}})
    }
    pub(crate) fn global_raw() -> Value {
        let mut data = raw();
        for (name, category, specs) in [
            ("accessibility", "option", control_reads::ACCESSIBILITY),
            ("general_option", "option", control_reads::GENERAL_OPTION),
            ("general_home", "general", control_reads::GENERAL_HOME),
            ("panel_care", "picture", control_reads::PANEL_CARE),
        ] {
            let mut values = serde_json::Map::new();
            let mut results = Vec::new();
            for spec in specs {
                let choices: &[&str] = match spec.id {
                    "audioGuidanceVolume" | "audioGuidancePitch" => {
                        &["veryLow", "low", "medium", "high", "veryHigh"]
                    }
                    "audioGuidanceSpeed" => &["veryFast", "fast", "medium", "slow", "verySlow"],
                    _ => &["on", "off"],
                };
                values.insert(
                    spec.id.into(),
                    json!(if choices.len() == 5 { "medium" } else { "on" }),
                );
                let array = choices
                    .iter()
                    .map(|value| json!({"value":value,"visible":true,"active":true}))
                    .collect::<Vec<_>>();
                results.push(json!({"key":spec.id,"category":category,"dimension":[],"vtype":"ArrayExt","ui":{"visible":true,"active":true},"values":{"arrayExt":array}}));
            }
            data["replies"][format!("{name}_values")] =
                json!({"returnValue":true,"category":category,"settings":values});
            data["replies"][format!("{name}_desc")] = json!({"returnValue":true,"results":results});
        }
        data
    }
    // Synthetic locale fixtures intentionally exercise the real parse/merge path.
    fn locale_data() -> Value {
        let mut data = global_raw();
        data["replies"][crate::locale_reads::LOCALE_VALUES_REPLY] = json!({
            "returnValue":true,"settings":{"localeInfo":{"locales":{
                "UI":"fr-CA","AUD":"en-CA","AUD2":"de_DE"
            }}}
        });
        data["replies"][crate::locale_reads::REGION_VALUES_REPLY] = json!({
            "returnValue":true,"category":"option",
            "settings":{"country":"CAN","smartServiceCountryCode2":"CA"}
        });
        data
    }
    fn locale_snapshot() -> Snapshot {
        Snapshot {
            captured_at: "synthetic locale transitions".into(),
            applications: Vec::new(),
            launch_ids: BTreeSet::new(),
            inputs: Vec::new(),
            context: Context {
                input: "old-input".into(),
                dynamic_range: "old-range".into(),
                picture_mode: "old-mode".into(),
            },
            settings: BTreeMap::new(),
        }
    }
    fn assert_general_locale_identity(snapshot: &Snapshot) {
        let rows = &snapshot.settings["general"];
        assert_eq!(rows.len(), 10);
        let legacy = control_reads::GENERAL_OPTION.iter()
            .chain(control_reads::GENERAL_HOME.iter());
        for (row, spec) in rows[..5].iter().zip(legacy) {
            assert_eq!((&row.id, row.focus_key), (&spec.id.to_string(), spec.key));
        }
        for (row, key) in rows[5..].iter().zip(crate::locale_reads::Key::ALL) {
            assert_eq!(row.id, key.id());
            assert_eq!(row.focus_key, key.focus_key());
            assert!(row.constraint.is_none());
        }
        assert!(rows.iter().all(|row| row.protected && row.context == snapshot.context));
    }
    #[test]
    fn locale_failure_with_current_legacy_general_withdraws_all_five_locale_values() {
        let mut snapshot = locale_snapshot();
        let mut fresh = Freshness::default();
        merge(&mut snapshot, &parse(&locale_data(), "self", 100, false), &mut fresh);
        assert_general_locale_identity(&snapshot);
        assert!(snapshot.settings["general"].iter().all(|row| row.available));
        let mut failed = locale_data();
        for name in [crate::locale_reads::LOCALE_VALUES_REPLY, crate::locale_reads::REGION_VALUES_REPLY] {
            failed["replies"][name] = json!({"returnValue":false});
        }
        merge(&mut snapshot, &parse(&failed, "self", 200, false), &mut fresh);
        assert_general_locale_identity(&snapshot);
        assert!(snapshot.settings["general"][..5].iter().all(|row| row.available));
        assert!(snapshot.settings["general"][5..].iter().all(|row| !row.available && row.value == "Unavailable"));
        assert_eq!(fresh.controls.get(&ControlSection::General), Some(&200));
    }
    #[test]
    fn locale_values_remain_current_when_both_legacy_general_groups_fail() {
        let mut data = locale_data();
        for name in ["general_option_values", "general_home_values"] {
            data["replies"][name] = json!({"returnValue":false});
        }
        let mut snapshot = locale_snapshot();
        let mut fresh = Freshness::default();
        merge(&mut snapshot, &parse(&data, "self", 300, false), &mut fresh);
        assert_general_locale_identity(&snapshot);
        assert!(snapshot.settings["general"][..5].iter().all(|row| !row.available));
        assert!(snapshot.settings["general"][5..].iter().all(|row| row.available));
        assert_eq!(snapshot.settings["general"][5..].iter().map(|row| row.value.as_str()).collect::<Vec<_>>(),
            ["fr-CA", "en-CA", "de_DE", "CAN", "CA"]);
        assert_eq!(fresh.controls.get(&ControlSection::General), Some(&300));
    }
    #[test]
    fn locale_and_region_groups_fail_independently_in_the_general_merge() {
        for locale_failed in [true, false] {
            let mut snapshot = locale_snapshot();
            let mut fresh = Freshness::default();
            merge(&mut snapshot, &parse(&locale_data(), "self", 100, false), &mut fresh);
            let mut data = locale_data();
            let denied = if locale_failed { crate::locale_reads::LOCALE_VALUES_REPLY }
                else { crate::locale_reads::REGION_VALUES_REPLY };
            data["replies"][denied] = json!({"returnValue":false});
            merge(&mut snapshot, &parse(&data, "self", 400, false), &mut fresh);
            assert_general_locale_identity(&snapshot);
            let rows = &snapshot.settings["general"];
            assert!(rows[..5].iter().all(|row| row.available));
            assert!(rows[5..8].iter().all(|row| row.available != locale_failed));
            assert!(rows[8..].iter().all(|row| row.available == locale_failed));
        }
    }
    #[test]
    fn expired_locale_values_do_not_resurrect_on_unrelated_or_legacy_general_refresh() {
        let mut snapshot = locale_snapshot();
        let mut fresh = Freshness::default();
        merge(&mut snapshot, &parse(&locale_data(), "self", 100, false), &mut fresh);
        assert!(!needs_expiry(&snapshot, &fresh, 15_100));
        assert!(needs_expiry(&snapshot, &fresh, 15_101));
        expire(&mut snapshot, &fresh, 15_101);
        assert!(!needs_expiry(&snapshot, &fresh, 15_101));
        assert_general_locale_identity(&snapshot);
        assert!(snapshot.settings["general"].iter().all(|row| !row.available));
        let mut unrelated = parse(&global_raw(), "self", 15_200, false);
        unrelated.controls.remove(&ControlSection::General);
        merge(&mut snapshot, &unrelated, &mut fresh);
        assert_general_locale_identity(&snapshot);
        assert!(snapshot.settings["general"].iter().all(|row| !row.available));
        assert!(!fresh.controls.contains_key(&ControlSection::General));
        assert!(!needs_expiry(&snapshot, &fresh, 15_200));
        // A fresh legacy General clock must not restore the old locale strings.
        merge(&mut snapshot, &parse(&global_raw(), "self", 15_300, false), &mut fresh);
        assert_general_locale_identity(&snapshot);
        assert!(snapshot.settings["general"][..5].iter().all(|row| row.available));
        assert!(snapshot.settings["general"][5..].iter().all(|row| !row.available && row.value == "Unavailable"));
        assert_eq!(fresh.controls.get(&ControlSection::General), Some(&15_300));
        // A wholly refused General update withdraws its shared clock immediately.
        let mut refused = global_raw();
        for name in ["general_option_values", "general_home_values"] {
            refused["replies"][name] = json!({"returnValue":false});
        }
        merge(&mut snapshot, &parse(&refused, "self", 15_400, false), &mut fresh);
        assert_general_locale_identity(&snapshot);
        assert!(snapshot.settings["general"].iter().all(|row| !row.available));
        assert!(!fresh.controls.contains_key(&ControlSection::General));
        assert!(!needs_expiry(&snapshot, &fresh, 15_400));
    }
    #[test]
    fn keyboard_capability_requires_unsimulated_known_platform_info() {
        let mut raw = serde_json::json!({"replies":{"system_info":{"returnValue":true,"modelName":"OLED55G26LA","firmwareVersion":"23.25.55","sdkVersion":"9.2.2"}}});
        assert!(parse(&raw, "own", 0, false).system_keyboard);
        assert!(!parse(&raw, "own", 0, true).system_keyboard);
        raw["replies"]["system_info"]["returnValue"] = false.into();
        assert!(!parse(&raw, "own", 0, false).system_keyboard);
        assert!(!parse(&serde_json::json!({}), "own", 0, false).system_keyboard);
    }
    #[test]
    fn global_controls_fail_independently_expire_and_keep_protected_focus_identity() {
        let mut data = global_raw();
        let first = parse(&data, "com.example.self", 100, false);
        assert!(matches!(
            first.controls[&ControlSection::Accessibility],
            Reading::Value(_)
        ));
        let mut snapshot = Snapshot {
            captured_at: "fixture".into(),
            applications: Vec::new(),
            launch_ids: BTreeSet::new(),
            inputs: Vec::new(),
            context: Context {
                input: "previous".into(),
                dynamic_range: "sdr".into(),
                picture_mode: "cinema".into(),
            },
            settings: BTreeMap::new(),
        };
        let mut fresh = Freshness::default();
        merge(&mut snapshot, &first, &mut fresh);
        assert_eq!(snapshot.settings["general"].len(), 10);
        assert!(snapshot.settings["panel_care"]
            .iter()
            .all(|r| r.protected && r.available));
        data["replies"]["picture"]["returnValue"] = false.into();
        data["replies"]["accessibility_desc"]["returnValue"] = false.into();
        data["replies"]["general_option_values"]["returnValue"] = false.into();
        merge(
            &mut snapshot,
            &parse(&data, "com.example.self", 200, false),
            &mut fresh,
        );
        assert!(snapshot.settings["accessibility"]
            .iter()
            .all(|r| !r.available));
        assert!(snapshot.settings["general"][..3]
            .iter()
            .all(|r| !r.available));
        assert!(snapshot.settings["general"][3..5]
            .iter()
            .all(|r| r.available && r.still_current(&snapshot.context)));
        assert!(snapshot.settings["panel_care"].iter().all(|r| r.available));
        expire(&mut snapshot, &fresh, 15_201);
        assert!(snapshot.settings["panel_care"].iter().all(|r| !r.available));
        assert_eq!(snapshot.settings["panel_care"][0].focus_key, 180);
    }
    #[test]
    fn picture_profile_change_during_expanded_read_withdraws_the_whole_context() {
        let mut data = raw();
        data["replies"]["picture_extra_values"] = json!({"returnValue":true});
        data["replies"]["picture_after"] = data["replies"]["picture"].clone();
        data["replies"]["picture_after"]["dimension"]["pictureMode"] = "cinema".into();
        assert!(matches!(
            parse(&data, "com.example.self", 10, false).picture,
            Reading::Failed(_)
        ));
    }
    #[test]
    fn provider_requires_success_and_shape_and_never_promotes_own_app_or_stubs() {
        let mut data = raw();
        let update = parse(&data, "com.example.self", 10, false);
        let Reading::Value(catalogue) = update.catalogue else {
            panic!("Catalogue should be valid")
        };
        assert_eq!(catalogue.apps.len(), 2);
        assert!(matches!(update.sound, Reading::Failed(_)));
        data["replies"]["apps"]["returnValue"] = json!(false);
        assert!(matches!(
            parse(&data, "com.example.self", 11, false).catalogue,
            Reading::Failed(_)
        ));
        data["replies"]["apps"]["returnValue"] = json!(true);
        data["replies"]["apps"]["apps"] = json!({});
        assert!(matches!(
            parse(&data, "com.example.self", 12, false).catalogue,
            Reading::Failed(_)
        ));
    }
    #[test]
    fn picture_context_mismatch_and_missing_input_flags_are_unavailable() {
        let mut data = raw();
        data["replies"]["picture"]["dimension"]["pictureMode"] = json!("cinema");
        assert!(matches!(
            parse(&data, "com.example.self", 10, false).picture,
            Reading::Failed(_)
        ));
        data["replies"]["inputs"]["devices"][0]
            .as_object_mut()
            .unwrap()
            .remove("hdmiPlugIn");
        assert!(matches!(
            parse(&data, "com.example.self", 10, false).inputs,
            Reading::Failed(_)
        ));
    }
    #[test]
    fn port_identity_is_normalized_even_when_service_order_changes() {
        let mut data = raw();
        data["replies"]["inputs"]["devices"]
            .as_array_mut()
            .unwrap()
            .reverse();
        let Reading::Value(ports) = parse(&data, "com.example.self", 10, false).inputs else {
            panic!("Inputs should be valid")
        };
        assert_eq!(
            ports.iter().map(|p| p.id.as_str()).collect::<Vec<_>>(),
            ["HDMI_1", "HDMI_2", "HDMI_3", "HDMI_4"]
        );
    }
    #[test]
    fn freshness_expires_and_does_not_accept_future_or_unrelated_clock_stamps() {
        assert!(Freshness::current(Some(100), 15000));
        assert!(!Freshness::current(Some(100), 15101));
        assert!(!Freshness::current(Some(101), 100));
        assert!(!Freshness::current(None, 100));
    }
    fn full_readings() -> Value {
        let mut data = raw();
        data["replies"]["audio"] =
            json!({"returnValue":true,"volume":10,"muteStatus":false,"scenario":"speaker_optical"});
        data["replies"]["audio_output_before"] =
            json!({"returnValue":true,"soundOutput":"external_optical"});
        data["replies"]["audio_output"] = data["replies"]["audio_output_before"].clone();
        data["replies"]["sound_settings"] = json!({"returnValue":true,"category":"sound","dimension":{"audioInput":"hdmi4"},"settings":{"soundMode":"standard","soundOutput":"external_optical"}});
        data["replies"]["network"] = json!({"returnValue":true,"isInternetConnectionAvailable":true,"wired":{"state":"connected","ipAddress":"203.0.113.10"},"wifi":{"state":"connected","ssid":"Example Wi-Fi","ipAddress":"2001:db8::10"}});
        data
    }
    fn sound_data() -> Value {
        let mut data = full_readings();
        data["replies"].as_object_mut().unwrap().extend(
            crate::sound_reads::tests::fixture()
                .as_object()
                .unwrap()
                .clone(),
        );
        data
    }
    #[test]
    fn optional_sound_failures_and_recovery_do_not_withdraw_or_replace_core_profiles() {
        let mut data = sound_data();
        let mut snapshot = Snapshot {
            captured_at: "fixture".into(),
            applications: Vec::new(),
            launch_ids: BTreeSet::new(),
            inputs: Vec::new(),
            context: Context {
                input: "previous".into(),
                dynamic_range: "sdr".into(),
                picture_mode: "cinema".into(),
            },
            settings: BTreeMap::new(),
        };
        let mut fresh = Freshness::default();
        let apply = |data: &Value, ms, snapshot: &mut Snapshot, fresh: &mut Freshness| {
            merge(snapshot, &parse(data, "com.example.self", ms, false), fresh);
        };
        apply(&data, 100, &mut snapshot, &mut fresh);
        let core = snapshot.settings["sound"][..5].to_vec();
        assert_eq!(
            snapshot.settings["sound"]
                .iter()
                .map(|r| r.focus_key)
                .collect::<Vec<_>>(),
            [20, 21, 22, 23, 24, 30, 31, 32]
        );
        assert_eq!(core[3].value, "standard");
        assert_eq!(core[4].value, "hdmi4");
        assert!(snapshot.settings["sound"][5..]
            .iter()
            .all(|r| r.protected && r.available && r.still_current(&snapshot.context)));
        data["replies"][crate::sound_reads::DESCRIPTORS_REPLY]["returnValue"] = false.into();
        apply(&data, 200, &mut snapshot, &mut fresh);
        assert_eq!(snapshot.settings["sound"][..5], core);
        assert!(snapshot.settings["sound"][5..]
            .iter()
            .all(|r| !r.available && r.constraint.is_none()));
        assert!(Freshness::current(fresh.sound, 200));
        assert!(!fresh.controls.contains_key(&ControlSection::Sound));
        data = sound_data();
        data["replies"]["audio"]["returnValue"] = false.into();
        apply(&data, 300, &mut snapshot, &mut fresh);
        assert!(snapshot.settings["sound"][..5].iter().all(|r| !r.available));
        assert!(snapshot.settings["sound"][5..].iter().all(|r| r.available));
        assert_eq!(fresh.sound, None);
        assert_eq!(fresh.controls.get(&ControlSection::Sound), Some(&300));
        data = sound_data();
        data["replies"][crate::sound_reads::VALUES_REPLY]["settings"]["audioBalance"] = "51".into();
        apply(&data, 400, &mut snapshot, &mut fresh);
        assert!(!snapshot.settings["sound"][5].available);
        assert!(snapshot.settings["sound"][6..].iter().all(|r| r.available));
        assert_eq!(snapshot.settings["sound"][..5], core);
        apply(&sound_data(), 500, &mut snapshot, &mut fresh);
        assert!(snapshot.settings["sound"].iter().all(|r| r.available));
        assert_eq!(snapshot.settings["sound"].len(), 8);
    }
    #[test]
    fn sound_groups_expire_independently_without_perpetual_redraw_or_value_resurrection() {
        for optional_old in [true, false] {
            let mut snapshot = Snapshot {
                captured_at: "fixture".into(),
                applications: Vec::new(),
                launch_ids: BTreeSet::new(),
                inputs: Vec::new(),
                context: Context {
                    input: "previous".into(),
                    dynamic_range: "sdr".into(),
                    picture_mode: "cinema".into(),
                },
                settings: BTreeMap::new(),
            };
            let mut fresh = Freshness::default();
            merge(
                &mut snapshot,
                &parse(&sound_data(), "self", 10_000, false),
                &mut fresh,
            );
            if optional_old {
                fresh.controls.insert(ControlSection::Sound, 100);
            } else {
                fresh.sound = Some(100);
            }
            assert!(!needs_expiry(&snapshot, &fresh, 15_100));
            assert!(needs_expiry(&snapshot, &fresh, 15_101));
            expire(&mut snapshot, &fresh, 15_101);
            assert!(!needs_expiry(&snapshot, &fresh, 15_101));
            assert!(snapshot.settings["sound"][..5]
                .iter()
                .all(|r| r.available == optional_old));
            assert!(snapshot.settings["sound"][5..]
                .iter()
                .all(|r| r.available != optional_old));
            expire(&mut snapshot, &fresh, 15_102);
            assert!(!needs_expiry(&snapshot, &fresh, 15_102));
            merge(
                &mut snapshot,
                &parse(&sound_data(), "self", 15_200, false),
                &mut fresh,
            );
            assert!(snapshot.settings["sound"].iter().all(|r| r.available));
            assert!(!needs_expiry(&snapshot, &fresh, 15_200));
        }
    }
    #[test]
    fn sound_output_transition_invalid_numbers_and_denied_replies_are_not_current() {
        let mut data = full_readings();
        assert!(matches!(
            parse(&data, "self", 100, false).sound,
            Reading::Value(_)
        ));
        data["replies"]["audio_output"]["soundOutput"] = "tv_speaker".into();
        assert!(matches!(
            parse(&data, "self", 100, false).sound,
            Reading::Failed(_)
        ));
        data = full_readings();
        data["replies"]["audio"]["volume"] = 101.into();
        assert!(matches!(
            parse(&data, "self", 100, false).sound,
            Reading::Failed(_)
        ));
        data = full_readings();
        data["replies"]["audio"]["muteStatus"] = "false".into();
        assert!(matches!(
            parse(&data, "self", 100, false).sound,
            Reading::Failed(_)
        ));
        data = full_readings();
        data["replies"]["audio"]["returnValue"] = false.into();
        assert!(matches!(
            parse(&data, "self", 100, false).sound,
            Reading::Failed(_)
        ));
    }
    #[test]
    fn mismatched_sound_profile_is_withheld_without_hiding_valid_global_audio() {
        let mut data = full_readings();
        data["replies"]["sound_settings"]["settings"]["soundOutput"] = "tv_speaker".into();
        let Reading::Value(sound) = parse(&data, "self", 100, false).sound else {
            panic!()
        };
        assert_eq!(sound.len(), 3);
        assert!(!sound.iter().any(|(key, _)| key == "soundMode"));
    }
    #[test]
    fn both_network_links_are_kept_distinct_and_disconnected_addresses_are_not_promoted() {
        let mut data = full_readings();
        let Reading::Value(network) = parse(&data, "self", 100, false).network else {
            panic!()
        };
        assert!(network.iter().any(|(key, _)| key == "wiredAddress"));
        assert!(network.iter().any(|(key, _)| key == "wifiAddress"));
        data["replies"]["network"]["wifi"]["state"] = "disconnected".into();
        let Reading::Value(network) = parse(&data, "self", 100, false).network else {
            panic!()
        };
        assert!(!network
            .iter()
            .any(|(key, _)| key == "wifiAddress" || key == "wifiName"));
        data["replies"]["network"]["wired"]["state"] = "unknown".into();
        assert!(matches!(
            parse(&data, "self", 100, false).network,
            Reading::Failed(_)
        ));
        data = full_readings();
        data["replies"]["network"]["isInternetConnectionAvailable"] = "true".into();
        assert!(matches!(
            parse(&data, "self", 100, false).network,
            Reading::Failed(_)
        ));
    }
    #[test]
    fn picture_integer_strings_are_normalized_but_invalid_or_out_of_range_values_fail() {
        let mut data = raw();
        for key in ["backlight", "contrast", "brightness"] {
            data["replies"]["picture"]["settings"][key] = "90".into();
        }
        let Reading::Value(picture) = parse(&data, "self", 100, false).picture else {
            panic!()
        };
        assert_eq!(picture.settings[1].value, "90");
        for bad in [
            json!("90.0"),
            json!("1e2"),
            json!("-1"),
            json!("101"),
            json!(101),
            json!(true),
        ] {
            data["replies"]["picture"]["settings"]["backlight"] = bad;
            assert!(matches!(
                parse(&data, "self", 100, false).picture,
                Reading::Failed(_)
            ));
        }
    }
    #[test]
    fn selected_input_uses_independent_eim_getter_not_device_activate_flags() {
        let mut data = raw();
        data["replies"]["current_input"] =
            serde_json::json!({"returnValue":true,"mainInputSourceId":"HDMI_4"});
        data["replies"]["inputs"]["devices"][0]["activate"] = true.into();
        let Reading::Value(ports) = parse(&data, "com.example.self", 100, false).inputs else {
            panic!("ports")
        };
        assert_eq!(
            ports.iter().map(|p| p.selected).collect::<Vec<_>>(),
            [Some(false), Some(false), Some(false), Some(true)]
        );
        for invalid in [
            serde_json::json!({"returnValue":false}),
            serde_json::json!({"returnValue":true,"mainInputSourceId":"UNKNOWN"}),
            serde_json::json!({"returnValue":true,"mainInputSourceId":true}),
        ] {
            data["replies"]["current_input"] = invalid;
            let Reading::Value(ports) = parse(&data, "com.example.self", 100, false).inputs else {
                panic!("cable/signal reads should remain")
            };
            assert!(ports.iter().all(|p| p.selected.is_none()));
        }
    }
}
