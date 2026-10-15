//! Native interactive shell or bounded proof, with ordinary-app reads and gated launches.
use native_home::{
    model::Page,
    preferences::Persistence,
    runtime::{poll, Window},
    shell::{Preferences, Shell, Snapshot},
};
use plx_machine::machine::PressFrom;
use plx_ui::screen::Dir;
use std::{
    io::Write,
    os::unix::fs::OpenOptionsExt,
    path::PathBuf,
    time::{Duration, Instant},
};
fn main() -> Result<(), Box<dyn std::error::Error>> {
    let device = cfg!(target_arch = "arm");
    let args = std::env::args().collect::<Vec<_>>();
    let assets = if device {
        plx_base::paths::app_dir().to_path_buf()
    } else {
        PathBuf::from(args.get(1).ok_or("Specify snapshot asset directory")?)
    };
    let interactive = args.iter().any(|arg| arg == "--session")
        || assets.join("interactive-session.json").exists();
    let navigation_proof = assets.join("navigation-proof.json").exists()
        || args.iter().any(|s| s == "--navigation-proof");
    if device
        && navigation_proof
        && (interactive || plx_base::paths::app_id() != native_home::identity::PROOF)
    {
        return Err("Navigation proof requires isolated diagnostic identity".into());
    }
    let text_probe = assets.join("text-input-proof.json").exists();
    if text_probe
        && (!device
            || interactive
            || plx_base::paths::app_id() != native_home::identity::PROOF
            || navigation_proof
            || [
                "description-ui-proof.json",
                "range-layout-proof.json",
                "description-level-proof.json",
                "pointer-proof.json",
                "pointer-ui-proof.json",
                "launch-proof.json",
                "management-proof.json",
                "controls-proof.json",
                "drawer-proof.json",
                "preferences-hold-probe.json",
            ]
            .iter()
            .any(|name| assets.join(name).exists()))
    {
        return Err("Text input proof requires isolated non-interactive TV identity".into());
    }
    let description_ui_proof = assets.join("description-ui-proof.json").exists();
    if description_ui_proof
        && (!device
            || interactive
            || plx_base::paths::app_id() != native_home::identity::PROOF
            || navigation_proof
            || [
                "range-layout-proof.json",
                "description-level-proof.json",
                "pointer-proof.json",
                "pointer-ui-proof.json",
                "launch-proof.json",
                "management-proof.json",
                "controls-proof.json",
                "drawer-proof.json",
                "preferences-hold-probe.json",
            ]
            .iter()
            .any(|name| assets.join(name).exists()))
    {
        return Err("Numeric UI proof requires an isolated non-interactive identity and no other action trial".into());
    }
    let range_layout_proof = assets.join("range-layout-proof.json").exists()
        || args.iter().any(|a| a == "--range-layout-proof");
    if range_layout_proof
        && (interactive
            || (device && plx_base::paths::app_id() != native_home::identity::PROOF)
            || navigation_proof
            || [
                "launch-proof.json",
                "pointer-proof.json",
                "pointer-ui-proof.json",
                "description-level-proof.json",
                "management-proof.json",
                "controls-proof.json",
                "drawer-proof.json",
                "preferences-hold-probe.json",
            ]
            .iter()
            .any(|p| assets.join(p).exists()))
    {
        return Err("Range layout proof requires isolated non-interactive identity and no other action trial".into());
    }
    let pointer_ui_proof = assets.join("pointer-ui-proof.json").exists();
    if pointer_ui_proof
        && (!device || interactive || plx_base::paths::app_id() != native_home::identity::PROOF)
    {
        return Err("Pointer UI proof requires isolated diagnostic identity".into());
    }
    let pointer_proof = assets.join("pointer-proof.json").exists();
    if pointer_proof
        && (!device || interactive || plx_base::paths::app_id() != native_home::identity::PROOF)
    {
        return Err("Pointer write proof requires isolated non-interactive TV identity".into());
    }
    let description_proof = assets.join("description-level-proof.json").exists();
    if description_proof
        && (!device
            || interactive
            || pointer_proof
            || pointer_ui_proof
            || [
                "launch-proof.json",
                "preferences-hold-probe.json",
                "management-proof.json",
                "controls-proof.json",
                "drawer-proof.json",
                "navigation-proof.json",
            ]
            .iter()
            .any(|name| assets.join(name).exists())
            || plx_base::paths::app_id() != native_home::identity::PROOF)
    {
        return Err("Numeric proof requires isolated non-interactive TV identity".into());
    }
    let session_capture = interactive && assets.join("session-proof.json").exists();
    let performance_proof = interactive && assets.join("performance-proof.json").exists();
    if performance_proof && device && plx_base::paths::app_id() != native_home::identity::PROOF {
        return Err("Performance diagnostics require the isolated identity".into());
    }
    let mut performance = performance_proof.then(native_home::performance::Monitor::default);
    if interactive
        && [
            "text-input-proof.json",
            "drawer-proof.json",
            "management-proof.json",
            "controls-proof.json",
            "launch-proof.json",
            "preferences-hold-probe.json",
            "native-read-probe.json",
            "description-ui-proof.json",
            "range-layout-proof.json",
            "description-level-proof.json",
            "pointer-proof.json",
            "pointer-ui-proof.json",
            "navigation-proof.json",
        ]
        .iter()
        .any(|name| assets.join(name).exists())
    {
        return Err("Interactive session cannot run scripted proof actions".into());
    }
    let output = if device {
        plx_base::paths::in_runtime_dir(&format!("shell-{}", std::process::id()))
    } else {
        PathBuf::from(args.get(2).ok_or("Specify new capture directory")?)
    };
    let capture_only = !device && !interactive && !args.iter().any(|a| a == "--interactive");
    let window = Window::new(!capture_only)?;
    let live_origin = Instant::now();
    let mut clock = native_home::clock::Clock::default();
    #[cfg(target_arch = "arm")]
    let clock_worker = native_home::clock::Worker::native(live_origin)?;
    let clock_fixture = assets.join("clock-fixture.json");
    if !device && clock_fixture.exists() {
        let bytes = std::fs::read(clock_fixture)?;
        if bytes.len() > 4096 {
            return Err("Oversized clock fixture".into());
        }
        let raw: serde_json::Value = serde_json::from_slice(&bytes)?;
        clock.accept(native_home::clock::parse(&raw, live_origin.elapsed(), true));
    }
    {
        use std::os::unix::fs::DirBuilderExt;
        let mut builder = std::fs::DirBuilder::new();
        builder.mode(0o700).create(&output)?;
    }
    let native_probe = if device && assets.join("native-read-probe.json").exists() {
        Some(
            std::thread::Builder::new()
                .name("home-read-probe".into())
                .spawn(native_home::native_probe::collect)?,
        )
    } else {
        None
    };
    #[allow(unused_mut)]
    let mut pointer_probe: Option<std::thread::JoinHandle<serde_json::Value>> = None;
    #[cfg(target_arch = "arm")]
    if pointer_proof {
        let folder = output.clone();
        pointer_probe = Some(
            std::thread::Builder::new()
                .name("pointer-write-proof".into())
                .spawn(move || pointer_write_proof(&folder))?,
        );
    }
    #[allow(unused_mut)]
    let mut description_probe: Option<std::thread::JoinHandle<serde_json::Value>> = None;
    #[cfg(target_arch = "arm")]
    if description_proof {
        let folder = output.clone();
        description_probe = Some(
            std::thread::Builder::new()
                .name("numeric-write-proof".into())
                .spawn(move || description_level_proof(&folder))?,
        );
    }
    let bytes = std::fs::read(assets.join("snapshot.json"))?;
    if bytes.len() > 1024 * 1024 {
        return Err("Snapshot too large".into());
    }
    let snapshot: Snapshot = serde_json::from_slice(&bytes)?;
    #[allow(unused_mut)]
    let mut live_worker = None;
    #[cfg(target_arch = "arm")]
    if device {
        live_worker = Some(native_home::live::Worker::native(
            plx_base::paths::app_id().to_owned(),
            live_origin,
        )?);
    }
    let fixture_path = assets.join("live-fixture.json");
    if !device && fixture_path.exists() {
        let bytes = std::fs::read(fixture_path)?;
        if bytes.len() > 2 * 1024 * 1024 {
            return Err("Live fixture too large".into());
        }
        let values: Vec<serde_json::Value> = serde_json::from_slice(&bytes)?;
        live_worker = Some(native_home::live::Worker::fixture(
            values,
            "com.ash.nativehome.preview".into(),
            live_origin,
            Duration::from_millis(500),
        )?);
    }
    let mut live_history = Vec::new();
    let proof_path = assets.join("launch-proof.json");
    let mut launch_proof = None;
    if proof_path.exists() {
        let bytes = std::fs::read(proof_path)?;
        if bytes.len() > 1024 {
            return Err("Oversized launch proof".into());
        }
        launch_proof = Some(serde_json::from_slice::<native_home::actions::Request>(
            &bytes,
        )?);
    }
    #[allow(unused_mut)]
    let mut action_worker: Option<native_home::actions::Worker> = None;
    #[allow(unused_mut)]
    let mut app_launches_enabled = false;
    #[allow(unused_mut)]
    let mut home_launches_enabled = false;
    #[cfg(target_arch = "arm")]
    if device {
        app_launches_enabled = native_home::identity::app_launches(
            plx_base::paths::app_id(),
            matches!(&launch_proof, Some(native_home::actions::Request::Application { id }) if id == native_home::identity::TARGET),
        );
        home_launches_enabled = native_home::identity::home_launches(
            plx_base::paths::app_id(),
            matches!(launch_proof, Some(native_home::actions::Request::LgHome {})),
        );
        action_worker = Some(native_home::actions::Worker::native(
            plx_base::paths::app_id().into(),
            native_home::actions::Capabilities {
                apps: app_launches_enabled,
                home: home_launches_enabled,
                inputs: plx_base::paths::app_id() == native_home::identity::PREVIEW
                    || matches!(
                        launch_proof,
                        Some(native_home::actions::Request::Input { .. })
                    ),
                settings: plx_base::paths::app_id() == native_home::identity::PREVIEW
                    || launch_proof
                        .as_ref()
                        .is_some_and(|request| request.is_settings()),
            },
        )?);
    }
    #[allow(unused_mut)]
    let mut pointer_worker: Option<native_home::pointer_speed::Worker> = None;
    #[cfg(target_arch = "arm")]
    if device && (plx_base::paths::app_id() == native_home::identity::PREVIEW || pointer_ui_proof) {
        pointer_worker = Some(native_home::pointer_speed::Worker::native(
            plx_base::paths::app_id().into(),
        )?);
    }
    let mut pointer_actions = Vec::new();
    #[allow(unused_mut)]
    let mut description_worker: Option<native_home::description_level::Worker> = None;
    #[cfg(target_arch = "arm")]
    if device
        && (plx_base::paths::app_id() == native_home::identity::PREVIEW || description_ui_proof)
    {
        description_worker = Some(native_home::description_level::Worker::native(
            plx_base::paths::app_id().into(),
        )?);
    }
    let mut description_actions = Vec::new();
    let mut description_attempts = 0u64;

    let mut action_receipts = Vec::new();
    let state_path = if device {
        native_home::identity::preferences_dir(plx_base::paths::app_id())
            .ok_or("Unexpected preference identity")?
    } else {
        PathBuf::from(
            std::env::var_os("PLXNATIVE_RUNTIME_DIR")
                .ok_or("An isolated host runtime is required")?,
        )
        .join("home-preferences")
    };
    let (mut persistence, loaded_preferences, mut storage_error) =
        match Persistence::start(&state_path) {
            Ok((writer, loaded)) => (Some(writer), loaded, None),
            Err(error) => (None, None, Some(error.to_string())),
        };
    let mut committed_preferences = None;
    let mut shell = Shell::new(snapshot, loaded_preferences.clone().unwrap_or_default());
    queue_preferences(&mut shell, &mut persistence, &mut storage_error);
    let preferences_at_boot = shell.preferences().clone();
    shell.enable_app_launches(app_launches_enabled);
    shell.enable_home_launches(home_launches_enabled);
    shell.load_icons(&assets);
    let mut now = 0;
    let mut capture_diagnostics = Vec::new();
    let mut drawer_proof = None;
    let mut management_proof = None;
    let mut controls_proof = None;
    let mut navigation = Vec::new();
    let mut navigation_page = None;
    let mut scripted_requests = Vec::new();
    let mut probe_identity = None;
    let mut probe_changed = false;
    if !interactive {
        for (page, name) in [
            (Page::Home, "home"),
            (Page::Applications, "applications"),
            (Page::Inputs, "inputs"),
            (Page::Controls, "controls"),
            (Page::Picture, "picture"),
            (Page::Sound, "sound"),
            (Page::Network, "network"),
            (Page::Accessibility, "accessibility"),
            (Page::General, "general"),
            (Page::PanelCare, "panel-care"),
        ] {
            shell.open(page);
            for _ in 0..45 {
                now += 17;
                poll_live(&live_worker, &mut shell, live_origin, &mut live_history);
                pump_description(
                    &description_worker,
                    &mut shell,
                    live_origin,
                    &mut description_actions,
                    &mut description_attempts,
                );
                if let Some(enabled) = action_worker.as_ref().and_then(|w| w.input_capability()) {
                    shell.enable_input_launches(enabled);
                }
                poll_pointer(
                    &pointer_worker,
                    &mut shell,
                    live_origin,
                    &mut pointer_actions,
                );
                #[cfg(target_arch = "arm")]
                if let Some(reply) = clock_worker.poll() {
                    clock.accept(reply);
                }
                shell.set_clock(clock.view(live_origin.elapsed()));
                shell.tick(now, 1.0 / 60.0);
                shell.draw();
                window.swap();
            }
            let before = window.graphics_state();
            shell.draw();
            let after = window.graphics_state();
            let layout = shell.presentation_probe();
            capture_diagnostics.push(serde_json::json!({"page":name,"before_draw":before,"after_draw":after,"layout":layout}));
            if before.error != 0 || after.error != 0 {
                return Err("GL state probe found an error".into());
            }
            window.capture(&output.join(format!("{name}.ppm")))?;
        }
        if assets.join("native-read-probe.json").exists() {
            shell.open(Page::Picture);
            if shell.focus_lg_settings() {
                drawer_capture(&mut shell, &window, &output, "picture-more", &mut now)?;
            }
        }
        if args.iter().any(|arg| arg == "--drawer-proof")
            || assets.join("drawer-proof.json").exists()
        {
            shell.open(Page::Applications);
            let all = shell.application_count();
            if !shell.focus_drawer_control(1) {
                return Err("No favourites control".into());
            }
            drawer_tap(&mut shell, &mut now);
            let favourites = shell.application_count();
            drawer_capture(&mut shell, &window, &output, "drawer-favourites", &mut now)?;
            shell.focus_drawer_control(0);
            drawer_tap(&mut shell, &mut now);
            shell.focus_drawer_control(2);
            drawer_tap(&mut shell, &mut now);
            for label in ["T", "V"] {
                let index = native_home::drawer::KEYS
                    .iter()
                    .position(|key| *key == label)
                    .unwrap();
                if !shell.focus_search_key(index) {
                    return Err("No keyboard key".into());
                }
                drawer_tap(&mut shell, &mut now);
            }
            let query = shell.drawer_state().query().to_owned();
            let results = shell.application_count();
            drawer_capture(&mut shell, &window, &output, "drawer-keyboard", &mut now)?;
            shell.focus_search_key(39);
            drawer_tap(&mut shell, &mut now);
            drawer_capture(&mut shell, &window, &output, "drawer-results", &mut now)?;
            let keyboard_closed = !shell.drawer_state().editing;
            let no_requests = shell.take_requests().is_empty();
            // Restore the full drawer before the independent navigation/application proofs.
            shell.focus_drawer_control(2);
            drawer_tap(&mut shell, &mut now);
            shell.focus_search_key(38);
            drawer_tap(&mut shell, &mut now);
            shell.focus_search_key(39);
            drawer_tap(&mut shell, &mut now);
            shell.focus_drawer_control(0);
            drawer_tap(&mut shell, &mut now);
            if let Some(first) = shell.first_application() {
                shell.focus_application(&first);
            }
            drawer_proof = Some(
                serde_json::json!({"all":all,"favourites":favourites,"query":query,"results":results,"keyboard_closed":keyboard_closed,"no_tv_requests":no_requests,"restored_all":shell.application_count()==all,"input":"scripted shared focus and OK"}),
            );
        }
        if args.iter().any(|arg| arg == "--management-proof")
            || assets.join("management-proof.json").exists()
        {
            shell.open(Page::Home);
            let target = shell
                .first_application()
                .ok_or("Management proof needs a favourite")?;
            let before = shell.preferences().clone();
            shell.open(Page::Applications);
            if !shell.focus_application(&target) {
                return Err("Management target disappeared".into());
            }
            shell.press(true, PressFrom::Key);
            for _ in 0..50 {
                now += 17;
                shell.repeat();
                shell.tick(now, 1.0 / 60.0);
            }
            shell.press(false, PressFrom::Key);
            for _ in 0..30 {
                now += 17;
                shell.tick(now, 1.0 / 60.0);
            }
            let hold_opened = shell.menu_application() == Some(target.as_str());
            let hold_no_write = shell.preferences() == &before;
            drawer_capture(&mut shell, &window, &output, "app-menu", &mut now)?;
            shell.direction(Dir::Left);
            let modal = shell.menu_application().is_some()
                && shell.focused_item().as_deref() == Some("app-option-1");
            if !shell.focus_menu_action(native_home::shell::MenuAction::Later) {
                return Err("No ordering menu".into());
            }
            drawer_tap(&mut shell, &mut now);
            let moved = shell.preferences().favourite_order != before.favourite_order;
            // Roundtrip pin membership using the same modal, without launching a TV app.
            shell.focus_menu_action(native_home::shell::MenuAction::ToggleFavourite);
            drawer_tap(&mut shell, &mut now);
            let unpinned = !shell.preferences().favourites.contains(&target);
            drawer_tap(&mut shell, &mut now);
            let repinned = shell.preferences().favourites == before.favourites;
            shell.back();
            let restored = shell.focused_item().as_deref() == Some(target.as_str());
            shell.open(Page::Home);
            let no_requests = shell.take_requests().is_empty();
            drawer_capture(&mut shell, &window, &output, "ordered-home", &mut now)?;
            queue_preferences(&mut shell, &mut persistence, &mut storage_error);
            management_proof = Some(
                serde_json::json!({"hold_opens_menu":hold_opened,"hold_release_does_not_write":hold_no_write,"modal_focus":modal,"order_changed":moved,"unpin_repinned":unpinned && repinned,"back_restores_identity":restored,"no_tv_requests":no_requests,"input":"scripted shared focus/hold/OK"}),
            );
            if !(hold_opened
                && hold_no_write
                && modal
                && moved
                && unpinned
                && repinned
                && restored
                && no_requests)
            {
                return Err("Management proof failed".into());
            }
        }
        if args.iter().any(|arg| arg == "--controls-proof")
            || assets.join("controls-proof.json").exists()
        {
            let preferences = shell.preferences().clone();
            shell.open(Page::Controls);
            let mut categories = Vec::new();
            for (index, name) in [
                "picture",
                "sound",
                "network",
                "accessibility",
                "general",
                "panel-care",
            ]
            .iter()
            .enumerate()
            {
                if !shell.focus_control_category(index) {
                    return Err("Missing Controls category".into());
                }
                drawer_capture(
                    &mut shell,
                    &window,
                    &output,
                    &format!("category-{name}"),
                    &mut now,
                )?;
                categories.push(shell.presentation_probe());
            }
            shell.focus_control_category(2);
            shell.draw();
            shell.focus_control_category(0);
            shell.pointer(1500.0, 550.0, true);
            let stale_pointer_ignored = shell.focused_item().as_deref() == Some("category-Picture");
            shell.focus_control_category(2);
            shell.direction(Dir::Right);
            let enters_detail = shell
                .focused_item()
                .is_some_and(|id| id.starts_with("control-row-"));
            for _ in 0..12 {
                shell.direction(Dir::Down);
                now += 17;
                shell.tick(now, 1.0 / 60.0);
            }
            drawer_capture(
                &mut shell,
                &window,
                &output,
                "controls-network-scrolled",
                &mut now,
            )?;
            let scrolling = shell.presentation_probe();
            shell.back();
            let back_restores_category =
                shell.focused_item().as_deref() == Some("category-Network");
            shell.draw();
            shell.pointer(621.0, 439.0, true);
            let pointer_selects_sound = shell.control_category() == Some(Page::Sound);
            drawer_capture(
                &mut shell,
                &window,
                &output,
                "controls-pointer-sound",
                &mut now,
            )?;
            shell.open(Page::Picture);
            if !shell.focus_lg_settings() {
                return Err("Missing existing Picture route".into());
            }
            shell.press(true, PressFrom::Key);
            shell.direction(Dir::Left);
            shell.direction(Dir::Down);
            shell.press(false, PressFrom::Key);
            for _ in 0..30 {
                now += 17;
                shell.tick(now, 1.0 / 60.0);
            }
            let route_cancelled =
                shell.take_requests().is_empty() && shell.control_category() == Some(Page::Sound);
            shell.back();
            let back_home = shell.page() == Page::Home;
            let preferences_unchanged = shell.preferences() == &preferences;
            controls_proof = Some(
                serde_json::json!({"categories":&categories,"right_enters_detail":enters_detail,"stale_pointer_ignored":stale_pointer_ignored,"network_scroll":&scrolling,"back_restores_category":back_restores_category,"pointer_selects_sound":pointer_selects_sound,"armed_picture_route_cancelled_on_category_change":route_cancelled,"back_returns_home":back_home,"preferences_unchanged":preferences_unchanged,"input":"scripted shared arrows/Back and hit-map pointer"}),
            );
            if !(stale_pointer_ignored
                && enters_detail
                && back_restores_category
                && pointer_selects_sound
                && route_cancelled
                && back_home
                && preferences_unchanged)
            {
                return Err("Controls proof failed".into());
            }
        }
        shell.open(Page::Applications);
        for dir in [
            Dir::Right,
            Dir::Down,
            Dir::Up,
            Dir::Left,
            Dir::Left,
            Dir::Down,
        ] {
            shell.direction(dir);
            now += 17;
            shell.tick(now, 1.0 / 60.0);
            navigation.push(shell.focused_item());
        }
        shell.press(true, PressFrom::Key);
        now += 50;
        shell.tick(now, 0.05);
        shell.press(false, PressFrom::Key);
        for _ in 0..30 {
            now += 17;
            shell.tick(now, 1.0 / 60.0);
        }
        navigation_page = Some(format!("{:?}", shell.page()));
        // This is a deterministic component probe, explicitly separate from physical SDL input.
        scripted_requests = shell
            .take_requests()
            .into_iter()
            .map(|a| format!("{a:?}"))
            .collect::<Vec<_>>();
        let probe_path = assets.join("preferences-hold-probe.json");
        if probe_path.exists() {
            #[derive(serde::Deserialize)]
            #[serde(deny_unknown_fields)]
            struct Probe {
                application_id: String,
            }
            let bytes = std::fs::read(probe_path)?;
            if bytes.len() > 1024 {
                return Err("Oversized preference probe".into());
            }
            let probe: Probe = serde_json::from_slice(&bytes)?;
            shell.open(Page::Applications);
            if !shell.focus_application(&probe.application_id) {
                return Err("Probe app is not in the drawer".into());
            }
            probe_identity = shell.focused_item();
            let before = shell
                .preferences()
                .favourites
                .contains(&probe.application_id);
            shell.press(true, PressFrom::Key);
            for _ in 0..45 {
                now += 17;
                shell.repeat();
                shell.tick(now, 1.0 / 60.0);
            }
            shell.press(false, PressFrom::Key);
            for _ in 0..30 {
                now += 17;
                shell.tick(now, 1.0 / 60.0);
            }
            if shell.menu_application().is_none() {
                return Err("Hold did not open app menu".into());
            }
            shell.focus_menu_action(native_home::shell::MenuAction::ToggleFavourite);
            drawer_tap(&mut shell, &mut now);
            shell.back();
            probe_changed = before
                != shell
                    .preferences()
                    .favourites
                    .contains(&probe.application_id);
            queue_preferences(&mut shell, &mut persistence, &mut storage_error);
        }
    }
    shell.open(Page::Home);
    let mut requests = Vec::new();
    #[allow(unused_mut)]
    let mut pointer_ui_receipt: Option<serde_json::Value> = None;
    #[cfg(target_arch = "arm")]
    if pointer_ui_proof {
        pointer_ui_receipt = Some(pointer_ui_trial(
            &mut shell,
            &window,
            &output,
            &pointer_worker,
            &live_worker,
            live_origin,
            &mut live_history,
            &mut pointer_actions,
            &mut now,
        )?);
        shell.open(Page::Home);
    }
    let navigation_receipt = if navigation_proof {
        Some(navigation_trial(&mut shell, &window, &output, &mut now)?)
    } else {
        None
    };
    let range_receipt = if range_layout_proof {
        Some(range_layout_trial(&mut shell, &window, &output, &mut now)?)
    } else {
        None
    };
    #[allow(unused_mut)]
    let mut description_ui_receipt = None::<serde_json::Value>;
    #[cfg(target_arch = "arm")]
    if description_ui_proof {
        description_ui_receipt = Some(description_ui_trial(
            &mut shell,
            &window,
            &output,
            &description_worker,
            live_origin,
            &mut description_actions,
            &mut description_attempts,
            &mut now,
        )?);
        shell.open(Page::Home);
    }
    #[allow(unused_mut)]
    let mut text_receipt = None::<serde_json::Value>;
    #[cfg(target_arch = "arm")]
    if text_probe {
        text_receipt = Some(text_input_trial(
            &mut shell,
            &window,
            &output,
            &live_worker,
            live_origin,
            &mut live_history,
            &mut now,
        )?);
        shell.open(Page::Home);
    }
    let mut proof_request = None;
    if let Some(request) = launch_proof {
        match &request {
            native_home::actions::Request::Application { id }
                if id == "com.ash.nativehome.targetproof" =>
            {
                shell.open(Page::Applications);
                if !shell.focus_application(id) {
                    return Err("Launch target is not in live drawer".into());
                }
            }
            native_home::actions::Request::LgHome {} => {
                if !shell.focus_lg_home() {
                    return Err("LG Home return unavailable".into());
                }
            }
            native_home::actions::Request::LgSettings {}
            | native_home::actions::Request::LgSettingsSection { .. } => {
                shell.open(request.settings_section().unwrap().page());
                if !shell.focus_lg_settings() {
                    return Err("LG Settings route unavailable".into());
                }
            }
            native_home::actions::Request::Input { id }
                if matches!(id.as_str(), "HDMI_1" | "HDMI_2" | "HDMI_3" | "HDMI_4") =>
            {
                shell.open(Page::Inputs);
                if !shell.focus_input(id) {
                    return Err("Requested HDMI source unavailable".into());
                }
            }
            _ => return Err("Unapproved isolated proof route".into()),
        }
        shell.press(true, PressFrom::Key);
        now += 50;
        shell.tick(now, 0.05);
        shell.press(false, PressFrom::Key);
        for _ in 0..30 {
            now += 17;
            shell.tick(now, 1.0 / 60.0);
        }
        proof_request = Some(request);
    }
    let mut settings_handoff = native_home::session::SettingsHandoff::default();
    dispatch_requests(
        &mut shell,
        &action_worker,
        &pointer_worker,
        proof_request.as_ref(),
        &mut requests,
        &mut settings_handoff,
        live_origin.elapsed(),
    );
    let start = Instant::now();
    let mut last = start;
    let mut events = 0_u32;
    let mut event_log = Vec::new();
    let mut hidden_at: Option<Instant> = None;
    let mut session = native_home::session::Session::new(interactive);
    let mut idle_blanked = false;
    let mut idle_presentations = 0_u32;
    let mut ready_written = false;
    let mut last_settings_observation = Duration::ZERO;
    let mut settings_returns = 0_u32;
    let mut capture_settings_return = false;

    let mut last_performance_snapshot = Duration::ZERO;
    let mut reason = "capture_complete";
    if !capture_only {
        plx_machine::idle::set_enabled(true);
        plx_machine::idle::wake();
        reason = "timeout";
        while !session.expired(start.elapsed()) && !shell.should_exit() {
            if performance_proof && start.elapsed() >= Duration::from_secs(240) {
                reason = "performance_proof_limit";
                break;
            }
            let current = Instant::now();
            let dt = (current - last).as_secs_f32().clamp(0.001, 0.05);
            last = current;
            plx_machine::idle::frame_begin(dt);
            now = now.wrapping_add((dt * 1000.0) as u32);
            poll_live(&live_worker, &mut shell, live_origin, &mut live_history);
            pump_description(
                &description_worker,
                &mut shell,
                live_origin,
                &mut description_actions,
                &mut description_attempts,
            );
            if let Some(enabled) = action_worker.as_ref().and_then(|w| w.input_capability()) {
                shell.enable_input_launches(enabled);
            }
            poll_pointer(
                &pointer_worker,
                &mut shell,
                live_origin,
                &mut pointer_actions,
            );
            #[cfg(target_arch = "arm")]
            if let Some(reply) = clock_worker.poll() {
                clock.accept(reply);
            }
            shell.set_clock(clock.view(live_origin.elapsed()));
            shell.tick(now, dt);
            while let Some(e) = poll() {
                if e.kind == 2 {
                    if let Some(m) = &mut performance {
                        m.input(e.key, start.elapsed());
                    }
                }
                if matches!(e.kind, 2..=6 | 9 | 10 | 11) {
                    plx_machine::idle::wake();
                }
                events = events.saturating_add(1_u32);
                // Only app-local input wakes this canvas; Home/power/volume stay LG-owned.
                // Consume the first wake input and cancel any gesture armed before blanking.
                if hidden_at.is_none()
                    && matches!(e.kind, 2..=6 | 10 | 11)
                    && !session.input(start.elapsed())
                {
                    shell.cancel_press();
                    if !matches!(e.kind, 10 | 11) {
                        continue;
                    }
                }
                match e.kind {
                    1 => {
                        reason = "window_closed";
                        shell.cancel_press();
                    }
                    7 | 8 => {
                        shell.cancel_press();
                        hidden_at.get_or_insert(current);
                    }
                    9 => {
                        if hidden_at.is_some() && settings_handoff.pending() {
                            settings_returns = settings_returns.saturating_add(1);
                            capture_settings_return = true;
                            settings_handoff.focus_returned();
                            shell.storage_notice("Returned from LG Settings");
                        }
                        hidden_at = None;
                    }
                    2 if hidden_at.is_none() || (e.key == 6 && !settings_handoff.pending()) => {
                        match e.key {
                            1 => shell.direction(Dir::Up),
                            2 => shell.direction(Dir::Down),
                            3 => shell.direction(Dir::Left),
                            4 => shell.direction(Dir::Right),
                            5 => {
                                if e.repeat != 0 {
                                    shell.repeat();
                                } else {
                                    shell.press(true, PressFrom::Key);
                                }
                            }
                            6 if e.repeat == 0 => shell.back(),
                            7 => {
                                shell.text_edit(&plx_machine::machine::TextEdit::Backspace);
                            }
                            8 => {
                                shell.text_delete_forward();
                            }
                            _ => {}
                        }
                    }
                    10 if hidden_at.is_none() => {
                        if let Some(text) = e.text() {
                            shell.text_input(text, false);
                        }
                    }
                    11 if hidden_at.is_none() => {
                        if let Some(text) = e.text() {
                            shell.text_preedit(text);
                        }
                    }
                    3 if e.key == 5 => shell.press(false, PressFrom::Key),
                    4 | 5 if hidden_at.is_none() => {
                        shell.pointer(e.x as f32, e.y as f32, e.kind == 5)
                    }
                    6 if hidden_at.is_none() => {
                        for _ in 0..e.wheel.unsigned_abs().min(3) {
                            shell.direction(if e.wheel > 0 { Dir::Up } else { Dir::Down });
                        }
                    }
                    _ => {}
                }
                if event_log.len() < 32 {
                    event_log.push(e);
                }
            }
            if reason == "window_closed" {
                break;
            }
            // Hidden windows never draw. A fresh, verified Settings overlay can retain only
            // our own underlying card; other departures retain the normal brief Back drain.
            queue_preferences(&mut shell, &mut persistence, &mut storage_error);
            if let Some(writer) = &mut persistence {
                accept_replies(
                    writer.poll(),
                    &mut committed_preferences,
                    &mut storage_error,
                );
            }
            if storage_error.is_some() {
                shell.storage_notice("Favourites could not be saved • previous data kept");
            }
            if let Some(worker) = &action_worker {
                for reply in worker.poll() {
                    if reply.request.is_settings() {
                        settings_handoff.reply(live_origin.elapsed(), reply.verified);
                    }
                    if !reply.verified {
                        shell.storage_notice(match reply.request {
                            native_home::actions::Request::LgSettings {}
                            | native_home::actions::Request::LgSettingsSection { .. } => {
                                "LG Settings unavailable • open it from LG Home"
                            }
                            native_home::actions::Request::LgHome {} => {
                                "LG Home return could not be confirmed"
                            }
                            native_home::actions::Request::Application { .. } => {
                                "Application launch could not be confirmed"
                            }
                            native_home::actions::Request::Input { .. } => {
                                "Input switch could not be confirmed"
                            }
                        });
                    }
                    action_receipts.push(reply);
                }
                for observation in worker.poll_windows() {
                    settings_handoff.window(
                        observation.generation,
                        live_origin.elapsed(),
                        observation.covering_own_card.unwrap_or(false),
                    );
                }
                if let Some(generation) = settings_handoff.observation_generation() {
                    if live_origin
                        .elapsed()
                        .saturating_sub(last_settings_observation)
                        >= Duration::from_millis(500)
                    {
                        worker.observe_settings(generation);
                        last_settings_observation = live_origin.elapsed();
                    }
                }
                if action_receipts.len() > 32 {
                    action_receipts.drain(..action_receipts.len() - 32);
                }
            }
            dispatch_requests(
                &mut shell,
                &action_worker,
                &pointer_worker,
                proof_request.as_ref(),
                &mut requests,
                &mut settings_handoff,
                live_origin.elapsed(),
            );
            if hidden_at.is_some_and(|t| t.elapsed() >= Duration::from_millis(500))
                && !settings_handoff.keep_hidden(live_origin.elapsed())
            {
                reason = "window_hidden";
                break;
            }
            if hidden_at.is_none() {
                if session.idle(start.elapsed()) {
                    if !idle_blanked {
                        shell.cancel_press();
                        plx_gfx::gfx::clip_clear();
                        plx_gfx::gfx::frame_clear(0.0, 0.0, 0.0);
                        if session_capture && idle_presentations == 0 {
                            window.capture(&output.join("session-idle.ppm"))?;
                        }
                        window.swap();
                        if let Some(m) = &mut performance {
                            m.idle_blanks += 1;
                            m.suspended();
                        }
                        idle_presentations = idle_presentations.saturating_add(1);
                        idle_blanked = true;
                    }
                } else if plx_machine::idle::should_present(now) {
                    let draw_start = Instant::now();
                    shell.draw();
                    let draw_elapsed = draw_start.elapsed();
                    if capture_settings_return
                        && matches!(
                            proof_request,
                            Some(
                                native_home::actions::Request::LgSettings {}
                                    | native_home::actions::Request::LgSettingsSection { .. }
                            )
                        )
                    {
                        window.capture(&output.join("settings-return.ppm"))?;
                        capture_settings_return = false;
                    }
                    if session_capture && !ready_written {
                        window.capture(&output.join("session-home.ppm"))?;
                    }
                    if session_capture && idle_blanked && session.wakes == 1 {
                        window.capture(&output.join("session-awake.ppm"))?;
                    }
                    let swap_start = Instant::now();
                    window.swap();
                    if let Some(m) = &mut performance {
                        m.presented(start.elapsed(), draw_elapsed, swap_start.elapsed());
                    }
                    plx_machine::idle::note_present(now);
                    idle_blanked = false;
                } else if let Some(m) = &mut performance {
                    m.suspended();
                }
                if interactive && !ready_written {
                    let ready = serde_json::json!({"interactive":true,"uid":unsafe{libc::geteuid()},"renderer":window.renderer,"scripted_actions":false,"capture_proof":session_capture,"idle_after_seconds":native_home::session::IDLE_AFTER.as_secs()});
                    let file = std::fs::OpenOptions::new()
                        .create_new(true)
                        .write(true)
                        .mode(0o600)
                        .custom_flags(libc::O_NOFOLLOW | libc::O_CLOEXEC)
                        .open(output.join("session-ready.json"))?;
                    serde_json::to_writer_pretty(file, &ready)?;
                    ready_written = true;
                }
            }
            if hidden_at.is_some() {
                if let Some(m) = &mut performance {
                    m.suspended();
                }
            }
            if let Some(monitor) = performance.as_ref().filter(|_| {
                start.elapsed().saturating_sub(last_performance_snapshot) >= Duration::from_secs(1)
            }) {
                last_performance_snapshot = start.elapsed();
                let sample = serde_json::json!({"elapsed_seconds":start.elapsed().as_secs_f64(),"page":format!("{:?}",shell.page()),"focus":shell.focused_item(),"category":shell.control_category(),"idle":idle_blanked,"metrics":monitor.report()});
                let path = output.join("performance-current.json");
                let temporary = output.join("performance-current-next.json");
                let file = std::fs::OpenOptions::new()
                    .create_new(true)
                    .write(true)
                    .mode(0o600)
                    .custom_flags(libc::O_NOFOLLOW | libc::O_CLOEXEC)
                    .open(&temporary)?;
                serde_json::to_writer(file, &sample)?;
                std::fs::rename(temporary, path)?;
            }
            // Bounded 60 Hz host cap also handles compositors whose swap does not block.
            let elapsed = current.elapsed();
            let period = if idle_blanked || hidden_at.is_some() {
                Duration::from_millis(50)
            } else {
                Duration::from_micros(16_667)
            };
            if elapsed < period {
                std::thread::sleep(period - elapsed);
            }
        }
        if shell.should_exit() {
            reason = "back_from_home";
        }
    }
    if let Some(worker) = action_worker {
        action_receipts.extend(worker.finish());
    }
    if let Some(worker) = pointer_worker {
        pointer_actions.extend(worker.finish());
    }
    if let Some(worker) = description_worker {
        for outcome in worker.finish() {
            description_attempts += u64::from(outcome.attempted);
            description_actions.push(outcome);
        }
    }
    if let Some(worker) = live_worker {
        worker.finish();
    }
    #[cfg(target_arch = "arm")]
    clock_worker.finish();
    if let Some(writer) = persistence {
        accept_replies(
            writer.finish(),
            &mut committed_preferences,
            &mut storage_error,
        );
    }
    let native_api_probe = match native_probe {
        Some(worker) => Some(
            worker
                .join()
                .map_err(|_| "Read capability worker panicked")?,
        ),
        None => None,
    };
    if let Some(data) = &native_api_probe {
        let mut file = std::fs::OpenOptions::new()
            .create_new(true)
            .write(true)
            .mode(0o600)
            .custom_flags(libc::O_NOFOLLOW | libc::O_CLOEXEC)
            .open(output.join("native-api-probe.json"))?;
        serde_json::to_writer_pretty(&mut file, data)?;
    }
    let pointer_proof_receipt = match pointer_probe {
        Some(worker) => Some(worker.join().map_err(|_| "Pointer proof worker panicked")?),
        None => None,
    };
    let description_receipt = match description_probe {
        Some(worker) => Some(worker.join().map_err(|_| "Numeric proof worker panicked")?),
        None => None,
    };
    let persistent_readback = committed_preferences.as_ref() == Some(shell.preferences())
        || (loaded_preferences.as_ref() == Some(shell.preferences()) && storage_error.is_none());
    let mut report = serde_json::json!({"device":device,"app_id":plx_base::paths::app_id(),"uid":unsafe{libc::geteuid()},"renderer":window.renderer,
        "interactive_session":interactive,"idle_presentations":idle_presentations,"idle_wakes":session.wakes,
        "clock":clock.view(live_origin.elapsed()),
        "settings_returns":settings_returns,
        "drawer_proof":drawer_proof,"management_proof":management_proof,"controls_proof":controls_proof,"capture_diagnostics":capture_diagnostics,"drawable":[1920,1080],"events":events,"event_log":event_log,"exit_reason":reason,"requests_not_executed":requests,
        "control_readouts":if interactive {None} else {Some(shell.control_readout_proof())},
        "setting_writes":false,"application_launches":action_receipts.iter().any(|r|r.admitted),"action_receipts":action_receipts,"data":if shell.live_updates()>0{if device{"ordinary app live readings"}else{"simulated live readings"}}else{"captured TV snapshot"},"live_updates":shell.live_updates(),"live_history":live_history,"preferences":shell.preferences(),
        "preferences_at_boot":preferences_at_boot,"preferences_loaded":loaded_preferences.is_some(),"preferences_readback_confirmed":persistent_readback,
        "storage_error":storage_error,"native_read_probe_registered":native_api_probe.as_ref().and_then(|v|v.get("registered")),"storage_class":if device{"developer media"}else{"host instance"},"hold_probe_identity":probe_identity,"hold_probe_changed":probe_changed,
        "performance_validated":false,"scripted_navigation":{"enabled":!interactive,"focus":navigation,"page_after_ok":navigation_page,"passed":navigation_page.as_deref()==Some("Inputs"),"requests_not_executed":scripted_requests},"elapsed_seconds":start.elapsed().as_secs_f64()});
    report["text_input_proof"] = serde_json::json!(text_receipt);
    report["numeric_control_available_at_last_poll"] = shell.description_value().is_some().into();
    report["description_ui_proof"] = serde_json::json!(description_ui_receipt);
    report["description_actions"] = serde_json::json!(description_actions);
    report["description_action_attempts"] = description_attempts.into();
    report["range_layout_proof"] = serde_json::json!(range_receipt);
    report["navigation_proof"] = serde_json::json!(navigation_receipt);
    report["source_switching_enabled"] = shell.input_launches_enabled().into();
    report["description_level_proof"] = serde_json::json!(description_receipt);
    report["pointer_proof"] = serde_json::json!(pointer_proof_receipt);
    report["pointer_ui_proof"] = serde_json::json!(pointer_ui_receipt);
    report["pointer_actions"] = serde_json::json!(pointer_actions);
    report["setting_writes"] = report["pointer_proof"]["change"]["attempted"]
        .as_bool()
        .unwrap_or(false)
        .into();
    report["setting_writes"] = (report["setting_writes"].as_bool() == Some(true)
        || report["description_level_proof"]["change"]["attempted"].as_bool() == Some(true)
        || report["description_level_proof"]["restoration"]["attempted"].as_bool() == Some(true)
        || report["description_level_proof"]["recovery"]["attempted"].as_bool() == Some(true)
        || description_attempts > 0
        || pointer_actions.iter().any(|r| r.attempted))
    .into();
    report["swap_interval_request_accepted"] = window.swap_interval_request_accepted.into();
    report["performance"] = performance
        .as_ref()
        .map(|m| m.report())
        .unwrap_or(serde_json::Value::Null);
    let mut file = std::fs::OpenOptions::new()
        .create_new(true)
        .write(true)
        .mode(0o640)
        .custom_flags(libc::O_NOFOLLOW | libc::O_CLOEXEC)
        .open(output.join("shell-receipt.json"))?;
    serde_json::to_writer_pretty(&mut file, &report)?;
    file.write_all(b"\n")?;
    println!("{}", report);
    Ok(())
}

