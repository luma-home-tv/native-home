//! Root-private installed policy, independent of the writable app hierarchy.
//! UI paths are read-only evidence; protected pins and actual kernel peer
//! identity authorize requests. No enrollment, launch, LS2, writes or repair.
use crate::{broker_protocol::PeerEvidence, broker_transport::Credentials};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use std::io;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Profile {
    Proof,
    Preview,
}
impl Profile {
    pub fn app_id(self) -> &'static str {
        match self {
            Self::Proof => crate::identity::PROOF,
            Self::Preview => crate::identity::PREVIEW,
        }
    }
    pub fn bundle(self) -> &'static str {
        match self {
            Self::Proof => "/media/developer/apps/usr/palm/applications/com.ash.nativehome.proof",
            Self::Preview => {
                "/media/developer/apps/usr/palm/applications/com.ash.nativehome.preview"
            }
        }
    }
    pub fn installation_dir(self) -> &'static str {
        match self {
            Self::Proof => "/var/lib/luma-home-proof",
            Self::Preview => "/var/lib/luma-home",
        }
    }
    pub fn helper_basename(self) -> &'static str {
        match self {
            Self::Proof => "artwork-broker-startup-proof",
            Self::Preview => "artwork-broker",
        }
    }
    pub fn jail_root(self) -> String {
        format!("/var/palm/jail/{}", self.app_id())
    }
    pub fn ui_path(self) -> String {
        format!("{}/home-preview", self.bundle())
    }
    pub fn peer_exe_path(self) -> String {
        format!("{}{}", self.jail_root(), self.ui_path())
    }
    pub fn helper_path(self) -> String {
        format!("{}/{}", self.installation_dir(), self.helper_basename())
    }
    pub fn config_path(self) -> String {
        format!("{}/broker-installation.json", self.installation_dir())
    }
}
fn invalid(detail: &'static str) -> io::Error {
    io::Error::new(io::ErrorKind::InvalidData, detail)
}
fn hex(value: &str, len: usize) -> bool {
    value.len() == len
        && value
            .bytes()
            .all(|b| b.is_ascii_digit() || (b'a'..=b'f').contains(&b))
}
/// Parsing alone supplies no authority. Only a securely loaded Installation can
/// use these values to admit a peer. The nonce is rendezvous, not authorization.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Pins {
    pub schema: u32,
    pub profile: Profile,
    pub rendezvous_nonce: String,
    pub ui_sha256: String,
    pub broker_sha256: String,
    pub context_reader_sha256: String,
}
impl Pins {
    pub fn parse(raw: &[u8]) -> io::Result<Self> {
        if raw.is_empty() || raw.len() > 4096 {
            return Err(invalid("Installed policy size"));
        }
        let value: Self =
            serde_json::from_slice(raw).map_err(|_| invalid("Closed installed policy JSON"))?;
        value.validate()?;
        Ok(value)
    }
    fn validate(&self) -> io::Result<()> {
        if self.schema != 1
            || !hex(&self.rendezvous_nonce, 32)
            || [
                &self.ui_sha256,
                &self.broker_sha256,
                &self.context_reader_sha256,
            ]
            .iter()
            .any(|v| !hex(v, 64))
        {
            return Err(invalid("Installed policy version/nonce/hash"));
        }
        Ok(())
    }
}
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
struct FileIdentity {
    device: u64,
    inode: u64,
    bytes: u64,
    mode: u32,
    uid: u32,
    gid: u32,
    links: u64,
    mtime: i64,
    mtime_ns: i64,
    ctime: i64,
    ctime_ns: i64,
}
#[cfg(any(target_arch = "arm", test))]
impl FileIdentity {
    fn admit(&self, mode: u32, cap: u64) -> io::Result<()> {
        if self.mode & 0o170000 != 0o100000
            || self.mode & 0o7777 != mode
            || self.uid != 0
            || self.gid != 0
            || self.links != 1
            || self.inode == 0
            || self.bytes == 0
            || self.bytes > cap
            || !(0..1_000_000_000).contains(&self.mtime_ns)
            || !(0..1_000_000_000).contains(&self.ctime_ns)
        {
            return Err(invalid(
                "Installed regular file ownership/mode/identity/bound",
            ));
        }
        Ok(())
    }
}
#[cfg(any(target_arch = "arm", test))]
fn unchanged(expected: &FileIdentity, current: &FileIdentity) -> io::Result<()> {
    if expected != current {
        return Err(invalid("Installed file identity changed"));
    }
    Ok(())
}
#[cfg(any(target_arch = "arm", test))]
fn pinned(measured: &Measurement, expected: &str) -> io::Result<()> {
    if measured.sha256 != expected {
        return Err(invalid("Executable differs from protected digest"));
    }
    Ok(())
}
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
struct DirectoryIdentity {
    device: u64,
    inode: u64,
    mode: u32,
    uid: u32,
    gid: u32,
}
#[cfg(any(target_arch = "arm", test))]
impl DirectoryIdentity {
    fn admit(&self, authority: bool) -> io::Result<()> {
        if self.mode & 0o170000 != 0o040000
            || self.inode == 0
            || self.uid != 0
            || self.gid != 0
            || (authority && self.mode & 0o022 != 0)
        {
            return Err(invalid(
                "Root installation anchor ownership/type/write authority",
            ));
        }
        Ok(())
    }
}
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
struct Anchor {
    path: String,
    identity: DirectoryIdentity,
}
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
struct Measurement {
    sha256: String,
    identity: FileIdentity,
}
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
struct RootProcess {
    pid: u32,
    uid_fields: [u32; 4],
    start_ticks: u64,
    exe_path: String,
    exe_sha256: String,
    root_path: String,
}
#[derive(Clone, Debug, PartialEq, Eq)]
struct Snapshot {
    config: Measurement,
    ui: Measurement,
    helper: Measurement,
    authority_anchors: Vec<Anchor>,
    ui_lookup_anchors: Vec<Anchor>,
    root_process: RootProcess,
}
/// No public constructor accepts pins, paths or process evidence as authority.
pub struct Installation {
    pins: Pins,
    snapshot: Snapshot,
}
impl Installation {
    pub fn load(profile: Profile) -> io::Result<Self> {
        #[cfg(target_arch = "arm")]
        {
            native::load(profile)
        }
        #[cfg(not(target_arch = "arm"))]
        {
            let _ = profile;
            Err(io::Error::new(
                io::ErrorKind::Unsupported,
                "Installed Root authorization requires native ARM qualification",
            ))
        }
    }
    pub fn pins(&self) -> &Pins {
        &self.pins
    }
    pub fn evidence(&self) -> Value {
        let profile = self.pins.profile;
        json!({"schema":1,"profile":profile,"app_id":profile.app_id(),"bundle":profile.bundle(),"pins":self.pins,
            "files":{"configuration":{"path":profile.config_path(),"sha256":self.snapshot.config.sha256,"identity":self.snapshot.config.identity,"trusted_for_execution":true},
            "ui":{"path":profile.ui_path(),"sha256":self.snapshot.ui.sha256,"identity":self.snapshot.ui.identity,"trusted_for_execution":false},
            "helper":{"path":profile.helper_path(),"sha256":self.snapshot.helper.sha256,"identity":self.snapshot.helper.identity,"trusted_for_execution":true}},
            "authority_anchors":self.snapshot.authority_anchors,"ui_lookup_anchors":self.snapshot.ui_lookup_anchors,
            "root_process":self.snapshot.root_process,"context_reader_measured_here":false,"normal_runtime_qualified":false,
            "scope":"Protected installed pins plus kernel peer identity; UI ancestry, HBC/loader/environment are not made trusted by a hash"})
    }
    pub fn revalidate(&self) -> io::Result<()> {
        let current = Self::load(self.pins.profile)?;
        if self.pins != current.pins || self.snapshot != current.snapshot {
            return Err(invalid("Installed authorization or Root process changed"));
        }
        Ok(())
    }
    pub fn admit_peer(
        &self,
        connect: &Credentials,
        message: &Credentials,
        peer: &PeerEvidence,
    ) -> io::Result<()> {
        self.revalidate()?;
        admit_pins(&self.pins, connect, message, peer)
    }
}
fn admit_pins(
    pins: &Pins,
    connect: &Credentials,
    message: &Credentials,
    peer: &PeerEvidence,
) -> io::Result<()> {
    pins.validate()?;
    if connect != message
        || connect.pid <= 1
        || u32::try_from(connect.pid).ok() != Some(peer.pid)
        || connect.uid < 1000
        || peer.uid_fields != [connect.uid; 4]
        || peer.start_ticks == 0
        || peer.exe_sha256 != pins.ui_sha256
        || peer.root_path != pins.profile.jail_root()
        || peer.exe_path != pins.profile.peer_exe_path()
        || (pins.profile == Profile::Proof && connect.uid == 5724)
    {
        return Err(invalid(
            "Actual peer credentials/UID4/start/profile/path/executable refused",
        ));
    }
    Ok(())
}

