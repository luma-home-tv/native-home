//! Identify-only broker protocol and pure admission policy. No I/O or icon operation.
//!
//! Kernel credentials, process evidence and service replies must be collected by
//! the server itself. Parsing this data does not establish its provenance.
use crate::broker_transport::Credentials;
use serde::{Deserialize, Serialize};
use serde_json::Value;

pub const MARKER_LIMIT: usize = 4096;
pub const ENROLLMENT_LIMIT: usize = 4096;
pub const REQUEST_LIMIT: usize = 1024;
pub const RESPONSE_LIMIT: usize = 4096;
pub const PROOF_JAIL_ROOT: &str = "/var/palm/jail/com.ash.nativehome.proof";
pub const PROOF_EXE_PATH: &str = "/var/palm/jail/com.ash.nativehome.proof/media/developer/apps/usr/palm/applications/com.ash.nativehome.proof/home-preview";

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Error {
    TooLarge,
    InvalidJson,
    Schema,
    Nonce,
    Hash,
    Client,
    Server,
    Expiry,
    Credentials,
    PeerIdentity,
    PeerPath,
    Context,
}
impl std::fmt::Display for Error {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(match self {
            Self::TooLarge => "broker message exceeds its size limit",
            Self::InvalidJson => "broker message violates its strict JSON schema",
            Self::Schema => "broker schema version is unsupported",
            Self::Nonce => "broker nonce must be 32 lowercase hexadecimal characters",
            Self::Hash => "broker executable hashes must be 64 lowercase hexadecimal characters",
            Self::Client => "broker client must be an ordinary isolated Proof process",
            Self::Server => "broker server must be a root process with a valid PID",
            Self::Expiry => "broker enrollment is expired or exceeds its 90-second lifetime",
            Self::Credentials => "broker connect and message credentials do not match enrollment",
            Self::PeerIdentity => "broker process evidence does not match enrollment",
            Self::PeerPath => "broker client executable or jail root is not the exact Proof path",
            Self::Context => "broker requires the exact G2 platform and sole foreground Proof card",
        })
    }
}
impl std::error::Error for Error {}

