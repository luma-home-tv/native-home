//! Isolated read-only service capability observation; never starts a broker.
use crate::{model::Page, runtime::Window, shell::Shell};
use serde::Deserialize;
use serde_json::Value;
use std::{
    io::{Read, Write},
    os::unix::fs::{MetadataExt, OpenOptionsExt},
    path::Path,
    time::{Duration, Instant},
};

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Marker {
    schema: u32,
}
impl Marker {
    pub fn read(path: &Path) -> Result<Self, Box<dyn std::error::Error>> {
        let file = std::fs::OpenOptions::new()
            .read(true)
            .custom_flags(libc::O_NOFOLLOW | libc::O_CLOEXEC | libc::O_NONBLOCK)
            .open(path)?;
        let a = file.metadata()?;
        if !a.is_file()
            || a.len() == 0
            || a.len() > 128
            || a.uid() != 0
            || a.gid() != 0
            || a.nlink() != 1
            || a.mode() & 0o7777 != 0o644
        {
            return Err("Invalid installed startup capability marker".into());
        }
        let mut bytes = Vec::new();
        (&file).take(129).read_to_end(&mut bytes)?;
        let b = file.metadata()?;
        if bytes.len() as u64 != a.len()
            || (
                a.dev(),
                a.ino(),
                a.len(),
                a.mode(),
                a.uid(),
                a.gid(),
                a.nlink(),
                a.mtime(),
                a.mtime_nsec(),
                a.ctime(),
                a.ctime_nsec(),
            ) != (
                b.dev(),
                b.ino(),
                b.len(),
                b.mode(),
                b.uid(),
                b.gid(),
                b.nlink(),
                b.mtime(),
                b.mtime_nsec(),
                b.ctime(),
                b.ctime_nsec(),
            )
        {
            return Err("Startup capability marker changed during read".into());
        }
        let marker: Self = serde_json::from_slice(&bytes)?;
        if marker.schema != 1 {
            return Err("Unsupported startup capability schema".into());
        }
        Ok(marker)
    }
}

pub fn run(
    shell: &mut Shell,
    window: &Window,
    output: &Path,
    now: &mut u32,
) -> Result<Value, Box<dyn std::error::Error>> {
    let preferences = shell.preferences().clone();
    shell.open(Page::Home);
    let worker = std::thread::Builder::new()
        .name("artwork-startup-check".into())
        .spawn(crate::artwork_startup_probe::collect)?;
    let start = Instant::now();
    let mut interrupted = false;
    // The operator binds /proc identity while this single Home card is alive.
    // LS2 setup/cancellation/destruction are outside its reply-wait budget;
    // this loop deliberately makes no hard process-lifetime claim.
    while start.elapsed() < Duration::from_secs(8) || !worker.is_finished() {
        while let Some(event) = crate::runtime::poll() {
            if matches!(event.kind, 1..=8 | 10 | 11) {
                interrupted = true;
            }
        }
        if !interrupted {
            *now = now.wrapping_add(17);
            shell.tick(*now, 1.0 / 60.0);
            shell.draw();
            window.swap();
        }
        std::thread::sleep(Duration::from_millis(17));
    }
    let mut report = worker
        .join()
        .map_err(|_| "Startup capability worker panicked")?;
    report["worker_joined"] = true.into();
    report["ui_hold_elapsed_ms"] = serde_json::json!(start.elapsed().as_millis());
    report["interrupted"] = interrupted.into();
    let mut file = std::fs::OpenOptions::new()
        .write(true)
        .create_new(true)
        .mode(0o600)
        .custom_flags(libc::O_NOFOLLOW | libc::O_CLOEXEC)
        .open(output.join("artwork-startup-probe.json"))?;
    serde_json::to_writer_pretty(&mut file, &report)?;
    file.write_all(b"\n")?;
    file.sync_all()?;
    if interrupted
        || shell.preferences() != &preferences
        || !shell.take_requests().is_empty()
        || shell.should_exit()
    {
        return Err("Startup capability observation interrupted or changed UI state".into());
    }
    shell.draw();
    if window.graphics_state().error != 0 {
        return Err("Startup capability capture GL error".into());
    }
    window.capture(&output.join("startup-capability-home.ppm"))?;
    window.swap();
    Ok(report)
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn marker_rejects_duplicate_unknown_coerced_and_missing_fields() {
        assert_eq!(
            serde_json::from_str::<Marker>(r#"{"schema":1}"#)
                .unwrap()
                .schema,
            1
        );
        for raw in [
            r#"{}"#,
            r#"{"schema":"1"}"#,
            r#"{"schema":true}"#,
            r#"{"schema":1,"schema":1}"#,
            r#"{"schema":1,"command":"x"}"#,
        ] {
            assert!(serde_json::from_str::<Marker>(raw).is_err());
        }
    }
}
