//! Fixed control-row wording using existing capability presentation decisions.
//! No services, clocks, actions, values or write admission are created here.
//! Callers retain their actual adapter, action and disabled-state validation.

use crate::control_capability::{
    self, Control, DescriptorState, DisplayState, EditEligibility, EvidenceState, Policy,
    Presentation, WriteCapability,
};
use crate::model::SettingsSection;

/// Only existing validated facts belong here. `Unavailable` must not be guessed
/// to mean `Stale`: the caller must retain evidence for any more precise reason.
/// `write` and `busy` refer to the matching existing fixed adapter operation.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct RowInput<'a> {
    pub section: Option<SettingsSection>,
    pub setting_id: &'a str,
    pub reading: EvidenceState,
    pub descriptor: DescriptorState,
    pub write: Option<WriteCapability<'a>>,
    pub lg_route_verified: bool,
    pub busy: bool,
}

/// Advisory display data, never a replacement for a setter's admission checks.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct RowPresentation {
    pub presentation: Presentation,
    pub detail: &'static str,
}
impl RowPresentation {
    /// Descriptor availability does not hide a separately current base reading.
    pub fn show_current_value(self) -> bool {
        self.presentation.value_is_current
    }
}

/// Fixed section-and-row identity policy; descriptor membership never adds an
/// operation. In particular, Sound's description-level reading is delegated,
/// while Accessibility's dedicated editor still needs its real adapter result.
pub fn policy(section: Option<SettingsSection>, setting_id: &str) -> Policy {
    use SettingsSection as S;
    match (section, setting_id) {
        (Some(S::General), "pointerSpeed")
        | (Some(S::Accessibility), "audioDescriptionVolume") => Policy::NativeCandidate,
        (Some(S::PanelCare | S::Network), _)
        | (Some(S::General), "quickStartMode" | "homeAutoLaunch"
            | "localeInfo.locales.UI" | "localeInfo.locales.AUD" | "localeInfo.locales.AUD2"
            | "country" | "smartServiceCountryCode2") => Policy::Protected,
        (
            Some(S::Picture),
            "pictureMode" | "backlight" | "contrast" | "brightness" | "color" | "sharpness"
            | "tint" | "colorTemperature" | "colorGamut" | "gamma" | "dynamicContrast"
            | "peakBrightness",
        )
        | (
            Some(S::Sound),
            "soundOutput" | "volume" | "muteStatus" | "soundMode" | "audioBalance"
            | "audioDescription" | "audioDescriptionVolume",
        )
        | (
            Some(S::Accessibility),
            "audioGuidance" | "highContrast" | "screenMagnification" | "menuTransparency"
            | "audioGuidanceVolume" | "audioGuidancePitch" | "audioGuidanceSpeed",
        )
        | (Some(S::General), "pointerSize" | "homePromotion") => Policy::LgManaged,
        // Metadata such as audioInput, and unknown future rows, remain readings.
        // Even caller-supplied matching write evidence cannot promote ViewOnly.
        _ => Policy::ViewOnly,
    }
}

fn wording(presentation: Presentation, reading: EvidenceState) -> &'static str {
    let pending = presentation.operation_pending;
    match (reading, pending) {
        (EvidenceState::Unavailable, false) => return "Reading unavailable",
        (EvidenceState::Unavailable, true) => return "Reading unavailable • update pending",
        (EvidenceState::Stale, false) => return "Reading expired",
        (EvidenceState::Stale, true) => return "Reading expired • update pending",
        (EvidenceState::WrongContext, false) => return "Unavailable in this context",
        (EvidenceState::WrongContext, true) => {
            return "Unavailable in this context • update pending"
        }
        (EvidenceState::Current, _) => {}
    }
    if pending {
        return "Applying • awaiting confirmation";
    }
    match presentation.state {
        DisplayState::NativeControl => "Adjust here",
        DisplayState::Protected => "Protected • view only",
        DisplayState::InactiveInContext if presentation.lg_settings_available => {
            "Inactive in this mode • LG-managed"
        }
        DisplayState::InactiveInContext => "Inactive in this mode • view only",
        DisplayState::LgManaged => {
            if presentation.lg_settings_available {
                "Adjust in LG Settings"
            } else {
                "View only"
            }
        }
        DisplayState::NativeView if presentation.edit == EditEligibility::DescriptorUnknown => {
            "View only • control not verified"
        }
        DisplayState::NativeView => "View only",
        // These states are covered by reading above; retain fail-closed wording
        // if the underlying presentation implementation later grows a new path.
        DisplayState::ReadingUnavailable => "Reading unavailable",
        DisplayState::ReadingStale => "Reading expired",
        DisplayState::ReadingWrongContext => "Unavailable in this context",
    }
}

