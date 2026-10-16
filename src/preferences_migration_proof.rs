//! Explicit metadata-only Preview permission proof. No Window, SDK, Root
//! dispatch, preference writer, arbitrary state path or automatic recovery.
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::{io, path::Path};

const MAX_MARKER: usize = 1024;
#[cfg(target_arch = "arm")]
const MARKER_NAME: &str = "preferences-migration-proof.json";
#[derive(Clone, Copy, Debug, PartialEq, Eq, Deserialize, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum Direction {
    Forward,
    Reverse,
}
#[derive(Clone, Debug, PartialEq, Eq, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct Marker {
    pub schema: u32,
    pub direction: Direction,
    pub expected_preferences_sha256: String,
    pub ui_sha256: String,
    pub expected_uid: u32,
    pub expected_gid: u32,
}
fn invalid(message: &str) -> io::Error {
    io::Error::new(io::ErrorKind::InvalidData, message)
}
fn need(ok: bool, message: &str) -> io::Result<()> {
    if ok {
        Ok(())
    } else {
        Err(invalid(message))
    }
}
pub fn parse_marker(raw: &[u8]) -> io::Result<Marker> {
    need(
        !raw.is_empty() && raw.len() <= MAX_MARKER,
        "Bounded migration marker required",
    )?;
    let marker: Marker = serde_json::from_slice(raw).map_err(|e| invalid(&e.to_string()))?;
    let hex = |s: &str| {
        s.len() == 64
            && s.bytes()
                .all(|b| b.is_ascii_digit() || (b'a'..=b'f').contains(&b))
    };
    need(
        marker.schema == 1
            && marker.expected_uid >= 1000
            && hex(&marker.ui_sha256)
            && hex(&marker.expected_preferences_sha256),
        "Closed migration identity/digest marker required",
    )?;
    Ok(marker)
}
#[cfg(any(target_arch = "arm", test))]
fn status(raw: &str, pid: u32, uid: u32, gid: u32) -> io::Result<()> {
    let rows = |label: &str| {
        raw.lines()
            .filter(|s| s.split_whitespace().next() == Some(label))
            .map(|s| s.split_whitespace().collect::<Vec<_>>())
            .collect::<Vec<_>>()
    };
    let pids = rows("Pid:");
    need(
        pid > 1 && uid >= 1000 && pids.len() == 1 && pids[0] == ["Pid:", &pid.to_string()],
        "Exact actual status PID required",
    )?;
    for (label, expected) in [("Uid:", uid), ("Gid:", gid)] {
        let found = rows(label);
        need(
            found.len() == 1 && found[0].len() == 5,
            "Exact UID4/GID4 status rows required",
        )?;
        for field in &found[0][1..] {
            need(
                !field.is_empty()
                    && field.bytes().all(|b| b.is_ascii_digit())
                    && field.parse::<u32>().ok() == Some(expected),
                "Ordinary UID4/GID4 must match marker",
            )?;
        }
    }
    Ok(())
}
#[cfg(any(target_arch = "arm", test))]
fn parent_policy(uid: u32, gid: u32, mode: u32, own_uid: u32, own_gid: u32) -> bool {
    ((uid == 0 && gid == 0) || (uid == own_uid && gid == own_gid)) && mode == 0o1777
        || uid == own_uid && gid == own_gid && mode == 0o700
}

#[cfg(any(target_arch = "arm", test))]
fn proof_receipt_admitted(
    marker: &Marker,
    receipt: &crate::preferences_migration::Receipt,
) -> bool {
    let (initial, final_modes, direction) = match marker.direction {
        Direction::Forward => (
            [0o777, 0o777],
            [0o700, 0o600],
            crate::preferences_migration::Direction::Forward,
        ),
        Direction::Reverse => (
            [0o700, 0o600],
            [0o777, 0o777],
            crate::preferences_migration::Direction::Reverse,
        ),
    };
    receipt.disposition == crate::preferences_migration::Disposition::Migrated
        && receipt.direction == direction
        && receipt.initial_modes == Some(initial)
        && receipt.observed_modes == Some(final_modes)
        && receipt.directory_chmod_attempted
        && receipt.file_chmod_attempted
        && receipt.bytes_sha256.as_deref() == Some(marker.expected_preferences_sha256.as_str())
}

