//! Controlled host-only metadata/Receiver tests. No HBC, Root process or TV.
use super::super::{tests as positive, AttemptStage};
use super::*;
use crate::artwork_worker::ProviderError;

struct Fixture {
    marker: Marker,
    profile: Profile,
    source: crate::app_artwork::Source,
    trace: AttemptTrace,
    hbc: Value,
    pending: PendingIdentity,
}
fn file(mode: u64, inode: u64) -> Value {
    json!({"device":1,"inode":inode,"bytes":128,"mode":0o100000|mode,"uid":0,"gid":0,"links":1,"mtime":1,"mtime_ns":0,"ctime":1,"ctime_ns":0})
}
fn measurement() -> Value {
    json!({"sha256":"d".repeat(64),"identity":{"device":1,"inode":53,"bytes":65536,"mode":0o100700,"uid":0,"gid":0,"links":1,"modified_seconds":1,"modified_nanos":0,"changed_seconds":1,"changed_nanos":0}})
}
// Set every reader and operation to the SAME observation. Malformed fixtures
// therefore exercise schema/regular-file validation, not equality mismatch.
fn replace_reader_measurements(value: &mut Value, measurement: &Value) {
    match value {
        Value::Object(map) => {
            if let Some(reader) = map.get_mut("_reader") {
                reader["executable"] = measurement.clone();
                reader["executable_after"] = measurement.clone();
                if let Some(operations) = reader["operations"].as_array_mut() {
                    for operation in operations {
                        operation["executable_before"] = measurement.clone();
                        operation["executable_after"] = measurement.clone();
                    }
                }
            }
            for child in map.values_mut() {
                replace_reader_measurements(child, measurement);
            }
        }
        Value::Array(rows) => {
            for child in rows {
                replace_reader_measurements(child, measurement);
            }
        }
        _ => (),
    }
}
fn anchor_rows(path: &str, authority: bool) -> Value {
    let mut paths = vec!["/".to_owned()];
    let mut prefix = String::new();
    for part in path.trim_start_matches('/').split('/') {
        prefix.push('/');
        prefix.push_str(part);
        paths.push(prefix.clone());
    }
    let count = paths.len();
    json!(paths
        .into_iter()
        .enumerate()
        .map(|(index, path)| {
            let mode = if authority && index + 1 == count {
                0o40700
            } else if !authority && path == "/media/developer" {
                0o40777
            } else {
                0o40755
            };
            json!({"path":path,"identity":{"device":1,"inode":index+1,"uid":0,"gid":0,"mode":mode}})
        })
        .collect::<Vec<_>>())
}
fn withdrawn(value: &mut Value) {
    value["foreground"]["appId"] = json!("com.webos.app.home");
    value["windows"]["windows"] =
        json!([{"appId":"com.webos.app.home","windowType":"_WEBOS_WINDOW_TYPE_CARD"}]);
}
impl Fixture {
    fn new(stage: Stage, profile: Profile) -> Self {
        Self::new_with_pid(stage, profile, None)
    }
    fn new_with_pid(stage: Stage, profile: Profile, pid: Option<u32>) -> Self {
        let (provider, source, mut exchange, mut hbc) = match profile {
            Profile::Proof => positive::fixture(),
            Profile::Preview => positive::preview_fixture(),
        };
        let marker = provider.marker.clone();
        if let Some(pid) = pid {
            fn rebind(v: &mut Value, pid: u32, root: i32) {
                match v {
                    Value::Array(rows) => rows.iter_mut().for_each(|v| rebind(v, pid, root)),
                    Value::Object(rows) => {
                        for (key, value) in rows.iter_mut() {
                            if matches!(
                                key.as_str(),
                                "pid" | "client_pid" | "server_pid" | "child_pid" | "process_group"
                            ) {
                                match value.as_u64() {
                                    Some(123) => *value = json!(pid),
                                    Some(789) => *value = json!(root),
                                    Some(child @ 1001..=1016) => {
                                        *value = json!(u64::from(pid) + 1000 + child - 1001)
                                    }
                                    _ => (),
                                }
                            } else {
                                rebind(value, pid, root);
                            }
                        }
                    }
                    _ => (),
                }
            }
            let root = pid.checked_add(100_000).unwrap() as i32;
            rebind(&mut hbc, pid, root);
            exchange.own.pid = pid;
            exchange.client.pid = pid as i32;
            exchange.root.pid = root;
            exchange.message.pid = root;
            exchange.identity.client_pid = pid;
            exchange.identity.server_pid = root as u32;
            let wire::Outcome::Artwork(image) = &mut exchange.outcome else {
                panic!("fixture")
            };
            image.header.identity = exchange.identity.clone();
            image.complete =
                wire::Complete::from_header_for_profile(&image.header, profile).unwrap();
            hbc["parsed"]["helper_report"]["artwork_reads"][0]["header"] = json!(image.header);
            hbc["parsed"]["helper_report"]["artwork_reads"][0]["complete"] = json!(image.complete);
        }
        let mut trace = AttemptTrace::new(profile, source.key());
        trace.stage = match stage {
            Stage::ReadAfter => AttemptStage::ReadArtwork,
            Stage::Finish => AttemptStage::Finish,
        };
        trace.own = Some(exchange.own.clone());
        trace.own_after = Some(exchange.own.clone());
        trace.root = Some(exchange.root);
        trace.final_root_peer = Some(exchange.root);
        trace.client = Some(exchange.client);
        trace.identify_message = Some(exchange.message);
        trace.identify_bound = true;
        trace.identify_response = Some(exchange.identity.clone());
        trace.identity = Some(exchange.identity.clone());
        trace.identify_request = Some(crate::broker_protocol::Request {
            schema: 1,
            op: crate::broker_protocol::Operation::Identify,
            nonce: marker.nonce.clone(),
        });
        trace.request = Some(exchange.request.clone());
        let wire::Outcome::Artwork(image) = exchange.outcome else {
            panic!("Synthetic metadata")
        };
        let mut receiver = wire::Receiver::new_for_profile(
            &exchange.request,
            &exchange.identity,
            exchange.root,
            profile,
        )
        .unwrap();
        receiver
            .push_packet(&serde_json::to_vec(&image.header).unwrap(), exchange.root)
            .unwrap();
        for (index, bytes) in image.bytes.chunks(wire::CHUNK_BYTES).enumerate() {
            receiver
                .push_packet(
                    &wire::encode_chunk(1, index as u32, bytes).unwrap(),
                    exchange.root,
                )
                .unwrap();
        }
        trace.header = Some(image.header.clone());
        trace.header_matches = Some(true);
        trace.resource_packets_received = image.header.chunk_count + 1;
        trace.resource_packets_validated = trace.resource_packets_received;
        trace.last_received_credentials = Some(exchange.root);
        trace.last_validated_credentials = Some(exchange.root);
        match stage {
            Stage::ReadAfter => {
                trace.discarded_read = Some(receiver.discard_incomplete_after_eof().unwrap())
            }
            Stage::Finish => {
                receiver
                    .push_packet(&serde_json::to_vec(&image.complete).unwrap(), exchange.root)
                    .unwrap();
                assert!(matches!(
                    receiver.finish().unwrap(),
                    wire::Outcome::Artwork(_)
                ));
                trace.complete = Some(image.complete.clone());
                trace.resource_complete_validated = Some(true);
                trace.finish_request = Some(wire::FinishRequest::new(2, &marker.nonce).unwrap());
                trace.resource_packets_received += 1;
                trace.resource_packets_validated += 1;
            }
        }
        trace.fail(
            AttemptFailureKind::UnexpectedEof,
            ProviderError::Unavailable,
        );
        let own = trace.own.as_ref().unwrap();
        let pending = PendingIdentity {
            schema: 1,
            profile,
            caller_pid: own.pid,
            caller_start_ticks: own.start_ticks,
            ui_sha256: marker.ui_sha256.clone(),
            nonce: marker.nonce.clone(),
        };
        let helper = &mut hbc["parsed"]["helper_report"];
        let installation = &mut helper["installation"];
        installation["schema"] = json!(1);
        installation["bundle"] = json!(profile.bundle());
        installation["context_reader_measured_here"] = json!(false);
        installation["normal_runtime_qualified"] = json!(false);
        installation["files"]["configuration"]["sha256"] = json!("e".repeat(64));
        for (name, mode, inode) in [
            ("configuration", 0o600, 20),
            ("helper", 0o700, 21),
            ("ui", 0o755, 22),
        ] {
            installation["files"][name]["identity"] = file(mode, inode);
        }
        installation["authority_anchors"] = anchor_rows(profile.installation_dir(), true);
        installation["ui_lookup_anchors"] = anchor_rows(profile.bundle(), false);
        let installation = installation.clone();
        helper["installation_after_context_withdrawal"] = installation;
        helper["identify_request"] = json!(trace.identify_request.as_ref().unwrap());
        helper["enrollment"] = json!({"schema":1,"nonce":marker.nonce,"client_pid":own.pid,"client_uid":own.uid,"client_start_ticks":own.start_ticks,"ui_sha256":marker.ui_sha256,"broker_sha256":marker.broker_sha256,"expires_unix":123456,"context_reader_sha256":"d".repeat(64)});
        helper["listener_bound"] = json!(true);
        helper["listener_closed"] = json!(true);
        helper["closed_negative_candidate"] = json!(true);
        helper["closed_negative_acceptance_enabled"] = json!(false);
        helper["qualified"] = json!(false);
        helper["finished_sent"] = json!(false);
        helper["finished"] = Value::Null;
        helper.as_object_mut().unwrap().remove("artwork_finished");
        helper["artwork_session_completed"] = json!(false);
        helper["outcome"] = json!("context_changed_closed");
        helper["handshake_only"] = json!(false);
        helper["artwork_reads_only"] = json!(true);
        helper["context_cli_gets_only"] = json!(false);
        let error = format!(
            "Installed artwork policy withdrawn at {} request {}",
            stage.name(),
            stage.request_id()
        );
        helper["error"] = json!(error);
        helper["failure"] =
            json!({"kind":"context_changed","stage":stage.name(),"request_id":stage.request_id()});
        helper["peer_after_context_withdrawal"] = helper["peer_before"].clone();
        helper["context_withdrawal_message_credentials"] = helper["connect_credentials"].clone();
        match stage {
            Stage::ReadAfter => {
                let record = &mut helper["artwork_reads"][0];
                record.as_object_mut().unwrap().remove("complete");
                record["completed"] = json!(false);
                record["error"] = json!(error);
                withdrawn(&mut record["context_after"]);
                for name in [
                    "artwork_finish_request",
                    "artwork_finish_context",
                    "artwork_finish_connect_credentials",
                    "artwork_finish_message_credentials",
                    "artwork_finish_peer_before",
                    "artwork_finish_peer_after",
                ] {
                    helper.as_object_mut().unwrap().remove(name);
                }
            }
            Stage::Finish => {
                helper["artwork_finish_request"] = json!(trace.finish_request.as_ref().unwrap());
                withdrawn(&mut helper["artwork_finish_context"]);
            }
        }
        let mut result = Self {
            marker,
            profile,
            source,
            trace,
            hbc,
            pending,
        };
        replace_reader_measurements(&mut result.hbc["parsed"]["helper_report"], &measurement());
        result.rebuild(None);
        result
    }
    fn events(&self) -> Vec<Value> {
        let helper = &self.hbc["parsed"]["helper_report"];
        let record = &helper["artwork_reads"][0];
        let mut events = Vec::new();
        for (value, catalogue) in [
            (&helper["context_before"], false),
            (&record["context_before"], false),
            (&record["catalogue_before"], true),
            (&record["catalogue_after"], true),
            (&record["context_after"], false),
            (&helper["artwork_finish_context"], false),
        ] {
            if let Some(rows) = value["_reader"]["operations"].as_array() {
                for row in rows {
                    events.push(json!({"schema":1,"stage":if catalogue{"catalogue_child_started"}else{"context_child_started"},"child_pid":row["child_pid"],"process_group":row["process_group"],"program_sha256":"d".repeat(64)}));
                }
            }
        }
        events
    }
    fn rebuild(&mut self, override_events: Option<Vec<Value>>) {
        let helper = self.hbc["parsed"]["helper_report"].clone();
        let mut rows = override_events.unwrap_or_else(|| self.events());
        rows.push(helper.clone());
        let stdout = rows.iter().map(|v| format!("{v}\n")).collect::<String>();
        let error = helper["error"].as_str().unwrap_or("");
        let stderr = format!("Error: {error:?}\n");
        let raw=json!({"returnValue":false,"errorText":"synthetic fixed helper exit1","stdoutString":stdout,"stdoutBytes":positive::base64(stdout.as_bytes()),"stderrString":stderr,"stderrBytes":positive::base64(stderr.as_bytes())}).to_string();
        self.install_raw(raw);
    }
    fn install_raw(&mut self, raw: String) {
        self.hbc["raw_bytes"] = json!(raw.len());
        self.hbc["raw_truncated"] = json!(false);
        self.hbc["raw"] = json!(raw.clone());
        self.hbc["parsed"] =
            crate::broker_startup_proof::parse_exec_resource_reply_for_profile(&raw, self.profile)
                .unwrap_or(Value::Null);
    }
    fn mutate(&mut self, change: impl FnOnce(&mut Value)) {
        change(&mut self.hbc["parsed"]["helper_report"]);
        self.rebuild(None);
    }
    fn grade(&self) -> Result<KnownClosed, ProviderError> {
        grade_closed_negative(NegativeEvidence {
            trace: &self.trace,
            joined_hbc: &self.hbc,
            marker: &self.marker,
            profile: self.profile,
            source: self.source.key(),
            current_pending: &self.pending,
        })
    }
}

