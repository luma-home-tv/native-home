use std::collections::{BTreeMap, BTreeSet};

use plx_ui::form::{Form, FormSection, RowKey, RowKind};
use plx_ui::table::Row;
use serde::{Deserialize, Serialize};
use crate::strings::Message;

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Application {
    pub id: String,
    pub title: String,
    #[serde(rename = "type")]
    pub kind: String,
    #[serde(default = "yes")]
    pub visible: bool,
    #[serde(default)]
    pub icon: Option<String>,
}

fn yes() -> bool { true }

/// A launch target comes from the platform's launch points, never an arbitrary user string.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Launchable {
    pub id: String,
    pub title: String,
    pub key: RowKey,
    pub icon: Option<String>,
}

#[derive(Default)]
pub struct Catalogue {
    keys: BTreeMap<String, RowKey>,
    next_key: u32,
    pub applications: Vec<Launchable>,
}

pub(crate) fn valid_app_id(id: &str) -> bool {
    !id.is_empty() && id.len() <= 180 && id.bytes().all(|c| c.is_ascii_alphanumeric() || b"._-".contains(&c))
}

impl Catalogue {
    /// Preserve keys by identity across reorders and refreshes. Store placeholders and hidden
    /// services are never promoted to actionable applications by a misleading title.
    pub fn refresh(&mut self, applications: &[Application], launch_ids: &BTreeSet<String>) {
        let mut unique = BTreeMap::new();
        for app in applications {
            if !app.visible || !matches!(app.kind.as_str(), "native" | "native_builtin" | "web")
                || !valid_app_id(&app.id) || !launch_ids.contains(&app.id) {
                continue;
            }
            unique.entry(app.id.clone()).or_insert(app);
        }
        self.applications = unique.into_values().map(|app| {
            let key = *self.keys.entry(app.id.clone()).or_insert_with(|| {
                self.next_key += 1;
                RowKey(self.next_key)
            });
            Launchable { id: app.id.clone(), title: app.title.clone(), key, icon: app.icon.clone() }
        }).collect();
        self.applications.sort_by(|a, b| a.title.to_lowercase().cmp(&b.title.to_lowercase()).then_with(|| a.id.cmp(&b.id)));
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum Page { Home, Applications, Inputs, Controls, Picture, Sound, Network, Accessibility, General, PanelCare, PointerSpeed, RangeEditor }

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Action {
    Inspect(String),
    RequestApplication(String),
    RequestInput(String),
    OpenLgHome,
    OpenPointerSpeed,
    OpenDescriptionLevel,
    SetPointerSpeed(crate::pointer_speed::Change),
    ClosePointerSpeed,
    OpenLgSettings,
    OpenLgSettingsSection(SettingsSection),
}

/// Fixed LG-managed overview/entry routes; no arbitrary target, item, flags or path.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SettingsSection { Picture, Sound, Network, Accessibility, General, PanelCare }
impl SettingsSection {
    pub fn page(self) -> Page { match self { Self::Picture => Page::Picture, Self::Sound => Page::Sound, Self::Network => Page::Network, Self::Accessibility => Page::Accessibility, Self::General => Page::General, Self::PanelCare => Page::PanelCare } }
    pub fn from_page(page: Page) -> Option<Self> { match page { Page::Picture => Some(Self::Picture), Page::Sound => Some(Self::Sound), Page::Network => Some(Self::Network), Page::Accessibility => Some(Self::Accessibility), Page::General => Some(Self::General), Page::PanelCare => Some(Self::PanelCare), _ => None } }
    pub fn focus_key(self) -> u32 { match self {Self::Picture=>1000,Self::Sound=>1010,Self::Network=>1020,Self::Accessibility=>1030,Self::General=>1040,Self::PanelCare=>1050} }
    pub fn target(self) -> &'static str { match self {Self::Picture => "picture", Self::Sound => "sound", Self::Network | Self::General => "general", Self::Accessibility => "accessibility", Self::PanelCare => "OledCare"} }
}

/// Validated readout bounds/options, never a write capability.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind")]
pub enum ControlConstraint {
    Range { minimum: i64, maximum: i64, step: i64, active: bool },
    Choices { values: Vec<String>, active: bool },
}

/// Registry metadata does not grant write authority. Pointer speed uses a separate fixed
/// adapter with operation-specific validation and readback.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Setting {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub constraint: Option<ControlConstraint>,
    pub id: String,
    /// A registry-assigned identity key, independent of row order or current value.
    pub focus_key: u32,
    pub label: String,
    pub value: String,
    pub context: Context,
    pub protected: bool,
    pub available: bool,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Context {
    pub input: String,
    pub dynamic_range: String,
    pub picture_mode: String,
}

impl Setting {
    pub fn still_current(&self, current: &Context) -> bool { &self.context == current }
}

pub fn application_form(catalogue: &Catalogue) -> Form<String, Action, Page> {
    let mut section = FormSection::new(Message::Applications.text());
    for app in &catalogue.applications {
        section = section.item_keyed(app.id.clone(), app.key, RowKind::Button,
            Action::RequestApplication(app.id.clone()), Row::new(&app.title).server_label());
    }
    Form::new().section(section)
}

pub(crate) fn display_value(setting: &Setting) -> &str {
    match (setting.id.as_str(), setting.value.as_str()) {
        ("soundOutput", "external_optical") => "Optical",
        ("soundOutput", "external_arc") => "HDMI ARC",
        ("soundOutput", "tv_speaker") => "TV speaker",
        (_, "on") => "On",
        (_, "off") => "Off",
        ("pointerSpeed", "normal") => "Normal",
        ("pointerSpeed", "slow") => "Slow",
        ("pointerSpeed", "fast") => "Fast",
        ("pointerSize", "small") => "Small",
        ("pointerSize", "medium") => "Medium",
        ("pointerSize", "large") => "Large",
        ("audioGuidanceVolume" | "audioGuidancePitch", "veryLow") => "Very low",
        ("audioGuidanceVolume" | "audioGuidancePitch", "low") => "Low",
        ("audioGuidanceVolume" | "audioGuidancePitch" | "audioGuidanceSpeed", "medium") => "Medium",
        ("audioGuidanceVolume" | "audioGuidancePitch", "high") => "High",
        ("audioGuidanceVolume" | "audioGuidancePitch", "veryHigh") => "Very high",
        ("audioGuidanceSpeed", "veryFast") => "Very fast",
        ("audioGuidanceSpeed", "fast") => "Fast",
        ("audioGuidanceSpeed", "slow") => "Slow",
        ("audioGuidanceSpeed", "verySlow") => "Very slow",
        ("colorGamut", "auto") => "Auto",
        ("colorGamut", "native") => "Native",
        ("colorGamut", "dynamic") => "Dynamic",
        ("colorGamut", "adobeRGB") => "Adobe RGB",
        ("gamma", "low") => "Low",
        ("gamma", "medium") => "Medium",
        ("gamma", "high1") => "High 1",
        ("gamma", "high2") => "High 2",
        ("dynamicContrast" | "peakBrightness", "low") => "Low",
        ("dynamicContrast" | "peakBrightness", "medium") => "Medium",
        ("dynamicContrast" | "peakBrightness", "high") => "High",
        ("logoLuminanceAdjust", "light") => "Light",
        ("logoLuminanceAdjust", "strong") => "Strong",
        ("energySaving", "auto") => "Auto",
        ("energySaving", "min") => "Minimum",
        ("energySaving", "med") => "Medium",
        ("energySaving", "max") => "Maximum",
        ("muteStatus", "false" | "False") => "Off",
        ("muteStatus", "true" | "True") => "On",
        ("pictureMode", "game") => "Game",
        ("soundMode", "standard") => "Standard",
        ("audioInput", "hdmi1") => "HDMI 1",
        ("audioInput", "hdmi2") => "HDMI 2",
        ("audioInput", "hdmi3") => "HDMI 3",
        ("audioInput", "hdmi4") => "HDMI 4",
        _ => &setting.value,
    }
}

pub fn controls_form(settings: &[Setting], current: &Context) -> Form<String, Action, Page> {
    controls_form_for(settings, current, true)
}

/// Only the verified Picture route may emit the current Picture-only handoff request.
pub fn controls_form_for(settings: &[Setting], current: &Context, picture_route: bool) -> Form<String, Action, Page> {
    controls_form_for_section(settings, current, picture_route.then_some(SettingsSection::Picture))
}

pub fn controls_form_for_section(settings: &[Setting], current: &Context, section_route: Option<SettingsSection>) -> Form<String, Action, Page> {
    controls_form_with_pointer(settings,current,section_route,false)
}
pub fn controls_form_with_pointer(settings: &[Setting], current: &Context, section_route: Option<SettingsSection>, pointer_enabled:bool) -> Form<String, Action, Page> {
    controls_form_with_numeric(settings,current,section_route,pointer_enabled,None)
}
pub fn controls_form_with_numeric(settings:&[Setting],current:&Context,section_route:Option<SettingsSection>,pointer_enabled:bool,description:Option<crate::description_level::Level>)->Form<String,Action,Page> {
    let mut section = FormSection::new(Message::CurrentSettings.text());
    for setting in settings {
        let editable=pointer_enabled && section_route==Some(SettingsSection::General) && setting.id=="pointerSpeed" && setting.available && setting.still_current(current)
            && matches!(&setting.constraint,Some(ControlConstraint::Choices{active:true,values}) if values.len()==3 && crate::pointer_speed::Speed::ALL.iter().all(|s|values.iter().any(|v|v==s.wire())));
        let unavailable = !setting.available || !setting.still_current(current);
        let detail = if editable {"Choose Slow, Normal or Fast".into()} else if !setting.available { "Reading unavailable".into() }
            else if unavailable { Message::UnavailableContext.text().into() }
            else if setting.protected { match &setting.constraint {
                Some(ControlConstraint::Range {minimum, maximum, active, ..}) if *active => format!("Range {minimum} to {maximum} • LG-managed"),
                Some(ControlConstraint::Range {active: false, ..} | ControlConstraint::Choices {active: false, ..}) => "Inactive in this mode • LG-managed".into(),
                _ => Message::ManagedByLg.text().into()
            } }
            else { String::new() };
        let value = if unavailable { "Unavailable" } else { display_value(setting) };
        section = section.item_keyed(setting.id.clone(), RowKey(setting.focus_key), RowKind::Button,
            if editable {Action::OpenPointerSpeed} else {Action::Inspect(setting.id.clone())}, Row::new(&setting.label).detail(detail).value(value).server_value().chevron(editable))
            .disabled(unavailable || (setting.protected && !editable));
    }
    if section_route==Some(SettingsSection::Accessibility) {
        section=section.item_keyed("audioDescriptionVolume".into(),RowKey(1580),RowKind::Button,Action::OpenDescriptionLevel,
            Row::new("Audio-description level").detail(if description.is_some(){"Signed level • stage then Apply"}else{"Adjust in LG Settings"})
                .value(description.map(|v|v.value().to_string()).unwrap_or_else(||"LG-managed".into())).chevron(description.is_some())).disabled(description.is_none());
    }
    let lg = if let Some(route) = section_route { FormSection::new(Message::MoreSettings.text()).item_keyed("lg-settings".to_owned(), RowKey(route.focus_key()),
        RowKind::Button, if route == SettingsSection::Picture {Action::OpenLgSettings} else {Action::OpenLgSettingsSection(route)},
        Row::new(if route == SettingsSection::Network {"Open LG Network entry"} else {Message::OpenLgSettings.text()})) }
        else { FormSection::new(Message::MoreSettings.text()).item_keyed("lg-managed".to_owned(), RowKey(1001),
            RowKind::Button, Action::Inspect("lg-managed".into()),
            Row::new("More controls").detail("Open LG Settings from LG Home").value("LG-managed")).disabled(true) };
    Form::new().section(section).section(lg)
}

#[cfg(test)]
mod tests {
    use super::*;
    use plx_ui::form::{Activation, FormTable};

    fn app(id: &str, kind: &str, visible: bool) -> Application {
        Application { id: id.into(), title: id.into(), kind: kind.into(), visible, icon: None }
    }
    fn context() -> Context {
        Context { input: "HDMI_1".into(), dynamic_range: "sdr".into(), picture_mode: "cinema".into() }
    }
    fn setting(id: &str, protected: bool) -> Setting {
        Setting { constraint: None, id: id.into(), focus_key: if protected { 1 } else { 2 }, label: id.into(), value: "50".into(), context: context(), protected, available: true }
    }

    #[test]
    fn drawer_excludes_placeholders_hidden_services_and_unregistered_targets() {
        let apps = [app("com.example.video", "native", true), app("com.example.store", "stub", true),
            app("com.example.service", "native", false), app("com.example.unregistered", "web", true),
            app("bad;command", "native", true)];
        let ids = apps.iter().map(|a| a.id.clone()).collect();
        let mut catalogue = Catalogue::default(); catalogue.refresh(&apps, &ids);
        // Remove an application from the authoritative launch-point list.
        let ids = BTreeSet::from(["com.example.video".into(), "com.example.store".into(), "com.example.service".into(), "bad;command".into()]);
        catalogue.refresh(&apps, &ids);
        assert_eq!(catalogue.applications.iter().map(|a| a.id.as_str()).collect::<Vec<_>>(), ["com.example.video"]);
    }

    #[test]
    fn drawer_focus_identity_survives_rename_reorder_and_duplicates() {
        let first = [app("com.example.a", "native", true), app("com.example.b", "web", true)];
        let ids = first.iter().map(|a| a.id.clone()).collect();
        let mut catalogue = Catalogue::default(); catalogue.refresh(&first, &ids);
        let key = catalogue.applications[0].key;
        let mut renamed = first[0].clone(); renamed.title = "Zulu".into();
        catalogue.refresh(&[first[1].clone(), renamed.clone(), renamed], &ids);
        assert_eq!(catalogue.applications.len(), 2);
        assert_eq!(catalogue.applications.iter().find(|a| a.id == "com.example.a").unwrap().key, key);
    }

    #[test]
    fn reused_native_form_cannot_activate_protected_or_stale_controls() {
        let settings = [setting("panel-protection", true), setting("brightness", false)];
        let mut table = FormTable::new(2000);
        table.set(controls_form(&settings, &context()), None);
        assert_eq!(table.activate(0), None);
        assert_eq!(table.activate(1), Some(Activation::Action(Action::Inspect("brightness".into()))));
        let mut hdr = context(); hdr.dynamic_range = "hdr".into();
        table.set(controls_form(&settings, &hdr), None);
        assert_eq!(table.activate(1), None);
        assert_eq!(table.table.sections[0].rows[1].value.as_deref(), Some("Unavailable"));
        assert_eq!(table.activate(2), Some(Activation::Action(Action::OpenLgSettings)));
    }

    #[test]
    fn displayed_labels_preserve_raw_state_and_never_hide_staleness() {
        let mut output=setting("soundOutput",false);output.value="external_optical".into();
        let mut table=FormTable::new(2000);table.set(controls_form(&[output.clone()],&context()),None);
        assert_eq!(table.table.sections[0].rows[0].value.as_deref(),Some("Optical"));
        assert_eq!(output.value,"external_optical");
        let mut changed=context();changed.input="HDMI_2".into();table.set(controls_form(&[output],&changed),None);
        assert_eq!(table.table.sections[0].rows[0].value.as_deref(),Some("Unavailable"));assert_eq!(table.activate(0),None);
    }

    #[test]
    fn application_activation_is_bound_to_registered_identity() {
        let apps = [app("com.example.video", "native", true)];
        let ids = BTreeSet::from([apps[0].id.clone()]);
        let mut catalogue = Catalogue::default(); catalogue.refresh(&apps, &ids);
        let mut table = FormTable::new(2000); table.set(application_form(&catalogue), None);
        assert_eq!(table.activate(0), Some(Activation::Action(Action::RequestApplication(apps[0].id.clone()))));
        catalogue.refresh(&[], &BTreeSet::new()); table.set(application_form(&catalogue), None);
        assert_eq!(table.activate(0), None);
    }
}
