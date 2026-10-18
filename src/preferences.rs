//! Dashboard-only preferences in an app-owned durable directory. Directory descriptors and
//! openat/renameat keep atomic writes inside that directory even if its parent is renamed.
use crate::{model::valid_app_id, shell::Preferences};
use serde::{Deserialize, Serialize};
use std::{
    ffi::CString,
    fs::File,
    io::{self, Read, Write},
    os::{
        fd::{AsRawFd, FromRawFd},
        unix::fs::MetadataExt,
    },
    path::Path,
    sync::atomic::{AtomicU64, Ordering},
};
const NAME: &std::ffi::CStr = c"preferences.json";
const HOME_FOCUS_NAME: &std::ffi::CStr = c"home-focus.json";
const HOME_FOCUS_MAX_BYTES: usize = 1024;
const MAX_BYTES: u64 = 64 * 1024;
static SERIAL: AtomicU64 = AtomicU64::new(0);
#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct Record {
    schema: u32,
    preferences: Preferences,
}
/// A separate sidecar keeps the existing favourites schema readable by older builds.
#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct HomeFocusRecord {
    schema: u32,
    app_id: String,
}
fn decode_home_focus(bytes: &[u8]) -> io::Result<String> {
    if bytes.len() > HOME_FOCUS_MAX_BYTES {
        return Err(invalid("Home focus record too large"));
    }
    let record: HomeFocusRecord =
        serde_json::from_slice(bytes).map_err(|_| invalid("Malformed home focus record"))?;
    if record.schema != 1 || !valid_app_id(&record.app_id) {
        return Err(invalid("Unsupported or invalid home focus record"));
    }
    Ok(record.app_id)
}
fn invalid(message: &str) -> io::Error {
    io::Error::new(io::ErrorKind::InvalidData, message)
}
fn validate(p: &Preferences) -> io::Result<()> {
    if p.favourites.len() > 256 || p.favourites.iter().any(|id| !valid_app_id(id)) {
        return Err(invalid("Invalid favourite identity"));
    }
    if p.favourite_order.len() > 256
        || p.favourite_order
            .iter()
            .any(|id| !p.favourites.contains(id))
        || p.favourite_order
            .iter()
            .collect::<std::collections::BTreeSet<_>>()
            .len()
            != p.favourite_order.len()
    {
        return Err(invalid("Invalid favourite order"));
    }
    Ok(())
}
fn private_file(file: &File) -> io::Result<()> {
    let meta = file.metadata()?;
    if !meta.is_file()
        || meta.uid() != unsafe { libc::geteuid() }
        || meta.mode() & 0o077 != 0
        || meta.nlink() != 1
    {
        return Err(invalid("Preference file is not privately owned"));
    }
    if meta.len() > MAX_BYTES {
        return Err(invalid("Preference file too large"));
    }
    Ok(())
}
/// Shared schema/identity validation for private storage and the explicit
/// owner-only legacy permission migration. This function performs no I/O.
pub(crate) fn decode_record(bytes: &[u8]) -> io::Result<Preferences> {
    if bytes.len() as u64 > MAX_BYTES {
        return Err(invalid("Preference file too large"));
    }
    let record: Record =
        serde_json::from_slice(bytes).map_err(|_| invalid("Malformed preference record"))?;
    if record.schema != 1 {
        return Err(invalid("Unsupported preference schema"));
    }
    validate(&record.preferences)?;
    Ok(record.preferences)
}