/// Controlled complete metadata only. Owner supplies a real host Guard/Ticket;
/// this function never claims native Root or opens a service.
pub(in crate::artwork_installed_provider) fn fixture_for_current(
    finish: bool,
    profile: Profile,
    pid: u32,
) -> (
    super::super::InstalledProvider,
    crate::app_artwork::Source,
    AttemptTrace,
    Value,
    PendingIdentity,
) {
    let f = Fixture::new_with_pid(
        if finish {
            Stage::Finish
        } else {
            Stage::ReadAfter
        },
        profile,
        Some(pid),
    );
    let (mut provider, _, _, _) = match profile {
        Profile::Proof => positive::fixture(),
        Profile::Preview => positive::preview_fixture(),
    };
    provider.marker = f.marker.clone();
    (provider, f.source, f.trace, f.hbc, f.pending)
}

// Groups 16/17 live with the owner's CurrentAttempt/Worker integration tests.
#[test]
fn group01_read_after_thirteen_children_returns_only_current_opaque_proof() {
    let f = Fixture::new(Stage::ReadAfter, Profile::Proof);
    assert_eq!(f.events().len(), 13);
    let proof = f.grade().unwrap();
    assert_eq!(f.trace.stage, AttemptStage::ReadArtwork);
    assert!(proof.matches_current(&f.trace.seal));
    assert!(!proof.matches_current(&Arc::new(())));
}
#[test]
fn group02_finish_sixteen_children_requires_real_complete_then_eof() {
    let f = Fixture::new(Stage::Finish, Profile::Proof);
    assert_eq!(f.events().len(), 16);
    assert!(f.grade().unwrap().matches_current(&f.trace.seal));
    assert!(f.trace.complete.is_some());
    assert!(f.trace.finished.is_none());
    assert!(!f.trace.clean_eof);
}
#[test]
fn group03_existing_positive_lifecycle_is_not_retrofitted_as_negative() {
    for profile in [Profile::Proof, Profile::Preview] {
        let (mut provider, source, exchange, hbc) = match profile {
            Profile::Proof => positive::fixture(),
            Profile::Preview => positive::preview_fixture(),
        };
        assert!(session::settle(
            &mut provider,
            source.key(),
            Ok(exchange),
            Ok(hbc),
            false,
            false
        )
        .is_ok());
        let mut f = Fixture::new(Stage::Finish, profile);
        f.mutate(|h| h["qualified"] = json!(true));
        assert!(f.grade().is_err());
    }
}
#[test]
fn group04_preidentify_readbefore_generic_errors_and_unobserved_eof_are_unknown() {
    for stage in [
        AttemptStage::AwaitStartup,
        AttemptStage::Connecting,
        AttemptStage::ConnectedBeforeIdentify,
        AttemptStage::Identify,
    ] {
        let mut f = Fixture::new(Stage::ReadAfter, Profile::Proof);
        f.trace.stage = stage;
        f.rebuild(None);
        assert!(f.grade().is_err());
    }
    for change in 0..4 {
        let mut f = Fixture::new(Stage::ReadAfter, Profile::Proof);
        match change {
            0 => f.trace.identify_bound = false,
            1 => f.trace.observed_unexpected_eof = false,
            2 => f.trace.failure.as_mut().unwrap().kind = AttemptFailureKind::ReceiveError,
            _ => f.trace.header = None,
        };
        f.rebuild(None);
        assert!(f.grade().is_err());
    }
    let mut f = Fixture::new(Stage::ReadAfter, Profile::Proof);
    f.mutate(|h| h["failure"]["stage"] = json!("read_before"));
    assert!(f.grade().is_err());
}
#[test]
fn group05_closed_profiles_do_not_derive_authority_from_wire_overlap() {
    assert!(Fixture::new(Stage::ReadAfter, Profile::Preview)
        .grade()
        .is_ok());
    for change in 0..5 {
        let mut f = Fixture::new(Stage::Finish, Profile::Preview);
        match change {
            0 => f.profile = Profile::Proof,
            1 => {
                f.hbc["command"] = json!(crate::broker_startup_proof::fixed_command(Profile::Proof))
            }
            2 => {
                f.hbc["parsed"]["helper_report"]["installation"]["pins"]["profile"] = json!("proof")
            }
            3 => {
                f.hbc["parsed"]["helper_report"]["peer_before"]["root_path"] =
                    json!(Profile::Proof.jail_root())
            }
            _ => {
                f.hbc["parsed"]["helper_report"]["installation"]["files"]["configuration"]["path"] =
                    json!(Profile::Proof.config_path())
            }
        };
        f.rebuild(None);
        assert!(f.grade().is_err());
    }
}
#[test]
fn group06_session_request_header_and_full_source_must_match() {
    for pointer in [
        "/nonce",
        "/response/nonce",
        "/enrollment/nonce",
        "/enrollment/ui_sha256",
        "/enrollment/broker_sha256",
        "/artwork_reads/0/request/nonce",
        "/artwork_reads/0/source_key/version",
        "/artwork_reads/0/source_key/class",
        "/artwork_reads/0/source_key/kind",
        "/artwork_reads/0/source_key/relative",
        "/artwork_reads/0/source_key_after/folder",
        "/artwork_reads/0/header/encoded_sha256",
    ] {
        let mut f = Fixture::new(Stage::ReadAfter, Profile::Proof);
        f.mutate(|h| *h.pointer_mut(pointer).unwrap() = json!("f".repeat(32)));
        assert!(f.grade().is_err(), "{pointer}");
    }
    for change in 0..3 {
        let mut f = Fixture::new(Stage::ReadAfter, Profile::Proof);
        match change {
            0 => f.trace.request.as_mut().unwrap().request_id = 2,
            1 => f.pending.caller_start_ticks += 1,
            _ => f.trace.source_key.relative.push("other.png".into()),
        };
        f.rebuild(None);
        assert!(f.grade().is_err());
    }
}
#[test]
fn group07_kernel_self_after_and_every_accepted_credential_are_required() {
    for change in 0..8 {
        let mut f = Fixture::new(Stage::ReadAfter, Profile::Proof);
        match change {
            0 => f.trace.own_after.as_mut().unwrap().pid += 1,
            1 => f.trace.own_after.as_mut().unwrap().ui_sha256 = "f".repeat(64),
            2 => f.trace.final_root_peer.as_mut().unwrap().pid += 1,
            3 => f.trace.identify_message.as_mut().unwrap().gid += 1,
            4 => f.trace.client.as_mut().unwrap().gid += 1,
            5 => f.trace.last_validated_credentials.as_mut().unwrap().uid = 1,
            6 => f.trace.last_received_credentials = None,
            _ => f.trace.own_after = None,
        };
        f.rebuild(None);
        assert!(f.grade().is_err());
    }
    let mut f = Fixture::new(Stage::Finish, Profile::Proof);
    f.mutate(|h| h["root_process"]["start_ticks"] = json!(0));
    assert!(f.grade().is_err());
}
#[test]
fn group08_protected_origin_and_final_evidence_cannot_drift_or_be_permissive() {
    for pointer in [
        "/installation/files/helper/path",
        "/installation/files/configuration/path",
        "/installation/pins/context_reader_sha256",
        "/installation_after_context_withdrawal/files/ui/sha256",
    ] {
        let mut f = Fixture::new(Stage::Finish, Profile::Proof);
        f.mutate(|h| *h.pointer_mut(pointer).unwrap() = json!("wrong"));
        assert!(f.grade().is_err());
    }
    for (key, value) in [
        ("mode", json!(0o100777)),
        ("uid", json!(6000)),
        ("links", json!(2)),
        ("inode", json!(0)),
        ("bytes", json!(0)),
        ("mtime_ns", json!(1_000_000_000)),
    ] {
        let mut f = Fixture::new(Stage::ReadAfter, Profile::Proof);
        f.mutate(|h| {
            h["installation"]["files"]["configuration"]["identity"][key] = value.clone();
            h["installation_after_context_withdrawal"] = h["installation"].clone();
        });
        assert!(f.grade().is_err());
    }
    let mut f = Fixture::new(Stage::ReadAfter, Profile::Proof);
    f.mutate(|h| {
        h["installation"]["authority_anchors"][1]["identity"]["mode"] = json!(0o40777);
        h["installation_after_context_withdrawal"] = h["installation"].clone();
    });
    assert!(f.grade().is_err());
}
#[test]
fn group09_original_raw_hbc_is_reparsed_with_no_diagnostic_authority() {
    for change in 0..7 {
        let mut f = Fixture::new(Stage::ReadAfter, Profile::Proof);
        let mut raw: Value = serde_json::from_str(f.hbc["raw"].as_str().unwrap()).unwrap();
        match change {
            0 => raw["stdoutBytes"] = json!("bad"),
            1 => raw["extra"] = json!(false),
            2 => raw["returnValue"] = json!(0),
            3 => raw
                .as_object_mut()
                .unwrap()
                .remove("errorText")
                .map(|_| ())
                .unwrap(),
            4 => raw["stderrBytes"] = json!(""),
            5 => raw["stdoutString"] = json!("{}\n"),
            _ => raw["returnValue"] = json!(true),
        };
        f.install_raw(raw.to_string());
        assert!(f.grade().is_err());
    }
    let mut f = Fixture::new(Stage::Finish, Profile::Proof);
    let raw = f.hbc["raw"].as_str().unwrap().to_owned();
    f.install_raw(raw.replacen(
        "\"returnValue\":false",
        "\"returnValue\":false,\"returnValue\":false",
        1,
    ));
    assert!(f.grade().is_err());
    let mut f = Fixture::new(Stage::Finish, Profile::Proof);
    let raw = f.hbc["raw"].as_str().unwrap().to_owned();
    f.install_raw(format!("{raw}{{}}"));
    assert!(f.grade().is_err());
    let mut f = Fixture::new(Stage::Finish, Profile::Proof);
    f.hbc["raw_bytes"] = json!(0);
    assert!(f.grade().is_err());
    let mut f = Fixture::new(Stage::Finish, Profile::Proof);
    f.hbc["parsed"]["helper_report"]["qualified"] = json!(true);
    assert!(f.grade().is_err());
    for key in ["raw_truncated", "error"] {
        let mut f = Fixture::new(Stage::ReadAfter, Profile::Proof);
        f.rebuild(None);
        f.hbc[key] = if key == "error" {
            json!("transport failed")
        } else {
            json!(true)
        };
        assert!(f.grade().is_err());
    }
}
#[test]
fn group10_only_exact_false_terminal_and_source_derived_stderr_is_accepted() {
    for (key, value) in [
        ("outcome", json!("refused")),
        ("qualified", json!(true)),
        ("finished_sent", json!(true)),
        ("connection_closed", json!(false)),
        ("listener_closed", json!(false)),
        ("closed_negative_acceptance_enabled", json!(true)),
        ("error", json!("context changed")),
        ("finished", json!({"op":"finished"})),
    ] {
        let mut f = Fixture::new(Stage::Finish, Profile::Proof);
        f.mutate(|h| h[key] = value);
        assert!(f.grade().is_err());
    }
    let mut f = Fixture::new(Stage::ReadAfter, Profile::Proof);
    let mut raw: Value = serde_json::from_str(f.hbc["raw"].as_str().unwrap()).unwrap();
    raw["stderrString"] = json!("unquoted policy rejection\n");
    raw["stderrBytes"] = json!(positive::base64(b"unquoted policy rejection\n"));
    f.install_raw(raw.to_string());
    assert!(f.grade().is_err());
}
#[test]
fn group11_discard_witness_requires_full_digest_and_is_not_itself_eof() {
    let (_, _, exchange, _) = positive::fixture();
    let wire::Outcome::Artwork(image) = exchange.outcome else {
        panic!("image")
    };
    for length in [0, image.bytes.len() - 1, image.bytes.len()] {
        let mut receiver =
            wire::Receiver::new(&exchange.request, &exchange.identity, exchange.root).unwrap();
        receiver
            .push_packet(&serde_json::to_vec(&image.header).unwrap(), exchange.root)
            .unwrap();
        if length > 0 {
            let mut bytes = image.bytes[..length].to_vec();
            if length == image.bytes.len() {
                bytes[0] ^= 1;
            }
            let _ = receiver.push_packet(&wire::encode_chunk(1, 0, &bytes).unwrap(), exchange.root);
        }
        assert!(receiver.discard_incomplete_after_eof().is_err());
    }
    let mut complete =
        wire::Receiver::new(&exchange.request, &exchange.identity, exchange.root).unwrap();
    complete
        .push_packet(&serde_json::to_vec(&image.header).unwrap(), exchange.root)
        .unwrap();
    for (index, bytes) in image.bytes.chunks(wire::CHUNK_BYTES).enumerate() {
        complete
            .push_packet(
                &wire::encode_chunk(1, index as u32, bytes).unwrap(),
                exchange.root,
            )
            .unwrap();
    }
    assert!(complete
        .push_packet(&serde_json::to_vec(&image.complete).unwrap(), exchange.root)
        .unwrap());
    assert!(complete.discard_incomplete_after_eof().is_err());
    let mut refused =
        wire::Receiver::new(&exchange.request, &exchange.identity, exchange.root).unwrap();
    let refusal = wire::Refusal::new(
        &exchange.request,
        &exchange.identity,
        wire::RefusalReason::Denied,
    )
    .unwrap();
    assert!(refused
        .push_packet(&serde_json::to_vec(&refusal).unwrap(), exchange.root)
        .unwrap());
    assert!(refused.discard_incomplete_after_eof().is_err());
    let mut f = Fixture::new(Stage::ReadAfter, Profile::Proof);
    f.trace.observed_unexpected_eof = false;
    f.rebuild(None);
    assert!(f.grade().is_err());
    let mut f = Fixture::new(Stage::ReadAfter, Profile::Proof);
    f.trace.discarded_read = None;
    f.rebuild(None);
    assert!(f.grade().is_err());
}
#[test]
fn group12_finish_requires_actual_complete_no_refusal_or_extra_packet() {
    for change in 0..6 {
        let mut f = Fixture::new(Stage::Finish, Profile::Proof);
        match change {
            0 => f.trace.complete = None,
            1 => f.trace.resource_complete_validated = Some(false),
            2 => f.trace.resource_packets_validated += 1,
            3 => f.trace.finish_request.as_mut().unwrap().request_id = 1,
            4 => f.trace.complete.as_mut().unwrap().encoded_sha256 = "f".repeat(64),
            _ => {
                f.trace.finished = Some(
                    wire::Finished::new_for_profile(
                        f.trace.finish_request.as_ref().unwrap(),
                        f.trace.identity.as_ref().unwrap(),
                        f.profile,
                    )
                    .unwrap(),
                )
            }
        };
        f.rebuild(None);
        assert!(f.grade().is_err());
    }
}
#[test]
fn group13_each_reader_closure_predicate_is_mandatory() {
    for (key, value) in [
        ("key", json!("unknown")),
        ("uri", json!("luna://other/get")),
        ("argv", json!([])),
        ("attempted", json!(false)),
        ("success", json!(false)),
        ("exit_code", json!(1)),
        ("signal", json!(9)),
        ("timed_out", json!(true)),
        ("error", json!("failed")),
        ("stdout_bytes", json!(0)),
        ("stderr_bytes", json!(1)),
        ("stdout_eof", json!(false)),
        ("stderr_eof", json!(false)),
        ("child_reaped", json!(false)),
        ("cleanup_known", json!(false)),
        ("held_empty_stdin", json!(false)),
        ("stdin_bytes_sent", json!(1)),
        ("group_cleanup_attempted", json!(false)),
        ("group_kill_errno", json!(1)),
        ("process_group", json!(9999)),
        ("executable_after", json!({"sha256":"wrong"})),
    ] {
        let mut f = Fixture::new(Stage::ReadAfter, Profile::Proof);
        f.mutate(|h| {
            h["artwork_reads"][0]["context_after"]["_reader"]["operations"][0][key] = value
        });
        assert!(f.grade().is_err(), "{key}");
    }
}
#[test]
fn group14_child_journals_are_exact_ordered_unique_and_complete() {
    for change in 0..5 {
        let mut f = Fixture::new(Stage::Finish, Profile::Proof);
        let mut events = f.events();
        match change {
            0 => {
                events.remove(0);
            }
            1 => events.swap(0, 1),
            2 => events.push(events[0].clone()),
            3 => events[0]["program_sha256"] = json!("e".repeat(64)),
            _ => events[0]["process_group"] = json!(42),
        };
        f.rebuild(Some(events));
        assert!(f.grade().is_err());
    }
    let mut f = Fixture::new(Stage::ReadAfter, Profile::Proof);
    f.mutate(|h| {
        let id = h["context_before"]["_reader"]["operations"][0]["child_pid"].clone();
        h["artwork_reads"][0]["context_after"]["_reader"]["operations"][0]["child_pid"] =
            id.clone();
        h["artwork_reads"][0]["context_after"]["_reader"]["operations"][0]["process_group"] = id;
    });
    assert!(f.grade().is_err());
}
#[test]
fn group15_only_narrow_terminal_withdrawal_and_admitted_prefix_is_known() {
    for stage in [Stage::ReadAfter, Stage::Finish] {
        for change in 0..6 {
            let mut f = Fixture::new(stage, Profile::Proof);
            f.mutate(|h| {
                let context = if stage == Stage::ReadAfter {
                    &mut h["artwork_reads"][0]["context_after"]
                } else {
                    &mut h["artwork_finish_context"]
                };
                match change {
                    0 => context["windows"]["windows"] = json!([]),
                    1 => context["foreground"]["appId"] = json!(""),
                    2 => context["windows"]["windows"][0]["windowType"] = json!("unknown"),
                    3 => context["system_info"]["modelName"] = json!("OLED65C26LD"),
                    4 => context["_reader"]["cleanup_known"] = json!(false),
                    _ => {
                        context["foreground"]["appId"] = json!(Profile::Proof.app_id());
                        context["windows"]["windows"][0]["appId"] = json!(Profile::Proof.app_id());
                    }
                }
            });
            assert!(f.grade().is_err());
        }
    }
    let mut f = Fixture::new(Stage::ReadAfter, Profile::Proof);
    f.mutate(|h| withdrawn(&mut h["artwork_reads"][0]["context_before"]));
    assert!(f.grade().is_err());
    let mut f = Fixture::new(Stage::Finish, Profile::Proof);
    f.mutate(|h|{let c=&mut h["artwork_finish_context"];c["foreground"]["appId"]=json!(Profile::Proof.app_id());c["windows"]["windows"]=json!([{"appId":Profile::Proof.app_id(),"windowType":"_WEBOS_WINDOW_TYPE_CARD"},{"appId":"com.webos.app.keyboard","windowType":"_WEBOS_WINDOW_TYPE_KEYBOARD"}]);});
    assert!(f.grade().is_ok());
}

