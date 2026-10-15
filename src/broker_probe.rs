//! Isolated Proof-only Identify handshake. No icon reads, commands, service
//! elevation, normal-app worker, or production launcher are provided here.
#[cfg(any(target_arch = "arm", test))]
use crate::broker_protocol::{Marker, Response};
#[cfg(any(target_arch = "arm", test))]
use crate::broker_transport::Credentials;
#[cfg(any(target_arch = "arm", test))]
use serde::Serialize;
use serde_json::Value;

#[cfg(any(target_arch = "arm", test))]
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
struct OwnIdentity {
    pid: u32,
    uid: u32,
    start_ticks: u64,
    ui_sha256: String,
}

#[cfg(any(target_arch = "arm", test))]
fn ordinary_proof(app_id: &str, uid: u32) -> bool {
    app_id == crate::identity::PROOF && uid >= 1000 && uid != 5724
}

#[cfg(any(target_arch = "arm", test))]
fn context_valid(context: &Value) -> bool {
    let info = &context["system_info"];
    let foreground = &context["foreground"];
    let windows = &context["windows"];
    info["returnValue"].as_bool() == Some(true)
        && info["modelName"].as_str() == Some("OLED55G26LA")
        && info["firmwareVersion"].as_str() == Some("23.25.55")
        && info["sdkVersion"].as_str() == Some("9.2.2")
        && foreground["returnValue"].as_bool() == Some(true)
        && foreground["appId"].as_str() == Some(crate::identity::PROOF)
        && windows["returnValue"].as_bool() == Some(true)
        && windows["windows"].as_array().is_some_and(|rows| {
            rows.len() == 1
                && rows[0]["appId"].as_str() == Some(crate::identity::PROOF)
                && rows[0]["windowType"].as_str() == Some("_WEBOS_WINDOW_TYPE_CARD")
        })
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
fn bind_response(
    marker: &Marker,
    own: &OwnIdentity,
    after: &OwnIdentity,
    peer: Credentials,
    message: Credentials,
    response: &Response,
) -> Result<(), String> {
    marker.validate().map_err(|error| error.to_string())?;
    response.validate().map_err(|error| error.to_string())?;
    if !ordinary_proof(crate::identity::PROOF, own.uid) || own.pid <= 1 || own.start_ticks == 0 {
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
fn own_identity() -> Result<OwnIdentity, String> {
    use std::fs::File;
    use std::io::Read;
    use std::os::unix::fs::MetadataExt;
    let pid = std::process::id();
    let uid = unsafe { libc::geteuid() };
    if !ordinary_proof(plx_base::paths::app_id(), uid) {
        return Err("Identify is restricted to an ordinary isolated Proof process".into());
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
fn read_marker_file(path: &std::path::Path) -> Result<Marker, String> {
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
fn read_marker() -> Result<Marker, String> {
    read_marker_file(std::path::Path::new(&plx_base::paths::in_app_dir(
        "artwork-broker-proof.json",
    )))
}

#[cfg(target_arch = "arm")]
fn read_context(
    bus: &plx_platform::webos::ls2::Registration,
    deadline: std::time::Instant,
) -> Value {
    use serde_json::json;
    use std::time::{Duration, Instant};
    let mut result = serde_json::Map::new();
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
        let remaining = deadline.saturating_duration_since(Instant::now());
        let value = if remaining.is_zero() {
            json!({"deadline":true})
        } else {
            match bus.call(
                uri,
                &payload.to_string(),
                remaining.min(Duration::from_millis(500)),
            ) {
                Ok(raw) if raw.len() <= 65536 => {
                    serde_json::from_str(&raw).unwrap_or_else(|_| json!({"parse_error":true}))
                }
                Ok(_) => json!({"oversize":true}),
                Err(_) => json!({"unavailable":true}),
            }
        };
        result.insert(key.into(), value);
    }
    Value::Object(result)
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