fn hex(value: &str, length: usize) -> bool {
    value.len() == length
        && value
            .bytes()
            .all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte))
}
fn identity(schema: u32, nonce: &str, ui: &str, broker: &str) -> Result<(), Error> {
    if schema != 1 {
        return Err(Error::Schema);
    }
    if !hex(nonce, 32) {
        return Err(Error::Nonce);
    }
    if !hex(ui, 64) || !hex(broker, 64) {
        return Err(Error::Hash);
    }
    Ok(())
}
fn client(pid: u32, uid: u32) -> Result<(), Error> {
    if pid <= 1 || uid < 1000 || uid == 5724 {
        return Err(Error::Client);
    }
    Ok(())
}
fn parse<T: serde::de::DeserializeOwned>(bytes: &[u8], limit: usize) -> Result<T, Error> {
    if bytes.len() > limit {
        return Err(Error::TooLarge);
    }
    serde_json::from_slice(bytes).map_err(|_| Error::InvalidJson)
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Marker {
    pub schema: u32,
    pub nonce: String,
    pub ui_sha256: String,
    pub broker_sha256: String,
}
impl Marker {
    pub fn validate(&self) -> Result<(), Error> {
        identity(
            self.schema,
            &self.nonce,
            &self.ui_sha256,
            &self.broker_sha256,
        )
    }
}
pub fn parse_marker(bytes: &[u8]) -> Result<Marker, Error> {
    let value: Marker = parse(bytes, MARKER_LIMIT)?;
    value.validate()?;
    Ok(value)
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Enrollment {
    pub schema: u32,
    pub nonce: String,
    pub client_pid: u32,
    pub client_uid: u32,
    pub client_start_ticks: u64,
    pub ui_sha256: String,
    pub broker_sha256: String,
    pub expires_unix: u64,
}
impl Enrollment {
    fn validate_identity(&self) -> Result<(), Error> {
        identity(
            self.schema,
            &self.nonce,
            &self.ui_sha256,
            &self.broker_sha256,
        )?;
        client(self.client_pid, self.client_uid)
    }
    /// Revalidate with a fresh clock before accept AND every RPC. Identity-only
    /// authorization below deliberately does not supply or read a clock.
    pub fn validate(&self, now: u64) -> Result<(), Error> {
        self.validate_identity()?;
        let latest = now.checked_add(90).ok_or(Error::Expiry)?;
        if self.expires_unix <= now || self.expires_unix > latest {
            return Err(Error::Expiry);
        }
        Ok(())
    }
}
pub fn parse_enrollment(bytes: &[u8], now: u64) -> Result<Enrollment, Error> {
    let value: Enrollment = parse(bytes, ENROLLMENT_LIMIT)?;
    value.validate(now)?;
    Ok(value)
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum Operation {
    Identify,
}
impl<'de> Deserialize<'de> for Operation {
    fn deserialize<D: serde::Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        // Derived externally-tagged enums can also accept a map representation.
        // The wire contract admits only the literal string "identify".
        let operation = String::deserialize(deserializer)?;
        if operation == "identify" {
            Ok(Self::Identify)
        } else {
            Err(serde::de::Error::unknown_variant(&operation, &["identify"]))
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Request {
    pub schema: u32,
    pub op: Operation,
    pub nonce: String,
}
impl Request {
    pub fn validate(&self) -> Result<(), Error> {
        if self.schema != 1 {
            return Err(Error::Schema);
        }
        if !hex(&self.nonce, 32) {
            return Err(Error::Nonce);
        }
        Ok(())
    }
}
pub fn parse_request(bytes: &[u8]) -> Result<Request, Error> {
    let value: Request = parse(bytes, REQUEST_LIMIT)?;
    value.validate()?;
    Ok(value)
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Response {
    pub schema: u32,
    pub op: Operation,
    pub nonce: String,
    pub server_pid: u32,
    pub server_uid: u32,
    pub client_pid: u32,
    pub client_uid: u32,
    pub client_start_ticks: u64,
    pub ui_sha256: String,
    pub broker_sha256: String,
}
impl Response {
    /// Structural validation only. The client must also compare every binding
    /// with its marker, own process identity and independently received kernel credentials.
    pub fn validate(&self) -> Result<(), Error> {
        identity(
            self.schema,
            &self.nonce,
            &self.ui_sha256,
            &self.broker_sha256,
        )?;
        client(self.client_pid, self.client_uid)?;
        if self.server_pid <= 1 || self.server_uid != 0 {
            return Err(Error::Server);
        }
        Ok(())
    }
}
pub fn parse_response(bytes: &[u8]) -> Result<Response, Error> {
    let value: Response = parse(bytes, RESPONSE_LIMIT)?;
    value.validate()?;
    Ok(value)
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct PeerEvidence {
    pub pid: u32,
    pub uid_fields: [u32; 4],
    pub start_ticks: u64,
    pub exe_sha256: String,
    pub exe_path: String,
    pub root_path: String,
}
/// Identity-only policy: callers must separately validate enrollment expiry,
/// request nonce and actual service context immediately around this check.
/// Neither caller-provided JSON nor a connection-time credential alone is evidence.
pub fn authorize(
    enrollment: &Enrollment,
    connect_cred: &Credentials,
    message_cred: &Credentials,
    peer: &PeerEvidence,
) -> Result<(), Error> {
    enrollment.validate_identity()?;
    if connect_cred != message_cred
        || u32::try_from(connect_cred.pid).ok() != Some(enrollment.client_pid)
        || connect_cred.uid != enrollment.client_uid
    {
        return Err(Error::Credentials);
    }
    if peer.pid != enrollment.client_pid
        || peer.uid_fields != [enrollment.client_uid; 4]
        || peer.start_ticks != enrollment.client_start_ticks
        || peer.exe_sha256 != enrollment.ui_sha256
    {
        return Err(Error::PeerIdentity);
    }
    if peer.exe_path != PROOF_EXE_PATH || peer.root_path != PROOF_JAIL_ROOT {
        return Err(Error::PeerPath);
    }
    Ok(())
}

pub fn validate_context(system: &Value, foreground: &Value, windows: &Value) -> Result<(), Error> {
    let sole_card = windows
        .get("windows")
        .and_then(Value::as_array)
        .is_some_and(|rows| {
            rows.len() == 1
                && rows[0].get("appId").and_then(Value::as_str) == Some(crate::identity::PROOF)
                && rows[0].get("windowType").and_then(Value::as_str)
                    == Some("_WEBOS_WINDOW_TYPE_CARD")
        });
    if system.get("returnValue").and_then(Value::as_bool) != Some(true)
        || system.get("modelName").and_then(Value::as_str) != Some("OLED55G26LA")
        || system.get("firmwareVersion").and_then(Value::as_str) != Some("23.25.55")
        || system.get("sdkVersion").and_then(Value::as_str) != Some("9.2.2")
        || foreground.get("returnValue").and_then(Value::as_bool) != Some(true)
        || foreground.get("appId").and_then(Value::as_str) != Some(crate::identity::PROOF)
        || windows.get("returnValue").and_then(Value::as_bool) != Some(true)
        || !sole_card
    {
        return Err(Error::Context);
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    fn marker() -> Marker {
        Marker {
            schema: 1,
            nonce: "a".repeat(32),
            ui_sha256: "b".repeat(64),
            broker_sha256: "c".repeat(64),
        }
    }
    fn enrollment() -> Enrollment {
        let m = marker();
        Enrollment {
            schema: 1,
            nonce: m.nonce,
            client_pid: 123,
            client_uid: 6583,
            client_start_ticks: 456,
            ui_sha256: m.ui_sha256,
            broker_sha256: m.broker_sha256,
            expires_unix: 190,
        }
    }
    fn peer() -> PeerEvidence {
        PeerEvidence {
            pid: 123,
            uid_fields: [6583; 4],
            start_ticks: 456,
            exe_sha256: "b".repeat(64),
            exe_path: PROOF_EXE_PATH.into(),
            root_path: PROOF_JAIL_ROOT.into(),
        }
    }
    fn credentials() -> Credentials {
        Credentials {
            pid: 123,
            uid: 6583,
            gid: 6583,
        }
    }
    fn context() -> (Value, Value, Value) {
        (
            json!({"returnValue":true,"modelName":"OLED55G26LA","firmwareVersion":"23.25.55","sdkVersion":"9.2.2"}),
            json!({"returnValue":true,"appId":crate::identity::PROOF}),
            json!({"returnValue":true,"windows":[{"appId":crate::identity::PROOF,"windowType":"_WEBOS_WINDOW_TYPE_CARD"}]}),
        )
    }

    #[test]
    fn roundtrips_use_only_the_identify_wire_operation() {
        let m = marker();
        assert_eq!(
            parse_marker(&serde_json::to_vec(&m).unwrap()),
            Ok(m.clone())
        );
        let e = enrollment();
        assert_eq!(
            parse_enrollment(&serde_json::to_vec(&e).unwrap(), 100),
            Ok(e)
        );
        let request = Request {
            schema: 1,
            op: Operation::Identify,
            nonce: m.nonce.clone(),
        };
        assert_eq!(
            parse_request(&serde_json::to_vec(&request).unwrap()),
            Ok(request)
        );
        let response = Response {
            schema: 1,
            op: Operation::Identify,
            nonce: m.nonce,
            server_pid: 987,
            server_uid: 0,
            client_pid: 123,
            client_uid: 6583,
            client_start_ticks: 456,
            ui_sha256: m.ui_sha256,
            broker_sha256: m.broker_sha256,
        };
        assert_eq!(
            parse_response(&serde_json::to_vec(&response).unwrap()),
            Ok(response)
        );
    }
    #[test]
    fn rejects_unknown_fields_operations_types_duplicates_and_extra_json() {
        let base = json!({"schema":1,"op":"identify","nonce":"a".repeat(32)});
        for extra in ["path", "command", "uri", "app_id", "icon_role", "fd"] {
            let mut request = base.clone();
            request[extra] = json!("forbidden");
            assert!(parse_request(&serde_json::to_vec(&request).unwrap()).is_err());
        }
        for op in [
            json!("get_icon"),
            json!("Identify"),
            json!({"identify":null}),
            json!(null),
            json!(true),
        ] {
            let mut request = base.clone();
            request["op"] = op;
            assert!(parse_request(&serde_json::to_vec(&request).unwrap()).is_err());
        }
        let mut bad = serde_json::to_vec(&base).unwrap();
        bad.extend_from_slice(b"{}");
        assert!(parse_request(&bad).is_err());
        assert!(parse_request(
            format!(
                "{{\"schema\":1,\"schema\":1,\"op\":\"identify\",\"nonce\":\"{}\"}}",
                "a".repeat(32)
            )
            .as_bytes()
        )
        .is_err());
        for schema in [json!(true), json!("1"), json!(1.0), json!(-1), json!(2)] {
            let mut request = base.clone();
            request["schema"] = schema;
            assert!(parse_request(&serde_json::to_vec(&request).unwrap()).is_err());
        }
        for mut value in [
            serde_json::to_value(marker()).unwrap(),
            serde_json::to_value(enrollment()).unwrap(),
        ] {
            value["extra"] = json!(true);
            let raw = serde_json::to_vec(&value).unwrap();
            assert!(parse_marker(&raw).is_err());
            assert!(parse_enrollment(&raw, 100).is_err());
        }
    }
    #[test]
    fn nonce_and_executable_hash_formats_are_exact() {
        for nonce in [
            "a".repeat(31),
            "a".repeat(33),
            "A".repeat(32),
            "g".repeat(32),
            "é".repeat(16),
        ] {
            let mut m = marker();
            m.nonce = nonce;
            assert_eq!(m.validate(), Err(Error::Nonce));
        }
        for hash in [
            "b".repeat(63),
            "b".repeat(65),
            "B".repeat(64),
            "z".repeat(64),
        ] {
            let mut m = marker();
            m.ui_sha256 = hash.clone();
            assert_eq!(m.validate(), Err(Error::Hash));
            let mut m = marker();
            m.broker_sha256 = hash;
            assert_eq!(m.validate(), Err(Error::Hash));
        }
    }
    #[test]
    fn response_rejects_nonroot_servers_invalid_clients_and_untyped_binding_fields() {
        let m = marker();
        let response = Response {
            schema: 1,
            op: Operation::Identify,
            nonce: m.nonce,
            server_pid: 987,
            server_uid: 0,
            client_pid: 123,
            client_uid: 6583,
            client_start_ticks: 456,
            ui_sha256: m.ui_sha256,
            broker_sha256: m.broker_sha256,
        };
        for pid in [0, 1] {
            let mut bad = response.clone();
            bad.server_pid = pid;
            assert_eq!(bad.validate(), Err(Error::Server));
        }
        let mut bad = response.clone();
        bad.server_uid = 6583;
        assert_eq!(bad.validate(), Err(Error::Server));
        let mut bad = response.clone();
        bad.client_uid = 5724;
        assert_eq!(bad.validate(), Err(Error::Client));
        let base = serde_json::to_value(response).unwrap();
        for key in [
            "schema",
            "server_pid",
            "server_uid",
            "client_pid",
            "client_uid",
            "client_start_ticks",
        ] {
            for value in [json!(null), json!(true), json!("1"), json!(-1), json!(1.5)] {
                let mut bad = base.clone();
                bad[key] = value;
                assert!(parse_response(&serde_json::to_vec(&bad).unwrap()).is_err());
            }
            let mut bad = base.clone();
            bad.as_object_mut().unwrap().remove(key);
            assert!(parse_response(&serde_json::to_vec(&bad).unwrap()).is_err());
        }
        let mut bad = base;
        bad["icon"] = json!("forbidden");
        assert!(parse_response(&serde_json::to_vec(&bad).unwrap()).is_err());
    }
    #[test]
    fn size_limits_apply_before_deserialization_including_whitespace() {
        let mut request = serde_json::to_vec(&Request {
            schema: 1,
            op: Operation::Identify,
            nonce: "a".repeat(32),
        })
        .unwrap();
        request.resize(REQUEST_LIMIT, b' ');
        assert!(parse_request(&request).is_ok());
        request.push(b' ');
        assert_eq!(parse_request(&request), Err(Error::TooLarge));
        let mut m = serde_json::to_vec(&marker()).unwrap();
        m.resize(MARKER_LIMIT, b' ');
        assert!(parse_marker(&m).is_ok());
        m.push(b' ');
        assert_eq!(parse_marker(&m), Err(Error::TooLarge));
        let mut e = serde_json::to_vec(&enrollment()).unwrap();
        e.resize(ENROLLMENT_LIMIT, b' ');
        assert!(parse_enrollment(&e, 100).is_ok());
        e.push(b' ');
        assert_eq!(parse_enrollment(&e, 100), Err(Error::TooLarge));
        assert_eq!(
            parse_response(&vec![b' '; RESPONSE_LIMIT + 1]),
            Err(Error::TooLarge)
        );
        assert!(parse_request(&[0xff]).is_err());
    }
    #[test]
    fn enrollment_expiry_and_proof_identity_fail_closed() {
        let mut e = enrollment();
        assert!(e.validate(100).is_ok());
        e.expires_unix = 191;
        assert_eq!(e.validate(100), Err(Error::Expiry));
        e.expires_unix = 100;
        assert_eq!(e.validate(100), Err(Error::Expiry));
        e.expires_unix = 99;
        assert_eq!(e.validate(100), Err(Error::Expiry));
        assert_eq!(enrollment().validate(u64::MAX), Err(Error::Expiry));
        for uid in [0, 999, 5724] {
            let mut e = enrollment();
            e.client_uid = uid;
            assert_eq!(e.validate(100), Err(Error::Client));
        }
        for pid in [0, 1] {
            let mut e = enrollment();
            e.client_pid = pid;
            assert_eq!(e.validate(100), Err(Error::Client));
        }
    }
    #[test]
    fn authorization_binds_both_credentials_and_every_process_identity_field() {
        let e = enrollment();
        let c = credentials();
        let evidence = peer();
        assert_eq!(authorize(&e, &c, &c, &evidence), Ok(()));
        for bad in [
            Credentials { pid: -1, ..c },
            Credentials { pid: 124, ..c },
            Credentials { uid: 0, ..c },
            Credentials { gid: 0, ..c },
        ] {
            assert!(authorize(&e, &c, &bad, &evidence).is_err());
            assert!(authorize(&e, &bad, &c, &evidence).is_err());
        }
        let mut bad = evidence.clone();
        bad.pid += 1;
        assert_eq!(authorize(&e, &c, &c, &bad), Err(Error::PeerIdentity));
        let mut bad = evidence.clone();
        bad.start_ticks += 1;
        assert_eq!(authorize(&e, &c, &c, &bad), Err(Error::PeerIdentity));
        for index in 0..4 {
            let mut bad = evidence.clone();
            bad.uid_fields[index] = 0;
            assert_eq!(authorize(&e, &c, &c, &bad), Err(Error::PeerIdentity));
        }
        let mut bad = evidence;
        bad.exe_sha256 = "d".repeat(64);
        assert_eq!(authorize(&e, &c, &c, &bad), Err(Error::PeerIdentity));
    }
    #[test]
    fn paths_require_exact_provisional_proof_jail_and_executable() {
        let e = enrollment();
        let c = credentials();
        for path in [
            "/",
            "/var/palm/jail/com.ash.nativehome.preview",
            "/var/palm/jail/com.ash.nativehome.proof/",
            "/tmp/var/palm/jail/com.ash.nativehome.proof",
        ] {
            let mut p = peer();
            p.root_path = path.into();
            assert_eq!(authorize(&e, &c, &c, &p), Err(Error::PeerPath));
        }
        for path in [
            "/media/developer/apps/usr/palm/applications/com.ash.nativehome.proof/home-preview"
                .to_owned(),
            format!("{PROOF_EXE_PATH} (deleted)"),
            format!("/tmp{PROOF_EXE_PATH}"),
        ] {
            let mut p = peer();
            p.exe_path = path;
            assert_eq!(authorize(&e, &c, &c, &p), Err(Error::PeerPath));
        }
    }
    #[test]
    fn context_accepts_only_exact_g2_and_one_foreground_proof_card() {
        let (system, fg, windows) = context();
        assert_eq!(validate_context(&system, &fg, &windows), Ok(()));
        for (key, value) in [
            ("modelName", json!("OLED65C26LD")),
            ("firmwareVersion", json!("23.25.56")),
            ("sdkVersion", json!("9.2.2-61")),
            ("returnValue", json!(false)),
        ] {
            let mut bad = system.clone();
            bad[key] = value;
            assert_eq!(validate_context(&bad, &fg, &windows), Err(Error::Context));
        }
        let mut bad = fg.clone();
        bad["appId"] = json!("com.webos.app.home");
        assert_eq!(
            validate_context(&system, &bad, &windows),
            Err(Error::Context)
        );
        for rows in [
            json!([]),
            json!([{"appId":crate::identity::PROOF,"windowType":"_WEBOS_WINDOW_TYPE_CARD"},{"appId":"com.webos.app.keyboard","windowType":"_WEBOS_WINDOW_TYPE_KEYBOARD"}]),
            json!([{"appId":crate::identity::PROOF,"windowType":"_WEBOS_WINDOW_TYPE_SPLASH"}]),
            json!([null]),
        ] {
            let mut bad = windows.clone();
            bad["windows"] = rows;
            assert_eq!(validate_context(&system, &fg, &bad), Err(Error::Context));
        }
        assert_eq!(
            validate_context(&Value::Null, &fg, &windows),
            Err(Error::Context)
        );
        assert_eq!(
            validate_context(&system, &Value::Null, &windows),
            Err(Error::Context)
        );
        assert_eq!(
            validate_context(&system, &fg, &Value::Null),
            Err(Error::Context)
        );
    }
}