fn queue_preferences(
    shell: &mut Shell,
    writer: &mut Option<Persistence>,
    error: &mut Option<String>,
) {
    if !shell.take_preferences_changed() {
        return;
    }
    if let Some(writer) = writer {
        if let Err(failure) = writer.queue(shell.preferences()) {
            *error = Some(failure);
        }
    }
}
fn accept_replies(
    replies: Vec<Result<Preferences, String>>,
    committed: &mut Option<Preferences>,
    error: &mut Option<String>,
) {
    for reply in replies {
        match reply {
            Ok(prefs) => *committed = Some(prefs),
            Err(failure) => *error = Some(failure),
        }
    }
}

fn poll_live(
    worker: &Option<native_home::live::Worker>,
    shell: &mut Shell,
    origin: Instant,
    history: &mut Vec<serde_json::Value>,
) {
    if let Some(update) = worker.as_ref().and_then(|worker| worker.poll()) {
        let status = |ok: bool| if ok { "read" } else { "unavailable" };
        if history.len() < 32 {
            history.push(
                serde_json::json!({"observed_ms":update.observed_ms,"simulated":update.simulated,
            "catalogue":status(matches!(update.catalogue,native_home::live::Reading::Value(_))),
            "inputs":status(matches!(update.inputs,native_home::live::Reading::Value(_))),
            "picture":status(matches!(update.picture,native_home::live::Reading::Value(_))),
            "sound":status(matches!(update.sound,native_home::live::Reading::Value(_))),"network":status(matches!(update.network,native_home::live::Reading::Value(_))),"controls":update.controls.iter().map(|(section,value)|(section.name(),status(matches!(value,native_home::live::Reading::Value(_))))).collect::<std::collections::BTreeMap<_,_>>()}),
            );
        }
        shell.apply_live(&update, origin.elapsed().as_millis() as u32);
    }
    shell.age_live(origin.elapsed().as_millis() as u32);
}

