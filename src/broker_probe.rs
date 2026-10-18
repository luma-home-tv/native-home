//! Proof Identify handshake with closed installed-profile identity primitives.
//! No icon reads, commands, service
//! elevation, normal-app worker, or production launcher are provided here.
#[cfg(any(target_arch = "arm", test))]
use crate::broker_installation::Profile;
#[cfg(any(target_arch = "arm", test))]
use crate::broker_protocol::{Marker, Response};
#[cfg(any(target_arch = "arm", test))]
use crate::broker_transport::Credentials;
#[cfg(any(target_arch = "arm", test))]
use serde::Serialize;
use serde_json::Value;

#[cfg(any(target_arch = "arm", test))]
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub(crate) struct OwnIdentity {
    pub(crate) pid: u32,
    pub(crate) uid: u32,
    pub(crate) start_ticks: u64,
    pub(crate) ui_sha256: String,
}

#[cfg(any(target_arch = "arm", test))]
pub(crate) fn ordinary_for_profile(app_id: &str, uid: u32, profile: Profile) -> bool {
    app_id == profile.app_id() && uid >= 1000 && (profile != Profile::Proof || uid != 5724)
}

#[cfg(test)]
fn ordinary_proof(app_id: &str, uid: u32) -> bool {
    ordinary_for_profile(app_id, uid, Profile::Proof)
}

#[cfg(any(target_arch = "arm", test))]
pub(crate) fn context_valid(context: &Value) -> bool {
    context_valid_for_profile(context, Profile::Proof)
}

#[cfg(any(target_arch = "arm", test))]
pub(crate) fn context_valid_for_profile(context: &Value, profile: Profile) -> bool {
    crate::broker_protocol::validate_context_for_profile(
        &context["system_info"],
        &context["foreground"],
        &context["windows"],
        profile,
    )
    .is_ok()
}

#[cfg(any(target_arch = "arm", test))]
fn start_ticks(raw: &str, pid: u32) -> Result<u64, String> {
    if pid <= 1 || !raw.starts_with(&format!("{pid} (")) {
        return Err("Self stat PID mismatch".into());
    }
    let end = raw.rfind(')').ok_or("Self stat comm missing")?;
    raw[end + 1..]
        .split_whitespace()
        .nth(19)
        .filter(|field| !field.is_empty() && field.bytes().all(|c| c.is_ascii_digit()))
        .and_then(|field| field.parse::<u64>().ok())
        .filter(|value| *value > 0)
        .ok_or_else(|| "Self stat start identity missing".into())
}

#[cfg(any(target_arch = "arm", test))]
pub(crate) fn bind_response(
    marker: &Marker,
    own: &OwnIdentity,
    after: &OwnIdentity,
    peer: Credentials,
    message: Credentials,
    response: &Response,
) -> Result<(), String> {
    bind_response_for_profile(marker, own, after, peer, message, response, Profile::Proof)
}

/// The profile is selected by installed source policy, never a response field.
#[cfg(any(target_arch = "arm", test))]
pub(crate) fn bind_response_for_profile(
    marker: &Marker,
    own: &OwnIdentity,
    after: &OwnIdentity,
    peer: Credentials,
    message: Credentials,
    response: &Response,
    profile: Profile,
) -> Result<(), String> {
    marker.validate().map_err(|error| error.to_string())?;
    response
        .validate_for_profile(profile)
        .map_err(|error| error.to_string())?;
    if !ordinary_for_profile(profile.app_id(), own.uid, profile)
        || own.pid <= 1
        || own.start_ticks == 0
    {
        return Err("Invalid ordinary client identity".into());
    }
    if peer.uid != 0 || peer.pid <= 1 || message != peer {
        return Err("Root server peer/message credentials differ".into());
    }
    if own != after || own.ui_sha256 != marker.ui_sha256 {
        return Err("Ordinary self identity changed or ELF differs from marker".into());
    }
    if response.nonce != marker.nonce
        || response.ui_sha256 != marker.ui_sha256
        || response.broker_sha256 != marker.broker_sha256
        || response.server_pid != peer.pid as u32
        || response.server_uid != peer.uid
        || response.client_pid != own.pid
        || response.client_uid != own.uid
        || response.client_start_ticks != own.start_ticks
    {
        return Err("Identify response does not bind this client/server session".into());
    }
    Ok(())
}

