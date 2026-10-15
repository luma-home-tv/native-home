//! Isolated owned-app artwork lifecycle test. No launcher, TV setter, Root
//! fallback or arbitrary file path. The operator changes only its own fixture.
use crate::{model::Page, runtime::Window, shell::Shell};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use std::{
    io::Write,
    os::unix::fs::{MetadataExt, OpenOptionsExt},
    path::Path,
    time::{Duration, Instant},
};

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Marker {
    pub schema: u32,
    pub target_id: String,
    pub encoded_sha256: [String; 3],
    pub versions: [String; 3],
    pub max_seconds: u32,
}
impl Marker {
    pub fn validate(&self) -> Result<(), &'static str> {
        if self.schema != 1
            || self.target_id != crate::identity::TARGET
            || self.versions != ["1.0.0", "1.0.0", "2.0.0"]
            || self.max_seconds != 120
            || self.encoded_sha256.iter().any(|v| {
                v.len() != 64
                    || !v
                        .bytes()
                        .all(|b| b.is_ascii_digit() || (b'a'..=b'f').contains(&b))
            })
            || self.encoded_sha256[0] == self.encoded_sha256[1]
            || self.encoded_sha256[0] == self.encoded_sha256[2]
            || self.encoded_sha256[1] == self.encoded_sha256[2]
        {
            return Err("Invalid fixed artwork lifecycle recipe");
        }
        Ok(())
    }
    pub fn read(path: &Path) -> Result<Self, Box<dyn std::error::Error>> {
        use std::io::Read;
        let file = std::fs::OpenOptions::new()
            .read(true)
            .custom_flags(libc::O_NOFOLLOW | libc::O_CLOEXEC | libc::O_NONBLOCK)
            .open(path)?;
        let before = file.metadata()?;
        if !before.is_file()
            || before.len() == 0
            || before.len() > 4096
            || before.uid() != 0
            || before.gid() != 0
            || before.nlink() != 1
            || before.mode() & 0o7777 != 0o644
        {
            return Err("Invalid installed live artwork proof marker".into());
        }
        let mut raw = Vec::new();
        (&file).take(4097).read_to_end(&mut raw)?;
        let after = file.metadata()?;
        if raw.len() as u64 != before.len()
            || (
                before.dev(),
                before.ino(),
                before.len(),
                before.mtime_nsec(),
                before.ctime_nsec(),
            ) != (
                after.dev(),
                after.ino(),
                after.len(),
                after.mtime_nsec(),
                after.ctime_nsec(),
            )
        {
            return Err("Artwork recipe changed during read".into());
        }
        let marker: Self = serde_json::from_slice(&raw)?;
        marker.validate()?;
        Ok(marker)
    }
}
fn save(output: &Path, name: &str, value: &Value) -> Result<(), Box<dyn std::error::Error>> {
    let destination = output.join(name);
    let temporary = output.join(format!("{name}.next"));
    if let Ok(meta) = std::fs::symlink_metadata(&destination) {
        if !meta.is_file()
            || meta.uid() != unsafe { libc::geteuid() }
            || meta.nlink() != 1
            || meta.mode() & 0o7777 != 0o600
        {
            return Err("Invalid existing artwork diagnostic destination".into());
        }
    }
    let mut file = std::fs::OpenOptions::new()
        .write(true)
        .create_new(true)
        .mode(0o600)
        .custom_flags(libc::O_NOFOLLOW | libc::O_CLOEXEC)
        .open(&temporary)?;
    serde_json::to_writer(&mut file, value)?;
    file.write_all(b"\n")?;
    file.sync_all()?;
    std::fs::rename(temporary, destination)?;
    Ok(())
}