fn dispatch_requests(
    shell: &mut Shell,
    worker: &Option<native_home::actions::Worker>,
    pointer_worker: &Option<native_home::pointer_speed::Worker>,
    proof_request: Option<&native_home::actions::Request>,
    unexecuted: &mut Vec<String>,
    handoff: &mut native_home::session::SettingsHandoff,
    now: Duration,
) {
    for action in shell.take_requests() {
        if let native_home::model::Action::SetPointerSpeed(change) = action {
            if let Some(worker) = pointer_worker {
                if let Err(message) = worker.queue(change) {
                    shell.pointer_reply(
                        &native_home::pointer_speed::Outcome {
                            change,
                            attempted: false,
                            admitted: false,
                            verified: false,
                            detail: message,
                        },
                        now.as_millis() as u32,
                    );
                }
            } else {
                shell.pointer_reply(
                    &native_home::pointer_speed::Outcome {
                        change,
                        attempted: false,
                        admitted: false,
                        verified: false,
                        detail: "Pointer-speed worker unavailable".into(),
                    },
                    now.as_millis() as u32,
                );
            }
            continue;
        }
        let Some(request) = native_home::actions::Request::from_action(action.clone()) else {
            unexecuted.push(format!("{action:?}"));
            continue;
        };
        let allowed = match proof_request {
            Some(proof) => &request == proof,
            None => matches!(
                request,
                native_home::actions::Request::Application { .. }
                    | native_home::actions::Request::Input { .. }
                    | native_home::actions::Request::LgHome {}
                    | native_home::actions::Request::LgSettings {}
                    | native_home::actions::Request::LgSettingsSection { .. }
            ),
        };
        if allowed {
            if let Some(worker) = worker {
                let settings = request.is_settings();
                if settings && handoff.pending() {
                    shell.storage_notice("LG Settings is already opening");
                    continue;
                }
                match worker.queue(request) {
                    Ok(()) => {
                        if settings {
                            handoff.queued(now);
                        }
                        shell.storage_notice(if settings {
                            "Opening LG Settings…"
                        } else {
                            "Opening application…"
                        });
                        continue;
                    }
                    Err(error) => shell.storage_notice(&error),
                }
            }
        }
        unexecuted.push(format!("{action:?}"));
    }
}

