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
    let session_capture = interactive && assets.join("session-proof.json").exists();
    if interactive
        && [
            "drawer-proof.json",
            "management-proof.json",
            "controls-proof.json",
            "launch-proof.json",
            "preferences-hold-probe.json",
            "native-read-probe.json",
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
    let mut action_worker = None;
    #[allow(unused_mut)]
    let mut app_launches_enabled = false;
    #[cfg(target_arch = "arm")]
    if device {
        app_launches_enabled = native_home::identity::app_launches(
            plx_base::paths::app_id(),
            matches!(&launch_proof, Some(native_home::actions::Request::Application { id }) if id == native_home::identity::TARGET),
        );
        action_worker = Some(native_home::actions::Worker::native(
            plx_base::paths::app_id().into(),
            native_home::actions::Capabilities {
                apps: app_launches_enabled,
                inputs: matches!(
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
        ] {
            shell.open(page);
            for _ in 0..45 {
                now += 17;
                poll_live(&live_worker, &mut shell, live_origin, &mut live_history);
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
            let query = shell.drawer_state().query.clone();
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
            native_home::actions::Request::LgSettings {}
            | native_home::actions::Request::LgSettingsSection { .. } => {
                shell.open(request.settings_section().unwrap().page());
                if !shell.focus_lg_settings() {
                    return Err("LG Settings route unavailable".into());
                }
            }
            native_home::actions::Request::Input { id } if id == "HDMI_4" => {
                shell.open(Page::Inputs);
                if !shell.focus_input(id) {
                    return Err("HDMI 4 unavailable".into());
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

    let mut reason = "capture_complete";
    if !capture_only {
        reason = "timeout";
        while !session.expired(start.elapsed()) && !shell.should_exit() {
            let current = Instant::now();
            let dt = (current - last).as_secs_f32().clamp(0.001, 0.05);
            last = current;
            now = now.wrapping_add((dt * 1000.0) as u32);
            poll_live(&live_worker, &mut shell, live_origin, &mut live_history);
            #[cfg(target_arch = "arm")]
            if let Some(reply) = clock_worker.poll() {
                clock.accept(reply);
            }
            shell.set_clock(clock.view(live_origin.elapsed()));
            shell.tick(now, dt);
            while let Some(e) = poll() {
                events = events.saturating_add(1_u32);
                // Only app-local input wakes this canvas; Home/power/volume stay LG-owned.
                // Consume the first wake input and cancel any gesture armed before blanking.
                if hidden_at.is_none() && matches!(e.kind, 2..=6) && !session.input(start.elapsed())
                {
                    shell.cancel_press();
                    continue;
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
                            _ => {}
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
                        idle_presentations = idle_presentations.saturating_add(1);
                        idle_blanked = true;
                    }
                } else {
                    shell.draw();
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
                    window.swap();
                    idle_blanked = false;
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
    let persistent_readback = committed_preferences.as_ref() == Some(shell.preferences())
        || (loaded_preferences.as_ref() == Some(shell.preferences()) && storage_error.is_none());
    let report = serde_json::json!({"device":device,"app_id":plx_base::paths::app_id(),"uid":unsafe{libc::geteuid()},"renderer":window.renderer,
        "interactive_session":interactive,"idle_presentations":idle_presentations,"idle_wakes":session.wakes,
        "clock":clock.view(live_origin.elapsed()),
        "settings_returns":settings_returns,
        "drawer_proof":drawer_proof,"management_proof":management_proof,"controls_proof":controls_proof,"capture_diagnostics":capture_diagnostics,"drawable":[1920,1080],"events":events,"event_log":event_log,"exit_reason":reason,"requests_not_executed":requests,
        "setting_writes":false,"application_launches":action_receipts.iter().any(|r|r.admitted),"action_receipts":action_receipts,"data":if shell.live_updates()>0{if device{"ordinary app live readings"}else{"simulated live readings"}}else{"captured TV snapshot"},"live_updates":shell.live_updates(),"live_history":live_history,"preferences":shell.preferences(),
        "preferences_at_boot":preferences_at_boot,"preferences_loaded":loaded_preferences.is_some(),"preferences_readback_confirmed":persistent_readback,
        "storage_error":storage_error,"native_read_probe_registered":native_api_probe.as_ref().and_then(|v|v.get("registered")),"storage_class":if device{"developer media"}else{"host instance"},"hold_probe_identity":probe_identity,"hold_probe_changed":probe_changed,
        "performance_validated":false,"scripted_navigation":{"enabled":!interactive,"focus":navigation,"page_after_ok":navigation_page,"passed":navigation_page.as_deref()==Some("Inputs"),"requests_not_executed":scripted_requests},"elapsed_seconds":start.elapsed().as_secs_f64()});
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
            "sound":status(matches!(update.sound,native_home::live::Reading::Value(_))),"network":status(matches!(update.network,native_home::live::Reading::Value(_)))}),
            );
        }
        shell.apply_live(&update, origin.elapsed().as_millis() as u32);
    }
    shell.age_live(origin.elapsed().as_millis() as u32);
}

fn dispatch_requests(
    shell: &mut Shell,
    worker: &Option<native_home::actions::Worker>,
    proof_request: Option<&native_home::actions::Request>,
    unexecuted: &mut Vec<String>,
    handoff: &mut native_home::session::SettingsHandoff,
    now: Duration,
) {
    for action in shell.take_requests() {
        let Some(request) = native_home::actions::Request::from_action(action.clone()) else {
            unexecuted.push(format!("{action:?}"));
            continue;
        };
        let allowed = match proof_request {
            Some(proof) => &request == proof,
            None => matches!(
                request,
                native_home::actions::Request::Application { .. }
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
