//! Passive, isolated main-loop observation. No text payloads, input injection, SDL,
//! service calls, timing changes or presentation invalidation belong in this module.
//!
//! Identities are validated package-marker declarations, not independent measurements.
//! The device owner must compare them with the installed package and actual executable.
//! `counts.received` counts adapter events supplied by the caller, not raw SDL events
//! rejected inside the C adapter. Received SDL origin remains unclassified; a separate
//! enqueue counter cannot prove which received event was synthetic.
use serde::{Deserialize, Serialize};
use std::ffi::CString;
use std::fs::{File, OpenOptions};
use std::io::{self, Write};
use std::os::fd::{AsRawFd, FromRawFd};
use std::os::unix::fs::{MetadataExt, OpenOptionsExt, PermissionsExt};
use std::path::Path;

pub const CURRENT_FILE: &str = "keyboard-observer-current.json";
pub const FINAL_FILE: &str = "keyboard-observer-final.json";
const OWNER_FILE: &str = ".keyboard-observer-owner";
const CHANGE_LIMIT: usize = 128;
const CURRENT_PERIOD_MS: u64 = 1_000;

macro_rules! roles {
    ($name:ident { $default:ident $(, $variant:ident)* $(,)? }) => {
        #[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize)]
        #[serde(rename_all = "snake_case")]
        pub enum $name { #[default] $default, $($variant),* }
    };
}
roles!(PageRole {
    Home,
    Apps,
    Inputs,
    Controls,
    Other
});
roles!(FocusRole {
    None,
    Home,
    Apps,
    Search,
    Keypad,
    Field,
    KeypadKey,
    Rail,
    Toolbar,
    DrawerItem,
    Menu,
    SettingsControl,
    Other
});
roles!(EditMode {
    None,
    System,
    Keypad
});
roles!(FixtureMatch {
    Other,
    Empty,
    Lu,
    Luma,
    LumaA
});
roles!(EventClass {
    Other,
    KeyDown,
    KeyUp,
    PointerMotion,
    PointerClick,
    Wheel,
    TextCommit,
    Preedit,
    Window,
    Quit
});
roles!(Decision {
    Ignored,
    Accepted,
    Drain,
    Hidden,
    NotEditing,
    Malformed
});
roles!(Origin { NativeUnclassified });
roles!(ExitReason {
    Other,
    BackFromHome,
    WindowClosed,
    WindowHidden,
    ObservationLimit,
    Error
});

impl FixtureMatch {
    /// Match only fixed benign fixtures, retaining neither input bytes nor a hash.
    pub fn classify(text: &str) -> Self {
        match text {
            "" => Self::Empty,
            "lu" => Self::Lu,
            "luma" => Self::Luma,
            "lumaA" => Self::LumaA,
            _ => Self::Other,
        }
    }
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize)]
pub struct Stamp {
    pub elapsed_ms: u64,
    pub controller_ms: u64,
    pub event_seq: u64,
    pub batch_seq: u64,
}
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize)]
pub struct KeyboardCounters {
    pub opens: u64,
    pub stops: u64,
    pub dismissals: u64,
    pub failures: u64,
}
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize)]
pub struct DraftMetrics {
    pub bytes: u16,
    pub caret: u16,
    pub preedit_bytes: u16,
    pub fixture: FixtureMatch,
}
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize)]
pub struct LoopCounters {
    pub draws: u64,
    pub swaps: u64,
    pub idle_presentations: u64,
    pub hidden_presentations: u64,
    pub live_cycles: u64,
    pub preference_write_attempts: u64,
    pub external_action_attempts: u64,
    /// Fixed diagnostic enqueue acknowledgements; not received-event attribution.
    pub synthetic_recipe_queued: u64,
}
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize)]
pub struct Snapshot {
    pub page: PageRole,
    pub focus: FocusRole,
    pub mode: EditMode,
    pub hidden: bool,
    pub idle: bool,
    pub editing: bool,
    pub requested: bool,
    pub owned: bool,
    pub compatibility_admitted: bool,
    /// SDL flags are metadata, not independent compositor-visibility evidence.
    pub sdk_active: bool,
    pub sdk_panel_flag: bool,
    pub sdk_focused: bool,
    pub keyboard: KeyboardCounters,
    pub draft: DraftMetrics,
    pub filtered_count: u32,
    pub fallback_targets: u32,
    pub counters: LoopCounters,
}
impl Snapshot {
    fn transition_eq(&self, other: &Self) -> bool {
        // Draw/live/etc. counters change during normal ticks. They belong in the
        // current summary, not in a change log that would fill in two seconds.
        let mut a = *self;
        let mut b = *other;
        a.counters = LoopCounters::default();
        b.counters = LoopCounters::default();
        a == b
    }
}
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
pub struct EventDecision {
    pub class: EventClass,
    pub decision: Decision,
    pub origin: Origin,
}
#[derive(Clone, Copy, Debug, Default, Serialize)]
pub struct ClassCounts {
    pub other: u64,
    pub key_down: u64,
    pub key_up: u64,
    pub pointer_motion: u64,
    pub pointer_click: u64,
    pub wheel: u64,
    pub text_commit: u64,
    pub preedit: u64,
    pub window: u64,
    pub quit: u64,
}
#[derive(Clone, Copy, Debug, Default, Serialize)]
pub struct EventCounts {
    pub received: u64,
    pub accepted: u64,
    pub ignored: u64,
    pub drain: u64,
    pub hidden: u64,
    pub not_editing: u64,
    pub malformed: u64,
    pub native_unclassified: u64,
    pub by_class: ClassCounts,
}
fn increment(count: &mut u64) {
    *count = count.saturating_add(1);
}
impl EventCounts {
    fn add(&mut self, event: EventDecision) {
        increment(&mut self.received);
        match event.origin {
            Origin::NativeUnclassified => increment(&mut self.native_unclassified),
        }
        increment(match event.decision {
            Decision::Accepted => &mut self.accepted,
            Decision::Ignored => &mut self.ignored,
            Decision::Drain => &mut self.drain,
            Decision::Hidden => &mut self.hidden,
            Decision::NotEditing => &mut self.not_editing,
            Decision::Malformed => &mut self.malformed,
        });
        increment(match event.class {
            EventClass::Other => &mut self.by_class.other,
            EventClass::KeyDown => &mut self.by_class.key_down,
            EventClass::KeyUp => &mut self.by_class.key_up,
            EventClass::PointerMotion => &mut self.by_class.pointer_motion,
            EventClass::PointerClick => &mut self.by_class.pointer_click,
            EventClass::Wheel => &mut self.by_class.wheel,
            EventClass::TextCommit => &mut self.by_class.text_commit,
            EventClass::Preedit => &mut self.by_class.preedit,
            EventClass::Window => &mut self.by_class.window,
            EventClass::Quit => &mut self.by_class.quit,
        });
    }
}
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Marker {
    schema: u32,
    source_revision: String,
    binary_sha256: String,
}
#[derive(Serialize)]
struct DeclaredIdentity {
    source_revision: String,
    binary_sha256: String,
    validation: &'static str,
}
#[derive(Clone, Copy, Serialize)]
struct Producer {
    pid: u32,
    uid: u32,
}
#[derive(Clone, Copy, Serialize)]
struct Observation {
    stamp: Stamp,
    state: Snapshot,
}
#[derive(Clone, Copy, Serialize)]
struct Change {
    stamp: Stamp,
    state: Snapshot,
    event: Option<EventDecision>,
}
#[derive(Serialize)]
struct Envelope<'a> {
    schema: u32,
    #[serde(rename = "loop")]
    loop_name: &'static str,
    // Constructor admission declarations; actual caller hooks and marker gates
    // must establish execution of the real loop without a helper.
    interactive: bool,
    helper_executions: u32,
    execution_validation: &'static str,
    producer: Producer,
    declared_identity: &'a DeclaredIdentity,
    written_at_ms: u64,
    stamp: Stamp,
    state: Snapshot,
    counts: EventCounts,
    changes: &'a [Change],
    overflow: bool,
    dropped_changes: u64,
    #[serde(skip_serializing_if = "Option::is_none")]
    exit_reason: Option<ExitReason>,
}