fn drawer_tap(shell: &mut Shell, now: &mut u32) {
    shell.press(true, PressFrom::Key);
    *now += 50;
    shell.tick(*now, 0.05);
    shell.press(false, PressFrom::Key);
    for _ in 0..30 {
        *now += 17;
        shell.tick(*now, 1.0 / 60.0);
    }
}
fn drawer_capture(
    shell: &mut Shell,
    window: &Window,
    output: &std::path::Path,
    name: &str,
    now: &mut u32,
) -> Result<(), Box<dyn std::error::Error>> {
    for _ in 0..45 {
        *now += 17;
        shell.tick(*now, 1.0 / 60.0);
        shell.draw();
        window.swap();
    }
    shell.draw();
    if window.graphics_state().error != 0 {
        return Err("Diagnostic capture found a GL error".into());
    }
    window.capture(&output.join(format!("{name}.ppm")))?;
    Ok(())
}

#[cfg(target_arch = "arm")]
fn pointer_write_proof(folder: &std::path::Path) -> serde_json::Value {
    use native_home::pointer_speed::{self, Change, Speed};
    let stage = |value: &serde_json::Value| -> Result<(), String> {
        let temp = folder.join("pointer-proof-next.json");
        let file = std::fs::OpenOptions::new()
            .create_new(true)
            .write(true)
            .mode(0o600)
            .custom_flags(libc::O_NOFOLLOW | libc::O_CLOEXEC)
            .open(&temp)
            .map_err(|e| e.to_string())?;
        serde_json::to_writer(file, value).map_err(|e| e.to_string())?;
        std::fs::rename(temp, folder.join("pointer-proof-current.json")).map_err(|e| e.to_string())
    };
    std::thread::sleep(Duration::from_secs(3));
    let mut p = match pointer_speed::Native::new() {
        Ok(p) => p,
        Err(e) => return serde_json::json!({"error":e}),
    };
    let original = match pointer_speed::inspect(&mut p, native_home::identity::PROOF) {
        Ok(v) => v,
        Err(e) => {
            let r = serde_json::json!({"stage":"refused","error":e});
            let _ = stage(&r);
            return r;
        }
    };
    let target = if original == Speed::Slow {
        Speed::Normal
    } else {
        Speed::Slow
    };
    let change = pointer_speed::apply(
        &mut p,
        native_home::identity::PROOF,
        Change {
            expected: original,
            value: target,
        },
        true,
    );
    let mut receipt =
        serde_json::json!({"stage":"changed","original":original,"target":target,"change":change});
    if !change.verified {
        receipt["stage"] = "refused".into();
        if change.attempted {
            let recovery = pointer_speed::apply(
                &mut p,
                native_home::identity::PROOF,
                Change {
                    expected: target,
                    value: original,
                },
                true,
            );
            receipt["recovery"] = serde_json::json!(recovery);
        }
        let _ = stage(&receipt);
        return receipt;
    }
    if let Err(e) = stage(&receipt) {
        receipt["stage_error"] = e.into();
    }
    std::thread::sleep(Duration::from_secs(8));
    let restore = pointer_speed::apply(
        &mut p,
        native_home::identity::PROOF,
        Change {
            expected: target,
            value: original,
        },
        true,
    );
    receipt["restoration"] = serde_json::json!(restore);
    receipt["stage"] = if restore.verified {
        "restored"
    } else {
        "restoration_failed"
    }
    .into();
    let _ = stage(&receipt);
    receipt
}

