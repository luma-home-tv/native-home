//! Fixed, ordinary read-only cache expectation. Never publisher/Root admission.
//! Current consumer identity and historical producer origin remain distinct.
use crate::durable_icon_cache::ProducerVersion;
use serde::{Deserialize, Serialize};
use std::io;
#[cfg(any(target_arch = "arm", test))]
use std::{
    ffi::CStr,
    fs::File,
    io::Read,
    os::{
        fd::{AsRawFd, FromRawFd},
        unix::fs::MetadataExt,
    },
};

pub const NAME: &str = "artwork-cache-consumer.json";
#[cfg(any(target_arch = "arm", test))]
const CNAME: &CStr = c"artwork-cache-consumer.json";
#[cfg(any(target_arch = "arm", test))]
const LIMIT: u64 = 4096;

/// Exact parser/source/codec policy bytes, not a claim of semantic forward compatibility.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ReaderContract {
    schema: u32,
    source_grammar_sha256: String,
    app_id_grammar_sha256: String,
    cargo_semantics_sha256: String,
    native_build_selection_sha256: String,
    record_parser_sha256: String,
    file_reader_sha256: String,
    decoder_policy_sha256: String,
    cargo_lock_sha256: String,
    dependency_lock_sha256: String,
    framework_patch_sha256: String,
}
#[cfg(any(target_arch = "arm", test))]
fn hex(raw: &[u8]) -> String {
    plx_base::sha256::sha256(raw)
        .iter()
        .map(|b| format!("{b:02x}"))
        .collect()
}
#[cfg(any(target_arch = "arm", test))]
impl ReaderContract {
    fn current() -> Self {
        // Compile-bound bytes, hashed once on the image worker; no source-file I/O.
        Self {
            schema: 1,
            source_grammar_sha256: hex(include_bytes!("app_artwork.rs")),
            app_id_grammar_sha256: hex(include_bytes!("model.rs")),
            cargo_semantics_sha256: env!("LUMA_CACHE_CARGO_SEMANTICS_SHA256").into(),
            native_build_selection_sha256: env!("LUMA_CACHE_NATIVE_BUILD_SELECTION_SHA256").into(),
            record_parser_sha256: hex(include_bytes!("durable_icon_cache.rs")),
            file_reader_sha256: hex(include_bytes!("packaged_icon_reader.rs")),
            decoder_policy_sha256: hex(include_bytes!("icon_decode.rs")),
            cargo_lock_sha256: hex(include_bytes!("../Cargo.lock")),
            dependency_lock_sha256: hex(include_bytes!("../dependency-lock.json")),
            framework_patch_sha256: hex(include_bytes!(
                "../patches/plxnative-framework-integration.patch"
            )),
        }
    }
}

