//! Explicit own application identities; diagnostic storage never shares user favourites.
pub const PREVIEW: &str = "com.ash.nativehome.preview";
pub const PROOF: &str = "com.ash.nativehome.proof";
pub const TARGET: &str = "com.ash.nativehome.targetproof";

pub fn dashboard(value: &str) -> Option<&'static str> {
    match value {
        PREVIEW => Some(PREVIEW),
        PROOF => Some(PROOF),
        _ => None,
    }
}
pub fn preferences_dir(value: &str) -> Option<std::path::PathBuf> {
    dashboard(value).map(|id| std::path::PathBuf::from(format!("/media/developer/.{id}-state")))
}
/// Passive test packages cannot open household apps through accidental input.
pub fn app_launches(value: &str, owned_target_proof: bool) -> bool {
    value == PREVIEW || (value == PROOF && owned_target_proof)
}
/// Fixed LG Home return is separate from opening household applications.
pub fn home_launches(value: &str, explicit_home_proof: bool) -> bool {
    value == PREVIEW || (value == PROOF && explicit_home_proof)
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn proof_storage_is_distinct_and_unexpected_identities_are_refused() {
        assert_ne!(preferences_dir(PREVIEW), preferences_dir(PROOF));
        assert_eq!(
            preferences_dir(PREVIEW).unwrap().to_str(),
            Some("/media/developer/.com.ash.nativehome.preview-state")
        );
        for id in [
            "com.webos.app.home",
            TARGET,
            "../preview",
            "com.ash.nativehome.preview.other",
        ] {
            assert!(dashboard(id).is_none());
            assert!(preferences_dir(id).is_none());
        }
    }
    #[test]
    fn passive_test_identity_blocks_launches_and_explicit_owned_proof_is_scoped() {
        assert!(app_launches(PREVIEW, false));
        assert!(!app_launches(PROOF, false));
        assert!(app_launches(PROOF, true));
        assert!(!app_launches("com.webos.app.home", true));
        assert!(home_launches(PREVIEW, false));
        assert!(!home_launches(PROOF, false));
        assert!(home_launches(PROOF, true));
        assert!(!home_launches(TARGET, true));
    }
}