#[cfg(any(target_arch = "arm", test))]
fn hash_reader(reader: &mut impl std::io::Read) -> Result<String, String> {
    let mut hash = plx_base::sha256::Sha256::new();
    let mut total = 0_usize;
    let mut buffer = [0_u8; 32768];
    loop {
        let permitted = (8 * 1024 * 1024 - total + 1).min(buffer.len());
        let count = reader
            .read(&mut buffer[..permitted])
            .map_err(|error| error.to_string())?;
        if count == 0 {
            break;
        }
        total = total.checked_add(count).ok_or("Self ELF size overflow")?;
        if total > 8 * 1024 * 1024 {
            return Err("Self ELF exceeds8MiB".into());
        }
        hash.update(&buffer[..count]);
    }
    if total == 0 {
        return Err("Empty self ELF".into());
    }
    Ok(hash
        .finish()
        .iter()
        .map(|byte| format!("{byte:02x}"))
        .collect())
}

#[cfg(target_arch = "arm")]
pub(crate) fn own_identity() -> Result<OwnIdentity, String> {
    own_identity_for_profile(Profile::Proof)
}

#[cfg(target_arch = "arm")]
pub(crate) fn own_identity_for_profile(profile: Profile) -> Result<OwnIdentity, String> {
    use std::fs::File;
    use std::io::Read;
    use std::os::unix::fs::MetadataExt;
    let pid = std::process::id();
    let uid = unsafe { libc::geteuid() };
    if !ordinary_for_profile(plx_base::paths::app_id(), uid, profile) {
        return Err("Identify requires the selected ordinary installed profile".into());
    }
    let stat = || -> Result<u64, String> {
        let file = File::open("/proc/self/stat").map_err(|error| error.to_string())?;
        let mut raw = String::new();
        file.take(4097)
            .read_to_string(&mut raw)
            .map_err(|error| error.to_string())?;
        if raw.len() > 4096 {
            return Err("Oversize self stat".into());
        }
        start_ticks(&raw, pid)
    };
    let start = stat()?;
    // /proc/self/exe intentionally follows the kernel's own executable link.
    // No caller-selected path, root filesystem traversal or fallback is used.
    let mut exe = File::open("/proc/self/exe").map_err(|error| error.to_string())?;
    let before = exe.metadata().map_err(|error| error.to_string())?;
    if !before.is_file() || before.len() == 0 || before.len() > 8 * 1024 * 1024 {
        return Err("Self executable is not a bounded regular file".into());
    }
    let ui_sha256 = hash_reader(&mut exe)?;
    let after = exe.metadata().map_err(|error| error.to_string())?;
    let identity = |m: &std::fs::Metadata| {
        (
            m.dev(),
            m.ino(),
            m.len(),
            m.mtime(),
            m.mtime_nsec(),
            m.ctime(),
            m.ctime_nsec(),
        )
    };
    if identity(&before) != identity(&after)
        || stat()? != start
        || unsafe { libc::geteuid() } != uid
        || !ordinary_for_profile(plx_base::paths::app_id(), uid, profile)
    {
        return Err("Self identity changed during executable measurement".into());
    }
    Ok(OwnIdentity {
        pid,
        uid,
        start_ticks: start,
        ui_sha256,
    })
}

#[cfg(any(target_arch = "arm", test))]
pub(crate) fn read_marker_file(path: &std::path::Path) -> Result<Marker, String> {
    use std::fs::OpenOptions;
    use std::io::Read;
    use std::os::unix::fs::{MetadataExt, OpenOptionsExt};
    let identity = |m: &std::fs::Metadata| {
        (
            m.dev(),
            m.ino(),
            m.len(),
            m.uid(),
            m.gid(),
            m.mode(),
            m.nlink(),
            m.mtime(),
            m.mtime_nsec(),
            m.ctime(),
            m.ctime_nsec(),
        )
    };
    let file = OpenOptions::new()
        .read(true)
        .custom_flags(libc::O_NOFOLLOW | libc::O_CLOEXEC | libc::O_NONBLOCK)
        .open(path)
        .map_err(|error| error.to_string())?;
    let metadata = file.metadata().map_err(|error| error.to_string())?;
    if !metadata.is_file()
        || metadata.len() == 0
        || metadata.len() > 4096
        || metadata.uid() != 0
        || metadata.gid() != 0
        || ![0o444, 0o644].contains(&(metadata.mode() & 0o7777))
        || metadata.nlink() != 1
    {
        return Err("Invalid broker proof marker file".into());
    }
    let mut raw = Vec::new();
    (&file)
        .take(4097)
        .read_to_end(&mut raw)
        .map_err(|error| error.to_string())?;
    if identity(&metadata) != identity(&file.metadata().map_err(|e| e.to_string())?)
        || identity(&metadata)
            != identity(&std::fs::symlink_metadata(path).map_err(|e| e.to_string())?)
        || raw.len() as u64 != metadata.len()
    {
        return Err("Broker proof marker changed during read".into());
    }
    crate::broker_protocol::parse_marker(&raw).map_err(|error| error.to_string())
}