#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "snake_case")]
#[cfg_attr(not(any(target_arch = "arm", test)), allow(dead_code))]
enum Selection {
    CurrentProducer,
    AcceptedProducer,
}
/// Worker-loaded association observation only: not current coverage or execution proof.
#[derive(Clone, Debug, Serialize)]
pub struct ConsumerAssociation {
    schema: u32,
    consumer_ui_sha256: String,
    producer: ProducerVersion,
    reader_contract: ReaderContract,
    selection: Selection,
    selector_sha256: Option<String>,
    association_only: bool,
}
/// Deliberately cannot be passed to population_expected/CachePublisher.
pub(crate) struct ReadExpectation {
    producer: ProducerVersion,
    association: Option<ConsumerAssociation>,
}
impl ReadExpectation {
    pub(crate) fn supplied(producer: ProducerVersion) -> Self {
        Self {
            producer,
            association: None,
        }
    }
    pub(crate) fn into_parts(self) -> (ProducerVersion, Option<ConsumerAssociation>) {
        (self.producer, self.association)
    }
}
#[cfg(any(target_arch = "arm", test))]
fn expectation(
    producer: ProducerVersion,
    ui: &str,
    selection: Selection,
    raw: Option<&[u8]>,
    contract: ReaderContract,
) -> ReadExpectation {
    ReadExpectation {
        association: Some(ConsumerAssociation {
            schema: 1,
            consumer_ui_sha256: ui.into(),
            producer: producer.clone(),
            reader_contract: contract,
            selection,
            selector_sha256: raw.map(hex),
            association_only: true,
        }),
        producer,
    }
}
#[cfg(any(target_arch = "arm", test))]
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Producer {
    schema: u32,
    app_id: String,
    producer_kind: String,
    source_revision: String,
    ui_sha256: String,
    broker_sha256: String,
}
#[cfg(any(target_arch = "arm", test))]
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Record {
    schema: u32,
    app_id: String,
    consumer_ui_sha256: String,
    reader_contract: ReaderContract,
    producer: Producer,
}
#[cfg(any(target_arch = "arm", test))]
fn invalid(message: &'static str) -> io::Error {
    io::Error::new(io::ErrorKind::InvalidData, message)
}
#[cfg(any(target_arch = "arm", test))]
fn hash(value: &str) -> io::Result<[u8; 32]> {
    if value.len() != 64
        || !value
            .bytes()
            .all(|b| b.is_ascii_digit() || (b'a'..=b'f').contains(&b))
    {
        return Err(invalid("Consumer/producer hash format"));
    }
    let mut result = [0; 32];
    for (i, b) in result.iter_mut().enumerate() {
        *b = u8::from_str_radix(&value[i * 2..i * 2 + 2], 16)
            .map_err(|_| invalid("Producer hash"))?;
    }
    Ok(result)
}
#[cfg(any(target_arch = "arm", test))]
fn parse(raw: &[u8], actual_ui: &str) -> io::Result<ReadExpectation> {
    let record: Record =
        serde_json::from_slice(raw).map_err(|_| invalid("Closed consumer selector schema"))?;
    let _ = hash(actual_ui)?;
    if record.schema != 1
        || record.app_id != crate::identity::PREVIEW
        || record.consumer_ui_sha256 != actual_ui
        || record.reader_contract != ReaderContract::current()
        || record.producer.schema != 1
        || record.producer.app_id != crate::identity::PREVIEW
        || record.producer.producer_kind != "joined-single-positive-v1"
    {
        return Err(invalid(
            "Actual consumer identity or exact reader/producer contract differs",
        ));
    }
    let producer = ProducerVersion::new(
        &record.producer.source_revision,
        hash(&record.producer.ui_sha256)?,
        hash(&record.producer.broker_sha256)?,
    )?;
    Ok(expectation(
        producer,
        actual_ui,
        Selection::AcceptedProducer,
        Some(raw),
        record.reader_contract,
    ))
}