pub struct Observer {
    directory: File,
    _owner: File,
    producer: Producer,
    declared: DeclaredIdentity,
    latest: Option<Observation>,
    counts: EventCounts,
    changes: Vec<Change>,
    dropped_changes: u64,
    last_call_elapsed: u64,
    last_current_write: Option<u64>,
    temporary_sequence: u64,
    finished: bool,
}
fn invalid(message: &'static str) -> io::Error {
    io::Error::new(io::ErrorKind::InvalidInput, message)
}
fn hex(value: &str, length: usize) -> bool {
    value.len() == length
        && value
            .bytes()
            .all(|b| b.is_ascii_digit() || (b'a'..=b'f').contains(&b))
}
impl Observer {
    pub fn open(
        runtime_dir: &Path,
        app_id: &str,
        interactive: bool,
        marker_json: &[u8],
    ) -> io::Result<Self> {
        if app_id != crate::identity::PROOF || !interactive {
            return Err(io::Error::new(
                io::ErrorKind::PermissionDenied,
                "observer requires interactive Proof identity",
            ));
        }
        if marker_json.len() > 512 {
            return Err(invalid("observer marker exceeds its bounded schema"));
        }
        let marker: Marker = serde_json::from_slice(marker_json)
            .map_err(|_| invalid("invalid observer marker schema"))?;
        if marker.schema != 1
            || !hex(&marker.source_revision, 40)
            || !hex(&marker.binary_sha256, 64)
        {
            return Err(invalid(
                "observer marker needs schema 1 and lowercase hex declarations",
            ));
        }
        let directory = OpenOptions::new()
            .read(true)
            .custom_flags(libc::O_DIRECTORY | libc::O_NOFOLLOW | libc::O_CLOEXEC)
            .open(runtime_dir)?;
        let producer = Producer {
            pid: unsafe { libc::getpid() } as u32,
            uid: unsafe { libc::geteuid() },
        };
        let meta = directory.metadata()?;
        if !meta.is_dir() || meta.uid() != producer.uid || meta.mode() & 0o7777 != 0o700 {
            return Err(io::Error::new(
                io::ErrorKind::PermissionDenied,
                "observer runtime must be an owned 0700 directory",
            ));
        }
        // Names are fixed and all I/O is anchored to the opened directory FD. A
        // renamed/replaced caller path cannot redirect a later snapshot write.
        let owner = create_at(&directory, OWNER_FILE)?;
        let observer = Self {
            directory,
            _owner: owner,
            producer,
            declared: DeclaredIdentity {
                source_revision: marker.source_revision,
                binary_sha256: marker.binary_sha256,
                validation: "package_marker_declaration",
            },
            latest: None,
            counts: EventCounts::default(),
            changes: Vec::with_capacity(CHANGE_LIMIT),
            dropped_changes: 0,
            last_call_elapsed: 0,
            last_current_write: None,
            temporary_sequence: 0,
            finished: false,
        };
        if observer.destination_exists(CURRENT_FILE)? || observer.destination_exists(FINAL_FILE)? {
            return Err(io::Error::new(
                io::ErrorKind::AlreadyExists,
                "observer requires a new runtime output set",
            ));
        }
        Ok(observer)
    }