#[cfg(target_arch = "arm")]
pub(crate) fn read_marker() -> Result<Marker, String> {
    read_marker_file(std::path::Path::new(&plx_base::paths::in_app_dir(
        "artwork-broker-proof.json",
    )))
}

// These diagnostics describe SDK-delivered UTF-8 text, not original wire bytes.
// They never participate in context or permission admission.
#[cfg(any(target_arch = "arm", test))]
#[derive(Clone, Debug, Serialize)]
pub(crate) struct ContextTrace {
    pub(crate) operations: Vec<ContextCallTrace>,
    pub(crate) trace_truncated: bool,
}
#[cfg(any(target_arch = "arm", test))]
#[derive(Clone, Debug, Serialize)]
pub(crate) struct ContextCallTrace {
    operation: &'static str,
    requested_budget_ns: u64,
    elapsed_ns: u64,
    #[serde(flatten)]
    result: ContextCallOutcome,
}
#[cfg(any(target_arch = "arm", test))]
#[derive(Clone, Debug, Serialize)]
#[serde(tag = "outcome", rename_all = "snake_case")]
enum ContextCallOutcome {
    Returned {
        projection: ContextProjection,
        sdk_text_bytes: usize,
    },
    Timeout,
    Setup {
        stage: String,
        code: Option<i32>,
        detail: String,
    },
    DeadlineNotCalled,
}
#[cfg(any(target_arch = "arm", test))]
#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "snake_case")]
enum ContextProjection {
    Json,
    ParseError,
    Oversize,
}
#[cfg(any(target_arch = "arm", test))]
enum ContextCallFailure {
    Timeout,
    Setup {
        stage: String,
        code: Option<i32>,
        detail: String,
    },
}
#[cfg(any(target_arch = "arm", test))]
trait ContextBackend {
    fn now(&self) -> std::time::Instant;
    fn call(
        &mut self,
        uri: &str,
        payload: &str,
        budget: std::time::Duration,
    ) -> Result<String, ContextCallFailure>;
}
#[cfg(any(target_arch = "arm", test))]
fn diagnostic_text(text: &str, cap: usize) -> (String, bool) {
    let mut end = text.len().min(cap);
    while !text.is_char_boundary(end) {
        end -= 1;
    }
    (text[..end].to_owned(), text.len() > cap)
}
#[cfg(any(target_arch = "arm", test))]
fn project_context_reply(
    reply: Result<String, ContextCallFailure>,
) -> (Value, ContextCallOutcome, bool) {
    use serde_json::json;
    match reply {
        Ok(raw) => {
            let sdk_text_bytes = raw.len();
            let (value, projection) = if sdk_text_bytes > 65536 {
                (json!({"oversize":true}), ContextProjection::Oversize)
            } else {
                match serde_json::from_str(&raw) {
                    Ok(value) => (value, ContextProjection::Json),
                    Err(_) => (json!({"parse_error":true}), ContextProjection::ParseError),
                }
            };
            (
                value,
                ContextCallOutcome::Returned {
                    projection,
                    sdk_text_bytes,
                },
                false,
            )
        }
        Err(ContextCallFailure::Timeout) => (
            json!({"unavailable":true}),
            ContextCallOutcome::Timeout,
            false,
        ),
        Err(ContextCallFailure::Setup {
            stage,
            code,
            detail,
        }) => {
            let (stage, stage_cut) = diagnostic_text(&stage, 64);
            let (detail, detail_cut) = diagnostic_text(&detail, 256);
            (
                json!({"unavailable":true}),
                ContextCallOutcome::Setup {
                    stage,
                    code,
                    detail,
                },
                stage_cut || detail_cut,
            )
        }
    }
}
#[cfg(any(target_arch = "arm", test))]
fn read_context_with(
    backend: &mut impl ContextBackend,
    deadline: std::time::Instant,
    observed: bool,
) -> (Value, ContextTrace) {
    use serde_json::json;
    use std::time::Duration;
    let mut result = serde_json::Map::new();
    let mut trace = ContextTrace {
        operations: Vec::with_capacity(if observed { 3 } else { 0 }),
        trace_truncated: false,
    };
    for (key, uri, payload) in [
        (
            "system_info",
            "luna://com.webos.service.tv.systemproperty/getSystemInfo",
            json!({"keys":["modelName","firmwareVersion","sdkVersion"]}),
        ),
        (
            "foreground",
            "luna://com.webos.applicationManager/getForegroundAppInfo",
            json!({}),
        ),
        (
            "windows",
            "luna://com.webos.surfacemanager/getForegroundWindowInfo",
            json!({}),
        ),
    ] {
        let began = backend.now();
        let remaining = deadline.saturating_duration_since(began);
        let budget = remaining.min(Duration::from_millis(500));
        let (value, outcome, truncated) = if remaining.is_zero() {
            (
                json!({"deadline":true}),
                ContextCallOutcome::DeadlineNotCalled,
                false,
            )
        } else {
            project_context_reply(backend.call(uri, &payload.to_string(), budget))
        };
        if observed {
            let elapsed = backend.now().saturating_duration_since(began).as_nanos();
            trace.trace_truncated |= truncated || elapsed > u64::MAX as u128;
            trace.operations.push(ContextCallTrace {
                operation: key,
                requested_budget_ns: budget.as_nanos() as u64,
                elapsed_ns: elapsed.min(u64::MAX as u128) as u64,
                result: outcome,
            });
        }
        result.insert(key.into(), value);
    }
    (Value::Object(result), trace)
}
#[cfg(target_arch = "arm")]
struct NativeContext<'a>(&'a plx_platform::webos::ls2::Registration);
#[cfg(target_arch = "arm")]
impl ContextBackend for NativeContext<'_> {
    fn now(&self) -> std::time::Instant {
        std::time::Instant::now()
    }
    fn call(
        &mut self,
        uri: &str,
        payload: &str,
        budget: std::time::Duration,
    ) -> Result<String, ContextCallFailure> {
        self.0
            .call(uri, payload, budget)
            .map_err(|error| match error {
                plx_platform::webos::ls2::Fail::Timeout => ContextCallFailure::Timeout,
                plx_platform::webos::ls2::Fail::Setup {
                    stage,
                    code,
                    detail,
                } => ContextCallFailure::Setup {
                    stage: stage.into(),
                    code,
                    detail,
                },
            })
    }
}
#[cfg(target_arch = "arm")]
pub(crate) fn read_context(
    bus: &plx_platform::webos::ls2::Registration,
    deadline: std::time::Instant,
) -> Value {
    read_context_with(&mut NativeContext(bus), deadline, false).0
}
#[cfg(target_arch = "arm")]
pub(crate) fn read_context_observed(
    bus: &plx_platform::webos::ls2::Registration,
    deadline: std::time::Instant,
) -> (Value, ContextTrace) {
    read_context_with(&mut NativeContext(bus), deadline, true)
}

