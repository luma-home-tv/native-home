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
}