fn poll_pointer(
    worker: &Option<native_home::pointer_speed::Worker>,
    shell: &mut Shell,
    origin: Instant,
    receipts: &mut Vec<native_home::pointer_speed::Outcome>,
) {
    if let Some(worker) = worker {
        if let Some(ready) = worker.ready() {
            shell.enable_pointer_editor(ready);
        }
        for outcome in worker.poll() {
            shell.pointer_reply(&outcome, origin.elapsed().as_millis() as u32);
            receipts.push(outcome);
        }
    }
}
#[cfg(target_arch = "arm")]
#[allow(clippy::too_many_arguments)]
fn pointer_ui_trial(
    shell: &mut Shell,
    window: &Window,
    output: &std::path::Path,
    worker: &Option<native_home::pointer_speed::Worker>,
    live: &Option<native_home::live::Worker>,
    origin: Instant,
    history: &mut Vec<serde_json::Value>,
    receipts: &mut Vec<native_home::pointer_speed::Outcome>,
    now: &mut u32,
) -> Result<serde_json::Value, Box<dyn std::error::Error>> {
    use native_home::{
        model::Action,
        pointer_speed::{Change, Speed},
    };
    let deadline = Instant::now() + Duration::from_secs(5);
    while shell.pointer_value().is_none() && Instant::now() < deadline {
        poll_live(live, shell, origin, history);
        poll_pointer(worker, shell, origin, receipts);
        *now += 17;
        shell.tick(*now, 1.0 / 60.0);
        shell.draw();
        window.swap();
        std::thread::sleep(Duration::from_millis(10));
    }
    let original = shell.pointer_value().ok_or("Pointer editor not eligible")?;
    let target = if original == Speed::Slow {
        Speed::Normal
    } else {
        Speed::Slow
    };
    shell.open(Page::General);
    if !shell.focus_pointer_setting() {
        return Err("No pointer-speed row".into());
    }
    drawer_tap(shell, now);
    if shell.page() != Page::PointerSpeed {
        return Err("Pointer choice page did not open".into());
    }
    drawer_capture(shell, window, output, "pointer-choices", now)?;
    shell.focus_pointer_choice(target);
    drawer_capture(shell, window, output, "pointer-focus-only", now)?;
    if !shell.take_requests().is_empty() {
        return Err("Pointer movement emitted an operation".into());
    }
    let phase = |stage: &str| -> Result<(), Box<dyn std::error::Error>> {
        let tmp = output.join("pointer-ui-next.json");
        let file = std::fs::OpenOptions::new()
            .create_new(true)
            .write(true)
            .mode(0o600)
            .custom_flags(libc::O_NOFOLLOW | libc::O_CLOEXEC)
            .open(&tmp)?;
        serde_json::to_writer(
            file,
            &serde_json::json!({"stage":stage,"original":original,"target":target}),
        )?;
        std::fs::rename(tmp, output.join("pointer-ui-current.json"))?;
        Ok(())
    };
    phase("focused")?;
    // A bounded observation window lets the host independently verify focus makes no write.
    let until = Instant::now() + Duration::from_secs(5);
    while Instant::now() < until {
        poll_live(live, shell, origin, history);
        poll_pointer(worker, shell, origin, receipts);
        *now += 17;
        shell.tick(*now, 1.0 / 60.0);
        shell.draw();
        window.swap();
        std::thread::sleep(Duration::from_millis(10));
    }
    for (expected, value, stage) in [
        (original, target, "changed"),
        (target, original, "restored"),
    ] {
        if stage == "restored" {
            shell.open(Page::General);
            if !shell.focus_pointer_setting() {
                return Err("Restoration row unavailable".into());
            }
            drawer_tap(shell, now);
            shell.focus_pointer_choice(value);
        }
        drawer_tap(shell, now);
        let requests = shell.take_requests();
        if requests != vec![Action::SetPointerSpeed(Change { expected, value })] {
            return Err("Pointer UI produced a different request".into());
        }
        worker
            .as_ref()
            .ok_or("Pointer worker missing")?
            .queue(Change { expected, value })?;
        let before = receipts.len();
        let deadline = Instant::now() + Duration::from_secs(7);
        while receipts.len() == before && Instant::now() < deadline {
            poll_live(live, shell, origin, history);
            poll_pointer(worker, shell, origin, receipts);
            *now += 17;
            shell.tick(*now, 1.0 / 60.0);
            shell.draw();
            window.swap();
            std::thread::sleep(Duration::from_millis(10));
        }
        if !receipts
            .last()
            .is_some_and(|r| r.change == Change { expected, value } && r.verified)
        {
            return Err("Pointer UI change or restoration unverified".into());
        }
        phase(stage)?;
        drawer_capture(
            shell,
            window,
            output,
            if stage == "changed" {
                "pointer-changed"
            } else {
                "pointer-restored"
            },
            now,
        )?;
        if stage == "changed" {
            let until = Instant::now() + Duration::from_secs(8);
            while Instant::now() < until {
                poll_live(live, shell, origin, history);
                poll_pointer(worker, shell, origin, receipts);
                *now += 17;
                shell.tick(*now, 1.0 / 60.0);
                shell.draw();
                window.swap();
                std::thread::sleep(Duration::from_millis(10));
            }
        }
    }
    Ok(
        serde_json::json!({"focus_only_no_request":true,"round_trip_verified":true,"change_count":2}),
    )
}

