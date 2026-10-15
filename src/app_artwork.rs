//! Ordinary-process artwork metadata and bounded, no-follow reads. No GL, URI,
//! policy changes, writes or privileged fallback. Live sources never replace the
//! separately packaged `Application.icon` fallback. The passive survey belongs
//! only in the isolated native probe, never in the periodic normal-app reader.
use crate::model::valid_app_id;
use serde::Serialize;
use serde_json::{json, Value};
use std::collections::{BTreeMap, BTreeSet};
use std::ffi::CString;
use std::fs::File;
use std::io::{self, Read};
use std::os::fd::{AsRawFd, FromRawFd, RawFd};
use std::os::unix::fs::MetadataExt;

pub const MAX_ENCODED_BYTES: usize = 1024 * 1024;
const MAX_APPS: usize = 512;
const MAX_RELATIVE_BYTES: usize = 512;
const MAX_COMPONENTS: usize = 8;

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum RootClass {
    LgSystem,
    EncryptedApps,
    DeveloperApps,
    SystemApps,
    OptionalContent,
}
impl RootClass {
    pub const ALL: [Self; 5] = [
        Self::LgSystem,
        Self::EncryptedApps,
        Self::DeveloperApps,
        Self::SystemApps,
        Self::OptionalContent,
    ];
    pub fn prefix(self) -> &'static str {
        match self {
            Self::LgSystem => "/usr/palm/applications",
            Self::EncryptedApps => "/media/cryptofs/apps/usr/palm/applications",
            Self::DeveloperApps => "/media/developer/apps/usr/palm/applications",
            Self::SystemApps => "/media/system/apps/usr/palm/applications",
            Self::OptionalContent => "/mnt/otncabi/usr/palm/applications",
        }
    }
}
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum IconKind {
    Icon,
    LargeIcon,
}
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Serialize)]
pub struct SourceKey {
    pub app_id: String,
    pub version: String,
    pub class: RootClass,
    pub folder: String,
    pub relative: Vec<String>,
    pub kind: IconKind,
}
/// Construction is private: callers cannot turn arbitrary paths into readers.
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct Source {
    key: SourceKey,
}
impl Source {
    pub fn key(&self) -> &SourceKey {
        &self.key
    }
    pub fn app_id(&self) -> &str {
        &self.key.app_id
    }
    pub fn root_class(&self) -> RootClass {
        self.key.class
    }
    pub fn components(&self) -> Vec<String> {
        self.key
            .folder
            .trim_start_matches('/')
            .split('/')
            .map(str::to_owned)
            .chain(self.key.relative.iter().cloned())
            .collect()
    }
}
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum MetadataError {
    UnavailableCatalogue,
    InvalidArray,
    TooManyEntries,
    MissingVersion,
    MissingFolder,
    UnknownRootOrFolder,
    MissingIcon,
    InvalidRelativePath,
}
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct Rejection {
    pub kind: IconKind,
    pub reason: MetadataError,
}
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct AppSources {
    pub app_id: String,
    pub version: Option<String>,
    pub icon: Option<Source>,
    pub large_icon: Option<Source>,
    pub rejections: Vec<Rejection>,
}
impl AppSources {
    pub fn preferred(&self) -> Option<&Source> {
        self.large_icon.as_ref().or(self.icon.as_ref())
    }
}
pub type SourceMap = BTreeMap<String, AppSources>;
fn text(value: &Value, limit: usize) -> Option<&str> {
    value
        .as_str()
        .filter(|s| !s.is_empty() && s.len() <= limit && !s.chars().any(char::is_control))
}
fn components(value: &str) -> Option<Vec<String>> {
    if value.is_empty()
        || value.len() > MAX_RELATIVE_BYTES
        || value.starts_with('/')
        || value.contains(['\\', ':'])
        || value.chars().any(char::is_control)
    {
        return None;
    }
    let out = value.split('/').collect::<Vec<_>>();
    if out.len() > MAX_COMPONENTS
        || out
            .iter()
            .any(|s| s.is_empty() || *s == "." || *s == ".." || s.len() > 128)
    {
        return None;
    }
    Some(out.into_iter().map(str::to_owned).collect())
}
fn source(
    app: &Value,
    id: &str,
    version: Option<&str>,
    kind: IconKind,
) -> Result<Source, MetadataError> {
    let version = version.ok_or(MetadataError::MissingVersion)?;
    let folder = text(&app["folderPath"], 512).ok_or(MetadataError::MissingFolder)?;
    let class = RootClass::ALL
        .into_iter()
        .find(|class| id != "." && id != ".." && folder == format!("{}/{id}", class.prefix()))
        .ok_or(MetadataError::UnknownRootOrFolder)?;
    let field = match kind {
        IconKind::Icon => "icon",
        IconKind::LargeIcon => "largeIcon",
    };
    let relative = text(&app[field], MAX_RELATIVE_BYTES).ok_or(MetadataError::MissingIcon)?;
    let relative = components(relative).ok_or(MetadataError::InvalidRelativePath)?;
    Ok(Source {
        key: SourceKey {
            app_id: id.into(),
            version: version.into(),
            class,
            folder: folder.into(),
            relative,
            kind,
        },
    })
}
/// Same admission as live::parse, including first valid duplicate ownership.
/// Admitted apps without usable artwork remain in the map with empty sources.
pub fn sources_from_raw(raw: &Value, own_id: &str) -> Result<SourceMap, MetadataError> {
    let replies = &raw["replies"];
    if replies["apps"]["returnValue"].as_bool() != Some(true)
        || replies["launch_points"]["returnValue"].as_bool() != Some(true)
    {
        return Err(MetadataError::UnavailableCatalogue);
    }
    let apps = replies["apps"]["apps"]
        .as_array()
        .ok_or(MetadataError::InvalidArray)?;
    let points = replies["launch_points"]["launchPoints"]
        .as_array()
        .ok_or(MetadataError::InvalidArray)?;
    if apps.len() > MAX_APPS || points.len() > MAX_APPS {
        return Err(MetadataError::TooManyEntries);
    }
    let registered = points
        .iter()
        .filter_map(|p| p["id"].as_str())
        .filter(|id| valid_app_id(id) && *id != own_id)
        .collect::<BTreeSet<_>>();
    let mut out = SourceMap::new();
    for app in apps {
        let Some(id) = app["id"].as_str() else {
            continue;
        };
        if !valid_app_id(id)
            || id == own_id
            || !registered.contains(id)
            || app["visible"].as_bool() != Some(true)
            || !matches!(
                app["type"].as_str(),
                Some("native" | "native_builtin" | "web")
            )
            || text(&app["title"], 512).is_none()
            || out.contains_key(id)
        {
            continue;
        }
        let version = text(&app["version"], 128);
        let mut entry = AppSources {
            app_id: id.into(),
            version: version.map(str::to_owned),
            icon: None,
            large_icon: None,
            rejections: Vec::new(),
        };
        for kind in [IconKind::Icon, IconKind::LargeIcon] {
            match source(app, id, version, kind) {
                Ok(source) => match kind {
                    IconKind::Icon => entry.icon = Some(source),
                    IconKind::LargeIcon => entry.large_icon = Some(source),
                },
                Err(reason) => entry.rejections.push(Rejection { kind, reason }),
            }
        }
        out.insert(id.into(), entry);
    }
    Ok(out)
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct FileIdentity {
    pub device: u64,
    pub inode: u64,
    pub bytes: u64,
    pub mode: u32,
    pub uid: u32,
    pub gid: u32,
    pub modified_seconds: i64,
    pub modified_nanos: i64,
    pub changed_seconds: i64,
    pub changed_nanos: i64,
}
impl FileIdentity {
    fn of(meta: &std::fs::Metadata) -> Self {
        Self {
            device: meta.dev(),
            inode: meta.ino(),
            bytes: meta.len(),
            mode: meta.mode(),
            uid: meta.uid(),
            gid: meta.gid(),
            modified_seconds: meta.mtime(),
            modified_nanos: meta.mtime_nsec(),
            changed_seconds: meta.ctime(),
            changed_nanos: meta.ctime_nsec(),
        }
    }
    // libc field widths vary between the desktop and ARM32 TV ABIs.
    #[allow(clippy::unnecessary_cast)]
    fn matches_path(&self, st: &libc::stat) -> bool {
        self.device == st.st_dev as u64
            && self.inode == st.st_ino as u64
            && self.bytes == st.st_size as u64
            && self.mode == st.st_mode as u32
            && self.uid == st.st_uid
            && self.gid == st.st_gid
    }
}
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum ReadReason {
    Io,
    PrivilegedProcess,
    InvalidComponent,
    Symlink,
    NotRegular,
    Empty,
    TooLarge,
    Allocation,
    Changed,
}
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct ReadError {
    pub reason: ReadReason,
    pub operation: &'static str,
    pub component: usize,
    pub errno: Option<i32>,
    pub io_kind: Option<String>,
}
fn refusal(reason: ReadReason, operation: &'static str, component: usize) -> ReadError {
    ReadError {
        reason,
        operation,
        component,
        errno: None,
        io_kind: None,
    }
}
fn failed(error: io::Error, operation: &'static str, component: usize) -> ReadError {
    ReadError {
        reason: ReadReason::Io,
        operation,
        component,
        errno: error.raw_os_error(),
        io_kind: Some(format!("{:?}", error.kind())),
    }
}
fn name(value: &str, index: usize) -> Result<CString, ReadError> {
    if value.is_empty() || matches!(value, "." | "..") || value.contains('/') {
        return Err(refusal(ReadReason::InvalidComponent, "component", index));
    }
    CString::new(value).map_err(|_| refusal(ReadReason::InvalidComponent, "component", index))
}
fn open_at(
    parent: RawFd,
    child: &CString,
    directory: bool,
    index: usize,
) -> Result<File, ReadError> {
    let operation = if directory {
        "open_directory"
    } else {
        "open_file"
    };
    let flags = libc::O_RDONLY
        | libc::O_NOFOLLOW
        | libc::O_CLOEXEC
        | if directory {
            libc::O_DIRECTORY
        } else {
            libc::O_NONBLOCK
        };
    let fd = unsafe { libc::openat(parent, child.as_ptr(), flags) };
    if fd < 0 {
        return Err(failed(io::Error::last_os_error(), operation, index));
    }
    Ok(unsafe { File::from_raw_fd(fd) })
}
fn stat_at(
    parent: RawFd,
    child: &CString,
    operation: &'static str,
    index: usize,
) -> Result<libc::stat, ReadError> {
    let mut st = std::mem::MaybeUninit::<libc::stat>::uninit();
    if unsafe {
        libc::fstatat(
            parent,
            child.as_ptr(),
            st.as_mut_ptr(),
            libc::AT_SYMLINK_NOFOLLOW,
        )
    } != 0
    {
        return Err(failed(io::Error::last_os_error(), operation, index));
    }
    Ok(unsafe { st.assume_init() })
}
#[derive(Debug)]
pub struct EncodedFile {
    pub bytes: Vec<u8>,
    pub identity: FileIdentity,
    pub digest: [u8; 32],
    /// Header observation only; not PNG validity, successful decode or allocation admission.
    pub png_dimensions: Option<[u32; 2]>,
}
fn png_dimensions(bytes: &[u8]) -> Option<[u32; 2]> {
    if bytes.len() < 33
        || &bytes[..8] != b"\x89PNG\r\n\x1a\n"
        || bytes[8..12] != 13_u32.to_be_bytes()
        || &bytes[12..16] != b"IHDR"
    {
        return None;
    }
    let width = u32::from_be_bytes(bytes[16..20].try_into().ok()?);
    let height = u32::from_be_bytes(bytes[20..24].try_into().ok()?);
    (width > 0 && height > 0).then_some([width, height])
}
fn read_components(
    base: &File,
    parts: &[String],
    after_read: impl FnOnce(),
) -> Result<EncodedFile, ReadError> {
    if parts.is_empty() {
        return Err(refusal(ReadReason::InvalidComponent, "components", 0));
    }
    let mut directory = base.try_clone().map_err(|e| failed(e, "clone_root", 0))?;
    for (i, part) in parts[..parts.len() - 1].iter().enumerate() {
        directory = open_at(directory.as_raw_fd(), &name(part, i)?, true, i)?;
    }
    let i = parts.len() - 1;
    let child = name(&parts[i], i)?;
    let path_before = stat_at(directory.as_raw_fd(), &child, "stat_before", i)?;
    if path_before.st_mode & libc::S_IFMT == libc::S_IFLNK {
        return Err(refusal(ReadReason::Symlink, "stat_before", i));
    }
    if path_before.st_mode & libc::S_IFMT != libc::S_IFREG {
        return Err(refusal(ReadReason::NotRegular, "stat_before", i));
    }
    let mut file = open_at(directory.as_raw_fd(), &child, false, i)?;
    let metadata = file.metadata().map_err(|e| failed(e, "fstat_before", i))?;
    let before = FileIdentity::of(&metadata);
    if !metadata.is_file() || !before.matches_path(&path_before) {
        return Err(refusal(ReadReason::Changed, "fstat_before", i));
    }
    if before.bytes > MAX_ENCODED_BYTES as u64 {
        return Err(refusal(ReadReason::TooLarge, "fstat_before", i));
    }
    if before.bytes == 0 {
        return Err(refusal(ReadReason::Empty, "fstat_before", i));
    }
    let mut bytes = Vec::new();
    bytes
        .try_reserve_exact(before.bytes as usize)
        .map_err(|_| refusal(ReadReason::Allocation, "reserve", i))?;
    bytes.resize(before.bytes as usize, 0);
    file.read_exact(&mut bytes)
        .map_err(|e| failed(e, "read_exact", i))?;
    let mut extra = [0_u8; 1];
    if file
        .read(&mut extra)
        .map_err(|e| failed(e, "read_tail", i))?
        != 0
    {
        return Err(refusal(ReadReason::Changed, "read_tail", i));
    }
    after_read();
    let after = FileIdentity::of(&file.metadata().map_err(|e| failed(e, "fstat_after", i))?);
    let path_after = stat_at(directory.as_raw_fd(), &child, "stat_after", i)?;
    if before != after || !after.matches_path(&path_after) {
        return Err(refusal(ReadReason::Changed, "identity_after", i));
    }
    Ok(EncodedFile {
        png_dimensions: png_dimensions(&bytes),
        digest: plx_base::sha256::sha256(&bytes),
        bytes,
        identity: after,
    })
}
pub fn read_encoded(source: &Source) -> Result<EncodedFile, ReadError> {
    if unsafe { libc::geteuid() } == 0 {
        return Err(refusal(ReadReason::PrivilegedProcess, "uid", 0));
    }
    read_admitted_source(source)
}
#[cfg_attr(not(all(target_os = "linux", target_arch = "arm")), allow(dead_code))]
fn root_helper_identity(
    arm_linux: bool,
    real_uid: u32,
    effective_uid: u32,
    executable: Option<&std::path::Path>,
) -> bool {
    arm_linux
        && real_uid == 0
        && effective_uid == 0
        && executable
            .and_then(std::path::Path::file_name)
            .is_some_and(|name| name == "artwork-broker-proof")
}
/// Explicit privileged helper entry, not an ordinary-reader fallback. The
/// server must independently authorize its peer/context and derive this sealed
/// Source from its own fresh catalogue before calling, then recheck it later.
#[cfg_attr(not(all(target_os = "linux", target_arch = "arm")), allow(dead_code))]
pub(crate) fn read_root_encoded(source: &Source) -> Result<EncodedFile, ReadError> {
    let executable = std::env::current_exe().ok();
    if !root_helper_identity(
        cfg!(all(target_os = "linux", target_arch = "arm")),
        unsafe { libc::getuid() },
        unsafe { libc::geteuid() },
        executable.as_deref(),
    ) {
        return Err(refusal(
            ReadReason::PrivilegedProcess,
            "root_helper_identity",
            0,
        ));
    }
    read_admitted_source(source)
}
/// Installed Proof authority is separate from the historical operator helper.
/// This accepts only an already sealed server-owned source, never a wire path.
#[cfg(all(target_os = "linux", target_arch = "arm"))]
pub(crate) fn read_installed_encoded(
    source: &Source,
    installation: &crate::broker_installation::Installation,
) -> Result<EncodedFile, ReadError> {
    let denied = || refusal(ReadReason::PrivilegedProcess, "installed_authority", 0);
    if installation.pins().profile != crate::broker_installation::Profile::Proof {
        return Err(denied());
    }
    // Revalidates Root UID4/start/ELF and the protected policy/tree as well as
    // its pinned UI lookup. No developer-app ancestor becomes Root authority.
    installation.revalidate().map_err(|_| denied())?;
    let file = read_admitted_source(source)?;
    installation.revalidate().map_err(|_| denied())?;
    Ok(file)
}
// Shared exact no-follow engine. Only the identity-gated entries above
// reach this function; none accepts a caller-supplied path.
fn read_admitted_source(source: &Source) -> Result<EncodedFile, ReadError> {
    let slash = CString::new("/").expect("literal root");
    let fd = unsafe {
        libc::open(
            slash.as_ptr(),
            libc::O_RDONLY | libc::O_DIRECTORY | libc::O_NOFOLLOW | libc::O_CLOEXEC,
        )
    };
    if fd < 0 {
        return Err(failed(io::Error::last_os_error(), "open_root", 0));
    }
    let root = unsafe { File::from_raw_fd(fd) };
    read_components(&root, &source.components(), || {})
}
fn hex(digest: &[u8]) -> String {
    digest.iter().map(|b| format!("{b:02x}")).collect()
}
/// At most five sequential reads (one admitted app/source per observed class),
/// at most1MiB each. No bytes survive into the report; no decode or GL is done.
/// Byte limits do not guarantee a wall-clock bound for a blocking filesystem.
pub fn survey(raw: &Value, own_id: &str) -> Value {
    let uid = unsafe { libc::geteuid() };
    let sources = match sources_from_raw(raw, own_id) {
        Ok(sources) => sources,
        Err(error) => {
            return json!({"schema":1,"uid":uid,"writes":false,"reads_attempted":0,"metadata_error":error})
        }
    };
    if uid == 0 {
        return json!({"schema":1,"uid":uid,"writes":false,"reads_attempted":0,"refusal":"privileged_process"});
    }
    let mut attempted = 0;
    let samples = RootClass::ALL.into_iter().map(|class| {
        let selected = sources.values().filter_map(AppSources::preferred).find(|s| s.root_class() == class);
        let Some(source) = selected else { return json!({"class":class,"attempted":false,"reason":"no_admitted_source"}); };
        attempted += 1;
        let result = match read_encoded(source) {
            Ok(file) => json!({"read_succeeded":true,"identity":file.identity,"encoded_bytes":file.bytes.len(),
                "content_sha256":hex(&file.digest),"png_header_dimensions":file.png_dimensions,"decode_attempted":false}),
            Err(error) => json!({"read_succeeded":false,"error":error}),
        };
        json!({"class":class,"attempted":true,"source":source.key(),"components":source.components(),"result":result})
    }).collect::<Vec<_>>();
    json!({"schema":1,"uid":uid,"writes":false,"maximum_encoded_bytes_per_read":MAX_ENCODED_BYTES,
        "maximum_samples":5,"reads_attempted":attempted,"samples":samples,
        "scope":"Ordinary-UID sample access and PNG header observations only; no decoder, texture, class-wide permission or normal-loader qualification"})
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use std::os::unix::fs::{symlink, PermissionsExt};
    use std::path::PathBuf;
    use std::sync::atomic::{AtomicU64, Ordering};

    fn app(id: &str, class: RootClass) -> Value {
        json!({"id":id,"title":"Fixture","type":"web","visible":true,"version":"1.0",
            "folderPath":format!("{}/{id}",class.prefix()),"icon":"icon.png","largeIcon":"images/large.png"})
    }
    fn raw(apps: Vec<Value>, ids: &[&str]) -> Value {
        json!({"replies":{"apps":{"returnValue":true,"apps":apps},
            "launch_points":{"returnValue":true,"launchPoints":ids.iter().map(|id|json!({"id":id})).collect::<Vec<_>>()}}})
    }
    fn parsed(a: Value) -> AppSources {
        sources_from_raw(&raw(vec![a], &["fixture.app"]), "own.app")
            .unwrap()
            .remove("fixture.app")
            .unwrap()
    }
    struct Temp(PathBuf);
    impl Temp {
        fn new() -> Self {
            static NEXT: AtomicU64 = AtomicU64::new(0);
            let stamp = std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos();
            let p = std::env::temp_dir().join(format!(
                "luma-artwork-{}-{stamp}-{}",
                std::process::id(),
                NEXT.fetch_add(1, Ordering::Relaxed)
            ));
            fs::create_dir(&p).unwrap();
            Self(p)
        }
        fn base(&self) -> File {
            File::open(&self.0).unwrap()
        }
        fn read(&self, parts: &[&str]) -> Result<EncodedFile, ReadError> {
            read_components(
                &self.base(),
                &parts.iter().map(|p| p.to_string()).collect::<Vec<_>>(),
                || {},
            )
        }
    }
    impl Drop for Temp {
        fn drop(&mut self) {
            let _ = fs::remove_dir_all(&self.0);
        }
    }
    fn header(width: u32, height: u32) -> Vec<u8> {
        let mut p = b"\x89PNG\r\n\x1a\n".to_vec();
        p.extend_from_slice(&13_u32.to_be_bytes());
        p.extend_from_slice(b"IHDR");
        p.extend_from_slice(&width.to_be_bytes());
        p.extend_from_slice(&height.to_be_bytes());
        p.extend_from_slice(&[8, 6, 0, 0, 0, 0, 0, 0, 0]);
        p
    }
    #[test]
    fn admission_matches_live_parser_and_first_valid_duplicate() {
        let mut invalid = app("fixture.app", RootClass::LgSystem);
        invalid["title"] = json!("");
        let first = app("fixture.app", RootClass::LgSystem);
        let mut second = first.clone();
        second["version"] = json!("2");
        let mut hidden = app("hidden", RootClass::LgSystem);
        hidden["visible"] = json!(false);
        let mut wrong = app("wrong", RootClass::LgSystem);
        wrong["type"] = json!("service");
        let mut noicon = app("noicon", RootClass::LgSystem);
        noicon["icon"] = Value::Null;
        noicon["largeIcon"] = Value::Null;
        let mut badtitle = app("badtitle", RootClass::LgSystem);
        badtitle["title"] = json!("bad\nname");
        let fixture = raw(
            vec![
                invalid,
                first,
                second,
                hidden,
                wrong,
                badtitle,
                app("own.app", RootClass::LgSystem),
                app("unregistered", RootClass::LgSystem),
                noicon,
                app("noicon", RootClass::LgSystem),
                app("..", RootClass::LgSystem),
            ],
            &[
                "fixture.app",
                "hidden",
                "wrong",
                "badtitle",
                "own.app",
                "noicon",
                "..",
            ],
        );
        let sources = sources_from_raw(&fixture, "own.app").unwrap();
        let crate::live::Reading::Value(live) =
            crate::live::parse(&fixture, "own.app", 1, false).catalogue
        else {
            panic!("fixture catalogue unavailable")
        };
        assert_eq!(
            sources.keys().cloned().collect::<Vec<_>>(),
            live.apps.iter().map(|a| a.id.clone()).collect::<Vec<_>>()
        );
        assert_eq!(sources["fixture.app"].version.as_deref(), Some("1.0"));
        assert!(sources["noicon"].preferred().is_none());
        assert!(sources[".."].preferred().is_none());
        assert!(live.apps.iter().all(|a| a.icon.is_none()));
    }
    #[test]
    fn all_five_exact_roots_and_nested_relative_paths() {
        for class in RootClass::ALL {
            let entry = parsed(app("fixture.app", class));
            let source = entry.preferred().unwrap();
            assert_eq!(source.root_class(), class);
            assert_eq!(source.key().relative, ["images", "large.png"]);
            assert_eq!(source.key().kind, IconKind::LargeIcon);
            assert_eq!(
                source.components().last().map(String::as_str),
                Some("large.png")
            );
        }
        for path in [
            "/usr/palm/applications/other",
            "/usr/palm/applications/fixture.app/",
            "/other/fixture.app",
            "/usr/palm/applications/fixture.app/../other",
        ] {
            let mut a = app("fixture.app", RootClass::LgSystem);
            a["folderPath"] = json!(path);
            let entry = parsed(a);
            assert!(entry.preferred().is_none(), "{path}");
            assert!(entry
                .rejections
                .iter()
                .all(|r| r.reason == MetadataError::UnknownRootOrFolder));
        }
    }
    #[test]
    fn unsafe_relative_metadata_is_rejected_without_losing_app() {
        for path in [
            "",
            "/icon.png",
            "../icon.png",
            "a/../icon.png",
            "./icon.png",
            "a//b",
            "a/",
            "https://host/icon.png",
            "file:icon.png",
            "a\\b",
            "bad\n.png",
            "a/b/c/d/e/f/g/h/i",
        ] {
            let mut a = app("fixture.app", RootClass::LgSystem);
            a["icon"] = json!(path);
            a["largeIcon"] = json!(path);
            let entry = parsed(a);
            assert!(entry.preferred().is_none(), "{path:?}");
            assert_eq!(entry.app_id, "fixture.app");
        }
        let mut a = app("fixture.app", RootClass::LgSystem);
        a["icon"] = json!("images/safe name-µ.png");
        assert!(parsed(a).icon.is_some());
    }
    #[test]
    fn version_and_path_are_keyed_and_map_exceeds_legacy_first64() {
        let original = parsed(app("fixture.app", RootClass::LgSystem))
            .icon
            .unwrap();
        for (field, value) in [("version", "2"), ("icon", "new.png")] {
            let mut a = app("fixture.app", RootClass::LgSystem);
            a[field] = json!(value);
            assert_ne!(original.key(), parsed(a).icon.unwrap().key());
        }
        let mut a = app("fixture.app", RootClass::LgSystem);
        a["version"] = Value::Null;
        assert!(parsed(a).preferred().is_none());
        let ids = (0..100)
            .map(|i| format!("fixture.{i:03}"))
            .collect::<Vec<_>>();
        let fixture = raw(
            ids.iter().map(|id| app(id, RootClass::LgSystem)).collect(),
            &ids.iter().map(String::as_str).collect::<Vec<_>>(),
        );
        assert_eq!(sources_from_raw(&fixture, "own.app").unwrap().len(), 100);
    }
    #[test]
    fn missing_malformed_and_oversized_catalogues_fail_closed() {
        assert_eq!(
            sources_from_raw(&json!({}), "own.app"),
            Err(MetadataError::UnavailableCatalogue)
        );
        let mut fixture = raw(vec![], &[]);
        fixture["replies"]["apps"]["apps"] = json!({});
        assert_eq!(
            sources_from_raw(&fixture, "own.app"),
            Err(MetadataError::InvalidArray)
        );
        fixture["replies"]["apps"]["apps"] =
            json!(vec![app("fixture.app", RootClass::LgSystem); 513]);
        assert_eq!(
            sources_from_raw(&fixture, "own.app"),
            Err(MetadataError::TooManyEntries)
        );
    }
    #[test]
    fn regular_file_bytes_identity_sha_and_header_are_observations() {
        let t = Temp::new();
        fs::create_dir(t.0.join("images")).unwrap();
        let bytes = header(4096, 2);
        fs::write(t.0.join("images/icon"), &bytes).unwrap();
        let read = t.read(&["images", "icon"]).unwrap();
        assert_eq!(read.bytes, bytes);
        assert_eq!(read.identity.bytes, 33);
        assert_eq!(read.digest, plx_base::sha256::sha256(&bytes));
        assert_eq!(read.png_dimensions, Some([4096, 2]));
        // Deliberately no CRC or pixel stream: header observation must not imply successful decoding.
        assert_eq!(png_dimensions(b"not an image"), None);
        assert_eq!(png_dimensions(&header(0, 2)), None);
        assert_eq!(png_dimensions(&bytes[..24]), None);
        let mut bad = bytes.clone();
        bad[12] = b'X';
        assert_eq!(png_dimensions(&bad), None);
        fs::write(t.0.join("plain"), b"abc").unwrap();
        assert_eq!(
            hex(&t.read(&["plain"]).unwrap().digest),
            "ba7816bf8f01cfea414140de5dae2223b00361a396177a9cb410ff61f20015ad"
        );
    }
    #[test]
    fn nofollow_blocks_final_and_parent_symlinks() {
        let t = Temp::new();
        fs::create_dir(t.0.join("real")).unwrap();
        fs::write(t.0.join("real/file"), b"safe").unwrap();
        symlink("real/file", t.0.join("link")).unwrap();
        assert_eq!(t.read(&["link"]).unwrap_err().reason, ReadReason::Symlink);
        symlink("real", t.0.join("parent")).unwrap();
        let error = t.read(&["parent", "file"]).unwrap_err();
        assert_eq!(error.operation, "open_directory");
        assert!(error.errno.is_some());
        assert!(matches!(
            error.errno,
            Some(libc::ELOOP) | Some(libc::ENOTDIR)
        ));
    }
    #[test]
    fn missing_and_invalid_components_preserve_failure_details() {
        let t = Temp::new();
        let missing = t.read(&["missing"]).unwrap_err();
        assert_eq!(missing.operation, "stat_before");
        assert_eq!(missing.errno, Some(libc::ENOENT));
        for path in ["..", ".", "a/b", "a\0b", ""] {
            assert_eq!(
                t.read(&[path]).unwrap_err().reason,
                ReadReason::InvalidComponent
            );
        }
        assert_eq!(
            t.read(&[]).unwrap_err().reason,
            ReadReason::InvalidComponent
        );
    }
    #[test]
    fn special_empty_and_oversize_files_are_refused_before_read() {
        let t = Temp::new();
        let p = CString::new(t.0.join("fifo").to_str().unwrap()).unwrap();
        assert_eq!(unsafe { libc::mkfifo(p.as_ptr(), 0o600) }, 0);
        assert_eq!(
            t.read(&["fifo"]).unwrap_err().reason,
            ReadReason::NotRegular
        );
        fs::write(t.0.join("empty"), []).unwrap();
        assert_eq!(t.read(&["empty"]).unwrap_err().reason, ReadReason::Empty);
        let big = File::create(t.0.join("big")).unwrap();
        big.set_len(MAX_ENCODED_BYTES as u64 + 1).unwrap();
        assert_eq!(t.read(&["big"]).unwrap_err().reason, ReadReason::TooLarge);
        assert_eq!(
            t.read(&["."]).unwrap_err().reason,
            ReadReason::InvalidComponent
        );
    }
    #[test]
    fn ordinary_permission_error_is_reported_without_fallback() {
        if unsafe { libc::geteuid() } == 0 {
            return;
        } // A privileged test cannot establish denial semantics.
        let t = Temp::new();
        let path = t.0.join("denied");
        fs::write(&path, b"private").unwrap();
        fs::set_permissions(&path, fs::Permissions::from_mode(0o000)).unwrap();
        let error = t.read(&["denied"]).unwrap_err();
        assert_eq!(error.errno, Some(libc::EACCES));
        assert_eq!(error.operation, "open_file");
        fs::set_permissions(&path, fs::Permissions::from_mode(0o600)).unwrap();
    }
    #[test]
    fn mutation_after_read_is_refused_even_with_same_length() {
        let t = Temp::new();
        let p = t.0.join("file");
        fs::write(&p, b"before").unwrap();
        let error = read_components(&t.base(), &["file".into()], || {
            fs::write(&p, b"after!").unwrap();
            fs::set_permissions(&p, fs::Permissions::from_mode(0o400)).unwrap();
        })
        .unwrap_err();
        assert_eq!(error.reason, ReadReason::Changed);
        assert_eq!(error.operation, "identity_after");
    }
    #[test]
    fn same_path_replacement_during_read_is_refused_and_next_read_is_fresh() {
        let t = Temp::new();
        fs::write(t.0.join("file"), b"before").unwrap();
        let original = t.read(&["file"]).unwrap();
        fs::write(t.0.join("replacement"), b"after!").unwrap();
        let error = read_components(&t.base(), &["file".into()], || {
            fs::rename(t.0.join("replacement"), t.0.join("file")).unwrap()
        })
        .unwrap_err();
        assert_eq!(error.reason, ReadReason::Changed);
        let fresh = t.read(&["file"]).unwrap();
        assert_ne!(original.identity.inode, fresh.identity.inode);
        assert_ne!(original.digest, fresh.digest);
        assert_eq!(fresh.bytes, b"after!");
    }
    #[test]
    fn explicit_root_reader_requires_arm_root_and_exact_helper_basename() {
        let helper = std::path::Path::new("/tmp/owned-proof/artwork-broker-proof");
        assert!(root_helper_identity(true, 0, 0, Some(helper)));
        for (platform, real, effective, path) in [
            (false, 0, 0, Some(helper)),
            (true, 501, 0, Some(helper)),
            (true, 0, 501, Some(helper)),
            (true, 0, 0, None),
            (true, 0, 0, Some(std::path::Path::new("/tmp/home-preview"))),
        ] {
            assert!(!root_helper_identity(platform, real, effective, path));
        }
        let entry = parsed(app("fixture.app", RootClass::LgSystem));
        let error = read_root_encoded(entry.preferred().unwrap()).unwrap_err();
        assert_eq!(error.reason, ReadReason::PrivilegedProcess);
        assert_eq!(error.operation, "root_helper_identity");
        // This host test never grants Root access or opens the admitted TV path.
    }
    #[test]
    fn ordinary_root_refusal_precedes_any_filesystem_read() {
        if unsafe { libc::geteuid() } == 0 {
            let entry = parsed(app("fixture.app", RootClass::LgSystem));
            let error = read_encoded(entry.preferred().unwrap()).unwrap_err();
            assert_eq!(error.reason, ReadReason::PrivilegedProcess);
            assert_eq!(error.operation, "uid");
        }
    }
}