    pub fn record(
        &mut self,
        stamp: Stamp,
        state: Snapshot,
        event: Option<EventDecision>,
    ) -> io::Result<()> {
        if self.finished {
            return Err(invalid("observer already finished"));
        }
        if stamp.elapsed_ms < self.last_call_elapsed
            || state.draft.bytes > 256
            || state.draft.preedit_bytes > 256
            || state.draft.caret > state.draft.bytes
        {
            return Err(invalid("invalid observer clock or draft metadata"));
        }
        if let Some(last) = self.latest {
            if stamp.controller_ms < last.stamp.controller_ms
                || stamp.event_seq < last.stamp.event_seq
                || stamp.batch_seq < last.stamp.batch_seq
                || (event.is_some() && stamp.event_seq == last.stamp.event_seq)
                || !counters_follow(state, last.state)
            {
                return Err(invalid("observer sequence or counters regressed"));
            }
        }
        let changed = self
            .latest
            .is_none_or(|last| !state.transition_eq(&last.state));
        let interesting_event = event.is_some_and(|event| {
            matches!(
                event.class,
                EventClass::KeyDown
                    | EventClass::KeyUp
                    | EventClass::PointerClick
                    | EventClass::TextCommit
                    | EventClass::Preedit
            ) || !matches!(event.decision, Decision::Accepted | Decision::Ignored)
        });
        if changed || interesting_event {
            if self.changes.len() < CHANGE_LIMIT {
                self.changes.push(Change {
                    stamp,
                    state,
                    event,
                });
            } else {
                increment(&mut self.dropped_changes);
            }
        }
        if let Some(event) = event {
            self.counts.add(event);
        }
        self.last_call_elapsed = stamp.elapsed_ms;
        self.latest = Some(Observation { stamp, state });
        Ok(())
    }