fn navigation_trial(
    shell: &mut Shell,
    window: &Window,
    output: &std::path::Path,
    now: &mut u32,
) -> Result<serde_json::Value, Box<dyn std::error::Error>> {
    let before = shell.preferences().clone();
    let mut layouts = Vec::new();
    let mut capture =
        |s: &mut Shell, name: &str, n: &mut u32| -> Result<(), Box<dyn std::error::Error>> {
            drawer_capture(s, window, output, name, n)?;
            if let Some((r, c)) = s.drawer_focus_bounds() {
                if r.y < c.y - 1.0 || r.y + r.h > c.y + c.h + 1.0 {
                    return Err("Focused app/caption is outside drawer viewport".into());
                }
            }
            layouts.push(serde_json::json!({"name":name,"layout":s.presentation_probe()}));
            Ok(())
        };
    shell.open(Page::Home);
    if !shell.focus_home_link(0) {
        return Err("No Home apps shortcut".into());
    }
    capture(shell, "home-view-all", now)?;
    drawer_tap(shell, now);
    if shell.page() != Page::Applications
        || shell.drawer_state().favourites_only
        || !shell.drawer_state().query().is_empty()
    {
        return Err("View all did not open full drawer".into());
    }
    capture(shell, "apps-from-home", now)?;
    shell.back();
    if shell.page() != Page::Home || shell.focused_item().as_deref() != Some("View all apps") {
        return Err("Apps Back lost shortcut origin".into());
    }
    shell.focus_home_link(1);
    drawer_tap(shell, now);
    if shell.page() != Page::Inputs {
        return Err("No Inputs detail route".into());
    }
    capture(shell, "inputs-local-navigation", now)?;
    shell.back();
    if shell.page() != Page::Home || shell.focused_item().as_deref() != Some("View all inputs") {
        return Err("Inputs Back lost shortcut origin".into());
    }
    shell.focus_home_link(0);
    drawer_tap(shell, now);
    let id = shell
        .last_application_id()
        .ok_or("No apps for bounded drawer proof")?
        .to_owned();
    if !shell.focus_application(&id) {
        return Err("Last app unavailable".into());
    }
    capture(shell, "apps-last", now)?;
    shell.press(true, PressFrom::Key);
    for _ in 0..55 {
        *now += 17;
        shell.repeat();
        shell.tick(*now, 1.0 / 60.0);
        shell.draw();
        window.swap();
    }
    shell.press(false, PressFrom::Key);
    *now += 17;
    shell.tick(*now, 1.0 / 60.0);
    if shell.menu_application() != Some(id.as_str()) {
        return Err("Held OK did not open the intended app menu".into());
    }
    capture(shell, "apps-menu-last", now)?;
    shell.back();
    capture(shell, "apps-last-return", now)?;
    if shell.focused_item().as_deref() != Some(id.as_str()) {
        return Err("Menu Back lost app identity".into());
    }
    shell.back();
    if shell.page() != Page::Home || shell.focused_item().as_deref() != Some("View all apps") {
        return Err("Drawer Back lost Home shortcut".into());
    }
    if shell.preferences() != &before || !shell.take_requests().is_empty() {
        return Err("Dashboard navigation emitted a TV operation or changed favourites".into());
    }
    shell.open(Page::Home);
    let receipt = serde_json::json!({"view_all_apps":true,"view_all_inputs":true,"back_restores_shortcut":true,"menu_restores_app":true,"favourites_unchanged":true,"tv_requests":0,"layouts":layouts});
    let file = std::fs::OpenOptions::new()
        .create_new(true)
        .write(true)
        .mode(0o600)
        .custom_flags(libc::O_NOFOLLOW | libc::O_CLOEXEC)
        .open(output.join("navigation-proof-complete.json"))?;
    serde_json::to_writer_pretty(file, &receipt)?;
    Ok(receipt)
}