pub fn run(
    shell: &mut Shell,
    window: &Window,
    output: &Path,
    marker: &Marker,
    mut poll_live: impl FnMut(&mut Shell),
) -> Result<Value, Box<dyn std::error::Error>> {
    marker.validate()?;
    let origin = Instant::now();
    let deadline = origin + Duration::from_secs(u64::from(marker.max_seconds));
    let preferences = shell.preferences().clone();
    let mut stages = Vec::new();
    shell.open(Page::Applications);
    let mut focus_set = false;
    for (index, phase) in ["a", "b", "c", "removed"].into_iter().enumerate() {
        loop {
            let frame_start = Instant::now();
            if frame_start >= deadline {
                return Err(format!("Artwork lifecycle timed out awaiting {phase}").into());
            }
            while let Some(event) = crate::runtime::poll() {
                if matches!(event.kind, 1 | 7 | 8) || matches!(event.kind, 2..=6 | 10 | 11) {
                    return Err(
                        "Artwork lifecycle interrupted by window departure or external input"
                            .into(),
                    );
                }
            }
            poll_live(shell);
            if !focus_set && shell.focus_application(&marker.target_id) {
                focus_set = true;
            }
            shell.tick(origin.elapsed().as_millis() as u32, 1.0 / 60.0);
            let target = shell.artwork_target(&marker.target_id, false);
            let accepted = if index < 3 {
                target.as_ref().is_some_and(|t| {
                    t["encoded_sha256"].as_str() == Some(&marker.encoded_sha256[index])
                        && t["source_key"]["version"].as_str() == Some(&marker.versions[index])
                        && t["width"] == 128
                        && t["height"] == 128
                })
            } else {
                shell.artwork_target_absent(&marker.target_id)
            };
            if shell.preferences() != &preferences {
                return Err("Artwork update changed test preferences".into());
            }
            if index < 3 && focus_set && shell.focused_item().as_deref() != Some(&marker.target_id)
            {
                return Err("Artwork update changed focused app".into());
            }
            shell.draw();
            window.swap();
            if accepted && shell.artwork_catalogue_current() {
                // Let focus/scroll springs settle before recording a visual
                // comparison. This is test settling, not an input-latency metric.
                for _ in 0..40 {
                    shell.tick(origin.elapsed().as_millis() as u32, 1.0 / 60.0);
                    shell.draw();
                    window.swap();
                }
                // Read the frame we just drew, before EGL can discard or
                // replace the back buffer during a swap.
                shell.draw();
                let graphics = window.graphics_state();
                if graphics.error != 0 {
                    return Err("Artwork frame has a GL error".into());
                }
                let receipt = shell.live_artwork_receipt();
                if receipt["upload_failures"] != 0
                    || receipt["gl"]["upload_failures"] != 0
                    || receipt["gl"]["deletion_failures"] != 0
                    || receipt["gl"]["observed_gl_errors"] != 0
                {
                    return Err("Artwork backend observed an allocation or GL error".into());
                }
                let name = format!("artwork-target-{phase}.ppm");
                window.capture(&output.join(&name))?;
                window.swap();
                let target = if index < 3 {
                    shell.artwork_target(&marker.target_id, true)
                } else {
                    None
                };
                let record = json!({"schema":1,"target_id":marker.target_id,"phase":phase,"stage_index":index+1,
                    "target":target,"runtime":receipt["runtime"],"textures":receipt["textures"],"gl":receipt["gl"],
                    "target_rect":shell.drawer_focus_bounds().map(|(r,_)|[r.x,r.y,r.w,r.h]),
                    "focused_item":shell.focused_item(),"preferences":shell.preferences(),"frame":name});
                save(output, "artwork-live-current.json", &record)?;
                stages.push(record);
                break;
            }
            let elapsed = frame_start.elapsed();
            if elapsed < Duration::from_micros(16667) {
                std::thread::sleep(Duration::from_micros(16667) - elapsed);
            }
        }
    }
    let shutdown = shell
        .finish_live_artwork()
        .ok_or("Live artwork runtime disappeared")?;
    if shutdown["joined"] != true
        || shutdown["panicked"] != false
        || shutdown["outstanding_resolved"] != true
        || shutdown["remaining_live_textures"] != 0
        || shutdown["gpu_cleanup_qualified"] != true
    {
        return Err("Live artwork worker/texture shutdown unqualified".into());
    }
    let result = json!({"schema":1,"qualified":true,"scope":"Owned developer-app ordinary I/O/decode/cache/GL lifecycle only",
        "stages":stages,"shutdown":shutdown,"reported_elapsed_ms":origin.elapsed().as_millis(),"normal_loader_qualified":false,
        "production_startup_qualified":false,"cross_jail_resource_route_used":false});
    save(output, "artwork-live-proof.json", &result)?;
    Ok(result)
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn recipe_cannot_select_other_app_path_or_timings() {
        let marker = Marker {
            schema: 1,
            target_id: crate::identity::TARGET.into(),
            encoded_sha256: ["a".repeat(64), "b".repeat(64), "c".repeat(64)],
            versions: ["1.0.0".into(), "1.0.0".into(), "2.0.0".into()],
            max_seconds: 120,
        };
        assert!(marker.validate().is_ok());
        let mut bad = marker.clone();
        bad.target_id = crate::identity::PREVIEW.into();
        assert!(bad.validate().is_err());
        let mut bad = marker.clone();
        bad.max_seconds = 3600;
        assert!(bad.validate().is_err());
        let mut bad = marker.clone();
        bad.encoded_sha256[1] = bad.encoded_sha256[0].clone();
        assert!(bad.validate().is_err());
        let mut value = serde_json::to_value(marker).unwrap();
        value["path"] = json!("/tmp/caller");
        assert!(serde_json::from_value::<Marker>(value).is_err());
    }
}
