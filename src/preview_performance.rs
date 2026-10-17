//! Opt-in passive Preview observations, never a capability or Root grant.
//! Default Preview and the pre-existing Proof selector do not select this worker.
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use std::{
    fs::{File, OpenOptions},
    io::{self, Read, Write},
    os::{
        fd::{AsRawFd, FromRawFd},
        unix::fs::{MetadataExt, OpenOptionsExt},
    },
    path::Path,
    sync::mpsc::{self, SyncSender, TrySendError},
    thread::{self, JoinHandle},
};
pub const NAME: &str = "preview-performance-observer.json";
pub const LIMIT_SECONDS: u64 = 240;
pub const SNAPSHOT_MS: u64 = 1000;
const CAP: usize = 16384;
fn invalid(message: &str) -> io::Error {
    io::Error::new(io::ErrorKind::InvalidData, message)
}
#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct Marker {
    schema: u32,
    lifetime_seconds: u64,
    snapshot_interval_ms: u64,
}
impl Marker {
    pub fn parse(raw: &[u8]) -> io::Result<Self> {
        if raw.len() > 256 {
            return Err(invalid("Observer marker bound"));
        }
        let m: Self =
            serde_json::from_slice(raw).map_err(|_| invalid("Closed observer marker schema"))?;
        if m.schema != 1
            || m.lifetime_seconds != LIMIT_SECONDS
            || m.snapshot_interval_ms != SNAPSHOT_MS
        {
            return Err(invalid("Fixed passive observer policy"));
        }
        Ok(m)
    }
}
/// Called before Window, persistence, ordinary or diagnostic workers are created.
/// Presence selects only a strict installed marker, never an env/CLI switch.
pub fn load(
    assets: &Path,
    device: bool,
    interactive: bool,
    app: &str,
    cli_switch: bool,
) -> io::Result<Option<Marker>> {
    let path = assets.join(NAME);
    let m = match std::fs::symlink_metadata(&path) {
        Ok(m) => m,
        Err(e) if e.kind() == io::ErrorKind::NotFound => return Ok(None),
        Err(e) => return Err(e),
    };
    if !device || !interactive || app != crate::identity::PREVIEW || cli_switch {
        return Err(invalid(
            "Passive observer requires native ordinary interactive Preview",
        ));
    }
    if !m.is_file()
        || m.nlink() != 1
        || m.len() > 256
        || !matches!(m.mode() & 0o7777, 0o444 | 0o644)
    {
        return Err(invalid("Private installed ordinary marker"));
    }
    const ALLOWED: &[&str] = &[
        "appinfo.json",
        "snapshot.json",
        "interactive-session.json",
        "artwork-cache-producer.json",
        "artwork-cache-consumer.json",
        NAME,
    ];
    for entry in std::fs::read_dir(assets)? {
        let entry = entry?;
        let name = entry.file_name();
        let name = name.to_str().ok_or_else(|| invalid("Asset UTF8"))?;
        if name.ends_with(".json") && !ALLOWED.contains(&name) {
            return Err(invalid(
                "Passive observer cannot mix diagnostic/capture/synthetic markers",
            ));
        }
    }
    if !assets.join("interactive-session.json").is_file()
        || !assets.join("artwork-cache-producer.json").is_file()
    {
        return Err(invalid(
            "Installed interactive and ordinary cache association required",
        ));
    }
    let mut f = OpenOptions::new()
        .read(true)
        .custom_flags(libc::O_NOFOLLOW | libc::O_CLOEXEC | libc::O_NONBLOCK)
        .open(&path)?;
    let a = f.metadata()?;
    let mut raw = Vec::new();
    Read::by_ref(&mut f).take(257).read_to_end(&mut raw)?;
    let z = f.metadata()?;
    let named = std::fs::symlink_metadata(path)?;
    if stamp(&a) != stamp(&m)
        || stamp(&a) != stamp(&z)
        || stamp(&a) != stamp(&named)
        || raw.len() as u64 != a.len()
    {
        return Err(invalid("Observer marker changed"));
    }
    Marker::parse(&raw).map(Some)
}
/// Observed actual native selections, not a replacement for those selections.
#[derive(Clone, Copy, Serialize)]
pub struct NormalProfile {
    pub live_worker: bool,
    pub action_worker: bool,
    pub pointer_worker: bool,
    pub description_worker: bool,
    pub app_launches: bool,
    pub home_launches: bool,
    pub ordinary_cache: bool,
}
impl NormalProfile {
    pub fn validate(self) -> io::Result<Self> {
        if [
            self.live_worker,
            self.action_worker,
            self.pointer_worker,
            self.description_worker,
            self.app_launches,
            self.home_launches,
            self.ordinary_cache,
        ]
        .iter()
        .all(|x| *x)
        {
            Ok(self)
        } else {
            Err(invalid(
                "Passive observer must retain actual normal workers/capabilities/cache selection",
            ))
        }
    }
}
fn stamp(m: &std::fs::Metadata) -> (u64, u64, u32, u32, u32, u64, u64, i64, i64, i64, i64) {
    (
        m.dev(),
        m.ino(),
        m.uid(),
        m.gid(),
        m.mode(),
        m.nlink(),
        m.len(),
        m.mtime(),
        m.mtime_nsec(),
        m.ctime(),
        m.ctime_nsec(),
    )
}
#[derive(Default, Debug, Serialize)]
pub struct WriterReport {
    pub published: u64,
    pub dropped: u64,
    pub worker_joined: bool,
    pub worker_panicked: bool,
    pub error: Option<String>,
    pub io_on_snapshot_worker: bool,
    pub snapshot_bytes_limit: usize,
    pub durable_snapshot_claimed: bool,
}
pub struct Writer {
    tx: Option<SyncSender<Value>>,
    worker: Option<JoinHandle<WriterReport>>,
    dropped: u64,
}
fn name(value: &str) -> std::ffi::CString {
    std::ffi::CString::new(value).expect("Static filename")
}
fn open_at(dir: &File, n: &str, flags: i32) -> io::Result<File> {
    let fd = unsafe {
        libc::openat(
            dir.as_raw_fd(),
            name(n).as_ptr(),
            flags | libc::O_NOFOLLOW | libc::O_CLOEXEC | libc::O_NONBLOCK,
            0o600,
        )
    };
    if fd < 0 {
        Err(io::Error::last_os_error())
    } else {
        Ok(unsafe { File::from_raw_fd(fd) })
    }
}
fn existing(dir: &File, n: &str) -> io::Result<Option<File>> {
    match open_at(dir, n, libc::O_RDONLY) {
        Ok(f) => Ok(Some(f)),
        Err(e) if e.kind() == io::ErrorKind::NotFound => Ok(None),
        Err(e) => Err(e),
    }
}
impl Writer {
    /// Startup only; the source-created shell-PID directory must still be own0700.
    pub fn start(output: &Path) -> io::Result<Self> {
        let dir = OpenOptions::new()
            .read(true)
            .custom_flags(libc::O_DIRECTORY | libc::O_NOFOLLOW | libc::O_CLOEXEC)
            .open(output)?;
        let m = dir.metadata()?;
        if !m.is_dir()
            || m.uid() != unsafe { libc::geteuid() }
            || m.gid() != unsafe { libc::getegid() }
            || m.mode() & 0o7777 != 0o700
        {
            return Err(invalid("Own private observer output directory"));
        }
        for n in ["performance-current.json", "performance-current-next.json"] {
            if existing(&dir, n)?.is_some() {
                return Err(invalid("Observer refuses existing output"));
            }
        }
        let (tx, rx) = mpsc::sync_channel::<Value>(1);
        let worker = thread::Builder::new()
            .name("home-passive-metrics".into())
            .spawn(move || {
                let mut report = WriterReport {
                    io_on_snapshot_worker: true,
                    snapshot_bytes_limit: CAP,
                    ..WriterReport::default()
                };
                let mut prior = None;
                for snapshot in rx {
                    let result = (|| -> io::Result<()> {
                        let raw = serde_json::to_vec(&snapshot)?;
                        if raw.len() > CAP {
                            return Err(invalid("Observer snapshot bound"));
                        }
                        let mut f = open_at(
                            &dir,
                            "performance-current-next.json",
                            libc::O_WRONLY | libc::O_CREAT | libc::O_EXCL,
                        )?;
                        let a = f.metadata()?;
                        if !a.is_file()
                            || a.uid() != unsafe { libc::geteuid() }
                            || a.gid() != unsafe { libc::getegid() }
                            || a.nlink() != 1
                            || a.mode() & 0o7777 != 0o600
                        {
                            return Err(invalid("Observer file identity"));
                        }
                        f.write_all(&raw)?;
                        f.flush()?;
                        let written = f.metadata()?;
                        if written.dev() != a.dev()
                            || written.ino() != a.ino()
                            || written.nlink() != 1
                            || written.len() != raw.len() as u64
                        {
                            return Err(invalid("Observer file changed"));
                        }
                        let current = existing(&dir, "performance-current.json")?;
                        if current
                            .as_ref()
                            .map(|f| f.metadata().map(|m| stamp(&m)))
                            .transpose()?
                            != prior
                        {
                            return Err(invalid("Observer current epoch changed"));
                        }
                        if unsafe {
                            libc::renameat(
                                dir.as_raw_fd(),
                                name("performance-current-next.json").as_ptr(),
                                dir.as_raw_fd(),
                                name("performance-current.json").as_ptr(),
                            )
                        } != 0
                        {
                            return Err(io::Error::last_os_error());
                        }
                        let current = existing(&dir, "performance-current.json")?
                            .ok_or_else(|| invalid("Published observer path absent"))?;
                        let cm = current.metadata()?;
                        if cm.dev() != written.dev()
                            || cm.ino() != written.ino()
                            || cm.len() != written.len()
                            || cm.nlink() != 1
                        {
                            return Err(invalid("Observer rename identity"));
                        }
                        prior = Some(stamp(&cm));
                        Ok(())
                    })();
                    if let Err(e) = result {
                        report.error = Some(e.to_string());
                        break;
                    }
                    report.published += 1;
                }
                report
            })?;
        Ok(Self {
            tx: Some(tx),
            worker: Some(worker),
            dropped: 0,
        })
    }
    /// Bounded one-slot queue: no file access, serialization or waiting on IO here.
    pub fn submit(&mut self, snapshot: Value) {
        if let Some(tx) = &self.tx {
            match tx.try_send(snapshot) {
                Ok(()) => {}
                Err(TrySendError::Full(_) | TrySendError::Disconnected(_)) => {
                    self.dropped = self.dropped.saturating_add(1)
                }
            }
        }
    }
    pub fn finish(mut self) -> WriterReport {
        self.tx.take();
        let mut r = match self.worker.take().expect("Owned worker").join() {
            Ok(r) => r,
            Err(_) => WriterReport {
                worker_panicked: true,
                ..WriterReport::default()
            },
        };
        r.worker_joined = true;
        r.dropped = self.dropped;
        r
    }
}
impl Drop for Writer {
    fn drop(&mut self) {
        self.tx.take();
        if let Some(w) = self.worker.take() {
            let _ = w.join();
        }
    }
}
pub fn snapshot(
    elapsed: f64,
    page: &str,
    focus: Option<&str>,
    category: &str,
    hidden: bool,
    blank: bool,
    metrics: Value,
) -> Value {
    json!({"schema":1,"profile":"preview-passive-observer","elapsed_seconds":elapsed,"page":page,"focus":focus.filter(|v|v.len()<=256),"focus_unavailable":focus.is_some_and(|v|v.len()>256),"category":category,"phase":if hidden{"hidden"}else if blank{"idle_black"}else{"foreground"},"metrics":metrics,"source":"native collect origin; whole-session summaries, no phase percentile subtraction","physical_or_GPU_completion_qualified":false})
}
#[cfg(test)]
#[path = "preview_performance_tests.rs"]
mod tests;