#[cfg(any(target_arch = "arm", test))]
fn directory(file: &File) -> io::Result<(u64, u64, u32, u32, u32)> {
    let m = file.metadata()?;
    if !m.is_dir() {
        return Err(invalid("Consumer selector parent"));
    }
    Ok((m.dev(), m.ino(), m.uid(), m.gid(), m.mode()))
}
#[cfg(any(target_arch = "arm", test))]
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
#[cfg(any(target_arch = "arm", test))]
fn named(parent: &File) -> io::Result<File> {
    let fd = unsafe {
        libc::openat(
            parent.as_raw_fd(),
            CNAME.as_ptr(),
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
    parent: &File,
    owner: u32,
    actual_ui: &str,
    after_read: impl FnOnce(),
) -> io::Result<Option<ReadExpectation>> {
    let anchor = directory(parent)?;
    let file = match named(parent) {
        Ok(file) => file,
        Err(e) if e.raw_os_error() == Some(libc::ENOENT) => {
            if directory(parent)? != anchor {
                return Err(invalid("Consumer parent changed"));
            }
            return Ok(None);
        }
        Err(e) => return Err(e),
    };
    let m = file.metadata()?;
    if !m.is_file()
        || m.uid() != owner
        || m.gid()
            != if owner == 0 {
                0
            } else {
                unsafe { libc::getegid() }
            }
        || !matches!(m.mode() & 0o7777, 0o444 | 0o644)
        || m.nlink() != 1
        || m.len() == 0
        || m.len() > LIMIT
    {
        return Err(invalid("Bounded immutable packaged consumer selector"));
    }
    let held = stamp(&m);
    let mut raw = Vec::with_capacity(m.len() as usize + 1);
    (&file).take(LIMIT + 1).read_to_end(&mut raw)?;
    after_read();
    if raw.len() as u64 != m.len()
        || stamp(&file.metadata()?) != held
        || stamp(&named(parent)?.metadata()?) != held
        || directory(parent)? != anchor
    {
        return Err(invalid("Consumer held/named file or parent changed"));
    }
    parse(&raw, actual_ui).map(Some)
}
#[cfg(any(target_arch = "arm", test))]
fn choose(
    loaded: io::Result<Option<ReadExpectation>>,
    fallback: impl FnOnce() -> io::Result<Option<ReadExpectation>>,
) -> io::Result<Option<ReadExpectation>> {
    match loaded? {
        Some(value) => Ok(Some(value)),
        None => fallback(),
    }
}

pub(crate) fn load_current() -> io::Result<Option<ReadExpectation>> {
    #[cfg(target_arch = "arm")]
    {
        use crate::broker_installation::Profile;
        use std::os::unix::fs::OpenOptionsExt;
        if plx_base::paths::app_id() != crate::identity::PREVIEW
            || unsafe { libc::geteuid() } < 1000
            || plx_base::paths::app_dir() != std::path::Path::new(Profile::Preview.bundle())
        {
            return Err(invalid("Exact native ordinary Preview consumer required"));
        }
        let actual = crate::broker_probe::own_identity_for_profile(Profile::Preview)
            .map_err(|_| invalid("Consumer self identity unavailable"))?;
        let path = plx_base::paths::app_dir();
        let parent = std::fs::OpenOptions::new()
            .read(true)
            .custom_flags(libc::O_DIRECTORY | libc::O_NOFOLLOW | libc::O_CLOEXEC)
            .open(path)?;
        let anchor = directory(&parent)?;
        let result = choose(load_at(&parent, 0, &actual.ui_sha256, || {}), || {
            crate::cache_producer_profile::load_current().map(|p| {
                p.map(|p| {
                    expectation(
                        p.version(),
                        &actual.ui_sha256,
                        Selection::CurrentProducer,
                        None,
                        ReaderContract::current(),
                    )
                })
            })
        })?;
        let m = std::fs::symlink_metadata(path)?;
        if !m.is_dir() || (m.dev(), m.ino(), m.uid(), m.gid(), m.mode()) != anchor {
            return Err(invalid("Consumer package directory replaced"));
        }
        Ok(result)
    }
    #[cfg(not(target_arch = "arm"))]
    {
        Err(io::Error::new(
            io::ErrorKind::Unsupported,
            "Consumer selector requires native Preview",
        ))
    }
}

#[cfg(test)]
pub(crate) fn load_for_test(parent: &File, actual_ui: &str) -> io::Result<Option<ReadExpectation>> {
    load_at(parent, unsafe { libc::geteuid() }, actual_ui, || {})
}
#[cfg(test)]
pub(crate) fn record_for_test(ui: &str, producer: serde_json::Value) -> serde_json::Value {
    serde_json::json!({"schema":1,"app_id":crate::identity::PREVIEW,"consumer_ui_sha256":ui,
        "reader_contract":ReaderContract::current(),"producer":producer})
}
#[cfg(test)]
#[path = "cache_consumer_profile_tests.rs"]
mod tests;
