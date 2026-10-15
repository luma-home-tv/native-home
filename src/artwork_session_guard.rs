//! Negative-only latch for cooperating ordinary artwork callers. This storage
//! supplies no Root authority, identity proof, recovery grant or process action.
//! The provider must establish actual start/hash/pin identity and known remote
//! completion independently. No crash/power-loss or hostile same-UID guarantee.
use crate::broker_installation::Profile;
use serde::{Deserialize, Serialize};
use std::{
    ffi::CStr,
    fs::{File, Metadata},
    io::{self, Read, Seek, SeekFrom, Write},
    os::{
        fd::{AsRawFd, FromRawFd},
        unix::fs::MetadataExt,
    },
    path::PathBuf,
    sync::Arc,
};
#[cfg(any(target_arch = "arm", test))]
use std::{fs::OpenOptions, os::unix::fs::OpenOptionsExt, path::Path};

const NAME: &CStr = c"artwork-session-pending.json";
const MAX_BYTES: usize = 1024;

fn invalid(message: &'static str) -> io::Error {
    io::Error::new(io::ErrorKind::InvalidData, message)
}
fn denied(message: &'static str) -> io::Error {
    io::Error::new(io::ErrorKind::PermissionDenied, message)
}
fn hex(value: &str, length: usize) -> bool {
    value.len() == length
        && value
            .bytes()
            .all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte))
}