#[cfg(any(target_arch = "arm", test))]
mod native {
    use super::*;
    use std::{
        ffi::CString,
        fs::{File, Metadata, OpenOptions},
        io::Read,
        os::{
            fd::{AsRawFd, FromRawFd},
            unix::fs::{MetadataExt, OpenOptionsExt},
        },
    };
    const ELF_LIMIT: u64 = 8 * 1024 * 1024;
    impl FileIdentity {
        fn of(m: &Metadata) -> Self {
            Self {
                device: m.dev(),
                inode: m.ino(),
                bytes: m.len(),
                mode: m.mode(),
                uid: m.uid(),
                gid: m.gid(),
                links: m.nlink(),
                mtime: m.mtime(),
                mtime_ns: m.mtime_nsec(),
                ctime: m.ctime(),
                ctime_ns: m.ctime_nsec(),
            }
        }
    }
    impl DirectoryIdentity {
        fn of(m: &Metadata) -> Self {
            Self {
                device: m.dev(),
                inode: m.ino(),
                mode: m.mode(),
                uid: m.uid(),
                gid: m.gid(),
            }
        }
    }
    fn child(parent: &File, name: &str, directory: bool) -> io::Result<File> {
        let name = CString::new(name).map_err(|_| invalid("Fixed component contains NUL"))?;
        let flags = libc::O_RDONLY
            | libc::O_CLOEXEC
            | libc::O_NOFOLLOW
            | if directory {
                libc::O_DIRECTORY
            } else {
                libc::O_NONBLOCK
            };
        let fd = unsafe { libc::openat(parent.as_raw_fd(), name.as_ptr(), flags) };
        if fd < 0 {
            return Err(io::Error::last_os_error());
        }
        Ok(unsafe { File::from_raw_fd(fd) })
    }
    struct Walk {
        files: Vec<File>,
        stamps: Vec<Anchor>,
        authority: bool,
    }
    impl Walk {
        fn open(path: &str, authority: bool) -> io::Result<Self> {
            let root = OpenOptions::new()
                .read(true)
                .custom_flags(libc::O_DIRECTORY | libc::O_NOFOLLOW | libc::O_CLOEXEC)
                .open("/")?;
            let first = DirectoryIdentity::of(&root.metadata()?);
            first.admit(authority)?;
            let mut walk = Self {
                files: vec![root],
                stamps: vec![Anchor {
                    path: "/".into(),
                    identity: first,
                }],
                authority,
            };
            let mut current = String::new();
            for part in path.trim_start_matches('/').split('/') {
                if part.is_empty() || matches!(part, "." | "..") {
                    return Err(invalid("Invalid fixed installation component"));
                }
                let file = child(
                    walk.files.last().ok_or_else(|| invalid("Missing anchor"))?,
                    part,
                    true,
                )?;
                let identity = DirectoryIdentity::of(&file.metadata()?);
                identity.admit(authority)?;
                current.push('/');
                current.push_str(part);
                walk.files.push(file);
                walk.stamps.push(Anchor {
                    path: current.clone(),
                    identity,
                });
            }
            walk.verify()?;
            Ok(walk)
        }
        fn last(&self) -> io::Result<&File> {
            self.files
                .last()
                .ok_or_else(|| invalid("Empty fixed path walk"))
        }
        fn verify(&self) -> io::Result<()> {
            for (index, (file, stamp)) in self.files.iter().zip(&self.stamps).enumerate() {
                let identity = DirectoryIdentity::of(&file.metadata()?);
                identity.admit(self.authority)?;
                if identity != stamp.identity {
                    return Err(invalid("Installation directory FD changed"));
                }
                let current = if index == 0 {
                    OpenOptions::new()
                        .read(true)
                        .custom_flags(libc::O_DIRECTORY | libc::O_NOFOLLOW | libc::O_CLOEXEC)
                        .open("/")?
                } else {
                    child(
                        &self.files[index - 1],
                        stamp
                            .path
                            .rsplit('/')
                            .next()
                            .ok_or_else(|| invalid("Missing component"))?,
                        true,
                    )?
                };
                if DirectoryIdentity::of(&current.metadata()?) != identity {
                    return Err(invalid("Installation directory current name changed"));
                }
            }
            Ok(())
        }
    }
    fn measure(
        file: &mut File,
        mode: u32,
        cap: u64,
        capture: bool,
    ) -> io::Result<(Measurement, Vec<u8>)> {
        let before = FileIdentity::of(&file.metadata()?);
        before.admit(mode, cap)?;
        let mut hash = plx_base::sha256::Sha256::new();
        let mut buffer = [0u8; 32768];
        let mut total = 0u64;
        let mut data = Vec::new();
        loop {
            let size = usize::try_from((cap - total + 1).min(buffer.len() as u64))
                .map_err(|_| invalid("Bound overflow"))?;
            let count = file.read(&mut buffer[..size])?;
            if count == 0 {
                break;
            }
            total += count as u64;
            if total > cap {
                return Err(invalid("Installed file grew past bound"));
            }
            hash.update(&buffer[..count]);
            let keep = if capture {
                count
            } else {
                count.min(52usize.saturating_sub(data.len()))
            };
            data.extend_from_slice(&buffer[..keep]);
        }
        if total != before.bytes {
            return Err(invalid("Installed file size changed while hashing"));
        }
        unchanged(&before, &FileIdentity::of(&file.metadata()?))?;
        if !capture
            && (data.len() < 52
                || data[..7] != [0x7f, b'E', b'L', b'F', 1, 1, 1]
                || !matches!(u16::from_le_bytes([data[16], data[17]]), 2 | 3)
                || u16::from_le_bytes([data[18], data[19]]) != 40
                || u32::from_le_bytes([data[20], data[21], data[22], data[23]]) != 1)
        {
            return Err(invalid("Installed native file is not ARM ELF"));
        }
        Ok((
            Measurement {
                sha256: hash.finish().iter().map(|b| format!("{b:02x}")).collect(),
                identity: before,
            },
            data,
        ))
    }
    fn fixed_file(
        walk: &Walk,
        name: &str,
        mode: u32,
        cap: u64,
        capture: bool,
    ) -> io::Result<(Measurement, Vec<u8>)> {
        let mut file = child(walk.last()?, name, false)?;
        let measured = measure(&mut file, mode, cap, capture)?;
        verify_named(walk, name, &measured.0)?;
        walk.verify()?;
        Ok(measured)
    }
    fn verify_named(walk: &Walk, name: &str, expected: &Measurement) -> io::Result<()> {
        unchanged(
            &expected.identity,
            &FileIdentity::of(&child(walk.last()?, name, false)?.metadata()?),
        )?;
        walk.verify()
    }
    fn proc_text(name: &str) -> io::Result<String> {
        let mut value = String::new();
        File::open(format!("/proc/self/{name}"))?
            .take(65537)
            .read_to_string(&mut value)?;
        if value.len() > 65536 {
            return Err(invalid("Oversize self proc evidence"));
        }
        Ok(value)
    }
    fn process(raw: &str, status: &str, pid: u32) -> io::Result<([u32; 4], u64)> {
        let rows: Vec<_> = status
            .lines()
            .filter(|line| line.split_whitespace().next() == Some("Uid:"))
            .collect();
        let ids: Vec<_> = status
            .lines()
            .filter(|line| line.split_whitespace().next() == Some("Pid:"))
            .collect();
        if ids.len() != 1
            || ids[0].split_whitespace().collect::<Vec<_>>() != ["Pid:", &pid.to_string()]
            || rows.len() != 1
        {
            return Err(invalid("Self status PID/UID identity"));
        }
        let parts: Vec<_> = rows[0].split_whitespace().skip(1).collect();
        if parts.len() != 4
            || parts
                .iter()
                .any(|s| s.is_empty() || !s.bytes().all(|b| b.is_ascii_digit()))
        {
            return Err(invalid("Self UID4 shape"));
        }
        let mut uids = [0; 4];
        for (index, text) in parts.iter().enumerate() {
            uids[index] = text.parse().map_err(|_| invalid("Self UID4 range"))?;
        }
        if pid <= 1 || !raw.starts_with(&format!("{pid} (")) {
            return Err(invalid("Self stat PID"));
        }
        let fields = raw
            .rsplit_once(") ")
            .ok_or_else(|| invalid("Self stat comm"))?
            .1;
        let start = fields
            .split_whitespace()
            .nth(19)
            .filter(|s| s.bytes().all(|b| b.is_ascii_digit()))
            .and_then(|s| s.parse::<u64>().ok())
            .filter(|s| *s > 0)
            .ok_or_else(|| invalid("Self process start"))?;
        Ok((uids, start))
    }
    fn root_self(profile: Profile, helper: &Measurement) -> io::Result<RootProcess> {
        let pid = std::process::id();
        let before = process(&proc_text("stat")?, &proc_text("status")?, pid)?;
        if before.0 != [0; 4]
            || std::fs::read_link("/proc/self/root")? != std::path::Path::new("/")
            || std::fs::read_link("/proc/self/exe")? != std::path::Path::new(&profile.helper_path())
        {
            return Err(invalid("Fixed installed Root helper process required"));
        }
        let mut file = File::open("/proc/self/exe")?;
        let measured = measure(&mut file, 0o700, ELF_LIMIT, false)?.0;
        if &measured != helper
            || process(&proc_text("stat")?, &proc_text("status")?, pid)? != before
            || std::fs::read_link("/proc/self/exe")? != std::path::Path::new(&profile.helper_path())
            || std::fs::read_link("/proc/self/root")? != std::path::Path::new("/")
        {
            return Err(invalid("Root helper proc executable/UID/start changed"));
        }
        Ok(RootProcess {
            pid,
            uid_fields: before.0,
            start_ticks: before.1,
            exe_path: profile.helper_path(),
            exe_sha256: measured.sha256,
            root_path: "/".into(),
        })
    }
    pub(super) fn load(profile: Profile) -> io::Result<Installation> {
        let status = process(
            &proc_text("stat")?,
            &proc_text("status")?,
            std::process::id(),
        )?;
        if status.0 != [0; 4] {
            return Err(io::Error::new(
                io::ErrorKind::PermissionDenied,
                "Only actual Root UID4 can load installed authorization",
            ));
        }
        let trusted = Walk::open(profile.installation_dir(), true)?;
        if trusted.last()?.metadata()?.mode() & 0o7777 != 0o700 {
            return Err(invalid(
                "Private installed product directory must be Root0700",
            ));
        }
        let (config, raw) = fixed_file(&trusted, "broker-installation.json", 0o600, 4096, true)?;
        let pins = Pins::parse(&raw)?;
        if pins.profile != profile {
            return Err(invalid("Installed profile differs from fixed helper role"));
        }
        let (helper, _) = fixed_file(&trusted, profile.helper_basename(), 0o700, ELF_LIMIT, false)?;
        // Writable app ancestry is not policy or a Root execution path. It is
        // opened only for additional pinned UI measurement; actual /proc peer
        // executable evidence must separately match the protected UI digest.
        let ui_walk = Walk::open(profile.bundle(), false)?;
        let (ui, _) = fixed_file(&ui_walk, "home-preview", 0o755, ELF_LIMIT, false)?;
        pinned(&ui, &pins.ui_sha256)?;
        pinned(&helper, &pins.broker_sha256)?;
        let root_process = root_self(profile, &helper)?;
        verify_named(&trusted, "broker-installation.json", &config)?;
        verify_named(&trusted, profile.helper_basename(), &helper)?;
        verify_named(&ui_walk, "home-preview", &ui)?;
        Ok(Installation {
            pins,
            snapshot: Snapshot {
                config,
                ui,
                helper,
                authority_anchors: trusted.stamps,
                ui_lookup_anchors: ui_walk.stamps,
                root_process,
            },
        })
    }
    #[cfg(test)]
    pub(super) fn nofollow_fixture(parent: &File, name: &str, directory: bool) -> io::Result<()> {
        child(parent, name, directory).map(|_| ())
    }
    #[cfg(test)]
    pub(super) fn process_fixture(
        stat: &str,
        status: &str,
        pid: u32,
    ) -> io::Result<([u32; 4], u64)> {
        process(stat, status, pid)
    }
    #[cfg(test)]
    pub(super) fn identity_fixture(file: &File) -> io::Result<FileIdentity> {
        Ok(FileIdentity::of(&file.metadata()?))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    fn pins(profile: Profile) -> Pins {
        Pins {
            schema: 1,
            profile,
            rendezvous_nonce: "a".repeat(32),
            ui_sha256: "b".repeat(64),
            broker_sha256: "c".repeat(64),
            context_reader_sha256: "d".repeat(64),
        }
    }
    fn peer(profile: Profile) -> PeerEvidence {
        PeerEvidence {
            pid: 123,
            uid_fields: [6583; 4],
            start_ticks: 456,
            exe_sha256: "b".repeat(64),
            exe_path: profile.peer_exe_path(),
            root_path: profile.jail_root(),
        }
    }
    fn cred() -> Credentials {
        Credentials {
            pid: 123,
            uid: 6583,
            gid: 5000,
        }
    }
    fn meta() -> FileIdentity {
        FileIdentity {
            device: 1,
            inode: 2,
            bytes: 100,
            mode: 0o100755,
            uid: 0,
            gid: 0,
            links: 1,
            mtime: 1,
            mtime_ns: 0,
            ctime: 1,
            ctime_ns: 0,
        }
    }
    #[test]
    fn policy_is_closed_pidless_and_format_strict() {
        let good = serde_json::to_value(pins(Profile::Proof)).unwrap();
        assert_eq!(
            Pins::parse(&serde_json::to_vec(&good).unwrap()).unwrap(),
            pins(Profile::Proof)
        );
        for (key, value) in [
            ("pid", json!(123)),
            ("path", json!("/tmp/other")),
            ("schema", json!(true)),
            ("profile", json!("Proof")),
            ("rendezvous_nonce", json!("A".repeat(32))),
            ("ui_sha256", json!("b".repeat(63))),
            ("broker_sha256", json!(null)),
            ("context_reader_sha256", json!("G".repeat(64))),
        ] {
            let mut bad = good.clone();
            bad[key] = value;
            assert!(Pins::parse(&serde_json::to_vec(&bad).unwrap()).is_err());
        }
        for raw in [
            br#"{"schema":1,"schema":1}"#.as_slice(),
            b"{}",
            b"[]",
            &[b' '; 4097],
        ] {
            assert!(Pins::parse(raw).is_err());
        }
    }
    #[test]
    fn proof_and_preview_are_separate_installed_roles_not_uid_relabels() {
        assert_eq!(
            Profile::Proof.installation_dir(),
            "/var/lib/luma-home-proof"
        );
        assert_eq!(
            Profile::Proof.helper_path(),
            "/var/lib/luma-home-proof/artwork-broker-startup-proof"
        );
        assert_eq!(
            Profile::Preview.helper_path(),
            "/var/lib/luma-home/artwork-broker"
        );
        for profile in [Profile::Proof, Profile::Preview] {
            assert!(admit_pins(&pins(profile), &cred(), &cred(), &peer(profile)).is_ok());
        }
        assert!(admit_pins(
            &pins(Profile::Proof),
            &cred(),
            &cred(),
            &peer(Profile::Preview)
        )
        .is_err());
        assert!(admit_pins(
            &pins(Profile::Preview),
            &cred(),
            &cred(),
            &peer(Profile::Proof)
        )
        .is_err());
        let c = Credentials {
            uid: 5724,
            ..cred()
        };
        let mut p = peer(Profile::Preview);
        p.uid_fields = [5724; 4];
        assert!(admit_pins(&pins(Profile::Preview), &c, &c, &p).is_ok());
        p.exe_path = Profile::Proof.peer_exe_path();
        p.root_path = Profile::Proof.jail_root();
        assert!(admit_pins(&pins(Profile::Proof), &c, &c, &p).is_err());
    }
    #[test]
    fn actual_uid4_pid_start_executable_and_both_kernel_credentials_are_required() {
        let policy = pins(Profile::Proof);
        let base = peer(Profile::Proof);
        for p in [
            PeerEvidence {
                pid: 124,
                ..base.clone()
            },
            PeerEvidence {
                uid_fields: [0; 4],
                ..base.clone()
            },
            PeerEvidence {
                uid_fields: [6583, 6583, 0, 6583],
                ..base.clone()
            },
            PeerEvidence {
                start_ticks: 0,
                ..base.clone()
            },
            PeerEvidence {
                exe_sha256: "c".repeat(64),
                ..base.clone()
            },
            PeerEvidence {
                root_path: "/".into(),
                ..base.clone()
            },
            PeerEvidence {
                exe_path: "/tmp/home-preview".into(),
                ..base.clone()
            },
        ] {
            assert!(admit_pins(&policy, &cred(), &cred(), &p).is_err());
        }
        for c in [
            Credentials { pid: 124, ..cred() },
            Credentials { uid: 0, ..cred() },
            Credentials { gid: 0, ..cred() },
        ] {
            assert!(admit_pins(&policy, &cred(), &c, &base).is_err());
        }
    }
    #[test]
    fn metadata_admission_refuses_links_special_modes_owners_and_oversize() {
        assert!(meta().admit(0o755, 8 * 1024 * 1024).is_ok());
        for value in [
            FileIdentity {
                uid: 1000,
                ..meta()
            },
            FileIdentity {
                gid: 1000,
                ..meta()
            },
            FileIdentity { links: 2, ..meta() },
            FileIdentity {
                mode: 0o120755,
                ..meta()
            },
            FileIdentity {
                mode: 0o104755,
                ..meta()
            },
            FileIdentity {
                mode: 0o100777,
                ..meta()
            },
            FileIdentity { bytes: 0, ..meta() },
            FileIdentity {
                bytes: 8 * 1024 * 1024 + 1,
                ..meta()
            },
            FileIdentity {
                mtime_ns: 1_000_000_000,
                ..meta()
            },
        ] {
            assert!(value.admit(0o755, 8 * 1024 * 1024).is_err());
        }
        assert!(FileIdentity {
            mode: 0o100600,
            bytes: 4096,
            ..meta()
        }
        .admit(0o600, 4096)
        .is_ok());
        assert!(FileIdentity {
            mode: 0o100600,
            bytes: 4097,
            ..meta()
        }
        .admit(0o600, 4096)
        .is_err());
        assert!(meta().admit(0o700, 8 * 1024 * 1024).is_err());
    }
    #[test]
    fn writable_app_lookup_never_becomes_root_execution_authority() {
        let app = DirectoryIdentity {
            device: 1,
            inode: 2,
            mode: 0o040777,
            uid: 0,
            gid: 0,
        };
        assert!(app.admit(false).is_ok());
        assert!(app.admit(true).is_err());
        for bad in [
            DirectoryIdentity {
                uid: 1000,
                ..app.clone()
            },
            DirectoryIdentity {
                mode: 0o120777,
                ..app.clone()
            },
        ] {
            assert!(bad.admit(false).is_err());
        }
    }
    #[test]
    fn complete_file_identity_detects_same_size_replacement_and_timestamp_races() {
        let base = meta();
        for next in [
            FileIdentity { inode: 3, ..meta() },
            FileIdentity {
                mtime_ns: 1,
                ..meta()
            },
            FileIdentity {
                ctime_ns: 1,
                ..meta()
            },
            FileIdentity { links: 2, ..meta() },
        ] {
            assert!(unchanged(&base, &next).is_err());
        }
        let a = Measurement {
            sha256: "a".repeat(64),
            identity: meta(),
        };
        assert!(pinned(&a, &"b".repeat(64)).is_err());
        assert!(pinned(&a, &"a".repeat(64)).is_ok());
    }
    #[test]
    fn actual_host_openat_rejects_leaf_and_parent_symlinks_without_root_admission() {
        use std::os::unix::fs::symlink;
        let dir =
            std::env::temp_dir().join(format!("luma-installation-nofollow-{}", std::process::id()));
        std::fs::create_dir(&dir).unwrap();
        {
            std::fs::write(dir.join("file"), b"fixture").unwrap();
            std::fs::create_dir(dir.join("real-dir")).unwrap();
            symlink("file", dir.join("link")).unwrap();
            symlink("real-dir", dir.join("dir-link")).unwrap();
            let fd = std::fs::File::open(&dir).unwrap();
            assert!(native::nofollow_fixture(&fd, "link", false).is_err());
            assert!(native::nofollow_fixture(&fd, "dir-link", true).is_err());
            assert!(native::nofollow_fixture(&fd, "file", false).is_ok());
            let before =
                native::identity_fixture(&std::fs::File::open(dir.join("file")).unwrap()).unwrap();
            std::fs::rename(dir.join("file"), dir.join("old")).unwrap();
            std::fs::write(dir.join("file"), b"fixture").unwrap();
            let after =
                native::identity_fixture(&std::fs::File::open(dir.join("file")).unwrap()).unwrap();
            assert!(unchanged(&before, &after).is_err());
        }
        std::fs::remove_dir_all(&dir).unwrap();
    }
    #[test]
    fn proc_parser_binds_root_uid4_and_positive_start_without_guessing() {
        let stat = format!(
            "123 (name with ) spaces) {}",
            [vec!["S"], vec!["0"; 18], vec!["456"]].concat().join(" ")
        );
        let status = "Pid: 123\nUid: 0 0 0 0\n";
        assert_eq!(
            native::process_fixture(&stat, status, 123).unwrap(),
            ([0; 4], 456)
        );
        for raw in [
            "Pid: 124\nUid: 0 0 0 0\n",
            "Pid: 123\nUid: 0 0 0 0\nUid: 0 0 0 0\n",
            "Pid: 123\nUid: +0 0 0 0\n",
        ] {
            assert!(native::process_fixture(&stat, raw, 123).is_err());
        }
        assert!(native::process_fixture(&stat.replace("456", "0"), status, 123).is_err());
    }
    #[cfg(not(target_arch = "arm"))]
    #[test]
    fn host_load_is_unsupported_not_a_simulated_root_installation_success() {
        let _native_signature: fn(Profile) -> io::Result<Installation> = native::load;
        for profile in [Profile::Proof, Profile::Preview] {
            assert_eq!(
                Installation::load(profile).err().unwrap().kind(),
                io::ErrorKind::Unsupported
            );
        }
    }
}
