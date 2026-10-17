//! Fixed ordinary packaged producer association, never Root/helper authority.
//! Load on a worker. No cache record, caller path, URI or environment selects it.
//! A matching helper hash is an expectation, not evidence that a helper executed.
#[cfg(any(target_arch = "arm", test))]
use crate::broker_protocol::Marker;
use crate::durable_icon_cache::ProducerVersion;
#[cfg(any(target_arch = "arm", test))]
use serde::Deserialize;
use std::io;
#[cfg(any(target_arch = "arm", test))]
use std::{
    ffi::CStr,
    fs::{File, Metadata},
    io::Read,
    os::{
        fd::{AsRawFd, FromRawFd},
        unix::fs::MetadataExt,
    },
};
#[cfg(any(target_arch = "arm", test))]
const NAME: &CStr = c"artwork-cache-producer.json";
#[cfg(any(target_arch = "arm", test))]
const LIMIT: u64 = 4096;
#[cfg(any(target_arch = "arm", test))]
fn invalid(message: &'static str) -> io::Error {
    io::Error::new(io::ErrorKind::InvalidData, message)
}
#[cfg(any(target_arch = "arm", test))]
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Record {
    schema: u32,
    app_id: String,
    producer_kind: String,
    source_revision: String,
    ui_sha256: String,
    broker_sha256: String,
}
/// Constructed only by the fixed packaged reader, independent of cache metadata.
/// Immutable association data, not a positive settlement or dispatch grant.
#[derive(Clone, Debug)]
pub struct ExpectedProducer {
    version: ProducerVersion,
    #[cfg(any(target_arch = "arm", test))]
    ui_sha256: String,
    #[cfg(any(target_arch = "arm", test))]
    broker_sha256: String,
}
impl ExpectedProducer {
    pub fn version(&self) -> ProducerVersion {
        self.version.clone()
    }
    #[cfg(any(target_arch = "arm", test))]
    pub(crate) fn matches_marker(&self, marker: &Marker) -> bool {
        self.ui_sha256 == marker.ui_sha256 && self.broker_sha256 == marker.broker_sha256
    }
    #[cfg(test)]
    pub(crate) fn load_for_test(directory: &File, actual_ui: &str) -> io::Result<Option<Self>> {
        load_at(directory, unsafe { libc::geteuid() }, actual_ui, || {})
    }
}
#[cfg(any(target_arch = "arm", test))]
fn hash(value: &str) -> io::Result<[u8; 32]> {
    if value.len() != 64
        || !value
            .bytes()
            .all(|b| b.is_ascii_digit() || (b'a'..=b'f').contains(&b))
    {
        return Err(invalid("Producer hash format"));
    }
    let mut out = [0; 32];
    for (i, byte) in out.iter_mut().enumerate() {
        *byte = u8::from_str_radix(&value[i * 2..i * 2 + 2], 16)
            .map_err(|_| invalid("Producer hash format"))?;
    }
    Ok(out)
}
#[cfg(any(target_arch = "arm", test))]
fn parse(raw: &[u8], actual_ui: &str) -> io::Result<ExpectedProducer> {
    let record: Record =
        serde_json::from_slice(raw).map_err(|_| invalid("Closed producer sidecar schema"))?;
    if record.schema != 1
        || record.app_id != crate::identity::PREVIEW
        || record.producer_kind != "joined-single-positive-v1"
        || record.ui_sha256 != actual_ui
    {
        return Err(invalid("Producer association or actual self UI differs"));
    }
    let version = ProducerVersion::new(
        &record.source_revision,
        hash(&record.ui_sha256)?,
        hash(&record.broker_sha256)?,
    )?;
    Ok(ExpectedProducer {
        version,
        ui_sha256: record.ui_sha256,
        broker_sha256: record.broker_sha256,
    })
}