/// Comparison data only. A nonce, PID or hash here never authorizes a Root call.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct PendingIdentity {
    pub schema: u32,
    pub profile: Profile,
    pub caller_pid: u32,
    pub caller_start_ticks: u64,
    pub ui_sha256: String,
    pub nonce: String,
}
impl PendingIdentity {
    fn validate(&self) -> io::Result<()> {
        if self.schema != 1
            || self.caller_pid <= 1
            || self.caller_pid > i32::MAX as u32
            || self.caller_start_ticks == 0
            || !hex(&self.ui_sha256, 64)
            || !hex(&self.nonce, 32)
        {
            return Err(invalid("Invalid closed pending-session identity"));
        }
        Ok(())
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
struct DirectoryStamp(u64, u64, u32, u32, u32);
impl DirectoryStamp {
    fn of(metadata: &Metadata) -> Self {
        // File creation changes directory times; child directories can change
        // nlink. Neither is a stable directory-handle identity predicate.
        Self(
            metadata.dev(),
            metadata.ino(),
            metadata.uid(),
            metadata.gid(),
            metadata.mode(),
        )
    }
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
struct FileStamp {
    device: u64,
    inode: u64,
    uid: u32,
    gid: u32,
    mode: u32,
    links: u64,
    bytes: u64,
    modified: (i64, i64),
    changed: (i64, i64),
}
impl FileStamp {
    fn of(metadata: &Metadata) -> Self {
        Self {
            device: metadata.dev(),
            inode: metadata.ino(),
            uid: metadata.uid(),
            gid: metadata.gid(),
            mode: metadata.mode(),
            links: metadata.nlink(),
            bytes: metadata.len(),
            modified: (metadata.mtime(), metadata.mtime_nsec()),
            changed: (metadata.ctime(), metadata.ctime_nsec()),
        }
    }
}
#[derive(Debug)]
struct Directory {
    path: PathBuf,
    file: File,
    stamp: DirectoryStamp,
    profile: Profile,
    uid: u32,
    gid: u32,
}
struct Record {
    _file: File,
    stamp: FileStamp,
    bytes: Vec<u8>,
    identity: PendingIdentity,
}
impl Directory {
    fn verify(&self) -> io::Result<()> {
        let named = std::fs::symlink_metadata(&self.path)?;
        let held = self.file.metadata()?;
        if named.file_type().is_symlink()
            || !named.is_dir()
            || !held.is_dir()
            || DirectoryStamp::of(&named) != self.stamp
            || DirectoryStamp::of(&held) != self.stamp
        {
            return Err(denied("Own pending-session directory changed"));
        }
        Ok(())
    }
    fn file_metadata(&self, metadata: &Metadata) -> io::Result<()> {
        if !metadata.is_file()
            || metadata.uid() != self.uid
            || metadata.gid() != self.gid
            || metadata.mode() & 0o7777 != 0o600
            || metadata.nlink() != 1
        {
            return Err(denied(
                "Pending-session file type, owner, mode or link count differs",
            ));
        }
        Ok(())
    }
    fn record(&self) -> io::Result<Option<Record>> {
        self.verify()?;
        let fd = unsafe {
            libc::openat(
                self.file.as_raw_fd(),
                NAME.as_ptr(),
                libc::O_RDONLY | libc::O_NOFOLLOW | libc::O_CLOEXEC | libc::O_NONBLOCK,
            )
        };
        if fd < 0 {
            let error = io::Error::last_os_error();
            if error.kind() == io::ErrorKind::NotFound {
                self.verify()?;
                return Ok(None);
            }
            return Err(error);
        }
        let file = unsafe { File::from_raw_fd(fd) };
        let before = file.metadata()?;
        self.file_metadata(&before)?;
        if before.len() == 0 || before.len() > MAX_BYTES as u64 {
            return Err(invalid("Pending-session record size is invalid"));
        }
        let mut bytes = Vec::new();
        (&file)
            .take((MAX_BYTES + 1) as u64)
            .read_to_end(&mut bytes)?;
        let after = file.metadata()?;
        if bytes.len() as u64 != before.len() || FileStamp::of(&before) != FileStamp::of(&after) {
            return Err(invalid("Pending-session record changed during read"));
        }
        let identity: PendingIdentity = serde_json::from_slice(&bytes)
            .map_err(|_| invalid("Malformed closed pending-session record"))?;
        identity.validate()?;
        if identity.profile != self.profile {
            return Err(invalid("Pending-session profile differs"));
        }
        self.verify()?;
        Ok(Some(Record {
            _file: file,
            stamp: FileStamp::of(&after),
            bytes,
            identity,
        }))
    }
}

#[derive(Clone, Debug)]
pub struct Guard {
    directory: Arc<Directory>,
}
impl Guard {
    #[cfg(test)]
    pub(crate) fn open_for_test(path: &Path, profile: Profile) -> io::Result<Self> {
        Self::open_exact(path, profile, unsafe { libc::geteuid() }, unsafe {
            libc::getegid()
        })
    }
    /// Opens existing own preferences storage only; never creates or repairs it.
    pub fn open(profile: Profile) -> io::Result<Self> {
        #[cfg(target_arch = "arm")]
        {
            let uid = unsafe { libc::geteuid() };
            let gid = unsafe { libc::getegid() };
            if plx_base::paths::app_id() != profile.app_id()
                || uid < 1000
                || (profile == Profile::Proof && uid == 5724)
            {
                return Err(denied(
                    "Pending-session guard requires its ordinary native profile",
                ));
            }
            let path = crate::identity::preferences_dir(profile.app_id())
                .ok_or_else(|| denied("Unknown preferences profile"))?;
            Self::open_exact(&path, profile, uid, gid)
        }
        #[cfg(not(target_arch = "arm"))]
        {
            let _ = profile;
            Err(io::Error::new(
                io::ErrorKind::Unsupported,
                "Pending-session guard requires native ARM",
            ))
        }
    }
    #[cfg(any(target_arch = "arm", test))]
    fn open_exact(path: &Path, profile: Profile, uid: u32, gid: u32) -> io::Result<Self> {
        let named = std::fs::symlink_metadata(path)?;
        if !named.is_dir()
            || named.file_type().is_symlink()
            || named.uid() != uid
            || named.gid() != gid
            || named.mode() & 0o7777 != 0o700
        {
            return Err(denied("Existing own preferences directory is not private"));
        }
        let file = OpenOptions::new()
            .read(true)
            .custom_flags(libc::O_DIRECTORY | libc::O_NOFOLLOW | libc::O_CLOEXEC)
            .open(path)?;
        let stamp = DirectoryStamp::of(&file.metadata()?);
        if stamp != DirectoryStamp::of(&named) {
            return Err(denied("Preferences directory changed while opening"));
        }
        let directory = Arc::new(Directory {
            path: path.to_path_buf(),
            file,
            stamp,
            profile,
            uid,
            gid,
        });
        directory.verify()?;
        Ok(Self { directory })
    }
    /// Existing valid records always block, regardless of age, PID or nonce.
    /// Invalid or unreadable records return an error, which also means BLOCK.
    pub fn pending(&self) -> io::Result<bool> {
        self.directory.record().map(|value| value.is_some())
    }
    /// Atomic O_EXCL arbitration. Every failure after file creation leaves the
    /// possibly partial marker in place. No Drop path removes it.
    pub fn begin(&self, identity: PendingIdentity) -> io::Result<Ticket> {
        identity.validate()?;
        if identity.profile != self.directory.profile || identity.caller_pid != std::process::id() {
            return Err(denied(
                "Pending-session identity is not this profile and caller",
            ));
        }
        self.directory.verify()?;
        let mut bytes = serde_json::to_vec(&identity).map_err(io::Error::other)?;
        bytes.push(b'\n');
        if bytes.len() > MAX_BYTES {
            return Err(invalid("Pending-session serialization exceeds bound"));
        }
        let fd = unsafe {
            libc::openat(
                self.directory.file.as_raw_fd(),
                NAME.as_ptr(),
                libc::O_RDWR
                    | libc::O_CREAT
                    | libc::O_EXCL
                    | libc::O_NOFOLLOW
                    | libc::O_CLOEXEC
                    | libc::O_NONBLOCK,
                0o600,
            )
        };
        if fd < 0 {
            return Err(io::Error::last_os_error());
        }
        let mut file = unsafe { File::from_raw_fd(fd) };
        self.directory.file_metadata(&file.metadata()?)?;
        file.write_all(&bytes)?;
        file.sync_all()?;
        let stamp = FileStamp::of(&file.metadata()?);
        let named = self
            .directory
            .record()?
            .ok_or_else(|| invalid("New pending-session name disappeared"))?;
        if named.stamp != stamp || named.bytes != bytes || named.identity != identity {
            return Err(invalid("New pending-session name or bytes changed"));
        }
        self.directory.file.sync_all()?;
        self.directory.verify()?;
        Ok(Ticket {
            directory: self.directory.clone(),
            file,
            stamp,
            identity,
            bytes,
        })
    }
}

/// Only this new record can be cleared, and only after independently known
/// completion. Dropping a Ticket intentionally leaves the pending name intact.
#[derive(Debug)]
#[must_use = "Dropping the ticket retains the pending marker and blocks later dispatch"]
pub struct Ticket {
    directory: Arc<Directory>,
    file: File,
    stamp: FileStamp,
    identity: PendingIdentity,
    bytes: Vec<u8>,
}
impl Ticket {
    pub fn finish_known(mut self) -> io::Result<()> {
        self.directory.verify()?;
        if FileStamp::of(&self.file.metadata()?) != self.stamp {
            return Err(invalid("Held pending-session record changed"));
        }
        self.file.seek(SeekFrom::Start(0))?;
        let mut held = Vec::new();
        (&self.file)
            .take((MAX_BYTES + 1) as u64)
            .read_to_end(&mut held)?;
        if held != self.bytes || FileStamp::of(&self.file.metadata()?) != self.stamp {
            return Err(invalid("Held pending-session bytes changed"));
        }
        let named = self
            .directory
            .record()?
            .ok_or_else(|| invalid("Pending-session name absent"))?;
        if named.stamp != self.stamp || named.identity != self.identity || named.bytes != self.bytes
        {
            return Err(invalid(
                "Pending-session name no longer identifies this ticket",
            ));
        }
        self.file.sync_all()?;
        self.directory.file.sync_all()?;
        self.directory.verify()?;
        let final_name = self
            .directory
            .record()?
            .ok_or_else(|| invalid("Pending-session name absent"))?;
        if final_name.stamp != self.stamp
            || final_name.bytes != self.bytes
            || final_name.identity != self.identity
        {
            return Err(invalid(
                "Pending-session name changed before known completion",
            ));
        }
        if unsafe { libc::unlinkat(self.directory.file.as_raw_fd(), NAME.as_ptr(), 0) } != 0 {
            return Err(io::Error::last_os_error());
        }
        // This sync is requested after an already-known completion. An error
        // means directory durability is unknown, not that an unknown Root
        // session can be treated as closed or that power-loss safety is proven.
        self.directory.file.sync_all()?;
        self.directory.verify()
    }
}
impl Drop for Ticket {
    fn drop(&mut self) {
        // File/dir handles close normally; there is intentionally no unlink.
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::{
        os::unix::fs::{symlink, PermissionsExt},
        sync::{
            atomic::{AtomicU64, Ordering},
            Barrier,
        },
    };
    static SERIAL: AtomicU64 = AtomicU64::new(0);
    struct Fixture {
        root: PathBuf,
        state: PathBuf,
    }
    impl Fixture {
        fn new() -> Self {
            let root = std::env::temp_dir().join(format!(
                "luma-session-guard-{}-{}",
                std::process::id(),
                SERIAL.fetch_add(1, Ordering::Relaxed)
            ));
            std::fs::create_dir(&root).unwrap();
            let state = root.join("state");
            std::fs::create_dir(&state).unwrap();
            std::fs::set_permissions(&state, std::fs::Permissions::from_mode(0o700)).unwrap();
            std::fs::write(
                state.join("preferences.json"),
                b"{\"favourites\":[\"synthetic.fixture\"]}\n",
            )
            .unwrap();
            Self { root, state }
        }
        fn guard(&self) -> Guard {
            Guard::open_exact(
                &self.state,
                Profile::Proof,
                unsafe { libc::geteuid() },
                unsafe { libc::getegid() },
            )
            .unwrap()
        }
        fn marker(&self) -> PathBuf {
            self.state.join(NAME.to_str().unwrap())
        }
        fn write_marker(&self, bytes: &[u8]) {
            std::fs::write(self.marker(), bytes).unwrap();
            std::fs::set_permissions(self.marker(), std::fs::Permissions::from_mode(0o600))
                .unwrap();
        }
    }
    impl Drop for Fixture {
        fn drop(&mut self) {
            let _ = std::fs::remove_dir_all(&self.root);
        }
    }
    fn identity() -> PendingIdentity {
        PendingIdentity {
            schema: 1,
            profile: Profile::Proof,
            caller_pid: std::process::id(),
            caller_start_ticks: 123,
            ui_sha256: "a".repeat(64),
            nonce: "b".repeat(32),
        }
    }
    #[test]
    fn known_completion_clears_only_its_new_record_and_preserves_favourites() {
        let f = Fixture::new();
        let before = std::fs::read(f.state.join("preferences.json")).unwrap();
        let g = f.guard();
        assert!(!g.pending().unwrap());
        let t = g.begin(identity()).unwrap();
        assert!(g.pending().unwrap());
        t.finish_known().unwrap();
        assert!(!g.pending().unwrap());
        g.begin(identity()).unwrap().finish_known().unwrap();
        assert_eq!(
            std::fs::read(f.state.join("preferences.json")).unwrap(),
            before
        );
    }
    #[test]
    fn dropped_ticket_and_restart_keep_pending_without_pid_or_nonce_recovery() {
        let f = Fixture::new();
        let g = f.guard();
        let t = g.begin(identity()).unwrap();
        let bytes = std::fs::read(f.marker()).unwrap();
        drop(t);
        drop(g);
        let restarted = f.guard();
        assert!(restarted.pending().unwrap());
        let mut next = identity();
        next.nonce = "c".repeat(32);
        assert_eq!(
            restarted.begin(next).unwrap_err().kind(),
            io::ErrorKind::AlreadyExists
        );
        assert_eq!(std::fs::read(f.marker()).unwrap(), bytes);
        let mut old = identity();
        old.caller_pid = 2;
        old.caller_start_ticks = 1;
        f.write_marker(&serde_json::to_vec(&old).unwrap());
        assert!(restarted.pending().unwrap());
    }
    #[test]
    fn two_independent_instances_arbitrate_one_atomic_creation() {
        let f = Fixture::new();
        let barrier = Arc::new(Barrier::new(3));
        let mut threads = Vec::new();
        for _ in 0..2 {
            let guard = f.guard();
            let ready = barrier.clone();
            threads.push(std::thread::spawn(move || {
                ready.wait();
                guard.begin(identity())
            }));
        }
        barrier.wait();
        let mut won = None;
        let mut refused = 0;
        for thread in threads {
            match thread.join().unwrap() {
                Ok(t) => {
                    assert!(won.replace(t).is_none());
                }
                Err(e) => {
                    assert_eq!(e.kind(), io::ErrorKind::AlreadyExists);
                    refused += 1;
                }
            }
        }
        assert_eq!(refused, 1);
        assert!(f.guard().pending().unwrap());
        won.unwrap().finish_known().unwrap();
        assert!(!f.guard().pending().unwrap());
    }
    #[test]
    fn malformed_partial_duplicate_and_wrong_profile_records_block() {
        let f = Fixture::new();
        let g = f.guard();
        let good = serde_json::to_string(&identity()).unwrap();
        let mut other = identity();
        other.profile = Profile::Preview;
        for raw in [
            Vec::new(),
            b"{}".to_vec(),
            good.replace("\"schema\":1", "\"schema\":true").into_bytes(),
            good.replacen('{', "{\"schema\":1,", 1).into_bytes(),
            good.replacen('{', "{\"recovery\":true,", 1).into_bytes(),
            serde_json::to_vec(&other).unwrap(),
            vec![b'x'; MAX_BYTES + 1],
        ] {
            f.write_marker(&raw);
            assert!(g.pending().is_err());
            assert!(g.begin(identity()).is_err());
            assert_eq!(std::fs::read(f.marker()).unwrap(), raw);
        }
    }
    #[test]
    fn symlink_and_hardlink_markers_never_follow_or_clear_favourites() {
        let f = Fixture::new();
        let before = std::fs::read(f.state.join("preferences.json")).unwrap();
        let g = f.guard();
        symlink(f.state.join("preferences.json"), f.marker()).unwrap();
        assert!(g.pending().is_err());
        assert!(g.begin(identity()).is_err());
        std::fs::remove_file(f.marker()).unwrap();
        let t = g.begin(identity()).unwrap();
        std::fs::hard_link(f.marker(), f.state.join("extra-link")).unwrap();
        assert!(g.pending().is_err());
        assert!(t.finish_known().is_err());
        assert!(f.marker().exists());
        assert_eq!(
            std::fs::read(f.state.join("preferences.json")).unwrap(),
            before
        );
    }
    #[test]
    fn changed_mode_or_bytes_refuses_known_clear_and_retains_pending() {
        for bytes in [false, true] {
            let f = Fixture::new();
            let g = f.guard();
            let t = g.begin(identity()).unwrap();
            if bytes {
                let mut changed = identity();
                changed.nonce = "c".repeat(32);
                f.write_marker(&serde_json::to_vec(&changed).unwrap());
            } else {
                std::fs::set_permissions(f.marker(), std::fs::Permissions::from_mode(0o400))
                    .unwrap();
            }
            assert!(t.finish_known().is_err());
            assert!(f.marker().exists());
        }
    }
    #[test]
    fn file_and_directory_replacement_refuse_old_ticket() {
        let f = Fixture::new();
        let g = f.guard();
        let t = g.begin(identity()).unwrap();
        std::fs::rename(f.marker(), f.state.join("old-marker")).unwrap();
        f.write_marker(&serde_json::to_vec(&identity()).unwrap());
        assert!(t.finish_known().is_err());
        assert!(g.pending().unwrap());
        let f = Fixture::new();
        let g = f.guard();
        let t = g.begin(identity()).unwrap();
        std::fs::rename(&f.state, f.root.join("old-state")).unwrap();
        std::fs::create_dir(&f.state).unwrap();
        std::fs::set_permissions(&f.state, std::fs::Permissions::from_mode(0o700)).unwrap();
        assert!(g.pending().is_err());
        assert!(g.begin(identity()).is_err());
        assert!(t.finish_known().is_err());
        assert!(f
            .root
            .join("old-state")
            .join(NAME.to_str().unwrap())
            .exists());
    }
    #[test]
    fn invalid_identity_or_unprotected_directory_is_never_repaired() {
        let f = Fixture::new();
        let g = f.guard();
        for change in 0..5 {
            let mut bad = identity();
            match change {
                0 => bad.schema = 2,
                1 => bad.caller_pid = std::process::id() + 1,
                2 => bad.caller_start_ticks = 0,
                3 => bad.ui_sha256 = "A".repeat(64),
                _ => bad.nonce = "x".into(),
            };
            assert!(g.begin(bad).is_err());
            assert!(!f.marker().exists());
        }
        std::fs::set_permissions(&f.state, std::fs::Permissions::from_mode(0o755)).unwrap();
        assert!(Guard::open_exact(
            &f.state,
            Profile::Proof,
            unsafe { libc::geteuid() },
            unsafe { libc::getegid() }
        )
        .is_err());
        assert_eq!(std::fs::metadata(&f.state).unwrap().mode() & 0o7777, 0o755);
    }
    #[test]
    fn wrong_owner_group_or_symlink_directory_is_refused_without_changes() {
        let f = Fixture::new();
        let uid = unsafe { libc::geteuid() };
        let gid = unsafe { libc::getegid() };
        assert!(Guard::open_exact(&f.state, Profile::Proof, uid + 1, gid).is_err());
        assert!(Guard::open_exact(&f.state, Profile::Proof, uid, gid + 1).is_err());
        let old = f.root.join("real-state");
        std::fs::rename(&f.state, &old).unwrap();
        symlink(&old, &f.state).unwrap();
        assert!(Guard::open_exact(&f.state, Profile::Proof, uid, gid).is_err());
        assert_eq!(
            std::fs::read(old.join("preferences.json")).unwrap(),
            b"{\"favourites\":[\"synthetic.fixture\"]}\n"
        );
    }
    #[cfg(not(target_arch = "arm"))]
    #[test]
    fn public_host_open_is_unsupported() {
        assert_eq!(
            Guard::open(Profile::Proof).unwrap_err().kind(),
            io::ErrorKind::Unsupported
        );
    }
}