pub fn present(input: RowInput<'_>) -> RowPresentation {
    let presentation = control_capability::evaluate(Control {
        setting_id: input.setting_id,
        policy: policy(input.section, input.setting_id),
        reading: input.reading,
        descriptor: input.descriptor,
        write: input.write,
        lg_route_verified: input.lg_route_verified,
        busy: input.busy,
    });
    RowPresentation {
        detail: wording(presentation, input.reading),
        presentation,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    fn input(section: SettingsSection, setting_id: &str) -> RowInput<'_> {
        RowInput {
            section: Some(section),
            setting_id,
            reading: EvidenceState::Current,
            descriptor: DescriptorState::Active,
            write: None,
            lg_route_verified: true,
            busy: false,
        }
    }
    #[test]
    fn current_base_reading_stays_visible_without_descriptor_or_native_editor() {
        let mut row = input(SettingsSection::Sound, "volume");
        row.descriptor = DescriptorState::Unknown;
        let view = present(row);
        assert!(view.show_current_value());
        assert_eq!(view.detail, "Adjust in LG Settings");
        assert!(!view.presentation.can_edit());
    }
    #[test]
    fn same_description_identity_cannot_transfer_editor_admission_to_sound() {
        let mut row = input(SettingsSection::Accessibility, "audioDescriptionVolume");
        row.write = Some(WriteCapability {
            setting_id: row.setting_id,
            state: EvidenceState::Current,
        });
        assert!(present(row).presentation.can_edit());
        row.section = Some(SettingsSection::Sound);
        let sound = present(row);
        assert!(!sound.presentation.can_edit());
        assert_eq!(sound.detail, "Adjust in LG Settings");
    }
    #[test]
    fn pointer_size_and_unknown_rows_never_gain_writes_from_a_descriptor() {
        for setting_id in ["pointerSize", "unknownFutureControl"] {
            let mut row = input(SettingsSection::General, setting_id);
            row.write = Some(WriteCapability {
                setting_id,
                state: EvidenceState::Current,
            });
            assert!(!present(row).presentation.can_edit());
        }
        let mut pointer = input(SettingsSection::General, "pointerSpeed");
        assert!(!present(pointer).presentation.can_edit());
        pointer.write = Some(WriteCapability {
            setting_id: "pointerSpeed",
            state: EvidenceState::Current,
        });
        assert_eq!(present(pointer).detail, "Adjust here");
        pointer.write.as_mut().unwrap().setting_id = "audioDescriptionVolume";
        assert!(!present(pointer).presentation.can_edit());
    }
    #[test]
    fn expiry_context_and_unavailability_hide_values_without_inventing_causes() {
        let mut row = input(SettingsSection::Picture, "contrast");
        for (evidence, text) in [
            (EvidenceState::Unavailable, "Reading unavailable"),
            (EvidenceState::Stale, "Reading expired"),
            (EvidenceState::WrongContext, "Unavailable in this context"),
        ] {
            row.reading = evidence;
            let view = present(row);
            assert!(!view.show_current_value());
            assert_eq!(view.detail, text);
            assert!(view.presentation.lg_settings_available);
        }
    }
    #[test]
    fn protected_current_readings_remain_legible_and_cannot_gain_editing() {
        for (section, id) in [
            (SettingsSection::PanelCare, "screenShift"),
            (SettingsSection::Network, "wired"),
            (SettingsSection::General, "quickStartMode"),
            (SettingsSection::General, "homeAutoLaunch"),
            (SettingsSection::General, "localeInfo.locales.UI"),
            (SettingsSection::General, "localeInfo.locales.AUD"),
            (SettingsSection::General, "localeInfo.locales.AUD2"),
            (SettingsSection::General, "country"),
            (SettingsSection::General, "smartServiceCountryCode2"),
        ] {
            let mut row = input(section, id);
            row.write = Some(WriteCapability {
                setting_id: id,
                state: EvidenceState::Current,
            });
            let view = present(row);
            assert!(view.show_current_value());
            assert!(!view.presentation.can_edit());
            assert_eq!(view.detail, "Protected • view only");
        }
    }
    #[test]
    fn inactive_metadata_and_missing_routes_have_honest_read_only_wording() {
        let mut row = input(SettingsSection::Sound, "audioBalance");
        row.descriptor = DescriptorState::Inactive;
        assert_eq!(present(row).detail, "Inactive in this mode • LG-managed");
        assert!(present(row).show_current_value());
        row.lg_route_verified = false;
        assert_eq!(present(row).detail, "Inactive in this mode • view only");
        row.descriptor = DescriptorState::Active;
        assert_eq!(present(row).detail, "View only");
        assert!(!present(row).presentation.lg_settings_available);
    }
    #[test]
    fn pending_operation_keeps_tracking_after_read_or_write_admission_is_lost() {
        let mut row = input(SettingsSection::General, "pointerSpeed");
        row.write = Some(WriteCapability {
            setting_id: row.setting_id,
            state: EvidenceState::Current,
        });
        row.busy = true;
        let busy = present(row);
        assert!(!busy.presentation.can_edit());
        assert_eq!(busy.detail, "Applying • awaiting confirmation");
        row.reading = EvidenceState::Stale;
        row.write = None;
        let expired = present(row);
        assert!(expired.presentation.operation_pending);
        assert!(!expired.show_current_value());
        assert_eq!(expired.detail, "Reading expired • update pending");
    }
    #[test]
    fn sectionless_and_unknown_rows_stay_view_only_despite_matching_admission() {
        let mut row = input(SettingsSection::Sound, "unknownFutureControl");
        row.section = None;
        row.write = Some(WriteCapability {
            setting_id: row.setting_id,
            state: EvidenceState::Current,
        });
        assert_eq!(present(row).detail, "View only");
        assert!(!present(row).presentation.can_edit());
        assert!(present(row).show_current_value());
    }
}