/// Called only from the marker-gated isolated native collector. This neither
/// launches the broker nor grants access to artwork/resources after Identify.
#[cfg(target_arch = "arm")]
pub fn collect() -> Value {
    use crate::broker_protocol::{Operation, Request};
    use crate::broker_transport::Connection;
    use serde_json::json;
    use std::time::{Duration, Instant};
    let origin = Instant::now();
    let mut report = json!({"schema":1,"app_id":plx_base::paths::app_id(),"uid":unsafe { libc::geteuid() },"handshake_only":true,"icon_reads":false,"decoder":false,
        "normal_loader_qualified":false,"root_filesystem_access_granted":false,"server_executable_independently_measured_by_client":false,"request_attempted":false,"request_sent":false,"verified":false});
    let mut stage = "identity";
    let result = (|| -> Result<(), String> {
        let own = own_identity()?;
        report["own_before"] = json!(own);
        stage = "marker";
        let marker = read_marker()?;
        if marker.ui_sha256 != own.ui_sha256 {
            return Err("Marker UI ELF identity mismatch".into());
        }
        report["marker"] = json!({"schema":marker.schema,"nonce":marker.nonce,"ui_sha256":marker.ui_sha256,"broker_sha256":marker.broker_sha256});
        stage = "anonymous_bus_registration";
        let bus = plx_platform::webos::ls2::register().map_err(|error| error.to_string())?;
        stage = "initial_foreground";
        let gate_deadline = Instant::now() + Duration::from_secs(5);
        let mut attempts = 0;
        loop {
            attempts += 1;
            let context = read_context(&bus, gate_deadline);
            let valid = context_valid(&context);
            report["initial_context"] = context;
            report["foreground_checks"] = json!(attempts);
            if valid {
                break;
            }
            let remaining = gate_deadline.saturating_duration_since(Instant::now());
            if remaining.is_zero() {
                return Err("Proof sole-CARD foreground/model gate unavailable within5s".into());
            }
            std::thread::sleep(remaining.min(Duration::from_millis(100)));
        }
        stage = "connect";
        let connection =
            Connection::connect_until(&marker.nonce, Instant::now() + Duration::from_secs(15))
                .map_err(|error| error.to_string())?;
        let peer = connection
            .peer_credentials()
            .map_err(|error| error.to_string())?;
        report["server_peer"] = json!({"pid":peer.pid,"uid":peer.uid,"gid":peer.gid});
        if peer.uid != 0 || peer.pid <= 1 {
            return Err("Connected peer is not a root server".into());
        }
        stage = "pre_send_context";
        let before = read_context(&bus, Instant::now() + Duration::from_secs(2));
        let valid = context_valid(&before);
        report["context_before"] = before;
        if !valid || own_identity()? != own {
            return Err("Proof context/identity changed before Identify".into());
        }
        stage = "identify";
        let exchange = (|| -> Result<(Response, Credentials), String> {
            let request = Request {
                schema: 1,
                op: Operation::Identify,
                nonce: marker.nonce.clone(),
            };
            let bytes = serde_json::to_vec(&request).map_err(|error| error.to_string())?;
            let deadline = Instant::now() + Duration::from_secs(3);
            report["request_attempted"] = json!(true);
            connection
                .send_until(&bytes, deadline)
                .map_err(|error| error.to_string())?;
            report["request_sent"] = json!(true);
            let packet = connection
                .recv_until(4096, deadline)
                .map_err(|error| error.to_string())?;
            report["response_bytes"] = json!(packet.bytes);
            report["message_credentials"] = json!({"pid":packet.credentials.pid,"uid":packet.credentials.uid,"gid":packet.credentials.gid});
            let response = crate::broker_protocol::parse_response(&packet.bytes)
                .map_err(|error| error.to_string())?;
            report["response"] =
                serde_json::from_slice(&packet.bytes).map_err(|error| error.to_string())?;
            Ok((response, packet.credentials))
        })();
        if let Err(error) = &exchange {
            report["exchange_error"] = json!(error);
        }
        // Read back even if send/receive/parse failed; snapshots do not imply
        // continuous foreground ownership or server-side authorization.
        let after_context = read_context(&bus, Instant::now() + Duration::from_secs(2));
        let context_stable = context_valid(&after_context);
        report["context_after"] = after_context;
        let after = own_identity()?;
        report["own_after"] = json!(after);
        let (response, message) = exchange?;
        stage = "readback_binding";
        if !context_stable {
            return Err("Proof foreground/model changed after Identify".into());
        }
        bind_response(&marker, &own, &after, peer, message, &response)?;
        report["verified"] = json!(true);
        Ok(())
    })();
    report["elapsed_ms"] = json!(origin.elapsed().as_millis());
    report["euid_after"] = json!(unsafe { libc::geteuid() });
    match result {
        Ok(()) => report["outcome"] = json!("identified"),
        Err(error) => {
            report["outcome"] = json!("unqualified");
            report["error"] = json!(error);
            report["stage"] = json!(stage);
        }
    }
    report
}