#[cfg(target_arch = "arm")]
fn description_level_proof(folder: &std::path::Path) -> serde_json::Value {
    use native_home::description_level::{self, Change};
    let stage = |value: &serde_json::Value| -> Result<(), String> {
        let temp = folder.join("description-level-proof-next.json");
        let file = std::fs::OpenOptions::new()
            .create_new(true)
            .write(true)
            .mode(0o600)
            .custom_flags(libc::O_NOFOLLOW | libc::O_CLOEXEC)
            .open(&temp)
            .map_err(|e| e.to_string())?;
        serde_json::to_writer(file, value).map_err(|e| e.to_string())?;
        std::fs::rename(temp, folder.join("description-level-proof-current.json"))
            .map_err(|e| e.to_string())
    };
    std::thread::sleep(Duration::from_secs(3));
    let mut p = match description_level::Native::new() {
        Ok(p) => p,
        Err(e) => return serde_json::json!({"error":e}),
    };
    let original = match description_level::inspect(&mut p, native_home::identity::PROOF) {
        Ok(v) => v,
        Err(e) => {
            let r = serde_json::json!({"stage":"refused","error":e});
            let _ = stage(&r);
            return r;
        }
    };
    let target = original.adjacent();
    let change = description_level::apply(
        &mut p,
        native_home::identity::PROOF,
        Change {
            expected: original,
            value: target,
        },
        true,
    );
    let mut receipt =
        serde_json::json!({"stage":"changed","original":original,"target":target,"change":change});
    if !change.verified {
        receipt["stage"] = "refused".into();
        if change.attempted {
            let recovery = description_level::apply(
                &mut p,
                native_home::identity::PROOF,
                Change {
                    expected: target,
                    value: original,
                },
                true,
            );
            receipt["recovery"] = serde_json::json!(recovery);
        }
        let _ = stage(&receipt);
        return receipt;
    }
    if let Err(e) = stage(&receipt) {
        receipt["stage_error"] = e.into();
    }
    std::thread::sleep(Duration::from_secs(8));
    let restore = description_level::apply(
        &mut p,
        native_home::identity::PROOF,
        Change {
            expected: target,
            value: original,
        },
        true,
    );
    receipt["restoration"] = serde_json::json!(restore);
    receipt["stage"] = if restore.verified {
        "restored"
    } else {
        "restoration_failed"
    }
    .into();
    let _ = stage(&receipt);
    receipt
}

/// Synthetic staged-edit interaction proof. It deliberately has no TV transport.
fn range_layout_trial(
    shell: &mut Shell,
    window: &Window,
    output: &std::path::Path,
    now: &mut u32,
) -> Result<serde_json::Value, Box<dyn std::error::Error>> {
    use plx_ui::widgets::RangeScale;
    let before = shell.preferences().clone();
    let scale = RangeScale::new(-15, 15, 1).unwrap();
    let mut layouts = Vec::new();
    let mut capture =
        |s: &mut Shell, name: &str, n: &mut u32| -> Result<(), Box<dyn std::error::Error>> {
            s.storage_notice("Interface preview • no TV settings will be changed");
            drawer_capture(s, window, output, name, n)?;
            layouts.push(serde_json::json!({"name":name,"layout":s.presentation_probe()}));
            Ok(())
        };
    shell.open(Page::Controls);
    shell.focus_control_category(3);
    let parent = shell.page();
    if !shell.open_range_editor("Preview level", scale, 0) {
        return Err("No range editor".into());
    }
    capture(shell, "range-initial", now)?;
    shell.direction(Dir::Right);
    if shell.range_state().unwrap().proposal() != Some((0, 1))
        || shell.take_range_request().is_some()
        || !shell.take_requests().is_empty()
    {
        return Err("Adjustment emitted an effect or changed saved value".into());
    }
    capture(shell, "range-staged", now)?;
    shell.back();
    if shell.page() != parent
        || shell.range_state().is_some()
        || shell.take_range_request().is_some()
    {
        return Err("Range Back did not discard staging".into());
    }
    shell.open_range_editor("Preview level", scale, 0);
    // Use the same public layout as painting, without restating its track in the driver.
    let layout = shell.range_layout().unwrap();
    shell.pointer(layout.track.x, layout.track.cy(), false);
    if shell.range_state().unwrap().staged() != 0 {
        return Err("Hover changed the staged value".into());
    }
    shell.pointer(layout.track.x, layout.track.cy(), true);
    if shell.range_state().unwrap().staged() != -15 {
        return Err("Pointer minimum mismatch".into());
    }
    capture(shell, "range-minimum", now)?;
    shell.pointer(layout.track.x + layout.track.w, layout.track.cy(), true);
    if shell.range_state().unwrap().staged() != 15 {
        return Err("Pointer maximum mismatch".into());
    }
    capture(shell, "range-maximum", now)?;
    shell.back();
    shell.open_range_editor("Preview level", scale, 0);
    shell.direction(Dir::Right);
    shell.direction(Dir::Down);
    drawer_tap(shell, now);
    if shell.take_range_request() != Some((0, 1)) || shell.take_range_request().is_some() {
        return Err("Apply did not emit exactly one typed proposal".into());
    }
    shell.back();
    shell.direction(Dir::Up);
    drawer_tap(shell, now);
    if shell.page() != Page::RangeEditor || shell.take_range_request().is_some() {
        return Err("Pending range edit was abandoned or duplicated".into());
    }
    // No native transport is connected in this proof. A refused result must never move Saved.
    if shell.range_reply(0, 1, false) || shell.range_state().unwrap().saved() != 0 {
        return Err("Unconfirmed result moved Saved".into());
    }
    capture(shell, "range-unconfirmed", now)?;
    shell.back();
    shell.open(Page::Home);
    if shell.preferences() != &before || !shell.take_requests().is_empty() {
        return Err("Range proof changed favourites or emitted TV actions".into());
    }
    let receipt = serde_json::json!({"synthetic_values":true,"transport":"none; staged interaction only","staging_emits_no_effect":true,"back_discards_staging":true,"pointer_hover_is_read_only":true,"pointer_endpoints_match_paint":true,"apply_emits_one_proposal":true,"pending_blocks_navigation_and_duplicates":true,"unconfirmed_value_never_becomes_saved":true,"favourites_unchanged":true,"tv_requests":0,"setting_writes":false,"layouts":layouts});
    let file = std::fs::OpenOptions::new()
        .create_new(true)
        .write(true)
        .mode(0o600)
        .custom_flags(libc::O_NOFOLLOW | libc::O_CLOEXEC)
        .open(output.join("range-layout-complete.json"))?;
    serde_json::to_writer_pretty(file, &receipt)?;
    Ok(receipt)
}

fn pump_description(
    worker: &Option<native_home::description_level::Worker>,
    shell: &mut Shell,
    origin: Instant,
    receipts: &mut Vec<native_home::description_level::Outcome>,
    attempts: &mut u64,
) {
    let now = origin.elapsed().as_millis() as u32;
    if let Some(worker) = worker {
        // Outcomes precede samples, so an older read cycle cannot rewind a confirmed write.
        for outcome in worker.poll() {
            shell.description_reply(&outcome, now);
            *attempts += u64::from(outcome.attempted);
            receipts.push(outcome);
            if receipts.len() > 32 {
                receipts.remove(0);
            }
        }
        if let Some(sample) = worker.sample() {
            let stamp = sample
                .started
                .checked_duration_since(origin)
                .unwrap_or_default()
                .as_millis() as u32;
            shell.description_sample(sample.value, stamp, now);
        }
    }
    shell.age_description(now);
    if let Some(change) = shell.take_description_request() {
        let error = match worker {
            Some(w) => w.queue(change).err(),
            None => Some("Numeric worker unavailable".into()),
        };
        if let Some(message) = error {
            shell.description_reply(
                &native_home::description_level::Outcome::refused(change, message),
                now,
            );
        }
    }
}

