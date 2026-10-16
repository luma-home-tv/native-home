//! Fixed, isolated native presentation diagnostic. Synthetic packaged artwork
//! only: no catalogue service, resource broker, app launch or TV setter.
use crate::{
    model::Page,
    runtime::Window,
    shell::{Shell, Snapshot},
};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use std::{
    io::{Read, Write},
    os::unix::fs::{MetadataExt, OpenOptionsExt},
    path::Path,
    time::{Duration, Instant},
};

pub const IDS: [&str; 4] = [
    "synthetic.fixture.icon.square",
    "synthetic.fixture.icon.wide",
    "synthetic.fixture.icon.tall",
    "synthetic.fixture.icon.missing",
];
const TITLES: [&str; 4] = ["Square Beacon", "Wide Banner", "Tall Totem", "Missing Mark"];
const ICONS: [Option<&str>; 4] = [
    Some("icons/square.png"),
    Some("icons/wide.png"),
    Some("icons/tall.png"),
    None,
];
const DIMENSIONS: [Option<[u32; 2]>; 4] =
    [Some([128, 128]), Some([128, 64]), Some([64, 128]), None];

#[derive(Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Marker {
    schema: u32,
    fixture_ids: [String; 4],
    dimensions: [Option<[u32; 2]>; 4],
}
impl Marker {
    pub fn validate(&self) -> Result<(), &'static str> {
        if self.schema != 1 || self.fixture_ids != IDS || self.dimensions != DIMENSIONS {
            return Err("Invalid fixed icon presentation recipe");
        }
        Ok(())
    }
    pub fn read(path: &Path) -> Result<Self, Box<dyn std::error::Error>> {
        let file = std::fs::OpenOptions::new()
            .read(true)
            .custom_flags(libc::O_NOFOLLOW | libc::O_CLOEXEC | libc::O_NONBLOCK)
            .open(path)?;
        let a = file.metadata()?;
        if !a.is_file()
            || a.len() == 0
            || a.len() > 4096
            || a.uid() != 0
            || a.gid() != 0
            || a.nlink() != 1
            || a.mode() & 0o7777 != 0o644
        {
            return Err("Invalid installed icon presentation marker".into());
        }
        let mut raw = Vec::new();
        (&file).take(4097).read_to_end(&mut raw)?;
        let b = file.metadata()?;
        if raw.len() as u64 != a.len()
            || (
                a.dev(),
                a.ino(),
                a.len(),
                a.mode(),
                a.uid(),
                a.gid(),
                a.nlink(),
            ) != (
                b.dev(),
                b.ino(),
                b.len(),
                b.mode(),
                b.uid(),
                b.gid(),
                b.nlink(),
            )
            || (a.mtime(), a.mtime_nsec(), a.ctime(), a.ctime_nsec())
                != (b.mtime(), b.mtime_nsec(), b.ctime(), b.ctime_nsec())
        {
            return Err("Icon presentation marker changed during read".into());
        }
        let marker: Self = serde_json::from_slice(&raw)?;
        marker.validate()?;
        Ok(marker)
    }
    pub fn validate_snapshot(&self, snapshot: &Snapshot) -> Result<(), &'static str> {
        self.validate()?;
        if snapshot.applications.len() != IDS.len()
            || !snapshot.inputs.is_empty()
            || snapshot.launch_ids != IDS.map(str::to_owned).into()
        {
            return Err("Icon presentation requires only the four synthetic fixtures");
        }
        for (i, id) in IDS.into_iter().enumerate() {
            let apps = snapshot
                .applications
                .iter()
                .filter(|a| a.id == id)
                .collect::<Vec<_>>();
            if apps.len() != 1
                || apps[0].title != TITLES[i]
                || !apps[0].visible
                || apps[0].kind != "native"
                || apps[0].icon.as_deref() != ICONS[i]
            {
                return Err("Icon presentation fixture identity or asset changed");
            }
        }
        Ok(())
    }
}

/// Wait only in explicitly requested capture diagnostics. Frame counts alone
/// cannot establish completion of an asynchronous ordinary image worker.
pub fn settle_packaged_for_capture(
    shell: &mut Shell,
    now: &mut u32,
    deadline: Instant,
) -> Result<(), Box<dyn std::error::Error>> {
    loop {
        if shell.packaged_icons_settled() {
            return Ok(());
        }
        while let Some(event) = crate::runtime::poll() {
            if matches!(event.kind, 1..=8 | 10 | 11) {
                shell.request_exit();
                return Err("Packaged capture lost its quiet foreground".into());
            }
        }
        if Instant::now() >= deadline {
            return Err("Visible packaged artwork did not settle before capture deadline".into());
        }
        *now = now.wrapping_add(17);
        shell.tick(*now, 1.0 / 60.0);
        std::thread::sleep(std::time::Duration::from_millis(1));
    }
}