#[cfg(not(target_arch = "arm"))]
pub fn collect() -> Value {
    serde_json::json!({"schema":1,"unsupported_host":true,"handshake_only":true,"request_attempted":false,"verified":false})
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::broker_protocol::Operation;
    use serde_json::json;
    use std::io::Read;
    #[test]
    fn malformed_marker_file_types_cannot_block_or_follow_links() {
        use std::os::unix::fs::symlink;
        let dir =
            std::env::temp_dir().join(format!("luma-broker-marker-test-{}", std::process::id()));
        std::fs::create_dir(&dir).unwrap();
        let fifo = dir.join("marker-fifo");
        let name = std::ffi::CString::new(fifo.to_str().unwrap()).unwrap();
        assert_eq!(unsafe { libc::mkfifo(name.as_ptr(), 0o600) }, 0);
        assert!(read_marker_file(&fifo).is_err());
        assert!(read_marker_file(&dir).is_err());
        let linked = dir.join("marker-link");
        symlink(&fifo, &linked).unwrap();
        assert!(read_marker_file(&linked).is_err());
        std::fs::remove_file(linked).unwrap();
        std::fs::remove_file(fifo).unwrap();
        std::fs::remove_dir(dir).unwrap();
    }
    fn context() -> Value {
        json!({"system_info":{"returnValue":true,"modelName":"OLED55G26LA","firmwareVersion":"23.25.55","sdkVersion":"9.2.2"},
            "foreground":{"returnValue":true,"appId":crate::identity::PROOF},
            "windows":{"returnValue":true,"windows":[{"appId":crate::identity::PROOF,"windowType":"_WEBOS_WINDOW_TYPE_CARD"}]}})
    }
    fn identities() -> (Marker, OwnIdentity, Credentials, Response) {
        let marker = Marker {
            schema: 1,
            nonce: "a".repeat(32),
            ui_sha256: "b".repeat(64),
            broker_sha256: "c".repeat(64),
        };
        let own = OwnIdentity {
            pid: 101,
            uid: 6001,
            start_ticks: 12345,
            ui_sha256: marker.ui_sha256.clone(),
        };
        let peer = Credentials {
            pid: 202,
            uid: 0,
            gid: 0,
        };
        let response = Response {
            schema: 1,
            op: Operation::Identify,
            nonce: marker.nonce.clone(),
            server_pid: 202,
            server_uid: 0,
            client_pid: 101,
            client_uid: 6001,
            client_start_ticks: 12345,
            ui_sha256: marker.ui_sha256.clone(),
            broker_sha256: marker.broker_sha256.clone(),
        };
        (marker, own, peer, response)
    }
    #[test]
    fn only_ordinary_proof_can_enter() {
        assert!(ordinary_proof(crate::identity::PROOF, 6001));
        for uid in [0, 999, 5724] {
            assert!(!ordinary_proof(crate::identity::PROOF, uid));
        }
        assert!(!ordinary_proof(crate::identity::PREVIEW, 6001));
        assert!(!ordinary_proof("com.ash.nativehome.proof.other", 6001));
    }
    #[test]
    fn exact_context_requires_sole_proof_card_and_firmware() {
        assert!(context_valid(&context()));
        for pointer in [
            "/system_info/modelName",
            "/system_info/firmwareVersion",
            "/system_info/sdkVersion",
            "/foreground/appId",
            "/windows/windows/0/appId",
            "/windows/windows/0/windowType",
        ] {
            let mut bad = context();
            *bad.pointer_mut(pointer).unwrap() = json!("different");
            assert!(!context_valid(&bad));
        }
        for pointer in [
            "/system_info/returnValue",
            "/foreground/returnValue",
            "/windows/returnValue",
        ] {
            let mut bad = context();
            *bad.pointer_mut(pointer).unwrap() = json!(false);
            assert!(!context_valid(&bad));
        }
        let mut bad = context();
        bad["windows"]["windows"]
            .as_array_mut()
            .unwrap()
            .push(json!({"appId":"other","windowType":"_WEBOS_WINDOW_TYPE_OVERLAY"}));
        assert!(!context_valid(&bad));
        assert!(!context_valid(&Value::Null));
    }
    #[test]
    fn every_response_identity_and_peer_component_is_bound() {
        let (marker, own, peer, response) = identities();
        assert!(bind_response(&marker, &own, &own, peer, peer, &response).is_ok());
        for bad in [
            Credentials { uid: 1000, ..peer },
            Credentials { pid: 1, ..peer },
        ] {
            assert!(bind_response(&marker, &own, &own, bad, bad, &response).is_err());
        }
        for bad in [
            Credentials { uid: 1, ..peer },
            Credentials { gid: 1, ..peer },
            Credentials { pid: 203, ..peer },
        ] {
            assert!(bind_response(&marker, &own, &own, peer, bad, &response).is_err());
        }
        for changed in [
            OwnIdentity {
                start_ticks: own.start_ticks + 1,
                ..own.clone()
            },
            OwnIdentity {
                pid: own.pid + 1,
                ..own.clone()
            },
            OwnIdentity {
                uid: own.uid + 1,
                ..own.clone()
            },
            OwnIdentity {
                ui_sha256: "d".repeat(64),
                ..own.clone()
            },
        ] {
            assert!(bind_response(&marker, &own, &changed, peer, peer, &response).is_err());
        }
        for field in [
            "nonce",
            "ui_sha256",
            "broker_sha256",
            "server_pid",
            "server_uid",
            "client_pid",
            "client_uid",
            "client_start_ticks",
        ] {
            let mut raw = serde_json::to_value(&response).unwrap();
            raw[field] = if raw[field].is_string() {
                json!("d".repeat(if field == "nonce" { 32 } else { 64 }))
            } else {
                json!(999)
            };
            if let Ok(bad) =
                crate::broker_protocol::parse_response(&serde_json::to_vec(&raw).unwrap())
            {
                assert!(bind_response(&marker, &own, &own, peer, peer, &bad).is_err());
            }
        }
    }
    #[test]
    fn preview_binding_requires_explicit_profile_and_keeps_kernel_identity_checks() {
        for uid in [5724, 7001] {
            let (marker, mut own, peer, mut response) = identities();
            own.uid = uid;
            response.client_uid = uid;
            assert!(ordinary_for_profile(
                crate::identity::PREVIEW,
                uid,
                Profile::Preview
            ));
            assert!(!ordinary_for_profile(
                crate::identity::PROOF,
                uid,
                Profile::Preview
            ));
            assert!(bind_response_for_profile(
                &marker,
                &own,
                &own,
                peer,
                peer,
                &response,
                Profile::Preview
            )
            .is_ok());
            if uid == 5724 {
                assert!(bind_response(&marker, &own, &own, peer, peer, &response).is_err());
            }
            let stale = OwnIdentity {
                start_ticks: own.start_ticks + 1,
                ..own.clone()
            };
            assert!(bind_response_for_profile(
                &marker,
                &own,
                &stale,
                peer,
                peer,
                &response,
                Profile::Preview
            )
            .is_err());
            let other_message = Credentials {
                gid: peer.gid + 1,
                ..peer
            };
            assert!(bind_response_for_profile(
                &marker,
                &own,
                &own,
                peer,
                other_message,
                &response,
                Profile::Preview
            )
            .is_err());
        }
        for uid in [0, 999] {
            assert!(!ordinary_for_profile(
                crate::identity::PREVIEW,
                uid,
                Profile::Preview
            ));
        }
    }
    #[test]
    fn profile_context_delegates_exact_protocol_policy_without_cross_role_cards() {
        let mut preview = context();
        preview["foreground"]["appId"] = json!(crate::identity::PREVIEW);
        preview["windows"]["windows"][0]["appId"] = json!(crate::identity::PREVIEW);
        preview["windows"]["windows"][0]["harmless_metadata"] = json!(123);
        assert!(context_valid_for_profile(&preview, Profile::Preview));
        assert!(!context_valid(&preview));
        assert!(!context_valid_for_profile(&context(), Profile::Preview));
        preview["windows"]["windows"][0]["appId"] = json!(crate::identity::PROOF);
        assert!(!context_valid_for_profile(&preview, Profile::Preview));
    }
    #[test]
    fn stat_start_uses_field22_after_parenthesized_comm() {
        let mut fields = vec!["0"; 20];
        fields[0] = "S";
        fields[19] = "12345";
        let raw = format!("101 (odd ) name) {}", fields.join(" "));
        assert_eq!(start_ticks(&raw, 101).unwrap(), 12345);
        assert!(start_ticks(&raw, 102).is_err());
        assert!(start_ticks("101 (x) S", 101).is_err());
    }
    #[test]
    fn streaming_hash_is_exact_and_bounded_before_admission() {
        let actual = hash_reader(&mut std::io::Cursor::new(b"abc")).unwrap();
        assert_eq!(
            actual,
            "ba7816bf8f01cfea414140de5dae2223b00361a396177a9cb410ff61f20015ad"
        );
        assert!(hash_reader(&mut std::io::Cursor::new(Vec::<u8>::new())).is_err());
        assert!(hash_reader(&mut std::io::repeat(1).take(8 * 1024 * 1024 + 1)).is_err());
    }
}

