//! Ordinary, untrusted icon-cache data only. Never Root/session/Ticket authority.
//! Call publish only AFTER the future producer has matched a positive transfer,
//! joined it and durably resolved its current Ticket. This module cannot prove
//! that precondition and is deliberately not connected to any provider or UI.
//! Worker-side I/O/codec only: no worker, GL, remote call or original-source read.
use crate::{
    app_artwork::{
        EncodedFile, FileIdentity as SourceIdentity, IconKind, RootClass, Source, SourceKey,
    },
    packaged_icon_reader::{
        DecodedIcon, FileIdentity, MAX_AXIS, MAX_ENCODED_BYTES, MAX_RGBA_BYTES,
    },
};
use serde::{Deserialize, Serialize};
use std::{
    collections::BTreeMap,
    ffi::{CStr, CString},
    fs::{File, Metadata},
    io::{self, Read, Seek, Write},
    mem::MaybeUninit,
    os::{
        fd::{AsRawFd, FromRawFd, IntoRawFd},
        unix::fs::{MetadataExt, OpenOptionsExt},
    },
    sync::{
        atomic::{AtomicBool, Ordering},
        Arc,
    },
};
const MAGIC: &[u8; 8] = b"LUMAIC01";
const HEADER_CAP: usize = 8192;
const TEMP: &CStr = c".publish-next";
const PREVIEW_LEAF: &CStr = c".com.ash.nativehome.preview-artwork-cache-v1";
const PROOF_LEAF: &CStr = c".com.ash.nativehome.proof-artwork-cache-v1";
const MAX_FILE: u64 = (12 + HEADER_CAP + MAX_ENCODED_BYTES) as u64;
const MAX_ENTRIES: usize = 512;
const MAX_TOTAL: u64 = 64 * 1024 * 1024;
#[cfg(target_vendor = "apple")]
unsafe fn errno_slot() -> *mut libc::c_int {
    unsafe { libc::__error() }
}
#[cfg(not(target_vendor = "apple"))]
unsafe fn errno_slot() -> *mut libc::c_int {
    unsafe { libc::__errno_location() }
}
fn invalid(message: &str) -> io::Error {
    io::Error::new(io::ErrorKind::InvalidData, message)
}
fn digest(raw: &[u8]) -> [u8; 32] {
    plx_base::sha256::sha256(raw)
}
fn hex(raw: &[u8]) -> String {
    raw.iter().map(|b| format!("{b:02x}")).collect()
}
#[derive(Clone, Copy, Debug)]
pub struct Config {
    pub entries: usize,
    pub encoded_bytes: u64,
}
impl Default for Config {
    fn default() -> Self {
        Self {
            entries: 256,
            encoded_bytes: 16 * 1024 * 1024,
        }
    }
}
impl Config {
    fn valid(self) -> io::Result<Self> {
        if self.entries == 0
            || self.entries > MAX_ENTRIES
            || self.encoded_bytes == 0
            || self.encoded_bytes > MAX_TOTAL
        {
            Err(invalid("Cache budget out of bounds"))
        } else {
            Ok(self)
        }
    }
}
/// Claimed producer association, not evidence of who actually wrote ordinary data.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ProducerVersion {
    source_revision: String,
    ui_sha256: [u8; 32],
    broker_sha256: [u8; 32],
}
impl ProducerVersion {
    pub fn new(
        source_revision: &str,
        ui_sha256: [u8; 32],
        broker_sha256: [u8; 32],
    ) -> io::Result<Self> {
        let value = Self {
            source_revision: source_revision.into(),
            ui_sha256,
            broker_sha256,
        };
        value.validate()?;
        Ok(value)
    }
    fn validate(&self) -> io::Result<()> {
        if self.source_revision.len() != 40
            || !self
                .source_revision
                .bytes()
                .all(|b| b.is_ascii_digit() || (b'a'..=b'f').contains(&b))
        {
            Err(invalid("Producer revision"))
        } else {
            Ok(())
        }
    }
}
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct SourceRecord {
    app_id: String,
    version: String,
    class: String,
    folder: String,
    relative: Vec<String>,
    kind: String,
}
impl SourceRecord {
    fn of(key: &SourceKey) -> Self {
        Self {
            app_id: key.app_id.clone(),
            version: key.version.clone(),
            class: match key.class {
                RootClass::LgSystem => "lg_system",
                RootClass::EncryptedApps => "encrypted_apps",
                RootClass::DeveloperApps => "developer_apps",
                RootClass::SystemApps => "system_apps",
                RootClass::OptionalContent => "optional_content",
            }
            .into(),
            folder: key.folder.clone(),
            relative: key.relative.clone(),
            kind: match key.kind {
                IconKind::Icon => "icon",
                IconKind::LargeIcon => "large_icon",
            }
            .into(),
        }
    }
    fn key(&self) -> io::Result<SourceKey> {
        let class = match self.class.as_str() {
            "lg_system" => RootClass::LgSystem,
            "encrypted_apps" => RootClass::EncryptedApps,
            "developer_apps" => RootClass::DeveloperApps,
            "system_apps" => RootClass::SystemApps,
            "optional_content" => RootClass::OptionalContent,
            _ => return Err(invalid("Cache root class")),
        };
        let kind = match self.kind.as_str() {
            "icon" => IconKind::Icon,
            "large_icon" => IconKind::LargeIcon,
            _ => return Err(invalid("Cache icon kind")),
        };
        if !crate::model::valid_app_id(&self.app_id)
            || matches!(self.app_id.as_str(), "." | "..")
            || self.version.is_empty()
            || self.version.len() > 128
            || self.version.chars().any(char::is_control)
            || self.folder != format!("{}/{}", class.prefix(), self.app_id)
            || self.relative.is_empty()
            || self.relative.len() > 8
            || self.relative.join("/").len() > 512
            || self.relative.iter().any(|p| {
                p.is_empty()
                    || p.len() > 128
                    || matches!(p.as_str(), "." | "..")
                    || p.chars()
                        .any(|c| c.is_control() || matches!(c, '/' | '\\' | ':'))
            })
        {
            return Err(invalid("Invalid complete source association"));
        }
        Ok(SourceKey {
            app_id: self.app_id.clone(),
            version: self.version.clone(),
            class,
            folder: self.folder.clone(),
            relative: self.relative.clone(),
            kind,
        })
    }
}
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct ObservedSource {
    device: u64,
    inode: u64,
    bytes: u64,
    mode: u32,
    uid: u32,
    gid: u32,
    modified_seconds: i64,
    modified_nanos: i64,
    changed_seconds: i64,
    changed_nanos: i64,
}
impl ObservedSource {
    fn of(x: &SourceIdentity) -> Self {
        Self {
            device: x.device,
            inode: x.inode,
            bytes: x.bytes,
            mode: x.mode,
            uid: x.uid,
            gid: x.gid,
            modified_seconds: x.modified_seconds,
            modified_nanos: x.modified_nanos,
            changed_seconds: x.changed_seconds,
            changed_nanos: x.changed_nanos,
        }
    }
    fn get(&self) -> SourceIdentity {
        SourceIdentity {
            device: self.device,
            inode: self.inode,
            bytes: self.bytes,
            mode: self.mode,
            uid: self.uid,
            gid: self.gid,
            modified_seconds: self.modified_seconds,
            modified_nanos: self.modified_nanos,
            changed_seconds: self.changed_seconds,
            changed_nanos: self.changed_nanos,
        }
    }
}
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct Record {
    schema: u32,
    producer_kind: String,
    producer: ProducerVersion,
    source: SourceRecord,
    source_identity: ObservedSource,
    encoded_bytes: u32,
    encoded_sha256: [u8; 32],
}
impl Record {
    fn validate(&self) -> io::Result<()> {
        self.producer.validate()?;
        self.source.key()?;
        if self.schema != 1
            || self.producer_kind != "joined-single-positive-v1"
            || self.encoded_bytes == 0
            || self.encoded_bytes as usize > MAX_ENCODED_BYTES
            || self.source_identity.bytes != u64::from(self.encoded_bytes)
            || self.source_identity.inode == 0
            || self.source_identity.mode & 0o170000 != 0o100000
            || !(0..1_000_000_000).contains(&self.source_identity.modified_nanos)
            || !(0..1_000_000_000).contains(&self.source_identity.changed_nanos)
        {
            return Err(invalid("Invalid encoded/source association"));
        }
        Ok(())
    }
    fn header(&self) -> io::Result<Vec<u8>> {
        self.validate()?;
        let raw = serde_json::to_vec(self)?;
        if raw.len() > HEADER_CAP {
            return Err(invalid("Cache metadata too large"));
        }
        Ok(raw)
    }
    fn name(&self) -> io::Result<CString> {
        CString::new(format!("{}.icon", hex(&digest(&self.header()?))))
            .map_err(|_| invalid("Cache leaf"))
    }
}
// Shared pure framing/schema logic for both the anchored FD reader and the
// supplied-byte maintenance inspector. Neither helper opens or mutates data.
fn record_header_size(prefix: &[u8; 12], file_bytes: u64) -> io::Result<usize> {
    if &prefix[..8] != MAGIC {
        return Err(invalid("Cache magic"));
    }
    let size = u32::from_be_bytes(
        prefix[8..]
            .try_into()
            .map_err(|_| invalid("Header prefix"))?,
    ) as usize;
    if size == 0 || size > HEADER_CAP || file_bytes < 12 + size as u64 {
        return Err(invalid("Cache header bound"));
    }
    Ok(size)
}
fn decode_record_header(raw: &[u8], name: &CStr, file_bytes: u64) -> io::Result<(Record, u64)> {
    if raw.is_empty() || raw.len() > HEADER_CAP {
        return Err(invalid("Cache header bound"));
    }
    let record: Record = serde_json::from_slice(raw)
        .map_err(|_| invalid("Malformed/duplicate/unknown cache metadata"))?;
    record.validate()?;
    let offset = 12 + raw.len() as u64;
    if offset + u64::from(record.encoded_bytes) != file_bytes || record.name()?.as_c_str() != name {
        return Err(invalid("Cache length/content-address association"));
    }
    Ok((record, offset))
}
fn stamp(m: &Metadata) -> FileIdentity {
    FileIdentity {
        device: m.dev(),
        inode: m.ino(),
        uid: m.uid(),
        gid: m.gid(),
        mode: m.mode(),
        links: m.nlink(),
        bytes: m.len(),
        modified_seconds: m.mtime(),
        modified_nanos: m.mtime_nsec(),
        changed_seconds: m.ctime(),
        changed_nanos: m.ctime_nsec(),
    }
}
// ABI integer widths differ between macOS and the actual ARM32 TV target.
#[allow(clippy::unnecessary_cast)]
fn named(parent: &File, name: &CStr) -> io::Result<FileIdentity> {
    let mut raw = MaybeUninit::<libc::stat>::uninit();
    if unsafe {
        libc::fstatat(
            parent.as_raw_fd(),
            name.as_ptr(),
            raw.as_mut_ptr(),
            libc::AT_SYMLINK_NOFOLLOW,
        )
    } < 0
    {
        return Err(io::Error::last_os_error());
    }
    let s = unsafe { raw.assume_init() };
    if s.st_size < 0 {
        return Err(invalid("Negative cache size"));
    }
    Ok(FileIdentity {
        device: s.st_dev as u64,
        inode: s.st_ino as u64,
        uid: s.st_uid,
        gid: s.st_gid,
        mode: s.st_mode as u32,
        links: s.st_nlink as u64,
        bytes: s.st_size as u64,
        modified_seconds: s.st_mtime as i64,
        modified_nanos: s.st_mtime_nsec as i64,
        changed_seconds: s.st_ctime as i64,
        changed_nanos: s.st_ctime_nsec as i64,
    })
}
#[derive(Clone, Debug, PartialEq, Eq)]
struct DirStamp {
    device: u64,
    inode: u64,
    uid: u32,
    gid: u32,
    mode: u32,
}
impl DirStamp {
    fn of(m: FileIdentity) -> io::Result<Self> {
        if m.mode & 0o170000 != 0o040000 {
            return Err(invalid("Cache not a directory"));
        }
        Ok(Self {
            device: m.device,
            inode: m.inode,
            uid: m.uid,
            gid: m.gid,
            mode: m.mode,
        })
    }
}
fn open_at(parent: &File, name: &CStr, directory: bool) -> io::Result<File> {
    let flags = libc::O_RDONLY
        | libc::O_NOFOLLOW
        | libc::O_CLOEXEC
        | if directory {
            libc::O_DIRECTORY
        } else {
            libc::O_NONBLOCK
        };
    let fd = unsafe { libc::openat(parent.as_raw_fd(), name.as_ptr(), flags) };
    if fd < 0 {
        Err(io::Error::last_os_error())
    } else {
        Ok(unsafe { File::from_raw_fd(fd) })
    }
}
struct Root {
    parent: File,
    directory: File,
    name: CString,
    anchor: DirStamp,
}
impl Root {
    fn check(&self) -> io::Result<()> {
        if unsafe { libc::geteuid() } == 0
            || self.anchor.uid != unsafe { libc::geteuid() }
            || self.anchor.gid != unsafe { libc::getegid() }
        {
            return Err(invalid("Cache owner/process identity changed"));
        }
        if DirStamp::of(stamp(&self.directory.metadata()?))? != self.anchor
            || DirStamp::of(named(&self.parent, &self.name)?)? != self.anchor
        {
            Err(invalid("Cache root replaced or permissions changed"))
        } else {
            Ok(())
        }
    }
    fn file(&self, name: &CStr) -> io::Result<(File, FileIdentity)> {
        self.check()?;
        let before = named(&self.directory, name)?;
        if before.mode & 0o170000 != 0o100000
            || before.mode & 0o7777 != 0o600
            || before.uid != unsafe { libc::geteuid() }
            || before.gid != unsafe { libc::getegid() }
            || before.bytes > MAX_FILE
        {
            return Err(invalid("Cache file is not own600 bounded regular data"));
        }
        let file = open_at(&self.directory, name, false)?;
        if stamp(&file.metadata()?) != before || named(&self.directory, name)? != before {
            return Err(invalid("Cache fd/name changed"));
        }
        Ok((file, before))
    }
    fn unchanged(&self, file: &File, name: &CStr, before: &FileIdentity) -> io::Result<()> {
        self.check()?;
        if stamp(&file.metadata()?) != *before || named(&self.directory, name)? != *before {
            return Err(invalid("Cache writer or named identity changed"));
        }
        Ok(())
    }
    fn record(&self, name: &CStr) -> io::Result<(Record, FileIdentity, u64)> {
        let (mut file, before) = self.file(name)?;
        if before.bytes < 12 {
            return Err(invalid("Cache header incomplete"));
        }
        let mut prefix = [0; 12];
        file.read_exact(&mut prefix)?;
        let size = record_header_size(&prefix, before.bytes)?;
        let mut raw = vec![0; size];
        file.read_exact(&mut raw)?;
        let (record, offset) = decode_record_header(&raw, name, before.bytes)?;
        self.unchanged(&file, name, &before)?;
        Ok((record, before, offset))
    }
}
#[derive(Clone)]
pub struct Generation {
    seal: Arc<Seal>,
}
struct Seal {
    current: AtomicBool,
    content: [u8; 32],
}
impl Generation {
    pub fn withdraw(&self) {
        self.seal.current.store(false, Ordering::Release);
    }
    pub fn content_id(&self) -> [u8; 32] {
        self.seal.content
    }
    pub fn current(&self) -> bool {
        self.seal.current.load(Ordering::Acquire)
    }
}
/// Ordinary immutable-record association only; never source/Root authority.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct Selection {
    record_sha256: [u8; 32],
    encoded_sha256: [u8; 32],
    encoded_bytes: u32,
}
#[derive(Clone)]
struct Entry {
    selection: Selection,
    record: Record,
    identity: FileIdentity,
    offset: u64,
    name: CString,
}
/// One adopted immutable in-memory index. A new adoption always gets a fresh
/// in-process seal, even when content_id repeats. It never hot-repoints a reader.
pub struct Snapshot {
    root: Arc<Root>,
    entries: Vec<Entry>,
    index: BTreeMap<SourceKey, Vec<usize>>,
    generation: Generation,
}
pub struct Cache {
    root: Arc<Root>,
    config: Config,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Stats {
    pub entries: usize,
    pub encoded_bytes: u64,
}
pub enum Lookup {
    Missing,
    Ambiguous,
    Ready(Box<Demand>),
}
pub struct Demand {
    root: Arc<Root>,
    entry: Entry,
    generation: Generation,
}
pub struct CachedIcon {
    selection: Selection,
    image: DecodedIcon,
    record: Record,
    generation: Generation,
}
impl Demand {
    pub(crate) fn selection(&self) -> Selection {
        self.entry.selection
    }
}
impl CachedIcon {
    pub(crate) fn selection(&self) -> Selection {
        self.selection
    }
    /// Caller must supply the CURRENT adopted generation and freshly admitted
    /// Source, after its ordinary worker result arrives. No GL authority implied.
    pub fn admit(
        self,
        current: &Generation,
        source: &Source,
        producer: &ProducerVersion,
    ) -> Option<DecodedIcon> {
        (current.current()
            && self.generation.current()
            && Arc::ptr_eq(&current.seal, &self.generation.seal)
            && self.record.source.key().ok().as_ref() == Some(source.key())
            && &self.record.producer == producer)
            .then_some(self.image)
    }
    pub fn observed_source_identity(&self) -> SourceIdentity {
        self.record.source_identity.get()
    }
}
impl Cache {
    /// No caller-selected role, path, URI or command. Ancestors are walked nofollow.
    pub fn open_current(config: Config) -> io::Result<Self> {
        if unsafe { libc::geteuid() } < 1000 {
            return Err(invalid("Ordinary native owner required"));
        }
        let leaf = match plx_base::paths::app_id() {
            crate::identity::PREVIEW => PREVIEW_LEAF,
            crate::identity::PROOF => PROOF_LEAF,
            _ => return Err(invalid("Unexpected own application identity")),
        };
        let root = std::fs::OpenOptions::new()
            .read(true)
            .custom_flags(libc::O_DIRECTORY | libc::O_NOFOLLOW | libc::O_CLOEXEC)
            .open("/")?;
        let media = open_at(&root, c"media", true)?;
        let developer = open_at(&media, c"developer", true)?;
        Self::open_in(developer, leaf, config)
    }
    #[cfg(test)]
    pub(crate) fn open_test_directory(parent: File, config: Config) -> io::Result<Self> {
        Self::open_in(parent, PREVIEW_LEAF, config)
    }
    #[cfg(test)]
    pub(crate) fn shares_snapshot_root_for_test(&self, snapshot: &Snapshot) -> bool {
        Arc::ptr_eq(&self.root, &snapshot.root)
    }
    fn open_in(parent: File, name: &CStr, config: Config) -> io::Result<Self> {
        let config = config.valid()?;
        if unsafe { libc::geteuid() } == 0 {
            return Err(invalid("Root cache writes refused"));
        }
        let made = unsafe { libc::mkdirat(parent.as_raw_fd(), name.as_ptr(), 0o700) };
        if made < 0 && io::Error::last_os_error().raw_os_error() != Some(libc::EEXIST) {
            return Err(io::Error::last_os_error());
        }
        let directory = open_at(&parent, name, true)?;
        let anchor = DirStamp::of(stamp(&directory.metadata()?))?;
        if anchor.uid != unsafe { libc::geteuid() }
            || anchor.gid != unsafe { libc::getegid() }
            || anchor.mode & 0o7777 != 0o700
        {
            return Err(invalid("Cache directory not own700; no repair"));
        }
        let root = Arc::new(Root {
            parent,
            directory,
            name: name.to_owned(),
            anchor,
        });
        root.check()?;
        if made == 0 {
            root.parent.sync_all()?;
        }
        Ok(Self { root, config })
    }
    fn inventory(&self) -> io::Result<Vec<CString>> {
        self.root.check()?;
        let fd = open_at(&self.root.directory, c".", true)?.into_raw_fd();
        let dir = unsafe { libc::fdopendir(fd) };
        if dir.is_null() {
            unsafe { libc::close(fd) };
            return Err(io::Error::last_os_error());
        }
        unsafe { libc::rewinddir(dir) };
        let mut names = Vec::new();
        let result = (|| {
            loop {
                unsafe { *errno_slot() = 0 };
                let entry = unsafe { libc::readdir(dir) };
                if entry.is_null() {
                    let error = unsafe { *errno_slot() };
                    if error != 0 {
                        return Err(io::Error::from_raw_os_error(error));
                    }
                    break;
                }
                let name = unsafe { CStr::from_ptr((*entry).d_name.as_ptr()) };
                if name == c"." || name == c".." {
                    continue;
                }
                if names.len() > self.config.entries {
                    return Err(invalid("Too many cache leaves"));
                }
                names.push(name.to_owned());
            }
            Ok(())
        })();
        unsafe { libc::closedir(dir) };
        result?;
        self.root.check()?;
        names.sort();
        Ok(names)
    }
    fn scan(&self) -> io::Result<(Vec<Entry>, Stats)> {
        let names = self.inventory()?;
        let original = names.clone();
        let mut entries = Vec::new();
        let mut total = 0_u64;
        for name in names {
            if name == TEMP {
                let (_, meta) = self.root.file(&name)?;
                if meta.bytes > MAX_FILE {
                    return Err(invalid("Uncertain ordinary temp bound"));
                }
                continue;
            }
            let raw = name.to_bytes();
            if raw.len() != 69
                || &raw[64..] != b".icon"
                || !raw[..64]
                    .iter()
                    .all(|b| b.is_ascii_digit() || (b'a'..=b'f').contains(b))
            {
                return Err(invalid("Unknown cache leaf; no traversal"));
            }
            let (record, identity, offset) = self.root.record(&name)?;
            total = total
                .checked_add(u64::from(record.encoded_bytes))
                .ok_or_else(|| invalid("Cache byte overflow"))?;
            entries.push(Entry {
                selection: Selection {
                    record_sha256: digest(&record.header()?),
                    encoded_sha256: record.encoded_sha256,
                    encoded_bytes: record.encoded_bytes,
                },
                record,
                identity,
                offset,
                name,
            });
            if entries.len() > self.config.entries || total > self.config.encoded_bytes {
                return Err(invalid("Cache published budget exceeded"));
            }
        }
        if self.inventory()? != original {
            return Err(invalid("Cache inventory changed while adopting snapshot"));
        }
        let count = entries.len();
        Ok((
            entries,
            Stats {
                entries: count,
                encoded_bytes: total,
            },
        ))
    }
    pub fn snapshot(&self) -> io::Result<Snapshot> {
        let (entries, _) = self.scan()?;
        let mut index: BTreeMap<SourceKey, Vec<usize>> = BTreeMap::new();
        for (i, entry) in entries.iter().enumerate() {
            index.entry(entry.record.source.key()?).or_default().push(i);
        }
        let names = entries
            .iter()
            .flat_map(|e| e.name.to_bytes().iter().copied())
            .collect::<Vec<_>>();
        let generation = Generation {
            seal: Arc::new(Seal {
                current: AtomicBool::new(true),
                content: digest(&names),
            }),
        };
        Ok(Snapshot {
            root: self.root.clone(),
            entries,
            index,
            generation,
        })
    }
    pub fn stats(&self) -> io::Result<Stats> {
        self.scan().map(|(_, stats)| stats)
    }
    fn writer_slot(&self) -> io::Result<Temporary<'_>> {
        self.root.check()?;
        let fd = unsafe {
            libc::openat(
                self.root.directory.as_raw_fd(),
                TEMP.as_ptr(),
                libc::O_WRONLY | libc::O_CREAT | libc::O_EXCL | libc::O_NOFOLLOW | libc::O_CLOEXEC,
                0o600,
            )
        };
        if fd < 0 {
            return Err(io::Error::last_os_error());
        }
        Ok(Temporary {
            root: &self.root,
            file: unsafe { File::from_raw_fd(fd) },
            owned_name: true,
        })
    }
    /// Ordinary data publication, NOT a positive-settlement verifier. Integration
    /// must gate this call after actual matching positive/joined/current closure.
    pub fn publish(
        &mut self,
        source: &Source,
        encoded: &EncodedFile,
        producer: ProducerVersion,
    ) -> io::Result<()> {
        if encoded.bytes.is_empty()
            || encoded.bytes.len() > MAX_ENCODED_BYTES
            || encoded.identity.bytes != encoded.bytes.len() as u64
            || digest(&encoded.bytes) != encoded.digest
        {
            return Err(invalid("Unmatched encoded payload"));
        }
        let record = Record {
            schema: 1,
            producer_kind: "joined-single-positive-v1".into(),
            producer,
            source: SourceRecord::of(source.key()),
            source_identity: ObservedSource::of(&encoded.identity),
            encoded_bytes: encoded.bytes.len() as u32,
            encoded_sha256: encoded.digest,
        };
        let header = record.header()?;
        let name = record.name()?;
        let mut temporary = self.writer_slot()?;
        // This create-new slot serializes cooperating publishers. Capacity must
        // be scanned AFTER taking it: an earlier scan can become stale while a
        // second Cache instance completes a different entry. It is not a quota
        // against same-UID bypasses or writers configured with larger budgets.
        let (entries, stats) = self.scan()?;
        if entries.iter().any(|e| e.name == name) {
            let entry = entries
                .into_iter()
                .find(|e| e.name == name)
                .ok_or_else(|| invalid("Existing cache entry"))?;
            let bytes = read_entry(&self.root, &entry, || {})?;
            if bytes != encoded.bytes {
                return Err(invalid("Existing leaf body mismatch"));
            }
            temporary.remove()?;
            return self.root.directory.sync_all();
        }
        if stats.entries >= self.config.entries
            || stats.encoded_bytes + encoded.bytes.len() as u64 > self.config.encoded_bytes
        {
            return Err(invalid("Cache full; no automatic eviction"));
        }
        temporary.file.write_all(MAGIC)?;
        temporary
            .file
            .write_all(&(header.len() as u32).to_be_bytes())?;
        temporary.file.write_all(&header)?;
        temporary.file.write_all(&encoded.bytes)?;
        temporary.file.sync_all()?;
        self.root.check()?;
        let meta = stamp(&temporary.file.metadata()?);
        if named(&self.root.directory, TEMP)? != meta
            || meta.uid != unsafe { libc::geteuid() }
            || meta.gid != unsafe { libc::getegid() }
            || meta.mode & 0o7777 != 0o600
            || meta.bytes != 12 + header.len() as u64 + encoded.bytes.len() as u64
        {
            return Err(invalid("Own new cache publication changed"));
        }
        // Exclusive hardlink publication is portable across desktop/TV. Body is
        // complete+fsynced before visible. No existing content leaf is replaced.
        if unsafe {
            libc::linkat(
                self.root.directory.as_raw_fd(),
                TEMP.as_ptr(),
                self.root.directory.as_raw_fd(),
                name.as_ptr(),
                0,
            )
        } < 0
        {
            return Err(io::Error::last_os_error());
        }
        // Keep the cooperating-writer slot through the final scan. Releasing it
        // first could let another writer add/remove the slot during inventory,
        // making both publishers report failure despite one complete new leaf.
        let (_, after) = self.scan()?;
        if after.entries > self.config.entries || after.encoded_bytes > self.config.encoded_bytes {
            return Err(invalid("Concurrent cache change exceeded budget"));
        }
        temporary.remove()?;
        self.root.directory.sync_all()?;
        Ok(())
    }
    /// Caller first withdraws/joins old consumers. Only this exact ordinary cache
    /// entry is removed; never preferences, an unknown temp, or a Root latch.
    pub fn retire_after_join(&mut self, demand: Demand) -> io::Result<()> {
        if demand.generation.current() {
            return Err(invalid(
                "Withdraw generation before retirement; caller must join worker",
            ));
        }
        if !Arc::ptr_eq(&self.root, &demand.root) {
            return Err(invalid("Retire through the originating anchored Cache"));
        }
        let mut writer = self.writer_slot()?;
        let (_, identity, _) = self.root.record(&demand.entry.name)?;
        if identity != demand.entry.identity {
            return Err(invalid("Retire entry changed"));
        }
        self.root.check()?;
        if unsafe {
            libc::unlinkat(
                self.root.directory.as_raw_fd(),
                demand.entry.name.as_ptr(),
                0,
            )
        } < 0
        {
            return Err(io::Error::last_os_error());
        }
        writer.remove()?;
        self.root.directory.sync_all()
    }
}
struct Temporary<'a> {
    root: &'a Root,
    file: File,
    owned_name: bool,
}
impl Temporary<'_> {
    fn remove(&mut self) -> io::Result<()> {
        if !self.owned_name {
            return Ok(());
        }
        self.root.check()?;
        let held = stamp(&self.file.metadata()?);
        if held.mode & 0o170000 != 0o100000
            || held.mode & 0o7777 != 0o600
            || held.uid != unsafe { libc::geteuid() }
            || held.gid != unsafe { libc::getegid() }
        {
            return Err(invalid("Own temporary metadata changed"));
        }
        if named(&self.root.directory, TEMP)? != held {
            return Err(invalid("Own temporary name changed"));
        }
        if unsafe { libc::unlinkat(self.root.directory.as_raw_fd(), TEMP.as_ptr(), 0) } < 0 {
            return Err(io::Error::last_os_error());
        }
        self.owned_name = false;
        Ok(())
    }
}
impl Drop for Temporary<'_> {
    fn drop(&mut self) {
        let _ = self.remove();
    }
}
fn read_entry(root: &Root, entry: &Entry, after_read: impl FnOnce()) -> io::Result<Vec<u8>> {
    let (mut file, before) = root.file(&entry.name)?;
    if before != entry.identity {
        return Err(invalid("Snapshot entry replaced or changed"));
    }
    file.seek(std::io::SeekFrom::Start(entry.offset))?;
    let length = entry.record.encoded_bytes as usize;
    let mut bytes = vec![0; length + 1];
    let mut used = 0;
    let mut capped = (&mut file).take(MAX_ENCODED_BYTES as u64 + 1);
    while used < bytes.len() {
        let n = capped.read(&mut bytes[used..])?;
        if n == 0 {
            break;
        }
        used += n;
    }
    after_read();
    root.unchanged(&file, &entry.name, &before)?;
    if used != length {
        return Err(invalid("Encoded cache length changed"));
    }
    bytes.truncate(used);
    if digest(&bytes) != entry.record.encoded_sha256 {
        return Err(invalid("Encoded cache digest mismatch"));
    }
    Ok(bytes)
}
impl Snapshot {
    pub fn generation(&self) -> Generation {
        self.generation.clone()
    }
    /// Source must come from the caller's CURRENT successful catalogue. The
    /// snapshot never turns a cached SourceKey into a Source or reads its path.
    pub fn lookup(&self, source: &Source, producer: &ProducerVersion) -> Lookup {
        if !self.generation.current() {
            return Lookup::Missing;
        }
        let matching = self
            .index
            .get(source.key())
            .into_iter()
            .flatten()
            .map(|index| &self.entries[*index])
            .filter(|entry| &entry.record.producer == producer)
            .collect::<Vec<_>>();
        match matching.as_slice() {
            [entry] => Lookup::Ready(Box::new(Demand {
                root: self.root.clone(),
                entry: (*entry).clone(),
                generation: self.generation.clone(),
            })),
            [] => Lookup::Missing,
            _ => Lookup::Ambiguous,
        }
    }
    pub fn decode(&self, demand: Demand) -> io::Result<CachedIcon> {
        self.decode_using(demand, crate::icon_decode::decode)
    }
    fn decode_using(
        &self,
        demand: Demand,
        decoder: impl FnOnce(&[u8]) -> Option<(u32, u32, Vec<u8>)>,
    ) -> io::Result<CachedIcon> {
        if !self.generation.current()
            || !demand.generation.current()
            || !Arc::ptr_eq(&self.generation.seal, &demand.generation.seal)
        {
            return Err(invalid("Withdrawn or old cache generation"));
        }
        let bytes = read_entry(&self.root, &demand.entry, || {})?;
        let (width, height, rgba) =
            decoder(&bytes).ok_or_else(|| invalid("Bounded cached icon decode failed"))?;
        let expected = u64::from(width)
            .checked_mul(u64::from(height))
            .and_then(|v| v.checked_mul(4));
        if !self.generation.current()
            || width == 0
            || height == 0
            || width > MAX_AXIS
            || height > MAX_AXIS
            || expected != Some(rgba.len() as u64)
            || rgba.len() > MAX_RGBA_BYTES
            || rgba.capacity() > MAX_RGBA_BYTES
        {
            return Err(invalid("Stale or oversized decoded icon"));
        }
        Ok(CachedIcon {
            selection: demand.entry.selection,
            image: DecodedIcon {
                width,
                height,
                rgba,
                encoded_identity: demand.entry.identity,
            },
            record: demand.entry.record,
            generation: self.generation.clone(),
        })
    }
}
/// Pure non-authorizing inspection only; no cache I/O or retirement callsite.
#[cfg(any(target_arch = "arm", test))]
#[path = "durable_icon_cache_maintenance.rs"]
pub mod maintenance;
#[cfg(test)]
#[path = "durable_icon_cache_tests.rs"]
mod tests;