    /// First flush is immediate; later current writes are separated by >=1000 ms.
    /// Recording alone never writes a file or generates a presentation request.
    pub fn flush_current(&mut self, elapsed_ms: u64) -> io::Result<bool> {
        if self.finished || elapsed_ms < self.last_call_elapsed {
            return Err(invalid("invalid observer flush state/clock"));
        }
        self.last_call_elapsed = elapsed_ms;
        if self
            .last_current_write
            .is_some_and(|at| elapsed_ms - at < CURRENT_PERIOD_MS)
        {
            return Ok(false);
        }
        self.write_snapshot(CURRENT_FILE, elapsed_ms, None)?;
        self.last_current_write = Some(elapsed_ms);
        Ok(true)
    }

    /// Final summary is separate and bypasses the current-file rate limit. It is
    /// not overwritten by a second finish or another producer in the same runtime.
    pub fn finish(&mut self, stamp: Stamp, state: Snapshot, reason: ExitReason) -> io::Result<()> {
        self.record(stamp, state, None)?;
        self.write_snapshot(FINAL_FILE, stamp.elapsed_ms, Some(reason))?;
        self.finished = true;
        Ok(())
    }

    fn destination_exists(&self, name: &str) -> io::Result<bool> {
        let name = CString::new(name).expect("internal fixed filename");
        let mut stat = std::mem::MaybeUninit::<libc::stat>::uninit();
        if unsafe {
            libc::fstatat(
                self.directory.as_raw_fd(),
                name.as_ptr(),
                stat.as_mut_ptr(),
                libc::AT_SYMLINK_NOFOLLOW,
            )
        } != 0
        {
            let error = io::Error::last_os_error();
            return if error.kind() == io::ErrorKind::NotFound {
                Ok(false)
            } else {
                Err(error)
            };
        }
        let stat = unsafe { stat.assume_init() };
        if stat.st_mode as u32 & libc::S_IFMT as u32 != libc::S_IFREG as u32
            || stat.st_uid != self.producer.uid
            || stat.st_mode as u32 & 0o7777 != 0o600
            || stat.st_nlink != 1
        {
            return Err(io::Error::new(
                io::ErrorKind::PermissionDenied,
                "observer destination is not a private owned regular file",
            ));
        }
        Ok(true)
    }

