//! Explicit two-object permission migration; never creates or rewrites preferences.
//! The caller must separately admit its app/device role and expected original digest.
use crate::shell::Preferences;
use serde::Serialize;
use std::{
    ffi::CString,
    fmt,
    fs::{self, File, Metadata},
    io::{self, Read, Seek, SeekFrom},
    os::{
        fd::{AsRawFd, FromRawFd},
        unix::fs::MetadataExt,
    },
    path::{Path, PathBuf},
};

const CHILD: &std::ffi::CStr = c"preferences.json";
const MAX_BYTES: u64 = 64 * 1024;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum Direction {
    Forward,
    Reverse,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum Disposition {
    Migrated,
    AlreadyPrivate,
    Refused,
    PartialUnknown,
}
#[derive(Debug, Serialize)]
pub struct Receipt {
    pub schema: u32,
    pub direction: Direction,
    pub disposition: Disposition,
    pub stage: &'static str,
    pub initial_modes: Option<[u32; 2]>,
    pub observed_modes: Option<[u32; 2]>,
    pub directory_chmod_attempted: bool,
    pub file_chmod_attempted: bool,
    pub bytes_sha256: Option<String>,
    pub preferences: Option<Preferences>,
}
#[derive(Debug)]
pub struct Failure {
    pub receipt: Box<Receipt>,
    pub error: io::Error,
}
impl fmt::Display for Failure {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "Preference migration {}: {}",
            self.receipt.stage, self.error
        )
    }
}
impl std::error::Error for Failure {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        Some(&self.error)
    }
}
fn invalid(message: &str) -> io::Error {
    io::Error::new(io::ErrorKind::InvalidData, message)
}
fn require(ok: bool, message: &str) -> io::Result<()> {
    if ok {
        Ok(())
    } else {
        Err(invalid(message))
    }
}
fn digest(bytes: &[u8]) -> String {
    let mut hash = plx_base::sha256::Sha256::new();
    hash.update(bytes);
    hash.finish()
        .iter()
        .map(|byte| format!("{byte:02x}"))
        .collect()
}
fn owner(uid: u32, gid: u32, actual_uid: u32, actual_gid: u32) -> io::Result<()> {
    require(
        uid != 0 && uid == actual_uid && gid == actual_gid,
        "Non-root effective owner and group required",
    )
}
fn mode(metadata: &Metadata) -> u32 {
    metadata.mode() & 0o7777
}
#[derive(Clone, Debug, PartialEq, Eq)]
struct Identity {
    device: u64,
    inode: u64,
    uid: u32,
    gid: u32,
}
impl Identity {
    fn of(meta: &Metadata) -> Self {
        Self {
            device: meta.dev(),
            inode: meta.ino(),
            uid: meta.uid(),
            gid: meta.gid(),
        }
    }
}
#[derive(Clone, Debug, PartialEq, Eq)]
struct FileStamp {
    identity: Identity,
    bytes: u64,
    modified: i64,
    modified_nanos: i64,
}
impl FileStamp {
    fn of(meta: &Metadata) -> Self {
        Self {
            identity: Identity::of(meta),
            bytes: meta.len(),
            modified: meta.mtime(),
            modified_nanos: meta.mtime_nsec(),
        }
    }
}
fn directory(meta: &Metadata, uid: u32, gid: u32) -> io::Result<()> {
    owner(uid, gid, meta.uid(), meta.gid())?;
    require(
        meta.is_dir() && mode(meta) & 0o7000 == 0,
        "Real directory without special permission bits required",
    )
}
fn regular(meta: &Metadata, uid: u32, gid: u32) -> io::Result<()> {
    owner(uid, gid, meta.uid(), meta.gid())?;
    require(
        meta.is_file() && meta.nlink() == 1 && meta.len() <= MAX_BYTES && mode(meta) & 0o7000 == 0,
        "Single-link bounded regular preference file required",
    )
}
fn open_directory(path: &Path) -> io::Result<File> {
    let name = CString::new(path.as_os_str().as_encoded_bytes())
        .map_err(|_| invalid("Invalid state path"))?;
    let fd = unsafe {
        libc::open(
            name.as_ptr(),
            libc::O_RDONLY
                | libc::O_DIRECTORY
                | libc::O_NOFOLLOW
                | libc::O_CLOEXEC
                | libc::O_NONBLOCK,
        )
    };
    if fd < 0 {
        return Err(io::Error::last_os_error());
    }
    Ok(unsafe { File::from_raw_fd(fd) })
}
fn open_child(dir: &File) -> io::Result<File> {
    let fd = unsafe {
        libc::openat(
            dir.as_raw_fd(),
            CHILD.as_ptr(),
            libc::O_RDONLY | libc::O_NOFOLLOW | libc::O_CLOEXEC | libc::O_NONBLOCK,
        )
    };
    if fd < 0 {
        return Err(io::Error::last_os_error());
    }
    Ok(unsafe { File::from_raw_fd(fd) })
}
struct Held {
    path: PathBuf,
    dir: File,
    file: File,
    directory: Identity,
    stamp: FileStamp,
    uid: u32,
    gid: u32,
}
impl Held {
    fn open(path: &Path) -> io::Result<Self> {
        let uid = unsafe { libc::geteuid() };
        let gid = unsafe { libc::getegid() };
        require(uid != 0, "Root permission migration is forbidden")?;
        let leaf = path
            .file_name()
            .ok_or_else(|| invalid("Named state directory required"))?;
        require(
            path.as_os_str()
                .as_encoded_bytes()
                .ends_with(leaf.as_encoded_bytes()),
            "Trailing separators/dot aliases are not admitted",
        )?;
        let dir = open_directory(path)?;
        let meta = dir.metadata()?;
        directory(&meta, uid, gid)?;
        let file = open_child(&dir)?;
        let file_meta = file.metadata()?;
        regular(&file_meta, uid, gid)?;
        Ok(Self {
            path: path.to_owned(),
            dir,
            file,
            directory: Identity::of(&meta),
            stamp: FileStamp::of(&file_meta),
            uid,
            gid,
        })
    }
    fn modes(&self) -> io::Result<[u32; 2]> {
        Ok([mode(&self.dir.metadata()?), mode(&self.file.metadata()?)])
    }
    fn read(&mut self) -> io::Result<Vec<u8>> {
        let before = self.file.metadata()?;
        regular(&before, self.uid, self.gid)?;
        require(
            FileStamp::of(&before) == self.stamp,
            "Preference identity/content metadata changed",
        )?;
        self.file.seek(SeekFrom::Start(0))?;
        let mut bytes = Vec::new();
        Read::by_ref(&mut self.file)
            .take(MAX_BYTES + 1)
            .read_to_end(&mut bytes)?;
        require(
            bytes.len() as u64 == self.stamp.bytes
                && bytes.len() as u64 <= MAX_BYTES
                && FileStamp::of(&self.file.metadata()?) == self.stamp,
            "Preference read changed or exceeded bound",
        )?;
        crate::preferences::decode_record(&bytes)?;
        Ok(bytes)
    }
    fn verify(&mut self, modes: [u32; 2], bytes: &[u8]) -> io::Result<()> {
        require(
            unsafe { libc::geteuid() } == self.uid && unsafe { libc::getegid() } == self.gid,
            "Effective owner changed",
        )?;
        let held_dir = self.dir.metadata()?;
        directory(&held_dir, self.uid, self.gid)?;
        require(
            Identity::of(&held_dir) == self.directory && mode(&held_dir) == modes[0],
            "Held directory changed",
        )?;
        let named_dir_meta = fs::symlink_metadata(&self.path)?;
        directory(&named_dir_meta, self.uid, self.gid)?;
        require(
            Identity::of(&named_dir_meta) == self.directory && mode(&named_dir_meta) == modes[0],
            "Named directory replaced",
        )?;
        let named_dir = open_directory(&self.path)?;
        require(
            Identity::of(&named_dir.metadata()?) == self.directory,
            "Named directory changed during open",
        )?;
        let named_file = open_child(&named_dir)?;
        let named_meta = named_file.metadata()?;
        regular(&named_meta, self.uid, self.gid)?;
        require(
            FileStamp::of(&named_meta) == self.stamp && mode(&named_meta) == modes[1],
            "Named preference file replaced or changed",
        )?;
        let held_meta = self.file.metadata()?;
        regular(&held_meta, self.uid, self.gid)?;
        require(
            mode(&held_meta) == modes[1] && self.read()? == bytes,
            "Held preference mode or bytes changed",
        )?;
        // Recheck the named directory after the content read as well. These are
        // sampled identity checks, not an exclusive lock against same-UID writers.
        let final_dir = fs::symlink_metadata(&self.path)?;
        require(
            final_dir.is_dir()
                && Identity::of(&final_dir) == self.directory
                && mode(&final_dir) == modes[0],
            "Named directory changed after read",
        )?;
        let final_file = open_child(&self.dir)?;
        let final_meta = final_file.metadata()?;
        regular(&final_meta, self.uid, self.gid)?;
        require(
            FileStamp::of(&final_meta) == self.stamp && mode(&final_meta) == modes[1],
            "Named preference changed after read",
        )
    }
    fn apply(
        &mut self,
        direction: Direction,
        expected: &str,
        receipt: &mut Receipt,
        mut after_step: impl FnMut(usize) -> io::Result<()>,
    ) -> io::Result<()> {
        let initial = self.modes()?;
        receipt.initial_modes = Some(initial);
        receipt.observed_modes = Some(initial);
        let bytes = self.read()?;
        receipt.bytes_sha256 = Some(digest(&bytes));
        self.verify(initial, &bytes)?;
        if direction == Direction::Forward && initial == [0o700, 0o600] {
            receipt.preferences = Some(crate::preferences::decode_record(&bytes)?);
            receipt.disposition = Disposition::AlreadyPrivate;
            receipt.stage = "already_private";
            return Ok(());
        }
        require(
            expected.len() == 64
                && expected
                    .bytes()
                    .all(|b| b.is_ascii_digit() || (b'a'..=b'f').contains(&b)),
            "Expected original SHA256 required",
        )?;
        require(
            receipt.bytes_sha256.as_deref() == Some(expected),
            "Original preference SHA256 mismatch",
        )?;
        let (from, to) = match direction {
            Direction::Forward => ([0o777, 0o777], [0o700, 0o600]),
            Direction::Reverse => ([0o700, 0o600], [0o777, 0o777]),
        };
        require(
            initial == from,
            "Exact migration source modes required; mixed/partial modes need explicit recovery",
        )?;
        self.verify(from, &bytes)?;
        // Refuse unsupported sync semantics before changing either object.
        // Later I/O failures still retain an explicit partial/unknown receipt.
        self.file.sync_all()?;
        self.dir.sync_all()?;
        let order = if direction == Direction::Forward {
            [0usize, 1usize]
        } else {
            [1usize, 0usize]
        };
        let mut current = from;
        for index in order {
            receipt.stage = if index == 0 {
                "directory_chmod"
            } else {
                "file_chmod"
            };
            self.verify(current, &bytes)?;
            if index == 0 {
                receipt.directory_chmod_attempted = true;
            } else {
                receipt.file_chmod_attempted = true;
            }
            let target = if index == 0 { &self.dir } else { &self.file };
            if unsafe { libc::fchmod(target.as_raw_fd(), to[index] as libc::mode_t) } < 0 {
                return Err(io::Error::last_os_error());
            }
            current[index] = to[index];
            receipt.observed_modes = self.modes().ok();
            target.sync_all()?;
            self.verify(current, &bytes)?;
            after_step(index)?;
        }
        receipt.stage = "final_sync";
        self.file.sync_all()?;
        self.dir.sync_all()?;
        self.verify(to, &bytes)?;
        receipt.observed_modes = Some(to);
        receipt.preferences = Some(crate::preferences::decode_record(&bytes)?);
        receipt.disposition = Disposition::Migrated;
        receipt.stage = "complete";
        Ok(())
    }
}
fn migrate(path: &Path, expected: &str, direction: Direction) -> Result<Receipt, Failure> {
    migrate_with(path, expected, direction, |_| Ok(()))
}
// Private hook for deterministic failure tests; public operations always use
// the no-op hook. It cannot select paths, operations or recovery behavior.
fn migrate_with(
    path: &Path,
    expected: &str,
    direction: Direction,
    after_step: impl FnMut(usize) -> io::Result<()>,
) -> Result<Receipt, Failure> {
    let mut receipt = Receipt {
        schema: 1,
        direction,
        disposition: Disposition::Refused,
        stage: "admission",
        initial_modes: None,
        observed_modes: None,
        directory_chmod_attempted: false,
        file_chmod_attempted: false,
        bytes_sha256: None,
        preferences: None,
    };
    let result = Held::open(path)
        .and_then(|mut held| held.apply(direction, expected, &mut receipt, after_step));
    match result {
        Ok(()) => Ok(receipt),
        Err(error) => {
            if receipt.directory_chmod_attempted || receipt.file_chmod_attempted {
                receipt.disposition = Disposition::PartialUnknown;
            }
            receipt.preferences = None;
            Err(Failure {
                receipt: Box::new(receipt),
                error,
            })
        }
    }
}
/// Tighten exactly the existing directory and fixed preference child. A valid
/// already-private record needs no historical digest and is never rewritten.
pub fn forward(path: &Path, expected_original_sha256: &str) -> Result<Receipt, Failure> {
    migrate(path, expected_original_sha256, Direction::Forward)
}
/// Restore the recorded legacy modes only when the original bytes still match.
/// A failure never automatically reverses a partially completed operation.
pub fn reverse(path: &Path, expected_original_sha256: &str) -> Result<Receipt, Failure> {
    migrate(path, expected_original_sha256, Direction::Reverse)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::{
        os::unix::fs::{symlink, PermissionsExt},
        sync::atomic::{AtomicU64, Ordering},
    };
    static SERIAL: AtomicU64 = AtomicU64::new(0);
    struct Scratch(PathBuf);
    impl Scratch {
        fn new() -> Self {
            assert_ne!(
                unsafe { libc::geteuid() },
                0,
                "Filesystem migration fixtures require an ordinary owner"
            );
            let path = std::env::temp_dir().join(format!(
                "luma-pref-migration-{}-{}",
                std::process::id(),
                SERIAL.fetch_add(1, Ordering::Relaxed)
            ));
            fs::create_dir(&path).unwrap();
            Self(path)
        }
        fn fixture(&self, bytes: &[u8], modes: [u32; 2]) -> PathBuf {
            let dir = self.0.join("state");
            fs::create_dir(&dir).unwrap();
            fs::write(dir.join("preferences.json"), bytes).unwrap();
            fs::set_permissions(&dir, fs::Permissions::from_mode(modes[0])).unwrap();
            fs::set_permissions(
                dir.join("preferences.json"),
                fs::Permissions::from_mode(modes[1]),
            )
            .unwrap();
            dir
        }
    }
    impl Drop for Scratch {
        fn drop(&mut self) {
            fs::remove_dir_all(&self.0).unwrap();
        }
    }
    fn record() -> Vec<u8> {
        b"{\"schema\":1,\"preferences\":{\"initialized\":true,\"favourites\":[\"com.example.video\"]}}\n".to_vec()
    }
    fn modes(path: &Path) -> [u32; 2] {
        [
            mode(&fs::metadata(path).unwrap()),
            mode(&fs::metadata(path.join("preferences.json")).unwrap()),
        ]
    }
    #[test]
    fn forward_reverse_preserve_all_bytes_and_owner_identity() {
        let tmp = Scratch::new();
        let bytes = record();
        let path = tmp.fixture(&bytes, [0o777, 0o777]);
        let before = FileStamp::of(&fs::metadata(path.join("preferences.json")).unwrap());
        let changed = forward(&path, &digest(&bytes)).unwrap();
        assert_eq!(changed.disposition, Disposition::Migrated);
        assert_eq!(modes(&path), [0o700, 0o600]);
        assert!(changed.directory_chmod_attempted && changed.file_chmod_attempted);
        assert_eq!(fs::read(path.join("preferences.json")).unwrap(), bytes);
        assert_eq!(
            FileStamp::of(&fs::metadata(path.join("preferences.json")).unwrap()),
            before
        );
        reverse(&path, &digest(&bytes)).unwrap();
        assert_eq!(modes(&path), [0o777, 0o777]);
        assert_eq!(fs::read(path.join("preferences.json")).unwrap(), bytes);
    }
    #[test]
    fn failure_after_first_chmod_is_partial_and_never_automatically_reversed() {
        let tmp = Scratch::new();
        let bytes = record();
        let path = tmp.fixture(&bytes, [0o777, 0o777]);
        let error = migrate_with(&path, &digest(&bytes), Direction::Forward, |step| {
            assert_eq!(step, 0);
            Err(io::Error::other("Injected failure after directory sync"))
        })
        .unwrap_err();
        assert_eq!(error.receipt.disposition, Disposition::PartialUnknown);
        assert!(error.receipt.directory_chmod_attempted);
        assert!(!error.receipt.file_chmod_attempted);
        assert!(error.receipt.preferences.is_none());
        assert_eq!(modes(&path), [0o700, 0o777]);
        assert_eq!(fs::read(path.join(CHILD.to_str().unwrap())).unwrap(), bytes);
        assert!(forward(&path, &digest(&bytes)).is_err());
        assert!(reverse(&path, &digest(&bytes)).is_err());
        assert_eq!(modes(&path), [0o700, 0o777]);
    }
    #[test]
    fn already_private_accepts_current_favourites_without_old_digest_or_changes() {
        let tmp = Scratch::new();
        let bytes = record();
        let path = tmp.fixture(&bytes, [0o700, 0o600]);
        let before = fs::metadata(path.join("preferences.json")).unwrap();
        let receipt = forward(&path, "obsolete").unwrap();
        assert_eq!(receipt.disposition, Disposition::AlreadyPrivate);
        assert!(!receipt.directory_chmod_attempted && !receipt.file_chmod_attempted);
        assert!(receipt.preferences.unwrap().initialized);
        assert_eq!(fs::read(path.join("preferences.json")).unwrap(), bytes);
        let after = fs::metadata(path.join("preferences.json")).unwrap();
        assert_eq!(
            (before.ctime(), before.ctime_nsec()),
            (after.ctime(), after.ctime_nsec())
        );
    }
    #[test]
    fn digest_or_schema_failure_does_not_change_any_modes() {
        for bytes in [
            record(),
            b"{\"schema\":2,\"preferences\":{\"favourites\":[]}}".to_vec(),
            b"{\"schema\":1,\"preferences\":{\"favourites\":[\"bad path\"]}}".to_vec(),
        ] {
            let tmp = Scratch::new();
            let path = tmp.fixture(&bytes, [0o777, 0o777]);
            let expected = if bytes == record() {
                "0".repeat(64)
            } else {
                digest(&bytes)
            };
            let failure = forward(&path, &expected).unwrap_err();
            assert_eq!(failure.receipt.disposition, Disposition::Refused);
            assert!(
                !failure.receipt.directory_chmod_attempted && !failure.receipt.file_chmod_attempted
            );
            assert_eq!(modes(&path), [0o777, 0o777]);
        }
    }
    #[test]
    fn reverse_always_requires_original_digest() {
        let tmp = Scratch::new();
        let bytes = record();
        let path = tmp.fixture(&bytes, [0o700, 0o600]);
        assert!(reverse(&path, &"0".repeat(64)).is_err());
        assert_eq!(modes(&path), [0o700, 0o600]);
    }
    #[test]
    fn symlink_directory_and_child_and_hardlink_are_refused_without_changes() {
        let tmp = Scratch::new();
        let bytes = record();
        let path = tmp.fixture(&bytes, [0o777, 0o777]);
        let alias = tmp.0.join("alias");
        symlink(&path, &alias).unwrap();
        assert!(forward(&alias, &digest(&bytes)).is_err());
        assert!(forward(
            &PathBuf::from(format!("{}/", alias.display())),
            &digest(&bytes)
        )
        .is_err());
        let child = path.join("preferences.json");
        let real = tmp.0.join("real");
        fs::rename(&child, &real).unwrap();
        symlink(&real, &child).unwrap();
        assert!(forward(&path, &digest(&bytes)).is_err());
        fs::remove_file(&child).unwrap();
        fs::rename(&real, &child).unwrap();
        fs::hard_link(&child, tmp.0.join("hardlink")).unwrap();
        assert!(forward(&path, &digest(&bytes)).is_err());
        assert_eq!(modes(&path), [0o777, 0o777]);
    }
    #[test]
    fn mixed_and_special_modes_refuse_without_repair() {
        for pair in [
            [0o700, 0o777],
            [0o777, 0o600],
            [0o1777, 0o777],
            [0o777, 0o4600],
        ] {
            let tmp = Scratch::new();
            let bytes = record();
            let path = tmp.fixture(&bytes, pair);
            assert!(forward(&path, &digest(&bytes)).is_err());
            assert_eq!(modes(&path), pair);
        }
    }
    #[test]
    fn root_foreign_owner_and_foreign_group_predicate_refuses() {
        assert!(owner(0, 0, 0, 0).is_err());
        assert!(owner(10, 20, 11, 20).is_err());
        assert!(owner(10, 20, 10, 21).is_err());
        assert!(owner(10, 20, 10, 20).is_ok());
    }
    #[test]
    fn renamed_named_directory_or_replaced_child_is_detected_before_chmod() {
        let tmp = Scratch::new();
        let bytes = record();
        let path = tmp.fixture(&bytes, [0o777, 0o777]);
        let mut held = Held::open(&path).unwrap();
        fs::rename(&path, tmp.0.join("moved")).unwrap();
        fs::create_dir(&path).unwrap();
        assert!(held.verify([0o777, 0o777], &bytes).is_err());
        assert_eq!(modes(&tmp.0.join("moved")), [0o777, 0o777]);
        fs::remove_dir(&path).unwrap();
        fs::rename(tmp.0.join("moved"), &path).unwrap();
        let mut held = Held::open(&path).unwrap();
        fs::rename(path.join("preferences.json"), tmp.0.join("old-file")).unwrap();
        fs::write(path.join("preferences.json"), &bytes).unwrap();
        assert!(held.verify([0o777, 0o777], &bytes).is_err());
        assert_eq!(mode(&fs::metadata(&path).unwrap()), 0o777);
    }
    #[test]
    fn fifo_child_is_nonblocking_and_refused() {
        let tmp = Scratch::new();
        let dir = tmp.0.join("state");
        fs::create_dir(&dir).unwrap();
        let name =
            CString::new(dir.join("preferences.json").as_os_str().as_encoded_bytes()).unwrap();
        assert_eq!(unsafe { libc::mkfifo(name.as_ptr(), 0o600) }, 0);
        assert!(forward(&dir, &"0".repeat(64)).is_err());
    }
}