#[cfg(target_arch = "arm")]
mod native {
    use super::*;
    use crate::{broker_installation::Profile, broker_probe::OwnIdentity};
    use serde_json::json;
    use std::{
        ffi::CString,
        fs::{self, File, OpenOptions},
        io::{Read, Seek, SeekFrom, Write},
        os::{
            fd::{AsRawFd, FromRawFd},
            unix::fs::{DirBuilderExt, MetadataExt, OpenOptionsExt},
        },
        path::PathBuf,
        time::{Duration, Instant},
    };
    #[derive(Clone, Debug, PartialEq, Eq, Serialize)]
    struct Stamp {
        device: u64,
        inode: u64,
        uid: u32,
        gid: u32,
        mode: u32,
    }
    fn stamp(m: &fs::Metadata) -> Stamp {
        Stamp {
            device: m.dev(),
            inode: m.ino(),
            uid: m.uid(),
            gid: m.gid(),
            mode: m.mode(),
        }
    }
    fn marker_stamp(m: &fs::Metadata) -> (Stamp, u64, u64, i64, i64, i64, i64) {
        (
            stamp(m),
            m.nlink(),
            m.len(),
            m.mtime(),
            m.mtime_nsec(),
            m.ctime(),
            m.ctime_nsec(),
        )
    }
    struct Request {
        path: PathBuf,
        file: File,
        raw: Vec<u8>,
        marker: Marker,
        stamp: (Stamp, u64, u64, i64, i64, i64, i64),
    }
    impl Request {
        fn read(assets: &Path) -> io::Result<Self> {
            need(
                assets == Path::new(Profile::Preview.bundle()),
                "Only actual Preview asset directory admitted",
            )?;
            let path = assets.join(MARKER_NAME);
            let mut file = OpenOptions::new()
                .read(true)
                .custom_flags(libc::O_NOFOLLOW | libc::O_CLOEXEC | libc::O_NONBLOCK)
                .open(&path)?;
            let metadata = file.metadata()?;
            need(
                metadata.is_file()
                    && metadata.uid() == 0
                    && metadata.gid() == 0
                    && metadata.mode() & 0o7777 == 0o644
                    && metadata.nlink() == 1
                    && metadata.len() > 0
                    && metadata.len() <= MAX_MARKER as u64,
                "Root-owned0644 single-link migration marker required",
            )?;
            let mut raw = Vec::new();
            Read::by_ref(&mut file)
                .take((MAX_MARKER + 1) as u64)
                .read_to_end(&mut raw)?;
            let marker = parse_marker(&raw)?;
            let request = Self {
                path,
                file,
                raw,
                marker,
                stamp: marker_stamp(&metadata),
            };
            request.verify()?;
            Ok(request)
        }
        fn verify(&self) -> io::Result<()> {
            let named = fs::symlink_metadata(&self.path)?;
            need(
                !named.file_type().is_symlink()
                    && marker_stamp(&named) == self.stamp
                    && marker_stamp(&self.file.metadata()?) == self.stamp,
                "Migration marker changed/replaced",
            )?;
            let mut file = self.file.try_clone()?;
            file.seek(SeekFrom::Start(0))?;
            let mut raw = Vec::new();
            file.take((MAX_MARKER + 1) as u64).read_to_end(&mut raw)?;
            need(
                raw == self.raw
                    && marker_stamp(&self.file.metadata()?) == self.stamp
                    && marker_stamp(&fs::symlink_metadata(&self.path)?) == self.stamp,
                "Migration marker content/name changed",
            )
        }
    }
    #[derive(Clone, Debug, PartialEq, Eq, Serialize)]
    struct SelfEvidence {
        own: OwnIdentity,
        uid_fields: [u32; 4],
        gid_fields: [u32; 4],
        exe_link: String,
        root_link: String,
    }
    fn self_evidence(marker: &Marker) -> io::Result<SelfEvidence> {
        let own = crate::broker_probe::own_identity_for_profile(Profile::Preview)
            .map_err(|e| invalid(&e))?;
        need(
            own.uid == marker.expected_uid
                && own.ui_sha256 == marker.ui_sha256
                && unsafe { libc::getegid() } == marker.expected_gid,
            "Marker differs from actual ordinary Preview",
        )?;
        let read_status = || -> io::Result<()> {
            let mut raw = String::new();
            File::open("/proc/self/status")?
                .take(65537)
                .read_to_string(&mut raw)?;
            need(raw.len() <= 65536, "Bounded self status required")?;
            status(&raw, own.pid, marker.expected_uid, marker.expected_gid)
        };
        read_status()?;
        let exe = fs::read_link("/proc/self/exe")?;
        let root = fs::read_link("/proc/self/root")?;
        need(
            exe == Path::new(&Profile::Preview.ui_path()) && root == Path::new("/"),
            "Exact in-jail Preview executable/root links required",
        )?;
        let after = crate::broker_probe::own_identity_for_profile(Profile::Preview)
            .map_err(|e| invalid(&e))?;
        read_status()?;
        need(
            after == own
                && fs::read_link("/proc/self/exe")? == exe
                && fs::read_link("/proc/self/root")? == root,
            "Self process/status/executable changed",
        )?;
        Ok(SelfEvidence {
            own,
            uid_fields: [marker.expected_uid; 4],
            gid_fields: [marker.expected_gid; 4],
            exe_link: exe.to_string_lossy().into_owned(),
            root_link: root.to_string_lossy().into_owned(),
        })
    }
    fn open_dir(path: &Path) -> io::Result<File> {
        OpenOptions::new()
            .read(true)
            .custom_flags(libc::O_DIRECTORY | libc::O_NOFOLLOW | libc::O_CLOEXEC | libc::O_NONBLOCK)
            .open(path)
    }
    struct Runtime {
        parent_path: PathBuf,
        path: PathBuf,
        parent: File,
        leaf: File,
        parent_stamp: Stamp,
        leaf_stamp: Stamp,
        parent_created: bool,
    }
    impl Runtime {
        fn create(own: &SelfEvidence) -> io::Result<Self> {
            let parent_path = PathBuf::from("/tmp/com.ash.nativehome.preview");
            let parent_created = match fs::symlink_metadata(&parent_path) {
                Ok(_) => false,
                Err(e) if e.kind() == io::ErrorKind::NotFound => {
                    fs::DirBuilder::new().mode(0o700).create(&parent_path)?;
                    true
                }
                Err(e) => return Err(e),
            };
            let parent = open_dir(&parent_path)?;
            let metadata = parent.metadata()?;
            need(
                metadata.is_dir()
                    && parent_policy(
                        metadata.uid(),
                        metadata.gid(),
                        metadata.mode() & 0o7777,
                        own.own.uid,
                        own.gid_fields[1],
                    ),
                "Existing runtime parent must be real Root/self sticky1777 or private self0700",
            )?;
            let parent_stamp = stamp(&metadata);
            let named = fs::symlink_metadata(&parent_path)?;
            need(
                !named.file_type().is_symlink() && stamp(&named) == parent_stamp,
                "Runtime parent replaced while opening",
            )?;
            let name = CString::new(format!("shell-{}", own.own.pid))
                .map_err(|e| invalid(&e.to_string()))?;
            if unsafe { libc::mkdirat(parent.as_raw_fd(), name.as_ptr(), 0o700) } < 0 {
                return Err(io::Error::last_os_error());
            }
            let fd = unsafe {
                libc::openat(
                    parent.as_raw_fd(),
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
            let leaf = unsafe { File::from_raw_fd(fd) };
            let m = leaf.metadata()?;
            need(
                m.is_dir()
                    && m.uid() == own.own.uid
                    && m.gid() == own.gid_fields[1]
                    && m.mode() & 0o7777 == 0o700,
                "New own runtime must be0700",
            )?;
            let runtime = Self {
                path: parent_path.join(name.to_str().map_err(|e| invalid(&e.to_string()))?),
                parent_path,
                parent,
                leaf,
                parent_stamp,
                leaf_stamp: stamp(&m),
                parent_created,
            };
            runtime.verify()?;
            Ok(runtime)
        }
        fn verify(&self) -> io::Result<()> {
            for (path, file, expected) in [
                (&self.parent_path, &self.parent, &self.parent_stamp),
                (&self.path, &self.leaf, &self.leaf_stamp),
            ] {
                let named = fs::symlink_metadata(path)?;
                need(
                    named.is_dir()
                        && !named.file_type().is_symlink()
                        && stamp(&named) == *expected
                        && stamp(&file.metadata()?) == *expected,
                    "Own runtime parent/leaf identity changed",
                )?;
            }
            Ok(())
        }
        fn save(&self, report: &Value) -> io::Result<()> {
            self.verify()?;
            let name = c"preferences-migration-proof.json";
            let fd = unsafe {
                libc::openat(
                    self.leaf.as_raw_fd(),
                    name.as_ptr(),
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
            let before = file.metadata()?;
            need(
                before.is_file()
                    && before.uid() == self.leaf_stamp.uid
                    && before.gid() == self.leaf_stamp.gid
                    && before.nlink() == 1
                    && before.mode() & 0o7777 == 0o600,
                "Own report must be a fresh0600 regular single link",
            )?;
            let bytes = serde_json::to_vec(report).map_err(io::Error::other)?;
            file.write_all(&bytes)?;
            file.write_all(b"\n")?;
            file.sync_all()?;
            let after = file.metadata()?;
            need(
                stamp(&before) == stamp(&after)
                    && after.nlink() == 1
                    && after.len() == bytes.len() as u64 + 1,
                "Own report identity changed while writing",
            )?;
            file.seek(SeekFrom::Start(0))?;
            let mut readback = Vec::new();
            Read::by_ref(&mut file)
                .take(bytes.len() as u64 + 2)
                .read_to_end(&mut readback)?;
            let mut expected = bytes;
            expected.push(b'\n');
            need(
                readback == expected
                    && stamp(&file.metadata()?) == stamp(&after)
                    && file.metadata()?.nlink() == 1,
                "Own report bytes changed after sync",
            )?;
            self.leaf.sync_all()?;
            self.verify()?;
            let current = fs::symlink_metadata(self.path.join(MARKER_NAME))?;
            need(
                !current.file_type().is_symlink()
                    && stamp(&current) == stamp(&after)
                    && current.nlink() == 1
                    && current.len() == after.len(),
                "Own named report replaced",
            )
        }
    }
    pub(super) fn run(assets: &Path) -> Result<Value, Box<dyn std::error::Error>> {
        let request = Request::read(assets)?;
        let marker = &request.marker;
        let own = self_evidence(marker)?;
        let runtime = Runtime::create(&own)?;
        let mut report = json!({"schema":1,"profile":"preview","marker":marker,"own_before":own,"own_before_migration":null,"own_after":null,
            "direction":marker.direction,"runtime":runtime.path,"parent_created":runtime.parent_created,"runtime_parent_identity":runtime.parent_stamp,"runtime_leaf_identity":runtime.leaf_stamp,"hold_target_ms":8000,"hold_ms":0,
            "migration_attempted":false,"migration_receipt":null,"migration_failure":null,"qualified":false,"error":null,
            "hbc_calls":0,"root_exec_attempts":0,"resource_jobs":0,"action_requests":0,"preference_content_writes":0,
            "window_created":false,"gl_qualified":false,"foreground_qualified":false,"model_sdk_queried":false,
            "host_jail_prefix_independently_verified":false,"normal_activation":false,"automatic_recovery":false});
        let observed = (|| -> Result<(), Box<dyn std::error::Error>> {
            let began = Instant::now();
            while began.elapsed() < Duration::from_secs(8) {
                std::thread::sleep(Duration::from_millis(20));
            }
            report["hold_ms"] = json!(began.elapsed().as_millis());
            request.verify()?;
            runtime.verify()?;
            let before = self_evidence(marker)?;
            report["own_before_migration"] = json!(before);
            need(before == own, "Self changed before migration")?;
            let state = crate::identity::preferences_dir(crate::identity::PREVIEW)
                .ok_or_else(|| invalid("Fixed Preview state missing"))?;
            report["migration_attempted"] = json!(true);
            let result = match marker.direction {
                Direction::Forward => crate::preferences_migration::forward(
                    &state,
                    &marker.expected_preferences_sha256,
                ),
                Direction::Reverse => crate::preferences_migration::reverse(
                    &state,
                    &marker.expected_preferences_sha256,
                ),
            };
            let migration_error = match result {
                Ok(receipt) => {
                    let admitted = proof_receipt_admitted(marker, &receipt);
                    report["migration_receipt"] = json!(receipt);
                    if admitted {
                        None
                    } else {
                        Some("Controlled proof requires the exact original bytes and both requested metadata steps")
                    }
                }
                Err(error) => {
                    report["migration_failure"] = json!({"receipt":error.receipt,"error":error.error.to_string(),"kind":format!("{:?}",error.error.kind())});
                    Some("Preference migration refused or partially unknown")
                }
            };
            let after = self_evidence(marker)?;
            report["own_after"] = json!(after);
            need(after == own, "Self changed after migration")?;
            request.verify()?;
            runtime.verify()?;
            if let Some(error) = migration_error {
                return Err(error.into());
            }
            Ok(())
        })();
        report["qualified"] = json!(observed.is_ok());
        if let Err(error) = &observed {
            report["error"] = json!(error.to_string());
        }
        runtime.save(&report)?;
        observed?;
        Ok(report)
    }
}

/// Invoked only by the exclusive actual-Preview metadata proof early return.
pub fn run(assets: &Path) -> Result<Value, Box<dyn std::error::Error>> {
    #[cfg(target_arch = "arm")]
    {
        native::run(assets)
    }
    #[cfg(not(target_arch = "arm"))]
    {
        let _ = assets;
        Err(io::Error::new(
            io::ErrorKind::Unsupported,
            "Migration proof requires native ordinary Preview",
        )
        .into())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    fn marker() -> Value {
        serde_json::json!({"schema":1,"direction":"forward","expected_preferences_sha256":"a".repeat(64),"ui_sha256":"b".repeat(64),"expected_uid":6249,"expected_gid":5000})
    }
    #[test]
    fn closed_marker_accepts_only_two_source_operations_and_typed_owner() {
        for d in ["forward", "reverse"] {
            let mut m = marker();
            m["direction"] = serde_json::json!(d);
            assert!(parse_marker(&serde_json::to_vec(&m).unwrap()).is_ok());
        }
        for (key, value) in [
            ("schema", serde_json::json!(true)),
            ("direction", serde_json::json!("repair")),
            ("expected_uid", serde_json::json!(0)),
            ("expected_gid", serde_json::json!(-1)),
            ("ui_sha256", serde_json::json!("B".repeat(64))),
            ("expected_preferences_sha256", serde_json::json!("short")),
            ("path", serde_json::json!("/tmp/foreign")),
        ] {
            let mut m = marker();
            m[key] = value;
            assert!(parse_marker(&serde_json::to_vec(&m).unwrap()).is_err());
        }
        let raw = serde_json::to_string(&marker()).unwrap();
        assert!(parse_marker(raw.replacen('{', "{\"schema\":1,", 1).as_bytes()).is_err());
        assert!(parse_marker(&vec![b'x'; MAX_MARKER + 1]).is_err());
    }
    #[test]
    fn status_requires_every_real_effective_saved_filesystem_uid_and_gid() {
        let raw = "Pid:\t123\nUid:\t6249\t6249\t6249\t6249\nGid:\t5000\t5000\t5000\t5000\n";
        assert!(status(raw, 123, 6249, 5000).is_ok());
        for bad in [
            raw.replace("Pid:\t123", "Pid:\t124"),
            raw.replace("6249\t6249", "6249\t0"),
            raw.replace("5000\t5000", "5000\t0"),
            format!("{raw}Uid:\t6249\t6249\t6249\t6249\n"),
        ] {
            assert!(status(&bad, 123, 6249, 5000).is_err());
        }
        assert!(status(raw, 123, 0, 5000).is_err());
    }
    #[test]
    fn fixed_runtime_parent_accepts_only_owned_private_or_root_self_sticky() {
        assert!(parent_policy(0, 0, 0o1777, 6249, 5000));
        assert!(parent_policy(6249, 5000, 0o1777, 6249, 5000));
        assert!(parent_policy(6249, 5000, 0o700, 6249, 5000));
        for (uid, gid, mode) in [
            (0, 0, 0o777),
            (1001, 5000, 0o1777),
            (6249, 1001, 0o700),
            (0, 0, 0o700),
            (6249, 5000, 0o777),
        ] {
            assert!(!parent_policy(uid, gid, mode, 6249, 5000));
        }
    }
    #[test]
    fn controlled_trial_rejects_noop_wrong_digest_and_partial_receipts() {
        use crate::preferences_migration::{Direction as MigrationDirection, Disposition, Receipt};
        let marker = parse_marker(&serde_json::to_vec(&marker()).unwrap()).unwrap();
        let mut receipt = Receipt {
            schema: 1,
            direction: MigrationDirection::Forward,
            disposition: Disposition::Migrated,
            stage: "complete",
            initial_modes: Some([0o777, 0o777]),
            observed_modes: Some([0o700, 0o600]),
            directory_chmod_attempted: true,
            file_chmod_attempted: true,
            bytes_sha256: Some(marker.expected_preferences_sha256.clone()),
            preferences: None,
        };
        assert!(proof_receipt_admitted(&marker, &receipt));
        receipt.bytes_sha256 = Some("c".repeat(64));
        assert!(!proof_receipt_admitted(&marker, &receipt));
        receipt.bytes_sha256 = Some(marker.expected_preferences_sha256.clone());
        receipt.disposition = Disposition::AlreadyPrivate;
        assert!(!proof_receipt_admitted(&marker, &receipt));
        receipt.disposition = Disposition::Migrated;
        receipt.observed_modes = Some([0o700, 0o777]);
        assert!(!proof_receipt_admitted(&marker, &receipt));
        receipt.observed_modes = Some([0o700, 0o600]);
        receipt.file_chmod_attempted = false;
        assert!(!proof_receipt_admitted(&marker, &receipt));
    }
    #[cfg(not(target_arch = "arm"))]
    #[test]
    fn host_entry_never_claims_native_success() {
        assert!(run(Path::new("/synthetic/no-side-effects")).is_err());
    }
}
