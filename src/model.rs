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
        // These are opaque preferences, not boolean setting values.
        ("localeInfo.timezone" | "localeInfo.locales.FMT", _) => &setting.value,
        (_, "on") => "On",
        (_, "off") => "Off",
        ("localeInfo.clock", "12") => "12-hour",
        ("localeInfo.clock", "24") => "24-hour",
        ("localeInfo.clock", "locale") => "Follow regional format",
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
    use crate::control_capability::{DescriptorState, EvidenceState, WriteCapability};
    use crate::control_row_presentation::{present, RowInput};
    let mut section = FormSection::new(Message::CurrentSettings.text());
    for setting in settings {
        let editable=pointer_enabled && section_route==Some(SettingsSection::General) && setting.id=="pointerSpeed" && setting.available && setting.still_current(current)
            && matches!(&setting.constraint,Some(ControlConstraint::Choices{active:true,values}) if values.len()==3 && crate::pointer_speed::Speed::ALL.iter().all(|s|values.iter().any(|v|v==s.wire())));
        let unavailable = !setting.available || !setting.still_current(current);
        let reading = if !setting.available { EvidenceState::Unavailable }
            else if !setting.still_current(current) { EvidenceState::WrongContext }
            else { EvidenceState::Current };
        let descriptor = match &setting.constraint {
            Some(ControlConstraint::Range {active:true,..} | ControlConstraint::Choices {active:true,..}) => DescriptorState::Active,
            Some(_) => DescriptorState::Inactive,
            None => DescriptorState::Unknown,
        };
        // Presentation describes the existing fixed adapter's admission. It cannot
        // create actions or change the independently maintained disabled binding.
        let hint = present(RowInput {section:section_route,setting_id:&setting.id,reading,descriptor,
            write:editable.then_some(WriteCapability {setting_id:&setting.id,state:EvidenceState::Current}),
            lg_route_verified:section_route.is_some(),busy:false});
        let detail = if editable {"Choose Slow, Normal or Fast".into()}
            else if let Some(ControlConstraint::Range {minimum,maximum,active:true,..}) = &setting.constraint {
                if hint.show_current_value() {format!("Range {minimum} to {maximum} • {}",hint.detail)} else {hint.detail.into()}
            } else {hint.detail.into()};
        let value = if hint.show_current_value() { display_value(setting) } else { "Unavailable" };
        section = section.item_keyed(setting.id.clone(), RowKey(setting.focus_key), RowKind::Button,
            if editable {Action::OpenPointerSpeed} else {Action::Inspect(setting.id.clone())}, Row::new(&setting.label).detail(detail).value(value).server_value().chevron(editable))
            .read_only(setting.protected && !editable)
            .disabled(unavailable || (setting.protected && descriptor == DescriptorState::Inactive));
    }
    if section_route==Some(SettingsSection::Accessibility) {
        section=section.item_keyed("audioDescriptionVolume".into(),RowKey(1580),RowKind::Button,Action::OpenDescriptionLevel,
            Row::new("Audio-description level").detail(if description.is_some(){"Signed level • stage then Apply"}else{"Adjust in LG Settings"})
                .value(description.map(|v|v.value().to_string()).unwrap_or_else(||"LG-managed".into())).chevron(description.is_some())).disabled(description.is_none());
    }
    // Keep LG management reachable before long or unavailable native readouts.
    let lg = if let Some(route) = section_route {
        let (label, detail) = match route {
            SettingsSection::Picture => ("Open LG Picture", "Opens the Picture overview"),
            SettingsSection::Sound => ("Open LG Sound", "Opens the Sound overview"),
            SettingsSection::Network => (
                "Open LG Network entry",
                "Opens General for network settings",
            ),
            SettingsSection::Accessibility => (
                "Open LG Accessibility",
                "Opens the Accessibility overview",
            ),
            SettingsSection::General => ("Open LG General", "Opens the General overview"),
            SettingsSection::PanelCare => ("Open LG OLED Care", "Opens the OLED Care overview"),
        };
        FormSection::new(Message::MoreSettings.text()).item_keyed(
            "lg-settings".to_owned(), RowKey(route.focus_key()), RowKind::Button,
            if route == SettingsSection::Picture {
                Action::OpenLgSettings
            } else {
                Action::OpenLgSettingsSection(route)
            },
            Row::new(label).detail(detail),
        )
    } else {
        FormSection::new(Message::MoreSettings.text()).item_keyed(
            "lg-managed".to_owned(), RowKey(1001), RowKind::Button,
            Action::Inspect("lg-managed".into()),
            Row::new("More controls").detail("Open LG Settings from LG Home").value("LG-managed"),
        ).disabled(true)
    };
    Form::new().section(lg).section(section)
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
        assert_eq!(table.activate(1), None);
        assert_eq!(table.activate(2), Some(Activation::Action(Action::Inspect("brightness".into()))));
        let mut hdr = context(); hdr.dynamic_range = "hdr".into();
        table.set(controls_form(&settings, &hdr), None);
        assert_eq!(table.activate(2), None);
        assert_eq!(table.table.sections[1].rows[1].value.as_deref(), Some("Unavailable"));
        assert_eq!(table.activate(0), Some(Activation::Action(Action::OpenLgSettings)));
    }

    #[test]
    fn displayed_labels_preserve_raw_state_and_never_hide_staleness() {
        let mut output=setting("soundOutput",false);output.value="external_optical".into();
        let mut table=FormTable::new(2000);table.set(controls_form(&[output.clone()],&context()),None);
        assert_eq!(table.table.sections[1].rows[0].value.as_deref(),Some("Optical"));
        assert_eq!(output.value,"external_optical");
        let mut changed=context();changed.input="HDMI_2".into();table.set(controls_form(&[output],&changed),None);
        assert_eq!(table.table.sections[1].rows[0].value.as_deref(),Some("Unavailable"));assert_eq!(table.activate(1),None);
    }

    #[test]
    fn control_descriptions_explain_current_readings_without_enabling_edits() {
        let mut table=FormTable::new(2000);
        table.set(controls_form(&[setting("brightness",false)],&context()),None);
        let row=&table.table.sections[1].rows[0];
        assert_eq!(row.value.as_deref(),Some("50"));
        assert_eq!(row.detail,"Adjust in LG Settings");
        assert_eq!(table.activate(1),Some(Activation::Action(Action::Inspect("brightness".into()))));
        let mut panel=setting("screenShift",true);panel.value="on".into();
        table.set(controls_form_for_section(&[panel],&context(),Some(SettingsSection::PanelCare)),None);
        assert_eq!(table.table.sections[1].rows[0].value.as_deref(),Some("On"));
        assert_eq!(table.table.sections[1].rows[0].detail,"Protected • view only");
        assert_eq!(table.activate(1),None);
        assert_eq!(table.activate(0),Some(Activation::Action(Action::OpenLgSettingsSection(SettingsSection::PanelCare))));
    }

    #[test]
    fn sound_range_description_never_reuses_the_accessibility_editor() {
        let mut level=setting("audioDescriptionVolume",true);level.value="0".into();
        level.constraint=Some(ControlConstraint::Range{minimum:-15,maximum:15,step:1,active:true});
        let mut table=FormTable::new(2000);
        table.set(controls_form_with_numeric(&[level.clone()],&context(),Some(SettingsSection::Sound),false,None),None);
        assert_eq!(table.table.sections[1].rows[0].detail,"Range -15 to 15 • Adjust in LG Settings");
        assert_eq!(table.activate(1),None);
        let mut other=context();other.input="HDMI_2".into();
        table.set(controls_form_for_section(&[level],&other,Some(SettingsSection::Sound)),None);
        assert_eq!(table.table.sections[1].rows[0].value.as_deref(),Some("Unavailable"));
        assert_eq!(table.table.sections[1].rows[0].detail,"Unavailable in this context");
        assert_eq!(table.activate(1),None);
        assert_eq!(table.activate(0),Some(Activation::Action(Action::OpenLgSettingsSection(SettingsSection::Sound))));
    }

    #[test]
    fn locale_readings_stay_visible_and_protected_in_the_actual_general_form() {
        let _serial = plx_base::testlock::serial();
        let replies = serde_json::json!({
            "locale_values": {"returnValue":true,"settings":{"localeInfo":{"locales":{
                "UI":"en-GB","AUD":"fr-FR","AUD2":"de_DE","FMT":"en-GB"
            },"clock":"locale","timezone":"Europe/London"}}},
            "region_values": {"returnValue":true,"category":"option","settings":{
                "country":"GBR","smartServiceCountryCode2":"GB"
            }},
            "locale_desc": {"returnValue":false,"errorText":"Optional descriptor unavailable"},
            "region_desc": {"returnValue":false,"errorText":"Optional descriptor unavailable"}
        });
        let current = context();
        let mut settings = crate::locale_reads::parse(&replies);
        assert_eq!(settings.len(), 8);
        // Live integration binds these globally validated readings to the current
        // presentation snapshot before building the same General form.
        for setting in &mut settings { setting.context = current.clone(); }
        let expected = [
            ("localeInfo.locales.UI", 145, "en-GB"),
            ("localeInfo.locales.AUD", 146, "fr-FR"),
            ("localeInfo.locales.AUD2", 147, "de_DE"),
            ("country", 148, "GBR"),
            ("smartServiceCountryCode2", 149, "GB"),
            ("localeInfo.clock", 153, "Follow regional format"),
            ("localeInfo.timezone", 154, "Europe/London"),
            ("localeInfo.locales.FMT", 155, "en-GB"),
        ];
        for pointer_enabled in [false, true] {
            let mut table = FormTable::new(2000);
            table.set(controls_form_with_numeric(
                &settings, &current, Some(SettingsSection::General), pointer_enabled, None
            ), None);
            for (index, (id, key, value)) in expected.iter().enumerate() {
                assert!(settings[index].available && settings[index].protected);
                assert!(settings[index].constraint.is_none());
                let binding = table.binding_at(index + 1).unwrap();
                assert_eq!(binding.id, *id);
                assert_eq!(binding.key, RowKey(*key));
                assert!(binding.disabled);
                assert_eq!(binding.action, Action::Inspect((*id).into()));
                let row = &table.table.sections[1].rows[index];
                assert_eq!(row.value.as_deref(), Some(*value));
                assert!(!row.dim, "a current protected reading must retain normal ink");
                assert_eq!(row.detail, "Protected • view only");
                assert!(row.ticon.is_none(), "readings must not advertise a drill-in editor");
                assert_eq!(table.activate(index + 1), None);
            }
            let route = table.binding_at(0).unwrap();
            assert_eq!(route.id, "lg-settings");
            assert_eq!(route.key, RowKey(1040));
            assert!(!route.disabled);
            assert_eq!(table.activate(0), Some(Activation::Action(
                Action::OpenLgSettingsSection(SettingsSection::General)
            )));
        }
    }

    #[test]
    fn stored_clock_format_labels_do_not_resolve_regional_hour_cycle() {
        for (wire, label) in [("12", "12-hour"), ("24", "24-hour"), ("locale", "Follow regional format")] {
            let mut value = setting("localeInfo.clock", true);
            value.value = wire.into();
            assert_eq!(display_value(&value), label);
            assert_eq!(value.value, wire);
        }
    }

    #[test]
    fn opaque_time_preferences_keep_wire_case_in_the_actual_general_form() {
        let _serial = plx_base::testlock::serial();
        let current = context();
        for wire in ["on", "off", "On", "OFF"] {
            let replies = serde_json::json!({
                "locale_values": {"returnValue":true,"settings":{"localeInfo":{
                    "clock":"locale","timezone":wire,"locales":{"FMT":wire}
                }}}
            });
            let mut settings = crate::locale_reads::parse(&replies);
            for setting in &mut settings { setting.context = current.clone(); }
            let mut form = FormTable::new(2000);
            form.set(controls_form_for_section(&settings, &current, Some(SettingsSection::General)), None);
            for id in ["localeInfo.timezone", "localeInfo.locales.FMT"] {
                let row = settings.iter().position(|setting| setting.id == id).unwrap();
                assert!(settings[row].available && settings[row].protected);
                assert_eq!(settings[row].value, wire);
                let position = form.index_of(&id.into()).unwrap();
                assert!(form.binding_at(position).unwrap().disabled);
                assert_eq!(form.activate(position), None);
                assert_eq!(form.table.sections[1].rows[row].value.as_deref(), Some(wire));
            }
            let clock = settings.iter().position(|setting| setting.id == "localeInfo.clock").unwrap();
            assert_eq!(settings[clock].value, "locale");
            assert_eq!(form.table.sections[1].rows[clock].value.as_deref(), Some("Follow regional format"));
        }
    }

    #[test]
    fn unavailable_wrong_context_and_inactive_readings_remain_dim_and_inert() {
        let _serial = plx_base::testlock::serial();
        let current = context();
        let ids = ["localeInfo.locales.UI", "localeInfo.locales.AUD", "country", "smartServiceCountryCode2"];
        let mut rows = ids.iter().enumerate().map(|(i, id)| {
            let mut row = setting(id, true);
            row.focus_key = 145 + i as u32;
            row.value = "GBR".into();
            row
        }).collect::<Vec<_>>();
        rows[1].available = false;
        rows[2].context.input = "HDMI_4".into();
        rows[3].constraint = Some(ControlConstraint::Choices {
            values: vec!["GBR".into()], active: false,
        });
        let mut table = FormTable::new(2000);
        table.set(controls_form_with_numeric(&rows, &current,
            Some(SettingsSection::General), true, None), None);
        assert!(!table.table.sections[1].rows[0].dim);
        for index in 0..4 {
            assert!(table.binding_at(index + 1).unwrap().disabled);
            assert_eq!(table.activate(index + 1), None);
            assert_eq!(table.table.sections[1].rows[index].dim, index != 0);
        }
        assert_eq!(table.table.sections[1].rows[1].value.as_deref(), Some("Unavailable"));
        assert_eq!(table.table.sections[1].rows[2].value.as_deref(), Some("Unavailable"));
    }

    #[test]
    fn every_category_keeps_its_existing_lg_route_first_when_readings_fail() {
        let _serial = plx_base::testlock::serial();
        let mut unavailable = setting("unavailable", true);
        unavailable.available = false;
        let mut stale = setting("stale", true);
        stale.focus_key = 3;
        stale.context.input = "HDMI_4".into();
        let cases = [
            (
                SettingsSection::Picture, "Open LG Picture",
                "Opens the Picture overview", Action::OpenLgSettings,
            ),
            (
                SettingsSection::Sound, "Open LG Sound",
                "Opens the Sound overview", Action::OpenLgSettingsSection(SettingsSection::Sound),
            ),
            (
                SettingsSection::Network, "Open LG Network entry",
                "Opens General for network settings", Action::OpenLgSettingsSection(SettingsSection::Network),
            ),
            (
                SettingsSection::Accessibility, "Open LG Accessibility",
                "Opens the Accessibility overview", Action::OpenLgSettingsSection(SettingsSection::Accessibility),
            ),
            (
                SettingsSection::General, "Open LG General",
                "Opens the General overview", Action::OpenLgSettingsSection(SettingsSection::General),
            ),
            (
                SettingsSection::PanelCare, "Open LG OLED Care",
                "Opens the OLED Care overview", Action::OpenLgSettingsSection(SettingsSection::PanelCare),
            ),
        ];
        let mut route_keys = BTreeSet::new();
        for (route, label, detail, action) in cases {
            let mut table = FormTable::new(2000);
            table.set(controls_form_for_section(
                &[unavailable.clone(), stale.clone()], &context(), Some(route),
            ), None);
            let binding = table.binding_at(0).unwrap();
            assert_eq!(binding.id, "lg-settings");
            assert_eq!(binding.key, RowKey(route.focus_key()));
            assert!(route_keys.insert(binding.key.0));
            assert!(!binding.disabled);
            assert_eq!(table.activate(0), Some(Activation::Action(action)));
            assert_eq!(table.table.sections[0].rows.len(), 1);
            assert_eq!(table.table.sections[0].rows[0].label, label);
            assert_eq!(table.table.sections[0].rows[0].detail, detail);
            for index in 1..=2 {
                assert_eq!(table.activate(index), None);
                assert_eq!(
                    table.table.sections[1].rows[index - 1].value.as_deref(),
                    Some("Unavailable"),
                );
            }
        }
        let mut table = FormTable::new(2000);
        table.set(controls_form_for_section(&[], &context(), None), None);
        assert!(table.binding_at(0).unwrap().disabled);
        assert_eq!(table.activate(0), None);
    }

    #[test]
    fn leading_lg_route_does_not_change_pointer_or_description_editor_admission() {
        let _serial = plx_base::testlock::serial();
        let mut pointer = setting("pointerSpeed", true);
        pointer.focus_key = 141;
        pointer.value = "normal".into();
        pointer.constraint = Some(ControlConstraint::Choices {
            values: vec!["slow".into(), "normal".into(), "fast".into()], active: true,
        });
        let mut table = FormTable::new(2000);
        for enabled in [false, true] {
            table.set(controls_form_with_pointer(
                &[pointer.clone()], &context(), Some(SettingsSection::General), enabled,
            ), None);
            assert_eq!(table.binding_at(1).unwrap().key, RowKey(141));
            assert_eq!(table.activate(1), if enabled {
                Some(Activation::Action(Action::OpenPointerSpeed))
            } else { None });
            assert_eq!(table.activate(0), Some(Activation::Action(
                Action::OpenLgSettingsSection(SettingsSection::General),
            )));
        }
        let level = crate::description_level::Level::new(0).unwrap();
        for admitted in [None, Some(level)] {
            table.set(controls_form_with_numeric(
                &[], &context(), Some(SettingsSection::Accessibility), false, admitted,
            ), None);
            assert_eq!(table.binding_at(1).unwrap().id, "audioDescriptionVolume");
            assert_eq!(table.binding_at(1).unwrap().key, RowKey(1580));
            assert_eq!(table.activate(1), admitted.map(|_| {
                Activation::Action(Action::OpenDescriptionLevel)
            }));
            assert_eq!(table.activate(0), Some(Activation::Action(
                Action::OpenLgSettingsSection(SettingsSection::Accessibility),
            )));
        }
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
