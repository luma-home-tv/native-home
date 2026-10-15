//! Pure control-row presentation policy. No services, rendering, writes or timers.
//! WriteCapability is a caller-supplied adapter result, never descriptor-derived authority.

/// Integration computes this using its existing freshness and exact-context checks.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum EvidenceState {
    Current,
    Unavailable,
    Stale,
    WrongContext,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Policy {
    ViewOnly,
    LgManaged,
    /// Only specifically implemented, reviewed operations use this policy.
    NativeCandidate,
    /// Never promoted by a write result, descriptor or available LG route.
    Protected,
}
/// Descriptor availability can withdraw an editor, but cannot create one.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum DescriptorState {
    Active,
    Inactive,
    Unknown,
}
/// A summary of a separately verified fixed adapter's admission, not an action token.
/// `state` must include the adapter's own expiry, identity, context and ownership gates.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct WriteCapability<'a> {
    pub setting_id: &'a str,
    pub state: EvidenceState,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Control<'a> {
    pub setting_id: &'a str,
    pub policy: Policy,
    pub reading: EvidenceState,
    pub descriptor: DescriptorState,
    pub write: Option<WriteCapability<'a>>,
    /// An independently qualified route; a missing reading must not remove it.
    pub lg_route_verified: bool,
    /// A matching operation is already pending; never admits another edit.
    pub busy: bool,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum EditEligibility {
    Ready,
    Protected,
    ViewOnly,
    LgManaged,
    ReadingUnavailable,
    ReadingStale,
    ReadingWrongContext,
    InactiveInContext,
    DescriptorUnknown,
    WriteUnverified,
    WriteUnavailable,
    WriteStale,
    WriteWrongContext,
    Busy,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum DisplayState {
    NativeControl,
    NativeView,
    LgManaged,
    Protected,
    ReadingUnavailable,
    ReadingStale,
    ReadingWrongContext,
    InactiveInContext,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Presentation {
    pub state: DisplayState,
    pub edit: EditEligibility,
    /// False means hide the value or explicitly label it last-known, never live/current.
    pub value_is_current: bool,
    /// Keep tracking an existing operation even after its admission is withdrawn.
    /// Edit eligibility never cancels a queued/in-flight operation.
    pub operation_pending: bool,
    /// Route admission is separate from value and native-edit admission.
    pub lg_settings_available: bool,
}
impl Presentation {
    pub fn can_edit(self) -> bool {
        self.edit == EditEligibility::Ready
    }
}

fn eligibility(control: Control<'_>) -> EditEligibility {
    use EditEligibility as E;
    if control.policy == Policy::Protected {
        return E::Protected;
    }
    match control.reading {
        EvidenceState::Unavailable => return E::ReadingUnavailable,
        EvidenceState::Stale => return E::ReadingStale,
        EvidenceState::WrongContext => return E::ReadingWrongContext,
        EvidenceState::Current => {}
    }
    match control.policy {
        Policy::ViewOnly => return E::ViewOnly,
        Policy::LgManaged => return E::LgManaged,
        Policy::Protected => unreachable!(),
        Policy::NativeCandidate => {}
    }
    match control.descriptor {
        DescriptorState::Inactive => return E::InactiveInContext,
        DescriptorState::Unknown => return E::DescriptorUnknown,
        DescriptorState::Active => {}
    }
    let Some(write) = control
        .write
        .filter(|write| !control.setting_id.is_empty() && write.setting_id == control.setting_id)
    else {
        return E::WriteUnverified;
    };
    match write.state {
        EvidenceState::Unavailable => return E::WriteUnavailable,
        EvidenceState::Stale => return E::WriteStale,
        EvidenceState::WrongContext => return E::WriteWrongContext,
        EvidenceState::Current => {}
    }
    if control.busy {
        E::Busy
    } else {
        E::Ready
    }
}

/// Re-evaluate on every relevant transition; never cache this as setter authority.
pub fn evaluate(control: Control<'_>) -> Presentation {
    let edit = eligibility(control);
    let state = if control.policy == Policy::Protected {
        DisplayState::Protected
    } else {
        match control.reading {
            EvidenceState::Unavailable => DisplayState::ReadingUnavailable,
            EvidenceState::Stale => DisplayState::ReadingStale,
            EvidenceState::WrongContext => DisplayState::ReadingWrongContext,
            EvidenceState::Current => {
                if control.descriptor == DescriptorState::Inactive {
                    DisplayState::InactiveInContext
                } else if matches!(edit, EditEligibility::Ready | EditEligibility::Busy) {
                    DisplayState::NativeControl
                } else if control.policy == Policy::LgManaged
                    || (control.policy == Policy::NativeCandidate && control.lg_route_verified)
                {
                    DisplayState::LgManaged
                } else {
                    DisplayState::NativeView
                }
            }
        }
    };
    Presentation {
        state,
        edit,
        value_is_current: control.reading == EvidenceState::Current,
        operation_pending: control.busy,
        lg_settings_available: control.lg_route_verified,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    fn candidate() -> Control<'static> {
        Control {
            setting_id: "pointerSpeed",
            policy: Policy::NativeCandidate,
            reading: EvidenceState::Current,
            descriptor: DescriptorState::Active,
            write: Some(WriteCapability {
                setting_id: "pointerSpeed",
                state: EvidenceState::Current,
            }),
            lg_route_verified: true,
            busy: false,
        }
    }
    #[test]
    fn descriptor_and_reading_never_grant_a_native_write() {
        let mut control = candidate();
        control.write = None;
        let view = evaluate(control);
        assert!(!view.can_edit());
        assert_eq!(view.edit, EditEligibility::WriteUnverified);
        assert_eq!(view.state, DisplayState::LgManaged);
        assert!(view.value_is_current && view.lg_settings_available);
        control.write = Some(WriteCapability {
            setting_id: "audioDescriptionVolume",
            state: EvidenceState::Current,
        });
        assert_eq!(evaluate(control).edit, EditEligibility::WriteUnverified);
        control.setting_id = "";
        control.write.as_mut().unwrap().setting_id = "";
        assert!(!evaluate(control).can_edit());
    }
    #[test]
    fn protected_and_delegated_policies_cannot_be_promoted_by_write_evidence() {
        let mut control = candidate();
        for (policy, reason, state) in [
            (
                Policy::Protected,
                EditEligibility::Protected,
                DisplayState::Protected,
            ),
            (
                Policy::LgManaged,
                EditEligibility::LgManaged,
                DisplayState::LgManaged,
            ),
            (
                Policy::ViewOnly,
                EditEligibility::ViewOnly,
                DisplayState::NativeView,
            ),
        ] {
            control.policy = policy;
            let view = evaluate(control);
            assert_eq!((view.edit, view.state), (reason, state));
            assert!(!view.can_edit());
            assert!(view.value_is_current && view.lg_settings_available);
        }
        control.policy = Policy::Protected;
        control.reading = EvidenceState::Stale;
        let view = evaluate(control);
        assert_eq!(view.state, DisplayState::Protected);
        assert!(!view.value_is_current);
    }
    #[test]
    fn reading_transitions_withdraw_editing_without_hiding_the_lg_route() {
        let mut control = candidate();
        assert!(evaluate(control).can_edit());
        for (reading, reason, state) in [
            (
                EvidenceState::WrongContext,
                EditEligibility::ReadingWrongContext,
                DisplayState::ReadingWrongContext,
            ),
            (
                EvidenceState::Stale,
                EditEligibility::ReadingStale,
                DisplayState::ReadingStale,
            ),
            (
                EvidenceState::Unavailable,
                EditEligibility::ReadingUnavailable,
                DisplayState::ReadingUnavailable,
            ),
        ] {
            control.reading = reading;
            let view = evaluate(control);
            assert_eq!((view.edit, view.state), (reason, state));
            assert!(!view.can_edit() && !view.value_is_current);
            assert!(view.lg_settings_available);
        }
        // A new-context reading alone cannot revive the old adapter admission.
        control.reading = EvidenceState::Current;
        control.write.as_mut().unwrap().state = EvidenceState::WrongContext;
        assert_eq!(evaluate(control).edit, EditEligibility::WriteWrongContext);
        control.write.as_mut().unwrap().state = EvidenceState::Current;
        assert!(evaluate(control).can_edit());
    }
    #[test]
    fn fresh_value_does_not_hide_stale_or_unavailable_write_admission() {
        let mut control = candidate();
        for (evidence, reason) in [
            (
                EvidenceState::Unavailable,
                EditEligibility::WriteUnavailable,
            ),
            (EvidenceState::Stale, EditEligibility::WriteStale),
            (
                EvidenceState::WrongContext,
                EditEligibility::WriteWrongContext,
            ),
        ] {
            control.write.as_mut().unwrap().state = evidence;
            let view = evaluate(control);
            assert_eq!(view.edit, reason);
            assert!(!view.can_edit());
            assert!(view.value_is_current);
        }
    }
    #[test]
    fn descriptor_veto_and_pending_operation_withdraw_native_editing() {
        let mut control = candidate();
        control.descriptor = DescriptorState::Inactive;
        let view = evaluate(control);
        assert_eq!(view.state, DisplayState::InactiveInContext);
        assert_eq!(view.edit, EditEligibility::InactiveInContext);
        assert!(view.value_is_current && !view.can_edit());
        control.descriptor = DescriptorState::Unknown;
        assert_eq!(evaluate(control).edit, EditEligibility::DescriptorUnknown);
        control.descriptor = DescriptorState::Active;
        control.busy = true;
        let view = evaluate(control);
        assert_eq!(view.state, DisplayState::NativeControl);
        assert_eq!(view.edit, EditEligibility::Busy);
        assert!(!view.can_edit());
        control.reading = EvidenceState::WrongContext;
        assert_eq!(evaluate(control).edit, EditEligibility::ReadingWrongContext);
    }
    #[test]
    fn native_reading_does_not_invent_a_settings_route() {
        let mut control = candidate();
        control.policy = Policy::ViewOnly;
        control.descriptor = DescriptorState::Unknown;
        control.write = None;
        control.lg_route_verified = false;
        let view = evaluate(control);
        assert_eq!(view.state, DisplayState::NativeView);
        assert!(view.value_is_current);
        assert!(!view.can_edit() && !view.lg_settings_available);
    }

    #[test]
    fn pending_operation_survives_expired_or_withdrawn_admission() {
        let mut control = candidate();
        control.busy = true;
        assert!(evaluate(control).operation_pending);
        for evidence in [
            EvidenceState::Stale,
            EvidenceState::WrongContext,
            EvidenceState::Unavailable,
        ] {
            control.reading = evidence;
            let view = evaluate(control);
            assert!(view.operation_pending && !view.can_edit());
            assert!(!view.value_is_current);
        }
        control.reading = EvidenceState::Current;
        control.write = None;
        control.descriptor = DescriptorState::Unknown;
        let view = evaluate(control);
        assert!(view.operation_pending && !view.can_edit());
        assert_eq!(view.edit, EditEligibility::DescriptorUnknown);
        // Only a worker completion/closeout, not freshness policy, clears pending.
        control.busy = false;
        assert!(!evaluate(control).operation_pending);
    }
}