pub fn run(
    shell: &mut Shell,
    window: &Window,
    output: &Path,
    now: &mut u32,
) -> Result<Value, Box<dyn std::error::Error>> {
    let deadline = Instant::now() + Duration::from_secs(45);
    let preferences = shell.preferences().clone();
    let mut stages = Vec::new();
    for (frame, selected) in [
        ("icons-home", None),
        ("icons-apps-rest", None),
        ("icons-focus-square", Some(IDS[0])),
        ("icons-focus-wide", Some(IDS[1])),
        ("icons-focus-tall", Some(IDS[2])),
        ("icons-focus-missing", Some(IDS[3])),
    ] {
        let home = frame == "icons-home";
        shell.open(if home { Page::Home } else { Page::Applications });
        let focused = if let Some(id) = selected {
            shell.focus_application(id)
        } else if home {
            shell.focus_home_link(0)
        } else {
            shell.focus_drawer_control(0)
        };
        if !focused {
            return Err("Missing fixed icon presentation focus target".into());
        }
        for _ in 0..45 {
            if Instant::now() >= deadline {
                return Err("Icon presentation deadline exceeded".into());
            }
            while let Some(event) = crate::runtime::poll() {
                if matches!(event.kind, 1..=8 | 10 | 11) {
                    return Err(
                        "Icon presentation interrupted by external input or departure".into(),
                    );
                }
            }
            *now = now.wrapping_add(17);
            shell.tick(*now, 1.0 / 60.0);
            shell.draw();
            window.swap();
        }
        settle_packaged_for_capture(shell, now, deadline.min(Instant::now() + std::time::Duration::from_secs(5)))?;
        let before = window.graphics_state();
        shell.draw();
        let graphics = window.graphics_state();
        if before.error != 0 || graphics.error != 0 {
            return Err("Icon presentation GL error".into());
        }
        let cards = shell.icon_presentation_cards();
        for (i, id) in IDS.into_iter().enumerate() {
            let card = cards
                .iter()
                .find(|c| c["id"] == id)
                .ok_or("Missing fixture card")?;
            if let Some([w, h]) = DIMENSIONS[i] {
                if card["width"] != w || card["height"] != h || card["texture_handle"] == 0 {
                    return Err("Icon fixture was not decoded at its declared dimensions".into());
                }
            } else if card["texture_handle"] != 0 {
                return Err("Missing fixture unexpectedly has artwork".into());
            }
        }
        // Preserve the actual framebuffer before any external pixel predicate.
        window.capture(&output.join(format!("{frame}.ppm")))?;
        window.swap();
        let resources = shell.icon_presentation_resources();
        let resource_validity = window.texture_validity(&resources);
        let after_resources = window.graphics_state();
        if resources.len() != 8
            || resources
                .iter()
                .copied()
                .collect::<std::collections::BTreeSet<_>>()
                .len()
                != 8
            || resource_validity
                .as_array()
                .is_none_or(|rows| rows.iter().any(|row| row["valid"] != true))
            || after_resources.error != 0
        {
            return Err("Icon presentation lost an owned image or branding texture".into());
        }
        stages.push(
            json!({"frame":frame,"page":if home {"Home"} else {"Applications"},
            "focused_item":shell.focused_item(),"cards":cards,"before_draw":before,
            "graphics":graphics,"resource_validity":resource_validity,"after_resources":after_resources}),
        );
        if shell.preferences() != &preferences
            || !shell.take_requests().is_empty()
            || shell.should_exit()
        {
            return Err("Icon presentation changed preferences or requested an action".into());
        }
    }
    let report = json!({"schema":1,"fixture_ids":IDS,"stages":stages,
        "preferences_unchanged":true,"live_catalogue_enabled":false,"application_launches":false,
        "setting_writes":false,"physical_input":false,"performance_validated":false});
    let mut file = std::fs::OpenOptions::new()
        .write(true)
        .create_new(true)
        .mode(0o600)
        .custom_flags(libc::O_NOFOLLOW | libc::O_CLOEXEC)
        .open(output.join("icon-presentation-proof.json"))?;
    serde_json::to_writer(&mut file, &report)?;
    file.write_all(b"\n")?;
    file.sync_all()?;
    Ok(report)
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn fixed_recipe_rejects_other_dimensions_paths_and_duplicate_fields() {
        let value = json!({"schema":1,"fixture_ids":IDS,"dimensions":DIMENSIONS});
        let marker: Marker = serde_json::from_value(value.clone()).unwrap();
        marker.validate().unwrap();
        for (key, bad) in [
            ("schema", json!(2)),
            (
                "dimensions",
                json!([[128, 128], [64, 128], [64, 128], null]),
            ),
            ("fixture_ids", json!(["netflix", IDS[1], IDS[2], IDS[3]])),
        ] {
            let mut changed = value.clone();
            changed[key] = bad;
            assert!(serde_json::from_value::<Marker>(changed)
                .unwrap()
                .validate()
                .is_err());
        }
        let mut changed = value;
        changed["path"] = json!("/tmp/anything");
        assert!(serde_json::from_value::<Marker>(changed).is_err());
        assert!(serde_json::from_str::<Marker>("{\"schema\":1,\"schema\":1}").is_err());
    }
    #[test]
    fn fixture_admission_excludes_household_or_launchable_extra_apps_and_path_changes() {
        let marker: Marker =
            serde_json::from_value(json!({"schema":1,"fixture_ids":IDS,"dimensions":DIMENSIONS}))
                .unwrap();
        let mut snapshot: Snapshot = serde_json::from_str(include_str!(
            "../docs/verification/control-row-presentation/snapshot.json"
        ))
        .unwrap();
        snapshot.applications = IDS
            .into_iter()
            .enumerate()
            .map(|(i, id)| crate::model::Application {
                id: id.into(),
                title: TITLES[i].into(),
                kind: "native".into(),
                visible: true,
                icon: ICONS[i].map(str::to_owned),
            })
            .collect();
        snapshot.launch_ids = IDS.map(str::to_owned).into();
        marker.validate_snapshot(&snapshot).unwrap();
        snapshot.applications[0].icon = Some("icons/other.png".into());
        assert!(marker.validate_snapshot(&snapshot).is_err());
        snapshot.applications[0].icon = ICONS[0].map(str::to_owned);
        snapshot.launch_ids.insert("another.application".into());
        assert!(marker.validate_snapshot(&snapshot).is_err());
    }
}