#[test]
fn group13_complete_core_schema_rejects_uniform_invalid_measurements() {
    for mode in [0o100700, 0o100755, 0o100555] {
        let mut f = Fixture::new(Stage::ReadAfter, Profile::Proof);
        let mut measured = measurement();
        measured["identity"]["mode"] = json!(mode);
        f.mutate(|helper| replace_reader_measurements(helper, &measured));
        assert!(f.grade().is_ok());
    }
    for bytes in [52, 8 * 1024 * 1024] {
        let mut f = Fixture::new(Stage::Finish, Profile::Preview);
        let mut measured = measurement();
        measured["identity"]["bytes"] = json!(bytes);
        f.mutate(|helper| replace_reader_measurements(helper, &measured));
        assert!(f.grade().is_ok());
    }
    for (key, value) in [
        ("device", json!(true)),
        ("inode", json!(0)),
        ("inode", json!(-1)),
        ("bytes", json!(51)),
        ("bytes", json!(8 * 1024 * 1024 + 1)),
        ("mode", json!(0o100777)),
        ("mode", json!(0o100600)),
        ("mode", json!(0o040700)),
        ("mode", json!(0o104700)),
        ("uid", json!(1000)),
        ("gid", json!(1000)),
        ("links", json!(0)),
        ("links", json!(2)),
        ("modified_seconds", json!("1")),
        ("changed_seconds", json!(true)),
        ("modified_nanos", json!(-1)),
        ("modified_nanos", json!(1_000_000_000)),
        ("changed_nanos", json!(-1)),
        ("changed_nanos", json!(1_000_000_000)),
    ] {
        let mut f = Fixture::new(Stage::Finish, Profile::Proof);
        let mut measured = measurement();
        measured["identity"][key] = value;
        f.mutate(|helper| replace_reader_measurements(helper, &measured));
        assert!(f.grade().is_err(), "Uniform invalid Core field: {key}");
    }
    for change in 0..5 {
        let mut f = Fixture::new(Stage::ReadAfter, Profile::Proof);
        let mut measured = measurement();
        match change {
            0 => {
                measured = json!({"sha256":"d".repeat(64)});
            }
            1 => {
                measured["identity"]
                    .as_object_mut()
                    .unwrap()
                    .remove("device");
            }
            2 => {
                measured["identity"]["extra"] = json!(true);
            }
            3 => {
                measured["extra"] = json!(false);
            }
            _ => {
                measured["sha256"] = json!("e".repeat(64));
            }
        }
        f.mutate(|helper| replace_reader_measurements(helper, &measured));
        assert!(f.grade().is_err());
    }
}