pub struct Store {
    dir: File,
}
impl Store {
    pub fn open(path: &Path) -> io::Result<Self> {
        Self::open_directory(path, true)
    }
    /// Reads an existing own private state directory without creating it.
    pub fn open_existing(path: &Path) -> io::Result<Self> {
        Self::open_directory(path, false)
    }
    fn open_directory(path: &Path, create: bool) -> io::Result<Self> {
        if unsafe { libc::geteuid() } == 0 {
            return Err(io::Error::new(
                io::ErrorKind::PermissionDenied,
                "Refusing root preference writes",
            ));
        }
        let path = CString::new(path.as_os_str().as_encoded_bytes())
            .map_err(|_| invalid("Invalid state path"))?;
        if create {
            let made = unsafe { libc::mkdir(path.as_ptr(), 0o700) };
            if made < 0 && io::Error::last_os_error().raw_os_error() != Some(libc::EEXIST) {
                return Err(io::Error::last_os_error());
            }
        }
        let fd = unsafe {
            libc::open(
                path.as_ptr(),
                libc::O_RDONLY | libc::O_DIRECTORY | libc::O_NOFOLLOW | libc::O_CLOEXEC,
            )
        };
        if fd < 0 {
            return Err(io::Error::last_os_error());
        }
        let dir = unsafe { File::from_raw_fd(fd) };
        let meta = dir.metadata()?;
        if !meta.is_dir() || meta.uid() != unsafe { libc::geteuid() } || meta.mode() & 0o077 != 0 {
            return Err(invalid("State directory is not privately owned"));
        }
        Ok(Self { dir })
    }
    pub fn load(&self) -> io::Result<Option<Preferences>> {
        let fd = unsafe {
            libc::openat(
                self.dir.as_raw_fd(),
                NAME.as_ptr(),
                libc::O_RDONLY | libc::O_NOFOLLOW | libc::O_CLOEXEC | libc::O_NONBLOCK,
            )
        };
        if fd < 0 {
            let err = io::Error::last_os_error();
            return if err.kind() == io::ErrorKind::NotFound {
                Ok(None)
            } else {
                Err(err)
            };
        }
        let mut file = unsafe { File::from_raw_fd(fd) };
        private_file(&file)?;
        let mut bytes = Vec::new();
        Read::by_ref(&mut file)
            .take(MAX_BYTES + 1)
            .read_to_end(&mut bytes)?;
        if bytes.len() > MAX_BYTES as usize {
            return Err(invalid("Preference file grew past its limit"));
        }
        decode_record(&bytes).map(Some)
    }
    pub fn save(&self, prefs: &Preferences) -> io::Result<()> {
        validate(prefs)?;
        // Never replace an unreadable, foreign or newer-schema state file with defaults.
        self.load()?;
        let temp = CString::new(format!(
            ".preferences-{}-{}.tmp",
            std::process::id(),
            SERIAL.fetch_add(1, Ordering::Relaxed)
        ))
        .unwrap();
        let fd = unsafe {
            libc::openat(
                self.dir.as_raw_fd(),
                temp.as_ptr(),
                libc::O_WRONLY | libc::O_CREAT | libc::O_EXCL | libc::O_NOFOLLOW | libc::O_CLOEXEC,
                0o600,
            )
        };
        if fd < 0 {
            return Err(io::Error::last_os_error());
        }
        let mut file = unsafe { File::from_raw_fd(fd) };
        let write = (|| {
            serde_json::to_writer(
                &mut file,
                &Record {
                    schema: 1,
                    preferences: prefs.clone(),
                },
            )?;
            file.write_all(b"\n")?;
            file.sync_all()?;
            if unsafe {
                libc::renameat(
                    self.dir.as_raw_fd(),
                    temp.as_ptr(),
                    self.dir.as_raw_fd(),
                    NAME.as_ptr(),
                )
            } < 0
            {
                return Err(io::Error::last_os_error());
            }
            self.dir.sync_all()?;
            Ok(())
        })();
        if write.is_err() {
            unsafe { libc::unlinkat(self.dir.as_raw_fd(), temp.as_ptr(), 0) };
        }
        write
    }
    /// Optional UI state only; this never changes favourites or supplies an action.
    pub fn load_home_focus(&self) -> io::Result<Option<String>> {
        let fd = unsafe {
            libc::openat(
                self.dir.as_raw_fd(),
                HOME_FOCUS_NAME.as_ptr(),
                libc::O_RDONLY | libc::O_NOFOLLOW | libc::O_CLOEXEC | libc::O_NONBLOCK,
            )
        };
        if fd < 0 {
            let error = io::Error::last_os_error();
            return if error.kind() == io::ErrorKind::NotFound {
                Ok(None)
            } else {
                Err(error)
            };
        }
        let mut file = unsafe { File::from_raw_fd(fd) };
        private_file(&file)?;
        if file.metadata()?.len() > HOME_FOCUS_MAX_BYTES as u64 {
            return Err(invalid("Home focus record too large"));
        }
        let mut bytes = Vec::new();
        Read::by_ref(&mut file)
            .take(HOME_FOCUS_MAX_BYTES as u64 + 1)
            .read_to_end(&mut bytes)?;
        decode_home_focus(&bytes).map(Some)
    }
    pub fn save_home_focus(&self, app_id: &str) -> io::Result<()> {
        let temp = CString::new(format!(
            ".home-focus-{}-{}.tmp",
            std::process::id(),
            SERIAL.fetch_add(1, Ordering::Relaxed)
        ))
        .unwrap();
        self.save_home_focus_at_temp(app_id, &temp)
    }
    fn save_home_focus_at_temp(&self, app_id: &str, temp: &std::ffi::CStr) -> io::Result<()> {
        if !valid_app_id(app_id) {
            return Err(invalid("Invalid home focus identity"));
        }
        // Preserve unreadable, foreign, corrupt or newer sidecars rather than repairing them.
        self.load_home_focus()?;
        let fd = unsafe {
            libc::openat(
                self.dir.as_raw_fd(),
                temp.as_ptr(),
                libc::O_WRONLY | libc::O_CREAT | libc::O_EXCL | libc::O_NOFOLLOW | libc::O_CLOEXEC,
                0o600,
            )
        };
        if fd < 0 {
            return Err(io::Error::last_os_error());
        }
        let mut file = unsafe { File::from_raw_fd(fd) };
        let write = (|| {
            serde_json::to_writer(
                &mut file,
                &HomeFocusRecord {
                    schema: 1,
                    app_id: app_id.into(),
                },
            )?;
            file.write_all(b"\n")?;
            file.sync_all()?;
            if unsafe {
                libc::renameat(
                    self.dir.as_raw_fd(),
                    temp.as_ptr(),
                    self.dir.as_raw_fd(),
                    HOME_FOCUS_NAME.as_ptr(),
                )
            } < 0
            {
                return Err(io::Error::last_os_error());
            }
            self.dir.sync_all()?;
            Ok(())
        })();
        if write.is_err() {
            unsafe { libc::unlinkat(self.dir.as_raw_fd(), temp.as_ptr(), 0) };
        }
        write
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    use std::{
        collections::BTreeSet,
        os::unix::fs::{symlink, PermissionsExt},
    };
    struct Scratch(std::path::PathBuf);
    impl Scratch {
        fn new() -> Self {
            let path = std::env::temp_dir().join(format!(
                "native-home-pref-{}-{}",
                std::process::id(),
                SERIAL.fetch_add(1, Ordering::Relaxed)
            ));
            std::fs::create_dir(&path).unwrap();
            Self(path)
        }
    }
    impl Drop for Scratch {
        fn drop(&mut self) {
            std::fs::remove_dir_all(&self.0).unwrap();
        }
    }
    fn prefs() -> Preferences {
        Preferences {
            initialized: true,
            favourites: BTreeSet::from(["com.example.video".into()]),
            favourite_order: Vec::new(),
        }
    }
    #[test]
    fn admission_read_never_creates_or_follows_a_state_directory() {
        let tmp = Scratch::new();
        let path = tmp.0.join("missing");
        assert!(Store::open_existing(&path).is_err());
        assert!(!path.exists());
        Store::open(&path).unwrap().save(&prefs()).unwrap();
        let before = std::fs::read(path.join("preferences.json")).unwrap();
        assert_eq!(
            Store::open_existing(&path)
                .unwrap()
                .load()
                .unwrap()
                .unwrap(),
            prefs()
        );
        assert_eq!(
            std::fs::read(path.join("preferences.json")).unwrap(),
            before
        );
        let alias = tmp.0.join("alias");
        symlink(&path, &alias).unwrap();
        assert!(Store::open_existing(&alias).is_err());
        assert_eq!(std::fs::read_dir(&path).unwrap().count(), 1);
    }
    #[test]
    fn saved_favourites_survive_closing_and_reopening_and_keep_explicit_empty_state() {
        let tmp = Scratch::new();
        let path = tmp.0.join("state");
        let store = Store::open(&path).unwrap();
        assert!(store.load().unwrap().is_none());
        store.save(&prefs()).unwrap();
        drop(store);
        let reopened = Store::open(&path).unwrap();
        assert_eq!(
            reopened.load().unwrap().unwrap().favourites,
            prefs().favourites
        );
        let empty = Preferences {
            initialized: true,
            favourites: BTreeSet::new(),
            favourite_order: Vec::new(),
        };
        reopened.save(&empty).unwrap();
        drop(reopened);
        let loaded = Store::open(&path).unwrap().load().unwrap().unwrap();
        assert!(loaded.initialized && loaded.favourites.is_empty());
        assert_eq!(std::fs::read_dir(&path).unwrap().count(), 1);
        let ordered = Preferences {
            initialized: true,
            favourites: BTreeSet::from(["com.example.a".into(), "com.example.b".into()]),
            favourite_order: vec!["com.example.b".into(), "com.example.a".into()],
        };
        Store::open(&path).unwrap().save(&ordered).unwrap();
        assert_eq!(
            Store::open(&path).unwrap().load().unwrap().unwrap(),
            ordered
        );
    }
    #[test]
    fn symlinked_directory_or_file_cannot_redirect_a_save() {
        let tmp = Scratch::new();
        let outside = tmp.0.join("outside");
        std::fs::create_dir(&outside).unwrap();
        std::fs::set_permissions(&outside, std::fs::Permissions::from_mode(0o700)).unwrap();
        symlink(&outside, tmp.0.join("state-link")).unwrap();
        assert!(Store::open(&tmp.0.join("state-link")).is_err());
        let store = Store::open(&tmp.0.join("state")).unwrap();
        let target = outside.join("sentinel");
        std::fs::write(&target, b"unchanged").unwrap();
        symlink(&target, tmp.0.join("state/preferences.json")).unwrap();
        assert!(store.load().is_err());
        assert!(store.save(&prefs()).is_err());
        assert_eq!(std::fs::read(target).unwrap(), b"unchanged");
    }
    #[test]
    fn future_schema_or_corrupt_state_is_preserved_and_invalid_id_refused() {
        let tmp = Scratch::new();
        let path = tmp.0.join("state");
        let store = Store::open(&path).unwrap();
        store.save(&prefs()).unwrap();
        let target = path.join("preferences.json");
        let original = b"{\"schema\":2,\"preferences\":{\"initialized\":true,\"favourites\":[]}}";
        std::fs::write(&target, original).unwrap();
        assert!(store.load().is_err());
        assert!(store.save(&prefs()).is_err());
        assert_eq!(std::fs::read(&target).unwrap(), original);
        std::fs::write(&target, b"bad-json").unwrap();
        assert!(store.save(&prefs()).is_err());
        assert_eq!(std::fs::read(&target).unwrap(), b"bad-json");
        std::fs::remove_file(target).unwrap();
        let bad = Preferences {
            initialized: true,
            favourites: BTreeSet::from(["../../other-app".into()]),
            favourite_order: Vec::new(),
        };
        assert!(store.save(&bad).is_err());
    }
    #[test]
    fn open_directory_descriptor_keeps_save_in_its_original_directory() {
        let tmp = Scratch::new();
        let original = tmp.0.join("state");
        let store = Store::open(&original).unwrap();
        let moved = tmp.0.join("moved");
        std::fs::rename(&original, &moved).unwrap();
        std::fs::create_dir(&original).unwrap();
        store.save(&prefs()).unwrap();
        assert!(moved.join("preferences.json").exists());
        assert!(!original.join("preferences.json").exists());
    }
}

/// Disk work runs off the render thread. Acknowledgements report readback, not merely queueing.
#[derive(Default)]
struct WriteBatch {
    preferences: Option<Preferences>,
    home_focus: Option<String>,
}
impl WriteBatch {
    fn merge(&mut self, newer: Self) {
        if newer.preferences.is_some() {
            self.preferences = newer.preferences;
        }
        if newer.home_focus.is_some() {
            self.home_focus = newer.home_focus;
        }
    }
    fn is_empty(&self) -> bool {
        self.preferences.is_none() && self.home_focus.is_none()
    }
}
pub struct Persistence {
    sender: std::sync::mpsc::SyncSender<WriteBatch>,
    pending: WriteBatch,
    replies: std::sync::mpsc::Receiver<Result<Preferences, String>>,
    focus_replies: std::sync::mpsc::Receiver<Result<String, String>>,
    loaded_focus: Result<Option<String>, String>,
    worker: std::thread::JoinHandle<()>,
}
pub struct PersistenceOutcome {
    pub preferences: Vec<Result<Preferences, String>>,
    pub home_focus: Vec<Result<String, String>>,
}
impl Persistence {
    pub fn start(path: &Path) -> io::Result<(Self, Option<Preferences>)> {
        let store = Store::open(path)?;
        let loaded = store.load()?;
        // Optional navigation damage must not make healthy favourites unavailable.
        let loaded_focus = store.load_home_focus().map_err(|e| e.to_string());
        let (sender, requests) = std::sync::mpsc::sync_channel::<WriteBatch>(1);
        let (ack, replies) = std::sync::mpsc::channel();
        let (focus_ack, focus_replies) = std::sync::mpsc::channel();
        let worker = std::thread::Builder::new()
            .name("home-preferences".into())
            .spawn(move || {
                while let Ok(mut batch) = requests.recv() {
                    // Coalesce each independent file without losing the other kind of write.
                    while let Ok(newest) = requests.try_recv() {
                        batch.merge(newest);
                    }
                    if let Some(prefs) = batch.preferences {
                        let result = (|| {
                            store.save(&prefs)?;
                            if store.load()? != Some(prefs.clone()) {
                                return Err(invalid("Preference readback mismatch"));
                            }
                            Ok(prefs)
                        })()
                        .map_err(|e: io::Error| e.to_string());
                        if ack.send(result).is_err() {
                            break;
                        }
                    }
                    if let Some(id) = batch.home_focus {
                        let result = (|| {
                            store.save_home_focus(&id)?;
                            if store.load_home_focus()?.as_deref() != Some(id.as_str()) {
                                return Err(invalid("Home focus readback mismatch"));
                            }
                            Ok(id)
                        })()
                        .map_err(|e: io::Error| e.to_string());
                        if focus_ack.send(result).is_err() {
                            break;
                        }
                    }
                }
            })?;
        Ok((
            Self {
                sender,
                pending: WriteBatch::default(),
                replies,
                focus_replies,
                loaded_focus,
                worker,
            },
            loaded,
        ))
    }
    pub fn loaded_home_focus(&self) -> Result<Option<String>, String> {
        self.loaded_focus.clone()
    }
    fn flush(&mut self) -> Result<(), String> {
        if self.pending.is_empty() {
            return Ok(());
        }
        let batch = std::mem::take(&mut self.pending);
        match self.sender.try_send(batch) {
            Ok(()) => Ok(()),
            Err(std::sync::mpsc::TrySendError::Full(batch)) => {
                self.pending = batch;
                Ok(())
            }
            Err(std::sync::mpsc::TrySendError::Disconnected(_)) => {
                Err("Preference writer stopped".into())
            }
        }
    }
    pub fn queue(&mut self, prefs: &Preferences) -> Result<(), String> {
        validate(prefs).map_err(|e| e.to_string())?;
        self.pending.preferences = Some(prefs.clone());
        self.flush()
    }
    pub fn queue_home_focus(&mut self, id: &str) -> Result<(), String> {
        if !valid_app_id(id) {
            return Err("Invalid home focus identity".into());
        }
        self.pending.home_focus = Some(id.to_owned());
        self.flush()
    }
    pub fn poll(&mut self) -> Vec<Result<Preferences, String>> {
        let mut results = self.replies.try_iter().collect::<Vec<_>>();
        if let Err(error) = self.flush() {
            results.push(Err(error));
        }
        results
    }
    pub fn poll_home_focus(&mut self) -> Vec<Result<String, String>> {
        self.focus_replies.try_iter().collect()
    }
    pub fn finish(self) -> Vec<Result<Preferences, String>> {
        self.finish_all().preferences
    }
    pub fn finish_all(self) -> PersistenceOutcome {
        let Self {
            sender,
            pending,
            replies,
            focus_replies,
            worker,
            ..
        } = self;
        let final_failed = !pending.is_empty() && sender.send(pending).is_err();
        drop(sender);
        if worker.join().is_err() {
            return PersistenceOutcome {
                preferences: vec![Err("Preference writer panicked".into())],
                home_focus: vec![Err("Home focus writer panicked".into())],
            };
        }
        let mut preferences = replies.try_iter().collect::<Vec<_>>();
        let mut home_focus = focus_replies.try_iter().collect::<Vec<_>>();
        if final_failed {
            preferences.push(Err("Final preference write failed".into()));
            home_focus.push(Err("Final home focus write failed".into()));
        }
        PersistenceOutcome {
            preferences,
            home_focus,
        }
    }
}
#[cfg(test)]
mod writer_tests {
    use super::*;
    #[test]
    fn asynchronous_writer_finishes_with_latest_readback_and_reopens_it() {
        let root = std::env::temp_dir().join(format!(
            "native-home-writer-{}-{}",
            std::process::id(),
            SERIAL.fetch_add(1, Ordering::Relaxed)
        ));
        let (mut worker, loaded) = Persistence::start(&root).unwrap();
        assert!(loaded.is_none());
        let first = Preferences {
            initialized: true,
            favourites: std::collections::BTreeSet::from(["com.example.first".into()]),
            favourite_order: Vec::new(),
        };
        let last = Preferences {
            initialized: true,
            favourites: std::collections::BTreeSet::new(),
            favourite_order: Vec::new(),
        };
        worker.queue(&first).unwrap();
        worker.queue(&last).unwrap();
        let replies = worker.finish();
        assert_eq!(replies.last().unwrap().as_ref().unwrap(), &last);
        assert_eq!(Store::open(&root).unwrap().load().unwrap(), Some(last));
        std::fs::remove_dir_all(root).unwrap();
    }
}

#[cfg(test)]
mod ordering_tests {
    use super::*;
    #[test]
    fn old_records_roundtrip_without_new_field_and_invalid_ranks_are_rejected() {
        let old = r#"{"initialized":true,"favourites":["com.example.a","com.example.b"]}"#;
        let mut p: Preferences = serde_json::from_str(old).unwrap();
        assert!(p.favourite_order.is_empty());
        assert_eq!(serde_json::to_string(&p).unwrap(), old);
        p.favourite_order = vec!["com.example.a".into(), "com.example.a".into()];
        assert!(validate(&p).is_err());
        p.favourite_order = vec!["com.example.other".into()];
        assert!(validate(&p).is_err());
        p.favourite_order = vec!["com.example.b".into(), "com.example.a".into()];
        assert!(validate(&p).is_ok());
    }
}

#[cfg(test)]
mod home_focus_tests {
    use super::*;
    use std::os::unix::fs::{symlink, PermissionsExt};
    struct Scratch(std::path::PathBuf);
    impl Scratch {
        fn new() -> Self {
            Self(std::env::temp_dir().join(format!(
                "native-home-focus-{}-{}",
                std::process::id(),
                SERIAL.fetch_add(1, Ordering::Relaxed)
            )))
        }
    }
    impl Drop for Scratch {
        fn drop(&mut self) {
            std::fs::remove_dir_all(&self.0).unwrap();
        }
    }
    fn prefs() -> Preferences {
        Preferences {
            initialized: true,
            favourites: ["com.example.a".into(), "com.example.b".into()].into(),
            favourite_order: Vec::new(),
        }
    }
    #[test]
    fn sidecar_roundtrip_keeps_old_favourite_bytes_and_explicit_empty_schema() {
        let path = Scratch::new();
        let store = Store::open(&path.0).unwrap();
        let empty = Preferences {
            initialized: true,
            ..Preferences::default()
        };
        store.save(&empty).unwrap();
        let before = std::fs::read(path.0.join("preferences.json")).unwrap();
        assert!(store.load_home_focus().unwrap().is_none());
        store.save_home_focus("com.example.a").unwrap();
        drop(store);
        let store = Store::open_existing(&path.0).unwrap();
        assert_eq!(
            store.load_home_focus().unwrap().as_deref(),
            Some("com.example.a")
        );
        assert_eq!(store.load().unwrap(), Some(empty));
        assert_eq!(
            std::fs::read(path.0.join("preferences.json")).unwrap(),
            before
        );
        assert_eq!(
            std::fs::metadata(path.0.join("home-focus.json"))
                .unwrap()
                .mode()
                & 0o777,
            0o600
        );
    }
    #[test]
    fn sidecar_unknown_corrupt_duplicate_trailing_and_oversized_records_are_preserved() {
        let path = Scratch::new();
        let store = Store::open(&path.0).unwrap();
        store.save(&prefs()).unwrap();
        store.save_home_focus("com.example.a").unwrap();
        let target = path.0.join("home-focus.json");
        for raw in [
            r#"{"schema":2,"app_id":"com.example.a"}"#.to_owned(),
            r#"{"schema":1,"app_id":"../../bad"}"#.into(),
            r#"{"schema":1,"app_id":"com.example.a","app_id":"com.example.b"}"#.into(),
            r#"{"schema":1,"app_id":"com.example.a"} {}"#.into(),
            r#"{"schema":1,"app_id":"com.example.a","action":"launch"}"#.into(),
            " ".repeat(HOME_FOCUS_MAX_BYTES + 1),
        ] {
            std::fs::write(&target, &raw).unwrap();
            assert!(store.load_home_focus().is_err());
            assert!(store.save_home_focus("com.example.b").is_err());
            assert_eq!(std::fs::read_to_string(&target).unwrap(), raw);
            assert_eq!(store.load().unwrap(), Some(prefs()));
        }
    }
    #[test]
    fn sidecar_symlink_hardlink_public_mode_and_fifo_cannot_redirect_or_block() {
        let path = Scratch::new();
        let store = Store::open(&path.0).unwrap();
        let target = path.0.join("home-focus.json");
        let outside = path.0.join("sentinel");
        std::fs::write(&outside, b"untouched").unwrap();
        symlink(&outside, &target).unwrap();
        assert!(store.save_home_focus("com.example.a").is_err());
        std::fs::remove_file(&target).unwrap();
        assert_eq!(std::fs::read(&outside).unwrap(), b"untouched");
        store.save_home_focus("com.example.a").unwrap();
        std::fs::hard_link(&target, path.0.join("link")).unwrap();
        assert!(store.load_home_focus().is_err());
        std::fs::remove_file(path.0.join("link")).unwrap();
        std::fs::set_permissions(&target, std::fs::Permissions::from_mode(0o644)).unwrap();
        assert!(store.save_home_focus("com.example.b").is_err());
        std::fs::remove_file(&target).unwrap();
        let name = CString::new(target.as_os_str().as_encoded_bytes()).unwrap();
        assert_eq!(unsafe { libc::mkfifo(name.as_ptr(), 0o600) }, 0);
        assert!(store.load_home_focus().is_err());
    }
    #[test]
    fn sidecar_writes_stay_in_held_original_directory_after_parent_rename() {
        let tmp = Scratch::new();
        std::fs::create_dir(&tmp.0).unwrap();
        let original = tmp.0.join("state");
        let moved = tmp.0.join("moved");
        let store = Store::open(&original).unwrap();
        std::fs::rename(&original, &moved).unwrap();
        std::fs::create_dir(&original).unwrap();
        store.save_home_focus("com.example.a").unwrap();
        assert!(moved.join("home-focus.json").is_file());
        assert!(!original.join("home-focus.json").exists());
    }
    #[test]
    fn one_writer_coalesces_both_files_and_restart_loads_only_acknowledged_latest_state() {
        let path = Scratch::new();
        let (mut writer, _) = Persistence::start(&path.0).unwrap();
        assert_eq!(writer.loaded_home_focus().unwrap(), None);
        writer.queue(&prefs()).unwrap();
        writer.queue_home_focus("com.example.a").unwrap();
        let mut newer = prefs();
        newer.favourites.clear();
        writer.queue(&newer).unwrap();
        writer.queue_home_focus("com.example.b").unwrap();
        let result = writer.finish_all();
        assert_eq!(result.preferences.last().unwrap().as_ref().unwrap(), &newer);
        assert_eq!(
            result.home_focus.last().unwrap().as_ref().unwrap(),
            "com.example.b"
        );
        let (writer, loaded) = Persistence::start(&path.0).unwrap();
        assert_eq!(loaded, Some(newer));
        assert_eq!(
            writer.loaded_home_focus().unwrap().as_deref(),
            Some("com.example.b")
        );
        let result = writer.finish_all();
        assert!(result.preferences.is_empty() && result.home_focus.is_empty());
    }
    #[test]
    fn damaged_optional_sidecar_does_not_disable_or_rewrite_favourites() {
        let path = Scratch::new();
        let store = Store::open(&path.0).unwrap();
        store.save(&prefs()).unwrap();
        store.save_home_focus("com.example.a").unwrap();
        let bad = br#"{"schema":2,"app_id":"com.example.a"}"#;
        std::fs::write(path.0.join("home-focus.json"), bad).unwrap();
        let (mut writer, loaded) = Persistence::start(&path.0).unwrap();
        assert_eq!(loaded, Some(prefs()));
        assert!(writer.loaded_home_focus().is_err());
        writer.queue(&prefs()).unwrap();
        writer.queue_home_focus("com.example.b").unwrap();
        let result = writer.finish_all();
        assert!(result.preferences.iter().all(Result::is_ok));
        assert!(result.home_focus.iter().any(Result::is_err));
        assert_eq!(std::fs::read(path.0.join("home-focus.json")).unwrap(), bad);
    }
}

#[cfg(test)]
mod home_focus_interrupted_tests {
    use super::*;
    #[test]
    fn existing_interrupted_temp_refuses_new_write_without_losing_prior_sidecar() {
        let path = std::env::temp_dir().join(format!(
            "native-home-focus-interrupted-{}-{}",
            std::process::id(),
            SERIAL.fetch_add(1, Ordering::Relaxed)
        ));
        let store = Store::open(&path).unwrap();
        store.save_home_focus("com.example.a").unwrap();
        let before = std::fs::read(path.join("home-focus.json")).unwrap();
        // Exercise the production atomic-write body against an existing interrupted temp.
        let name = c".home-focus-interrupted-test.tmp";
        let temp = path.join(name.to_str().unwrap());
        std::fs::write(&temp, b"incomplete prior attempt").unwrap();
        assert!(store
            .save_home_focus_at_temp("com.example.b", name)
            .is_err());
        assert_eq!(std::fs::read(path.join("home-focus.json")).unwrap(), before);
        assert_eq!(std::fs::read(&temp).unwrap(), b"incomplete prior attempt");
        std::fs::remove_dir_all(path).unwrap();
    }
}