#[cfg(test)]
mod context_diagnostic_tests {
    use super::*;
    use serde_json::json;
    use std::collections::VecDeque;
    use std::time::{Duration, Instant};

    struct FakeContext {
        origin: Instant,
        elapsed: Duration,
        replies: VecDeque<(Result<String, ContextCallFailure>, Duration)>,
        calls: Vec<(String, String, Duration)>,
    }
    impl FakeContext {
        fn new(replies: Vec<(Result<String, ContextCallFailure>, Duration)>) -> Self {
            Self {
                origin: Instant::now(),
                elapsed: Duration::ZERO,
                replies: replies.into(),
                calls: Vec::new(),
            }
        }
    }
    impl ContextBackend for FakeContext {
        fn now(&self) -> Instant {
            self.origin + self.elapsed
        }
        fn call(
            &mut self,
            uri: &str,
            payload: &str,
            budget: Duration,
        ) -> Result<String, ContextCallFailure> {
            self.calls.push((uri.into(), payload.into(), budget));
            let (reply, cost) = self.replies.pop_front().expect("Unexpected extra call");
            self.elapsed += cost;
            reply
        }
    }
    #[test]
    fn same_fixed_calls_order_payloads_and_remaining_budget_with_or_without_trace() {
        for observed in [false, true] {
            let mut fake = FakeContext::new(vec![
                (
                    Ok("{\"returnValue\":true}".into()),
                    Duration::from_millis(200),
                ),
                (
                    Ok("{\"unavailable\":true}".into()),
                    Duration::from_millis(200),
                ),
                (Ok("[]".into()), Duration::from_millis(10)),
            ]);
            let deadline = fake.origin + Duration::from_millis(600);
            let (value, trace) = read_context_with(&mut fake, deadline, observed);
            assert_eq!(
                value,
                json!({"system_info":{"returnValue":true},
                "foreground":{"unavailable":true},"windows":[]})
            );
            assert_eq!(
                fake.calls,
                vec![
                    (
                        "luna://com.webos.service.tv.systemproperty/getSystemInfo".into(),
                        "{\"keys\":[\"modelName\",\"firmwareVersion\",\"sdkVersion\"]}".into(),
                        Duration::from_millis(500)
                    ),
                    (
                        "luna://com.webos.applicationManager/getForegroundAppInfo".into(),
                        "{}".into(),
                        Duration::from_millis(400)
                    ),
                    (
                        "luna://com.webos.surfacemanager/getForegroundWindowInfo".into(),
                        "{}".into(),
                        Duration::from_millis(200)
                    ),
                ]
            );
            assert_eq!(trace.operations.len(), if observed { 3 } else { 0 });
            if observed {
                let trace = serde_json::to_value(trace).unwrap();
                assert_eq!(trace["operations"][1]["operation"], "foreground");
                assert_eq!(trace["operations"][1]["outcome"], "returned");
                assert_eq!(trace["operations"][1]["projection"], "json");
                assert_eq!(
                    trace["operations"][1]["requested_budget_ns"],
                    400_000_000u64
                );
                assert_eq!(trace["operations"][1]["elapsed_ns"], 200_000_000u64);
            }
        }
    }
    #[test]
    fn timeout_setup_and_returned_sentinel_have_same_projection_but_distinct_trace() {
        let mut fake = FakeContext::new(vec![
            (Ok("{\"unavailable\":true}".into()), Duration::ZERO),
            (Err(ContextCallFailure::Timeout), Duration::from_millis(500)),
            (
                Err(ContextCallFailure::Setup {
                    stage: "LSCallOneReply".into(),
                    code: Some(-7),
                    detail: "SDK detail".into(),
                }),
                Duration::ZERO,
            ),
        ]);
        let deadline = fake.origin + Duration::from_secs(8);
        let (value, trace) = read_context_with(&mut fake, deadline, true);
        for key in ["system_info", "foreground", "windows"] {
            assert_eq!(value[key], json!({"unavailable":true}));
        }
        let trace = serde_json::to_value(trace).unwrap();
        assert_eq!(trace["operations"][0]["outcome"], "returned");
        assert_eq!(trace["operations"][1]["outcome"], "timeout");
        assert_eq!(trace["operations"][2]["outcome"], "setup");
        assert_eq!(trace["operations"][2]["stage"], "LSCallOneReply");
        assert_eq!(trace["operations"][2]["code"], -7);
        assert_eq!(trace["operations"][2]["detail"], "SDK detail");
        assert_eq!(trace["trace_truncated"], false);
    }
    #[test]
    fn reply_projection_limits_and_parsing_remain_exact() {
        let oversized = " ".repeat(65537);
        let mut fake = FakeContext::new(vec![
            (Ok("[".into()), Duration::ZERO),
            (Ok(oversized), Duration::ZERO),
            (Ok(format!("{}null", " ".repeat(65532))), Duration::ZERO),
        ]);
        let deadline = fake.origin + Duration::from_secs(8);
        let (value, trace) = read_context_with(&mut fake, deadline, true);
        assert_eq!(
            value,
            json!({"system_info":{"parse_error":true},
            "foreground":{"oversize":true},"windows":null})
        );
        let trace = serde_json::to_value(trace).unwrap();
        assert_eq!(trace["operations"][0]["projection"], "parse_error");
        assert_eq!(trace["operations"][1]["projection"], "oversize");
        assert_eq!(trace["operations"][1]["sdk_text_bytes"], 65537);
        assert_eq!(trace["operations"][2]["projection"], "json");
        assert_eq!(trace["operations"][2]["sdk_text_bytes"], 65536);
    }
    #[test]
    fn deadline_does_not_call_or_retry_and_records_skipped_operations() {
        let mut fake = FakeContext::new(vec![(
            Err(ContextCallFailure::Timeout),
            Duration::from_millis(500),
        )]);
        let deadline = fake.origin + Duration::from_millis(500);
        let (value, trace) = read_context_with(&mut fake, deadline, true);
        assert_eq!(fake.calls.len(), 1);
        assert_eq!(
            value,
            json!({"system_info":{"unavailable":true},
            "foreground":{"deadline":true},"windows":{"deadline":true}})
        );
        let trace = serde_json::to_value(trace).unwrap();
        for i in [1, 2] {
            assert_eq!(trace["operations"][i]["outcome"], "deadline_not_called");
            assert_eq!(trace["operations"][i]["requested_budget_ns"], 0);
        }
    }
    #[test]
    fn error_detail_limits_are_utf8_safe_bytes_with_explicit_truncation() {
        let (value, outcome, truncated) = project_context_reply(Err(ContextCallFailure::Setup {
            stage: format!("{}€tail", "s".repeat(63)),
            code: None,
            detail: format!("{}€tail", "d".repeat(255)),
        }));
        assert_eq!(value, json!({"unavailable":true}));
        assert!(truncated);
        let result = serde_json::to_value(outcome).unwrap();
        assert_eq!(result["stage"].as_str().unwrap().len(), 63);
        assert_eq!(result["detail"].as_str().unwrap().len(), 255);
        assert!(result["code"].is_null());
        let (_, _, truncated) = project_context_reply(Err(ContextCallFailure::Setup {
            stage: "é".repeat(32),
            code: None,
            detail: "é".repeat(128),
        }));
        assert!(!truncated);
    }
}