/// Pure inspection for ordinary retention planning only. The private parsed
/// ExpectedProducer never escapes; returned data grants no publishing/Root right.
#[cfg(any(target_arch = "arm", test))]
pub(crate) fn inspect_for_maintenance(raw: &[u8], actual_ui: &str) -> io::Result<ProducerVersion> {
    if raw.is_empty() || raw.len() as u64 > LIMIT {
        return Err(invalid("Bounded maintenance producer bytes"));
    }
    parse(raw, actual_ui).map(|expected| expected.version())
}
#[cfg(any(target_arch = "arm", test))]
#[derive(PartialEq, Eq)]
struct WriterStamp {
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
#[cfg(any(target_arch = "arm", test))]
impl WriterStamp {
    fn of(m: &Metadata) -> Self {
        Self {
            device: m.dev(),
            inode: m.ino(),
            uid: m.uid(),
            gid: m.gid(),
            mode: m.mode(),
            links: m.nlink(),
            bytes: m.len(),
            modified: (m.mtime(), m.mtime_nsec()),
            changed: (m.ctime(), m.ctime_nsec()),
        }
    }
}
#[cfg(any(target_arch = "arm", test))]
fn directory_stamp(file: &File) -> io::Result<(u64, u64, u32, u32, u32)> {
    let m = file.metadata()?;
    if !m.is_dir() {
        return Err(invalid("Producer parent is not a directory"));
    }
    Ok((m.dev(), m.ino(), m.uid(), m.gid(), m.mode()))
}
#[cfg(any(target_arch = "arm", test))]
fn open_named(directory: &File) -> io::Result<File> {
    let fd = unsafe {
        libc::openat(
            directory.as_raw_fd(),
            NAME.as_ptr(),
            libc::O_RDONLY | libc::O_NOFOLLOW | libc::O_NONBLOCK | libc::O_CLOEXEC,
        )
    };
    if fd < 0 {
        return Err(io::Error::last_os_error());
    }
    Ok(unsafe { File::from_raw_fd(fd) })
}
#[cfg(any(target_arch = "arm", test))]
fn load_at(
    directory: &File,
    owner: u32,
    actual_ui: &str,
    after_read: impl FnOnce(),
) -> io::Result<Option<ExpectedProducer>> {
    let anchor = directory_stamp(directory)?;
    let file = match open_named(directory) {
        Ok(file) => file,
        Err(error) if error.raw_os_error() == Some(libc::ENOENT) => {
            if directory_stamp(directory)? != anchor {
                return Err(invalid("Producer directory changed"));
            }
            return Ok(None);
        }
        Err(error) => return Err(error),
    };
    let before = file.metadata()?;
    if !before.is_file()
        || before.uid() != owner
        || before.gid()
            != (if owner == 0 {
                0
            } else {
                unsafe { libc::getegid() }
            })
        || !matches!(before.mode() & 0o7777, 0o444 | 0o644)
        || before.nlink() != 1
        || before.len() == 0
        || before.len() > LIMIT
    {
        return Err(invalid("Bounded single-link packaged producer metadata"));
    }
    let stamp = WriterStamp::of(&before);
    let mut raw = vec![0; before.len() as usize + 1];
    let mut limited = (&file).take(LIMIT + 1);
    let mut used = 0;
    while used < raw.len() {
        let n = limited.read(&mut raw[used..])?;
        if n == 0 {
            break;
        }
        used += n;
    }
    raw.truncate(used);
    after_read();
    if raw.len() as u64 != before.len()
        || WriterStamp::of(&file.metadata()?) != stamp
        || WriterStamp::of(&open_named(directory)?.metadata()?) != stamp
        || directory_stamp(directory)? != anchor
    {
        return Err(invalid("Producer held/named bytes or directory changed"));
    }
    parse(&raw, actual_ui).map(Some)
}
/// Native Preview fixed package reader only. Absence disables ordinary cache;
/// malformed present is a diagnostic error, never a Root activation request.
pub fn load_current() -> io::Result<Option<ExpectedProducer>> {
    #[cfg(target_arch = "arm")]
    {
        use crate::broker_installation::Profile;
        use std::os::unix::fs::OpenOptionsExt;
        if plx_base::paths::app_id() != crate::identity::PREVIEW
            || unsafe { libc::geteuid() } < 1000
            || plx_base::paths::app_dir() != std::path::Path::new(Profile::Preview.bundle())
        {
            return Err(invalid("Exact ordinary Preview producer reader required"));
        }
        let actual = crate::broker_probe::own_identity_for_profile(Profile::Preview)
            .map_err(|_| invalid("Actual Preview self identity unavailable"))?;
        let path = plx_base::paths::app_dir();
        let directory = std::fs::OpenOptions::new()
            .read(true)
            .custom_flags(libc::O_DIRECTORY | libc::O_NOFOLLOW | libc::O_CLOEXEC)
            .open(path)?;
        let anchor = directory_stamp(&directory)?;
        let value = load_at(&directory, 0, &actual.ui_sha256, || {})?;
        let named = std::fs::symlink_metadata(path)?;
        if !named.is_dir()
            || (
                named.dev(),
                named.ino(),
                named.uid(),
                named.gid(),
                named.mode(),
            ) != anchor
        {
            return Err(invalid("Actual packaged producer directory replaced"));
        }
        Ok(value)
    }
    #[cfg(not(target_arch = "arm"))]
    {
        Err(io::Error::new(
            io::ErrorKind::Unsupported,
            "Packaged producer reader requires native Preview",
        ))
    }
}
#[cfg(test)]
#[path = "cache_producer_profile_tests.rs"]
mod tests;