    fn write_snapshot(
        &mut self,
        name: &str,
        written_at_ms: u64,
        reason: Option<ExitReason>,
    ) -> io::Result<()> {
        let latest = self
            .latest
            .ok_or_else(|| invalid("observer has no snapshot"))?;
        if self.destination_exists(name)? && name == FINAL_FILE {
            return Err(io::Error::new(
                io::ErrorKind::AlreadyExists,
                "observer final summary already exists",
            ));
        }
        self.temporary_sequence = self
            .temporary_sequence
            .checked_add(1)
            .ok_or_else(|| invalid("observer temporary sequence exhausted"))?;
        let temporary = format!(
            ".keyboard-observer-next-{}-{}",
            self.producer.pid, self.temporary_sequence
        );
        let mut file = create_at(&self.directory, &temporary)?;
        let result = (|| {
            let envelope = Envelope {
                schema: 1,
                loop_name: "main_interactive",
                interactive: true,
                helper_executions: 0,
                execution_validation: "constructor_declaration",
                producer: self.producer,
                declared_identity: &self.declared,
                written_at_ms,
                stamp: latest.stamp,
                state: latest.state,
                counts: self.counts,
                changes: &self.changes,
                overflow: self.dropped_changes != 0,
                dropped_changes: self.dropped_changes,
                exit_reason: reason,
            };
            serde_json::to_writer(&mut file, &envelope).map_err(io::Error::other)?;
            file.write_all(b"\n")?;
            file.flush()?;
            if reason.is_some() {
                file.sync_all()?;
            }
            // Recheck without following links before replacing our current file.
            if self.destination_exists(name)? && name == FINAL_FILE {
                return Err(io::Error::new(
                    io::ErrorKind::AlreadyExists,
                    "observer final summary already exists",
                ));
            }
            let from = CString::new(temporary.as_str()).expect("internal filename");
            let to = CString::new(name).expect("internal fixed filename");
            if unsafe {
                libc::renameat(
                    self.directory.as_raw_fd(),
                    from.as_ptr(),
                    self.directory.as_raw_fd(),
                    to.as_ptr(),
                )
            } != 0
            {
                return Err(io::Error::last_os_error());
            }
            Ok(())
        })();
        if result.is_err() {
            let temporary = CString::new(temporary).expect("internal filename");
            unsafe {
                libc::unlinkat(self.directory.as_raw_fd(), temporary.as_ptr(), 0);
            }
        }
        result
    }
}
fn create_at(directory: &File, name: &str) -> io::Result<File> {
    let name = CString::new(name).expect("internal filename");
    let fd = unsafe {
        libc::openat(
            directory.as_raw_fd(),
            name.as_ptr(),
            libc::O_WRONLY | libc::O_CREAT | libc::O_EXCL | libc::O_NOFOLLOW | libc::O_CLOEXEC,
            0o600,
        )
    };
    if fd < 0 {
        Err(io::Error::last_os_error())
    } else {
        let file = unsafe { File::from_raw_fd(fd) };
        file.set_permissions(std::fs::Permissions::from_mode(0o600))?;
        Ok(file)
    }
}
fn counters_follow(now: Snapshot, last: Snapshot) -> bool {
    let a = now.keyboard;
    let b = last.keyboard;
    let x = now.counters;
    let y = last.counters;
    a.opens >= b.opens
        && a.stops >= b.stops
        && a.dismissals >= b.dismissals
        && a.failures >= b.failures
        && x.draws >= y.draws
        && x.swaps >= y.swaps
        && x.idle_presentations >= y.idle_presentations
        && x.hidden_presentations >= y.hidden_presentations
        && x.live_cycles >= y.live_cycles
        && x.preference_write_attempts >= y.preference_write_attempts
        && x.external_action_attempts >= y.external_action_attempts
        && x.synthetic_recipe_queued >= y.synthetic_recipe_queued
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::os::unix::fs::{symlink, PermissionsExt};
    use std::path::PathBuf;
    use std::sync::atomic::{AtomicU64, Ordering};
    static NEXT: AtomicU64 = AtomicU64::new(0);
    struct PrivateDir(PathBuf);
    impl PrivateDir {
        fn new() -> Self {
            loop {
                let path = std::env::temp_dir().join(format!(
                    "luma-keyboard-observer-{}-{}",
                    unsafe { libc::getpid() },
                    NEXT.fetch_add(1, Ordering::Relaxed)
                ));
                match std::fs::create_dir(&path) {
                    Ok(()) => {
                        std::fs::set_permissions(&path, std::fs::Permissions::from_mode(0o700))
                            .unwrap();
                        return Self(path);
                    }
                    Err(e) if e.kind() == io::ErrorKind::AlreadyExists => continue,
                    Err(e) => panic!("private test directory: {e}"),
                }
            }
        }
    }
    impl Drop for PrivateDir {
        fn drop(&mut self) {
            let _ = std::fs::remove_dir_all(&self.0);
        }
    }
    fn marker() -> Vec<u8> {
        serde_json::to_vec(&serde_json::json!({"schema":1,"source_revision":"a".repeat(40),"binary_sha256":"b".repeat(64)})).unwrap()
    }
    fn open(dir: &PrivateDir) -> Observer {
        Observer::open(&dir.0, crate::identity::PROOF, true, &marker()).unwrap()
    }
    fn stamp(ms: u64, seq: u64) -> Stamp {
        Stamp {
            elapsed_ms: ms,
            controller_ms: ms,
            event_seq: seq,
            batch_seq: seq,
        }
    }
    fn read(dir: &PrivateDir, name: &str) -> serde_json::Value {
        serde_json::from_slice(&std::fs::read(dir.0.join(name)).unwrap()).unwrap()
    }
    #[test]
    fn admission_rejects_nonproof_noninteractive_and_every_unqualified_marker() {
        let dir = PrivateDir::new();
        for (id, interactive) in [
            (crate::identity::PREVIEW, true),
            (crate::identity::PROOF, false),
            ("unknown", true),
        ] {
            assert!(Observer::open(&dir.0, id, interactive, &marker()).is_err());
        }
        for bytes in [b"{}".to_vec(), b"null".to_vec(), b"not json".to_vec(),
            serde_json::to_vec(&serde_json::json!({"schema":2,"source_revision":"a".repeat(40),"binary_sha256":"b".repeat(64)})).unwrap(),
            serde_json::to_vec(&serde_json::json!({"schema":1,"source_revision":"A".repeat(40),"binary_sha256":"b".repeat(64)})).unwrap(),
            serde_json::to_vec(&serde_json::json!({"schema":1,"source_revision":"a".repeat(39),"binary_sha256":"b".repeat(64)})).unwrap(),
            serde_json::to_vec(&serde_json::json!({"schema":1,"source_revision":"a".repeat(40),"binary_sha256":"g".repeat(64)})).unwrap(),
            serde_json::to_vec(&serde_json::json!({"schema":1,"source_revision":"a".repeat(40),"binary_sha256":"b".repeat(64),"text":"secret"})).unwrap()] {
            assert!(Observer::open(&dir.0, crate::identity::PROOF, true, &bytes).is_err());
        }
        assert!(
            std::fs::read_dir(&dir.0).unwrap().next().is_none(),
            "rejected admission performs no writes"
        );
        assert!(Observer::open(&dir.0, crate::identity::PROOF, true, &vec![b' '; 513]).is_err());
        let _observer = open(&dir);
        assert!(
            Observer::open(&dir.0, crate::identity::PROOF, true, &marker()).is_err(),
            "one producer owns each runtime"
        );
    }
    #[test]
    fn current_is_rate_limited_final_is_separate_and_counter_ticks_do_not_fill_log() {
        let dir = PrivateDir::new();
        let mut observer = open(&dir);
        let mut state = Snapshot::default();
        observer.record(stamp(0, 0), state, None).unwrap();
        assert!(observer.flush_current(0).unwrap());
        let first = std::fs::read(dir.0.join(CURRENT_FILE)).unwrap();
        for ms in 1..1_000 {
            state.counters.draws += 1;
            observer.record(stamp(ms, 0), state, None).unwrap();
            assert!(!observer.flush_current(ms).unwrap());
        }
        assert_eq!(std::fs::read(dir.0.join(CURRENT_FILE)).unwrap(), first);
        observer.record(stamp(1_000, 0), state, None).unwrap();
        assert!(observer.flush_current(1_000).unwrap());
        assert_eq!(
            read(&dir, CURRENT_FILE)["changes"]
                .as_array()
                .unwrap()
                .len(),
            1
        );
        observer
            .finish(stamp(1_001, 0), state, ExitReason::BackFromHome)
            .unwrap();
        assert_eq!(read(&dir, FINAL_FILE)["exit_reason"], "back_from_home");
        assert_eq!(read(&dir, CURRENT_FILE)["written_at_ms"], 1_000);
        assert!(observer
            .finish(stamp(1_002, 0), state, ExitReason::Other)
            .is_err());
        assert!(observer.record(stamp(1_002, 0), state, None).is_err());
    }
    #[test]
    fn change_log_is_bounded_but_latest_state_and_aggregate_counts_continue() {
        let dir = PrivateDir::new();
        let mut observer = open(&dir);
        let state = Snapshot::default();
        for seq in 1..=200 {
            observer
                .record(
                    stamp(seq, seq),
                    state,
                    Some(EventDecision {
                        class: EventClass::KeyUp,
                        decision: Decision::Drain,
                        origin: Origin::NativeUnclassified,
                    }),
                )
                .unwrap();
        }
        observer
            .finish(stamp(201, 200), state, ExitReason::ObservationLimit)
            .unwrap();
        let value = read(&dir, FINAL_FILE);
        assert_eq!(value["changes"].as_array().unwrap().len(), 128);
        assert_eq!(value["overflow"], true);
        assert_eq!(value["dropped_changes"], 72);
        assert_eq!(value["counts"]["received"], 200);
        assert_eq!(value["counts"]["drain"], 200);
        assert_eq!(value["counts"]["native_unclassified"], 200);
        assert_eq!(value["stamp"]["event_seq"], 200);
    }
    #[test]
    fn metadata_schema_never_contains_queries_hashes_key_labels_or_synthetic_origin_guesses() {
        let dir = PrivateDir::new();
        let mut observer = open(&dir);
        let sensitive = "household private input";
        let state = Snapshot {
            page: PageRole::Apps,
            focus: FocusRole::KeypadKey,
            draft: DraftMetrics {
                bytes: sensitive.len() as u16,
                caret: sensitive.len() as u16,
                preedit_bytes: 0,
                fixture: FixtureMatch::classify(sensitive),
            },
            counters: LoopCounters {
                synthetic_recipe_queued: 1,
                ..Default::default()
            },
            ..Default::default()
        };
        observer
            .record(
                stamp(1, 1),
                state,
                Some(EventDecision {
                    class: EventClass::TextCommit,
                    decision: Decision::Accepted,
                    origin: Origin::NativeUnclassified,
                }),
            )
            .unwrap();
        observer
            .finish(stamp(2, 1), state, ExitReason::Other)
            .unwrap();
        let bytes = std::fs::read(dir.0.join(FINAL_FILE)).unwrap();
        let text = std::str::from_utf8(&bytes).unwrap();
        assert!(!text.contains(sensitive));
        let value = read(&dir, FINAL_FILE);
        assert_eq!(value["interactive"], true);
        assert_eq!(value["helper_executions"], 0);
        assert_eq!(value["execution_validation"], "constructor_declaration");
        assert_eq!(value["state"]["compatibility_admitted"], false);
        assert_eq!(value["loop"], "main_interactive");
        assert_eq!(value["schema"], 1);
        assert_eq!(
            value["declared_identity"]["validation"],
            "package_marker_declaration"
        );
        assert_eq!(value["producer"]["uid"], unsafe { libc::geteuid() });
        assert_eq!(value["producer"]["pid"], unsafe { libc::getpid() });
        assert_eq!(value["state"]["focus"], "keypad_key");
        assert_eq!(value["state"]["draft"]["fixture"], "other");
        assert_eq!(value["counts"]["native_unclassified"], 1);
        assert_eq!(
            value["changes"][0]["event"]["origin"],
            "native_unclassified"
        );
        let keys = value["state"]["draft"]
            .as_object()
            .unwrap()
            .keys()
            .map(String::as_str)
            .collect::<Vec<_>>();
        assert_eq!(keys, ["bytes", "caret", "fixture", "preedit_bytes"]);
        assert_eq!(FixtureMatch::classify("lu"), FixtureMatch::Lu);
        assert_eq!(FixtureMatch::classify("luma"), FixtureMatch::Luma);
        assert_eq!(FixtureMatch::classify("lumaA"), FixtureMatch::LumaA);
    }
    #[test]
    fn regressing_stamps_counters_and_invalid_draft_metadata_are_rejected_transactionally() {
        let dir = PrivateDir::new();
        let mut observer = open(&dir);
        let state = Snapshot::default();
        observer.record(stamp(10, 2), state, None).unwrap();
        for wrong in [
            stamp(9, 2),
            Stamp {
                controller_ms: 9,
                ..stamp(10, 2)
            },
            stamp(10, 1),
            Stamp {
                batch_seq: 1,
                ..stamp(10, 2)
            },
        ] {
            assert!(observer.record(wrong, state, None).is_err());
        }
        assert!(observer
            .record(
                stamp(10, 2),
                state,
                Some(EventDecision {
                    class: EventClass::KeyUp,
                    decision: Decision::Drain,
                    origin: Origin::NativeUnclassified
                })
            )
            .is_err());
        assert!(observer
            .record(
                stamp(10, 2),
                Snapshot {
                    draft: DraftMetrics {
                        bytes: 257,
                        ..Default::default()
                    },
                    ..state
                },
                None
            )
            .is_err());
        assert!(observer
            .record(
                stamp(10, 2),
                Snapshot {
                    draft: DraftMetrics {
                        caret: 1,
                        ..Default::default()
                    },
                    ..state
                },
                None
            )
            .is_err());
        let state = Snapshot {
            counters: LoopCounters {
                draws: 1,
                ..Default::default()
            },
            ..state
        };
        observer.record(stamp(11, 2), state, None).unwrap();
        assert!(observer
            .record(stamp(12, 2), Snapshot::default(), None)
            .is_err());
        observer
            .finish(stamp(12, 2), state, ExitReason::Other)
            .unwrap();
        assert_eq!(read(&dir, FINAL_FILE)["counts"]["received"], 0);
    }
    #[test]
    fn writes_are_private_atomic_and_reject_link_destinations_without_touching_targets() {
        let dir = PrivateDir::new();
        let target = PrivateDir::new();
        std::fs::write(target.0.join("victim"), b"untouched").unwrap();
        let mut observer = open(&dir);
        observer
            .record(stamp(0, 0), Snapshot::default(), None)
            .unwrap();
        symlink(target.0.join("victim"), dir.0.join(CURRENT_FILE)).unwrap();
        assert!(observer.flush_current(0).is_err());
        assert_eq!(
            std::fs::read(target.0.join("victim")).unwrap(),
            b"untouched"
        );
        std::fs::remove_file(dir.0.join(CURRENT_FILE)).unwrap();
        assert!(observer.flush_current(0).unwrap());
        assert_eq!(
            std::fs::metadata(dir.0.join(CURRENT_FILE)).unwrap().mode() & 0o7777,
            0o600
        );
        for ms in [1_000, 2_000, 3_000] {
            observer
                .record(stamp(ms, 0), Snapshot::default(), None)
                .unwrap();
            assert!(observer.flush_current(ms).unwrap());
            assert_eq!(read(&dir, CURRENT_FILE)["written_at_ms"], ms);
        }
        let names = std::fs::read_dir(&dir.0)
            .unwrap()
            .map(|entry| entry.unwrap().file_name().to_string_lossy().into_owned())
            .collect::<Vec<_>>();
        assert!(
            !names.iter().any(|name| name.contains("next")),
            "temporary entries are removed after publication"
        );
        symlink(target.0.join("victim"), dir.0.join(FINAL_FILE)).unwrap();
        assert!(observer
            .finish(stamp(3_001, 0), Snapshot::default(), ExitReason::Other)
            .is_err());
        assert_eq!(
            std::fs::read(target.0.join("victim")).unwrap(),
            b"untouched"
        );
    }
    #[test]
    fn hardlinks_and_preexisting_temporary_entries_cannot_be_overwritten() {
        let dir = PrivateDir::new();
        let mut observer = open(&dir);
        observer
            .record(stamp(0, 0), Snapshot::default(), None)
            .unwrap();
        observer.flush_current(0).unwrap();
        let original = std::fs::read(dir.0.join(CURRENT_FILE)).unwrap();
        std::fs::hard_link(dir.0.join(CURRENT_FILE), dir.0.join("linked-current")).unwrap();
        assert!(observer.flush_current(1_000).is_err());
        assert_eq!(std::fs::read(dir.0.join(CURRENT_FILE)).unwrap(), original);
        std::fs::remove_file(dir.0.join("linked-current")).unwrap();
        let temporary = dir.0.join(format!(".keyboard-observer-next-{}-2", unsafe {
            libc::getpid()
        }));
        std::fs::write(&temporary, b"preexisting").unwrap();
        assert!(observer.flush_current(1_000).is_err());
        assert_eq!(std::fs::read(&temporary).unwrap(), b"preexisting");
        assert_eq!(std::fs::read(dir.0.join(CURRENT_FILE)).unwrap(), original);
        assert!(
            observer.flush_current(1_000).unwrap(),
            "a retry uses a new temporary sequence"
        );
        let current = read(&dir, CURRENT_FILE);
        assert_eq!(current["written_at_ms"], 1_000);
    }
    #[test]
    fn admission_requires_private_directory_and_open_fd_prevents_path_redirection() {
        let dir = PrivateDir::new();
        std::fs::set_permissions(&dir.0, std::fs::Permissions::from_mode(0o755)).unwrap();
        assert!(Observer::open(&dir.0, crate::identity::PROOF, true, &marker()).is_err());
        std::fs::set_permissions(&dir.0, std::fs::Permissions::from_mode(0o700)).unwrap();
        let outer = PrivateDir::new();
        symlink(&dir.0, outer.0.join("link")).unwrap();
        assert!(Observer::open(
            &outer.0.join("link"),
            crate::identity::PROOF,
            true,
            &marker()
        )
        .is_err());
        let mut observer = open(&dir);
        observer
            .record(stamp(0, 0), Snapshot::default(), None)
            .unwrap();
        let moved = outer.0.join("original");
        std::fs::rename(&dir.0, &moved).unwrap();
        symlink(&outer.0, &dir.0).unwrap();
        assert!(observer.flush_current(0).unwrap());
        assert!(moved.join(CURRENT_FILE).is_file());
        assert!(!outer.0.join(CURRENT_FILE).exists());
        std::fs::remove_file(&dir.0).unwrap();
        std::fs::rename(moved, &dir.0).unwrap();
    }
}
