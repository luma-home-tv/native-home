//! Actual Shell data/filter/Back flows for empty-state copy; no GL/Window/TV.
use super::*;
use crate::live::{CatalogueReading, Reading};

fn snapshot(count: usize) -> Snapshot {
    let applications = (0..count)
        .map(|i| Application {
            id: format!("fixture.empty.app{i}"),
            title: format!("Available Application {i}"),
            kind: "native".into(),
            visible: true,
            icon: None,
        })
        .collect::<Vec<_>>();
    Snapshot {
        captured_at: "synthetic empty states".into(),
        launch_ids: applications.iter().map(|app| app.id.clone()).collect(),
        applications,
        inputs: Vec::new(),
        context: Context {
            input: "HDMI_1".into(),
            dynamic_range: "sdr".into(),
            picture_mode: "cinema".into(),
        },
        settings: BTreeMap::new(),
    }
}
fn shell(count: usize) -> Shell {
    let mut shell = Shell::new(
        snapshot(count),
        Preferences {
            initialized: true,
            ..Default::default()
        },
    );
    shell.open(Page::Applications);
    shell
}
fn title(shell: &Shell) -> Option<&'static str> {
    shell
        .drawer_empty_copy()
        .map(|(title, _)| title.to_str().unwrap())
}
fn toolbar(shell: &mut Shell, index: usize) {
    assert!(shell.focus_drawer_control(index));
    shell.activate(
        FocusKey {
            entry: entry(Page::Applications),
            elem: TOOLBAR_BASE + index as u32,
        },
        false,
    );
}
fn update(apps: Reading<CatalogueReading>, at: u32) -> Update {
    Update {
        system_keyboard: false,
        observed_ms: at,
        simulated: false,
        catalogue: apps,
        artwork: Reading::Failed("omitted".into()),
        inputs: Reading::Failed("omitted".into()),
        picture: Reading::Failed("omitted".into()),
        sound: Reading::Failed("omitted".into()),
        network: Reading::Failed("omitted".into()),
        controls: BTreeMap::new(),
    }
}

#[test]
fn empty_favourites_offer_existing_all_apps_recovery_without_writes() {
    let _serial = plx_base::testlock::serial();
    let mut shell = shell(2);
    let preferences = shell.preferences().clone();
    assert!(shell.drawer_empty_copy().is_none());
    toolbar(&mut shell, 1);
    assert!(shell.drawer.favourites_only && shell.surface.cards.items.is_empty());
    assert_eq!(title(&shell), Some("No favourites yet"));
    assert!(shell
        .drawer_empty_copy()
        .unwrap()
        .1
        .to_str()
        .unwrap()
        .contains("All apps"));
    assert!(
        shell.focused_item().is_some(),
        "empty filter keeps an existing navigation target"
    );
    toolbar(&mut shell, 0);
    assert_eq!(shell.surface.cards.items.len(), 2);
    assert!(shell.drawer_empty_copy().is_none());
    assert_eq!(shell.preferences(), &preferences);
    assert!(!shell.take_preferences_changed());
    assert!(shell.take_requests().is_empty());
}

#[test]
fn no_results_back_and_filter_switch_keep_query_and_distinguish_favourites() {
    let _serial = plx_base::testlock::serial();
    let mut shell = shell(2);
    shell.system_keyboard_available = true;
    toolbar(&mut shell, 2);
    assert!(shell.drawer.system_keyboard());
    assert!(shell.text_input("no-such-title", false));
    assert_eq!(title(&shell), Some("No matching apps"));
    shell.back();
    assert!(!shell.drawer.editing && !shell.drawer.system_keyboard());
    assert_eq!(shell.focused_item().as_deref(), Some("Search apps"));
    assert_eq!(shell.drawer.query(), "no-such-title");
    toolbar(&mut shell, 1);
    assert_eq!(title(&shell), Some("No matching favourites"));
    toolbar(&mut shell, 0);
    assert_eq!(title(&shell), Some("No matching apps"));
    assert_eq!(
        shell.drawer.query(),
        "no-such-title",
        "All apps changes only the favourite filter"
    );
    assert!(!shell.take_preferences_changed());
    assert!(shell.take_requests().is_empty());
}

#[test]
fn empty_current_list_and_failed_or_stale_readings_are_not_search_misses() {
    let _serial = plx_base::testlock::serial();
    let mut shell = shell(0);
    assert_eq!(title(&shell), Some("No apps to show"));
    shell.apply_live(
        &update(Reading::Failed("read unavailable".into()), 100),
        100,
    );
    assert_eq!(title(&shell), Some("Apps unavailable"));
    shell.apply_live(
        &update(
            Reading::Value(CatalogueReading {
                apps: Vec::new(),
                launch_ids: BTreeSet::new(),
            }),
            200,
        ),
        200,
    );
    assert_eq!(title(&shell), Some("No apps to show"));
    shell.age_live(100_000);
    assert_eq!(title(&shell), Some("Apps unavailable"));
    assert!(shell.focus_drawer_control(0));
    shell.back();
    assert_eq!(shell.page(), Page::Home);
    assert!(!shell.should_exit());
    assert!(shell.drawer_empty_copy().is_none());
    assert!(shell.take_requests().is_empty());
}

#[test]
fn stale_empty_search_does_not_assert_current_absence_or_discard_cached_apps() {
    let _serial = plx_base::testlock::serial();
    let mut shell = shell(2);
    toolbar(&mut shell, 3); // Existing fallback Keypad, not a new text path.
    assert!(shell.text_input("unmatched", false));
    shell.apply_live(
        &update(Reading::Failed("read unavailable".into()), 100),
        100,
    );
    assert_eq!(title(&shell), Some("Apps unavailable"));
    shell.back();
    toolbar(&mut shell, 3);
    assert!(shell.text_edit(&plx_machine::machine::TextEdit::Clear));
    shell.back();
    assert_eq!(
        shell.surface.cards.items.len(),
        2,
        "failed readings keep cached apps for inspection"
    );
    assert!(shell.drawer_empty_copy().is_none());
    assert!(shell.reading_label().contains("unavailable"));
    assert!(shell.take_requests().is_empty());
}