#[cfg(target_arch = "arm")]
#[allow(clippy::too_many_arguments)]
fn description_ui_trial(
    shell: &mut Shell,
    window: &Window,
    output: &std::path::Path,
    worker: &Option<native_home::description_level::Worker>,
    origin: Instant,
    receipts: &mut Vec<native_home::description_level::Outcome>,
    attempts: &mut u64,
    now: &mut u32,
) -> Result<serde_json::Value, Box<dyn std::error::Error>> {
    use native_home::description_level::Level;
    let before = shell.preferences().clone();
    let phase =
        |stage: &str, original: Level, target: Level| -> Result<(), Box<dyn std::error::Error>> {
            let tmp = output.join("description-ui-next.json");
            let file = std::fs::OpenOptions::new()
                .create_new(true)
                .write(true)
                .mode(0o600)
                .custom_flags(libc::O_NOFOLLOW | libc::O_CLOEXEC)
                .open(&tmp)?;
            serde_json::to_writer(
                file,
                &serde_json::json!({"stage":stage,"original":original,"target":target}),
            )?;
            std::fs::rename(tmp, output.join("description-ui-current.json"))?;
            Ok(())
        };
    let deadline = Instant::now() + Duration::from_secs(12);
    while shell.description_value().is_none() && Instant::now() < deadline {
        pump_description(worker, shell, origin, receipts, attempts);
        *now += 17;
        shell.tick(*now, 0.017);
        shell.draw();
        window.swap();
        std::thread::sleep(Duration::from_millis(10));
    }
    let original = shell
        .description_value()
        .ok_or("Native numeric capability unavailable")?;
    let adjacent = original.adjacent();
    if !shell.focus_description_setting() {
        return Err("Numeric entry unavailable".into());
    }
    drawer_tap(shell, now);
    if shell.page() != Page::RangeEditor {
        return Err("Numeric entry did not open range editor".into());
    }
    shell.direction(if adjacent.value() > original.value() {
        Dir::Right
    } else {
        Dir::Left
    });
    if shell.take_description_request().is_some() {
        return Err("Staging wrote a setting".into());
    }
    drawer_capture(shell, window, output, "numeric-staged", now)?;
    phase("staged", original, adjacent)?;
    let until = Instant::now() + Duration::from_secs(5);
    while Instant::now() < until {
        pump_description(worker, shell, origin, receipts, attempts);
        *now += 17;
        shell.tick(*now, 0.017);
        shell.draw();
        window.swap();
        std::thread::sleep(Duration::from_millis(10));
    }
    shell.back();
    if shell.page() != Page::Accessibility || shell.range_state().is_some() || *attempts != 0 {
        return Err("Cancel did not restore the numeric entry without writing".into());
    }
    let mut transitions = 0;
    for (value, stage) in [
        (adjacent, "changed"),
        (Level::new(-15).unwrap(), "minimum"),
        (Level::new(15).unwrap(), "maximum"),
        (original, "restored"),
    ] {
        let expected = shell
            .description_value()
            .ok_or("Numeric context withdrawn before next selection")?;
        if !shell.focus_description_setting() {
            return Err("Numeric row unavailable".into());
        }
        drawer_tap(shell, now);
        while shell.range_state().unwrap().staged() != i64::from(value.value()) {
            let staged = shell.range_state().unwrap().staged();
            shell.direction(if staged < i64::from(value.value()) {
                Dir::Right
            } else {
                Dir::Left
            });
        }
        let count = receipts.len();
        shell.direction(Dir::Down);
        drawer_tap(shell, now);
        let deadline = Instant::now() + Duration::from_secs(15);
        while expected != value && receipts.len() == count && Instant::now() < deadline {
            pump_description(worker, shell, origin, receipts, attempts);
            *now += 17;
            shell.tick(*now, 0.017);
            shell.draw();
            window.swap();
            std::thread::sleep(Duration::from_millis(10));
        }
        if expected != value {
            let outcome = receipts.last().ok_or("No numeric completion")?;
            if receipts.len() != count + 1
                || !outcome.verified
                || outcome.change.expected != expected
                || outcome.change.value != value
            {
                return Err("Numeric UI readback not confirmed".into());
            }
            transitions += 1;
        }
        if shell
            .range_state()
            .is_none_or(|r| r.saved() != i64::from(value.value()))
        {
            return Err("Saved marker did not follow verified value".into());
        }
        drawer_capture(shell, window, output, &format!("numeric-{stage}"), now)?;
        phase(stage, original, value)?;
        let until = Instant::now() + Duration::from_secs(4);
        while Instant::now() < until {
            pump_description(worker, shell, origin, receipts, attempts);
            *now += 17;
            shell.tick(*now, 0.017);
            shell.draw();
            window.swap();
            std::thread::sleep(Duration::from_millis(10));
        }
    }
    shell.back();
    shell.open(Page::Home);
    if shell.preferences() != &before || !shell.take_requests().is_empty() {
        return Err("Numeric trial changed favourites or emitted unrelated actions".into());
    }
    Ok(
        serde_json::json!({"staging_no_write":true,"cancel_no_write":true,"native_entry_and_apply":true,"endpoints_verified":true,"original_restored":true,"verified_transitions":transitions,"favourites_unchanged":true,"transport":"fixed ordinary-app numeric worker"}),
    )
}

#[cfg(target_arch = "arm")]
#[allow(clippy::too_many_arguments)]
fn text_input_trial(
    shell: &mut Shell,
    window: &Window,
    output: &std::path::Path,
    live: &Option<native_home::live::Worker>,
    origin: Instant,
    history: &mut Vec<serde_json::Value>,
    now: &mut u32,
) -> Result<serde_json::Value, Box<dyn std::error::Error>> {
    struct Stop<'a>(&'a Window);
    impl Drop for Stop<'_> {
        fn drop(&mut self) {
            self.0.text_stop();
        }
    }
    let _stop = Stop(window);
    let before = shell.preferences().clone();
    let mut statuses = Vec::new();
    let mut kinds = Vec::new();
    let mut commits = 0;
    let phase = |name: &str| -> Result<(), Box<dyn std::error::Error>> {
        let tmp = output.join("text-input-next.json");
        let f = std::fs::OpenOptions::new()
            .create_new(true)
            .write(true)
            .mode(0o600)
            .custom_flags(libc::O_NOFOLLOW | libc::O_CLOEXEC)
            .open(&tmp)?;
        serde_json::to_writer(f, &serde_json::json!({"stage":name}))?;
        std::fs::rename(tmp, output.join("text-input-current.json"))?;
        Ok(())
    };
    for cycle in 1..=2 {
        shell.open(Page::Applications);
        shell.focus_drawer_control(2);
        drawer_tap(shell, now);
        shell.text_edit(&plx_machine::machine::TextEdit::Clear);
        let deadline = Instant::now() + Duration::from_secs(8);
        while window.keyboard_status().focused == 0 && Instant::now() < deadline {
            while let Some(e) = poll() {
                if kinds.len() < 64 {
                    kinds.push(e.kind);
                }
            }
            *now += 17;
            shell.tick(*now, 0.017);
            shell.draw();
            window.swap();
            std::thread::sleep(Duration::from_millis(20));
        }
        statuses.push(serde_json::json!({"cycle":cycle,"before":window.keyboard_status()}));
        if !window.text_start() {
            phase("unavailable")?;
            return Err("SDL text input could not acquire the app window".into());
        }
        for text in ["É", "cran"] {
            if !window.diagnostic_text(text) {
                return Err("Native-only text injection refused".into());
            }
        }
        let deadline = Instant::now() + Duration::from_secs(3);
        while shell.drawer_state().query() != "Écran" && Instant::now() < deadline {
            while let Some(e) = poll() {
                if kinds.len() < 64 {
                    kinds.push(e.kind);
                }
                if e.kind == 10 {
                    if let Some(text) = e.text() {
                        shell.text_input(text, false);
                        commits += 1;
                    }
                } else if e.kind == 11 {
                    if let Some(text) = e.text() {
                        shell.text_preedit(text);
                    }
                }
            }
            *now += 17;
            shell.tick(*now, 0.017);
            shell.draw();
            window.swap();
            std::thread::sleep(Duration::from_millis(10));
        }
        if shell.drawer_state().query() != "Écran" {
            return Err("Native SDL commit boundary did not reach the owned draft".into());
        }
        phase(&format!("open-{cycle}"))?;
        let until = Instant::now() + Duration::from_secs(5);
        while Instant::now() < until {
            poll_live(live, shell, origin, history);
            while let Some(e) = poll() {
                if kinds.len() < 64 {
                    kinds.push(e.kind);
                }
                if e.kind == 10 {
                    if let Some(text) = e.text() {
                        shell.text_input(text, false);
                    }
                }
            }
            *now += 17;
            shell.tick(*now, 0.017);
            shell.draw();
            window.swap();
            std::thread::sleep(Duration::from_millis(10));
        }
        drawer_capture(shell, window, output, &format!("text-open-{cycle}"), now)?;
        statuses.push(serde_json::json!({"cycle":cycle,"open_flag_not_visibility_proof":window.keyboard_status()}));
        window.text_stop();
        phase(&format!("closed-{cycle}"))?;
        let until = Instant::now() + Duration::from_secs(5);
        while Instant::now() < until {
            while let Some(e) = poll() {
                if kinds.len() < 64 {
                    kinds.push(e.kind);
                }
            }
            *now += 17;
            shell.tick(*now, 0.017);
            shell.draw();
            window.swap();
            std::thread::sleep(Duration::from_millis(10));
        }
        statuses.push(serde_json::json!({"cycle":cycle,"closed":window.keyboard_status()}));
        shell.back();
    }
    shell.open(Page::Home);
    if shell.preferences() != &before || !shell.take_requests().is_empty() {
        return Err("Text-only proof changed preferences or requested a TV action".into());
    }
    phase("complete")?;
    Ok(
        serde_json::json!({"native_sdl_commits":commits,"query_match_each_cycle":true,"cycles":2,"injection":"native diagnostic SDL queue, not human typing","statuses":statuses,"window_event_kinds":kinds,"favourites_unchanged":true,"tv_requests":0,"setting_writes":false}),
    )
}
