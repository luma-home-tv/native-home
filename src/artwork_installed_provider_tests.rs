//! Controlled host receipts/packets and own temporary observation files only. No HBC or device.
use super::*;
use crate::{
    app_artwork::{self, FileIdentity},
    artwork_transfer as wire,
    broker_probe::OwnIdentity,
    broker_transport::Credentials,
};
use serde_json::{json, Value};
use std::sync::{
    atomic::{AtomicBool, Ordering},
    Arc,
};
pub(super) fn source() -> Source {
    let id = "synthetic.fixture.installed";
    let raw = json!({"replies":{"apps":{"returnValue":true,"apps":[{"id":id,"title":"Fixture","type":"native","visible":true,"version":"1.0.0","folderPath":format!("/media/developer/apps/usr/palm/applications/{id}"),"icon":"icon.png","largeIcon":"images/large.png"}]},"launch_points":{"returnValue":true,"launchPoints":[{"id":id}]}}});
    app_artwork::sources_from_raw(&raw, crate::identity::PROOF).unwrap()[id]
        .preferred()
        .unwrap()
        .clone()
}
pub(super) fn marker() -> Marker {
    Marker {
        schema: 1,
        nonce: "a".repeat(32),
        ui_sha256: "b".repeat(64),
        broker_sha256: "c".repeat(64),
    }
}
pub(super) fn context() -> Value {
    json!({"system_info":{"returnValue":true,"modelName":"OLED55G26LA","firmwareVersion":"23.25.55","sdkVersion":"9.2.2"},"foreground":{"returnValue":true,"appId":crate::identity::PROOF},"windows":{"returnValue":true,"windows":[{"appId":crate::identity::PROOF,"windowType":"_WEBOS_WINDOW_TYPE_CARD"}]}})
}
fn hex(bytes: &[u8]) -> String {
    use std::fmt::Write;
    let mut out = String::new();
    for byte in plx_base::sha256::sha256(bytes) {
        let _ = write!(&mut out, "{byte:02x}");
    }
    out
}
pub(super) fn base64(bytes: &[u8]) -> String {
    let table = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";
    let mut out = String::new();
    for b in bytes.chunks(3) {
        out.push(table[(b[0] >> 2) as usize] as char);
        out.push(
            table[(((b[0] & 3) << 4) | (b.get(1).copied().unwrap_or(0) >> 4)) as usize] as char,
        );
        out.push(if b.len() > 1 {
            table[(((b[1] & 15) << 2) | (b.get(2).copied().unwrap_or(0) >> 6)) as usize] as char
        } else {
            '='
        });
        out.push(if b.len() > 2 {
            table[(b[2] & 63) as usize] as char
        } else {
            '='
        });
    }
    out
}
pub(super) fn reader(catalogue: bool, next: &mut u64) -> Value {
    let context_ops = [
        (
            "system_info",
            "luna://com.webos.service.tv.systemproperty/getSystemInfo",
            r#"{"keys":["modelName","firmwareVersion","sdkVersion"]}"#,
        ),
        (
            "foreground",
            "luna://com.webos.applicationManager/getForegroundAppInfo",
            "{}",
        ),
        (
            "windows",
            "luna://com.webos.surfacemanager/getForegroundWindowInfo",
            "{}",
        ),
    ];
    let catalogue_ops = [
        ("apps", "luna://com.webos.applicationManager/listApps", "{}"),
        (
            "launch_points",
            "luna://com.webos.applicationManager/listLaunchPoints",
            "{}",
        ),
    ];
    let wanted: &[(&str, &str, &str)] = if catalogue {
        &catalogue_ops
    } else {
        &context_ops
    };
    let measurement = json!({"sha256":"d".repeat(64),"identity":{"synthetic_fixture":true}});
    let mut operations = Vec::new();
    for (key, uri, payload) in wanted {
        let pid = *next;
        *next += 1;
        operations.push(json!({"key":key,"uri":uri,"argv":["/usr/bin/luna-send","-i","-n","1","-w","5000","-f",uri,payload],"attempted":true,"success":true,"child_pid":pid,"process_group":pid,"exit_code":0,"signal":null,"timed_out":false,"error":null,"stdout_bytes":100,"stderr_bytes":0,"stderr_sha256":hex(&[]),"stdout_eof":true,"stderr_eof":true,"child_reaped":true,"cleanup_known":true,"held_empty_stdin":true,"stdin_bytes_sent":0,"group_cleanup_attempted":true,"group_kill_errno":null,"executable_before":measurement,"executable_after":measurement}));
    }
    let mut value = if catalogue {
        json!({"app_count":1,"launch_point_count":1})
    } else {
        context()
    };
    value["_reader"] = json!({"schema":1,"kind":"fixed_core_luna_send","program":"/usr/bin/luna-send","approved":true,"qualified":true,"cleanup_known":true,"error":null,"executable":measurement,"executable_after":measurement,"operations":operations});
    value
}
pub(super) fn sync(hbc: &mut Value) {
    sync_for_profile(hbc, Profile::Proof);
}
pub(super) fn sync_for_profile(hbc: &mut Value, profile: Profile) {
    let helper = &hbc["parsed"]["helper_report"];
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
        if value.is_null() {
            continue;
        }
        for row in value["_reader"]["operations"].as_array().unwrap() {
            events.push(json!({"schema":1,"stage":if catalogue{"catalogue_child_started"}else{"context_child_started"},"child_pid":row["child_pid"],"process_group":row["process_group"],"program_sha256":"d".repeat(64)}));
        }
    }
    let mut rows = events;
    rows.push(helper.clone());
    let stdout = rows
        .iter()
        .map(|row| format!("{row}\n"))
        .collect::<String>();
    let raw=json!({"returnValue":true,"stdoutString":stdout,"stdoutBytes":base64(stdout.as_bytes()),"stderrString":"","stderrBytes":""}).to_string();
    hbc["raw_bytes"] = json!(raw.len());
    hbc["raw_truncated"] = json!(false);
    hbc["raw"] = json!(raw.clone());
    hbc["parsed"] =
        crate::broker_startup_proof::parse_exec_resource_reply_for_profile(&raw, profile).unwrap();
}
pub(super) fn fixture() -> (InstalledProvider, Source, session::Exchange, Value) {
    let marker = marker();
    let source = source();
    let own = OwnIdentity {
        pid: 123,
        uid: 6583,
        start_ticks: 456,
        ui_sha256: marker.ui_sha256.clone(),
    };
    let root = Credentials {
        pid: 789,
        uid: 0,
        gid: 0,
    };
    let client = Credentials {
        pid: 123,
        uid: 6583,
        gid: 6583,
    };
    let identity = wire::Identity {
        schema: 1,
        op: crate::broker_protocol::Operation::Identify,
        nonce: marker.nonce.clone(),
        server_pid: 789,
        server_uid: 0,
        client_pid: 123,
        client_uid: 6583,
        client_start_ticks: 456,
        ui_sha256: marker.ui_sha256.clone(),
        broker_sha256: marker.broker_sha256.clone(),
    };
    let request =
        wire::Request::new(1, &marker.nonce, source.app_id(), wire::Variant::Preferred).unwrap();
    // Wire bytes are deliberately synthetic. This test does not qualify decode.
    let bytes = b"bounded encoded fixture bytes".to_vec();
    let file = EncodedFile {
        identity: FileIdentity {
            device: 1,
            inode: 12,
            bytes: bytes.len() as u64,
            mode: 0o100644,
            uid: 0,
            gid: 0,
            modified_seconds: 1,
            modified_nanos: 0,
            changed_seconds: 1,
            changed_nanos: 0,
        },
        digest: plx_base::sha256::sha256(&bytes),
        bytes,
        png_dimensions: None,
    };
    let header = wire::Header::from_file(&request, &identity, source.key(), &file).unwrap();
    let complete = wire::Complete::from_header(&header).unwrap();
    let finish_request = wire::FinishRequest::new(2, &marker.nonce).unwrap();
    let finished = wire::Finished::new(&finish_request, &identity).unwrap();
    let client_value = json!({"pid":123,"uid":6583,"gid":6583});
    let peer = json!({"pid":123,"uid_fields":[6583,6583,6583,6583],"start_ticks":456,"exe_path":crate::broker_protocol::PROOF_EXE_PATH,"root_path":crate::broker_protocol::PROOF_JAIL_ROOT,"exe_sha256":marker.ui_sha256});
    let mut next = 1001;
    let initial = reader(false, &mut next);
    let before = reader(false, &mut next);
    let cat_before = reader(true, &mut next);
    let cat_after = reader(true, &mut next);
    let after = reader(false, &mut next);
    let final_context = reader(false, &mut next);
    let record = json!({"request":request,"request_bytes":serde_json::to_vec(&request).unwrap(),"connect_credentials":client_value,"message_credentials":client_value,"peer_before":peer,"peer_after":peer,"context_before":before,"catalogue_before":cat_before,"catalogue_after":cat_after,"context_after":after,"source_key":source.key(),"source_key_after":source.key(),"header":header,"complete":complete,"completed":true,"encoded_data_released":true,"read_identity":header.file_identity,"readback_identity":header.file_identity,"encoded_sha256":header.encoded_sha256,"readback_sha256":header.encoded_sha256,"encoded_bytes":file.bytes.len()});
    let root_process = json!({"pid":789,"uid_fields":[0,0,0,0],"start_ticks":321,"root_path":"/","exe_path":crate::broker_startup_proof::COMMAND,"exe_sha256":marker.broker_sha256});
    let installation = json!({"profile":"proof","app_id":crate::identity::PROOF,"root_process":root_process,"pins":{"schema":1,"profile":"proof","rendezvous_nonce":marker.nonce,"ui_sha256":marker.ui_sha256,"broker_sha256":marker.broker_sha256,"context_reader_sha256":"d".repeat(64)},"files":{"helper":{"path":crate::broker_startup_proof::COMMAND,"sha256":marker.broker_sha256,"trusted_for_execution":true},"configuration":{"path":"/var/lib/luma-home-proof/broker-installation.json","trusted_for_execution":true},"ui":{"path":Profile::Proof.ui_path(),"sha256":marker.ui_sha256,"trusted_for_execution":false}}});
    let mut helper = json!({"qualified":true,"outcome":"identified_artwork_finished","resource_mode":true,"artwork_session_completed":true,"connection_closed":true,"response_sent":true,"finished_sent":true,"finished":finished,"artwork_finished":finished,"icon_reads":true,"server_pid":789,"root_process":root_process,"broker_sha256":marker.broker_sha256,"nonce":marker.nonce,"response":identity,"commands":false,"setting_writes":false,"root_file_writes":false,"daemonized":false,"normal_loader_qualified":false,"production_startup_qualified":false,"connect_credentials":client_value,"identify_message_credentials":client_value,"artwork_finish_connect_credentials":client_value,"artwork_finish_message_credentials":client_value,"peer_before":peer,"peer_at_identify_send":peer,"artwork_finish_peer_before":peer,"artwork_finish_peer_after":peer,"context_before":initial,"artwork_reads":[record],"artwork_finish_context":final_context,"installation":installation});
    helper["schema"] = json!(1);
    helper["stage"] = json!("startup_proof_complete");
    helper["profile"] = json!("proof");
    let cap = json!({"verified":true,"registered":true,"request_attempted":true,"check_root_calls":1,"uri":"luna://org.webosbrew.hbchannel.service/checkRoot","payload":{},"identity_stable":true,"operation_deadline_expired":false,"own_before":own,"own_at_call":own,"own_after":own,"context_before":context(),"context_after":context(),"response":crate::artwork_startup_probe::parse_response(r#"{"returnValue":true}"#)});
    let mut hbc = json!({"uri":crate::broker_startup_proof::EXEC_URI,"command":crate::broker_startup_proof::COMMAND,"attempts":1,"own_at_call":own,"context":context(),"capability":cap,"parsed":{"helper_report":helper}});
    sync(&mut hbc);
    let outcome = wire::Outcome::Artwork(Box::new(wire::EncodedArtwork {
        header,
        bytes: file.bytes,
        complete,
    }));
    let exchange = session::Exchange {
        own,
        root,
        message: root,
        client,
        identity,
        request,
        outcome,
        finished: Some(finished),
        clean_eof: true,
        header_matches: true,
    };
    (
        InstalledProvider {
            marker,
            profile: Profile::Proof,
            quarantined: false,
            ticket_resolved: false,
            observation: None,
            guard: None,
            last_attempt: None,
            cache_publisher: None,
        },
        source,
        exchange,
        hbc,
    )
}
// Convert only the role-specific synthetic fixture values. The source icon,
// encoded bytes, child journals and Core measurements remain independent of role.
fn preview_values(value: &mut Value, key: Option<&str>) {
    match value {
        Value::String(text) => {
            let replacements = [
                ("proof".to_owned(), "preview".to_owned()),
                (
                    Profile::Proof.app_id().to_owned(),
                    Profile::Preview.app_id().to_owned(),
                ),
                (
                    Profile::Proof.peer_exe_path(),
                    Profile::Preview.peer_exe_path(),
                ),
                (Profile::Proof.jail_root(), Profile::Preview.jail_root()),
                (Profile::Proof.ui_path(), Profile::Preview.ui_path()),
                (Profile::Proof.config_path(), Profile::Preview.config_path()),
                (
                    crate::broker_startup_proof::fixed_command(Profile::Proof).to_owned(),
                    crate::broker_startup_proof::fixed_command(Profile::Preview).to_owned(),
                ),
            ];
            if let Some((_, replacement)) = replacements
                .iter()
                .find(|(original, _)| original.as_str() == text.as_str())
            {
                *text = replacement.clone();
            }
        }
        Value::Number(number) if number.as_u64() == Some(6583) => {
            *value = json!(if key == Some("gid") { 5000 } else { 5724 });
        }
        Value::Array(values) => {
            for value in values {
                preview_values(value, None);
            }
        }
        Value::Object(values) => {
            for (key, value) in values {
                preview_values(value, Some(key));
            }
        }
        _ => (),
    }
}
pub(super) fn preview_fixture() -> (InstalledProvider, Source, session::Exchange, Value) {
    let (mut provider, source, mut exchange, mut hbc) = fixture();
    provider.profile = Profile::Preview;
    exchange.own.uid = 5724;
    exchange.client.uid = 5724;
    exchange.client.gid = 5000;
    exchange.identity.client_uid = 5724;
    preview_values(&mut hbc, None);
    let wire::Outcome::Artwork(artwork) = &mut exchange.outcome else {
        panic!("Synthetic artwork fixture")
    };
    artwork.header.identity = exchange.identity.clone();
    artwork.complete =
        wire::Complete::from_header_for_profile(&artwork.header, Profile::Preview).unwrap();
    hbc["parsed"]["helper_report"]["artwork_reads"][0]["header"] = json!(artwork.header);
    hbc["parsed"]["helper_report"]["artwork_reads"][0]["complete"] = json!(artwork.complete);
    let finish = wire::FinishRequest::new(2, &provider.marker.nonce).unwrap();
    exchange.finished = Some(
        wire::Finished::new_for_profile(&finish, &exchange.identity, Profile::Preview).unwrap(),
    );
    hbc["parsed"]["helper_report"]["finished"] = json!(exchange.finished);
    hbc["parsed"]["helper_report"]["artwork_finished"] = json!(exchange.finished);
    sync_for_profile(&mut hbc, Profile::Preview);
    (provider, source, exchange, hbc)
}

#[test]
fn explicit_preview_reserved_uid_closed_session_returns_only_bound_encoded_bytes() {
    let (mut provider, source, exchange, hbc) = preview_fixture();
    assert_eq!(exchange.own.uid, 5724);
    assert_eq!(exchange.client.gid, 5000);
    assert!(
        exchange.identity.validate().is_err(),
        "Old API remains Proof-only"
    );
    let file = session::settle(
        &mut provider,
        source.key(),
        Ok(exchange),
        Ok(hbc),
        false,
        false,
    )
    .unwrap();
    assert_eq!(file.bytes, b"bounded encoded fixture bytes");
    assert!(!provider.quarantined);
    assert!(
        provider.observation.is_none(),
        "Ordinary workers have no diagnostic cap/hold"
    );
}
#[test]
fn selected_profile_rejects_cross_role_command_pins_context_and_peer_paths() {
    for change in 0..8 {
        let (mut provider, source, exchange, mut hbc) = preview_fixture();
        match change {
            0 => hbc["command"] = json!(crate::broker_startup_proof::fixed_command(Profile::Proof)),
            1 => hbc["parsed"]["helper_report"]["installation"]["pins"]["profile"] = json!("proof"),
            2 => {
                hbc["parsed"]["helper_report"]["context_before"]["foreground"]["appId"] =
                    json!(Profile::Proof.app_id())
            }
            3 => {
                hbc["parsed"]["helper_report"]["peer_before"]["root_path"] =
                    json!(Profile::Proof.jail_root())
            }
            4 => {
                hbc["parsed"]["helper_report"]["peer_before"]["exe_path"] =
                    json!(Profile::Proof.peer_exe_path())
            }
            5 => {
                hbc["parsed"]["helper_report"]["installation"]["files"]["configuration"]["path"] =
                    json!(Profile::Proof.config_path())
            }
            6 => {
                hbc["parsed"]["helper_report"]["installation"]["files"]["ui"]["path"] =
                    json!(Profile::Proof.ui_path())
            }
            _ => {
                hbc["capability"]["context_after"]["windows"]["windows"][0]["appId"] =
                    json!(Profile::Proof.app_id())
            }
        }
        sync_for_profile(&mut hbc, Profile::Preview);
        assert!(
            session::settle(
                &mut provider,
                source.key(),
                Ok(exchange),
                Ok(hbc),
                false,
                false
            )
            .is_err(),
            "Cross-role variant {change}"
        );
        assert!(provider.quarantined);
    }
    let (mut provider, source, exchange, hbc) = preview_fixture();
    provider.profile = Profile::Proof;
    assert!(session::settle(
        &mut provider,
        source.key(),
        Ok(exchange),
        Ok(hbc),
        false,
        false
    )
    .is_err());
    let (mut provider, source, exchange, hbc) = fixture();
    provider.profile = Profile::Preview;
    assert!(session::settle(
        &mut provider,
        source.key(),
        Ok(exchange),
        Ok(hbc),
        false,
        false
    )
    .is_err());
}
#[test]
fn preview_wrong_source_or_unknown_child_lifecycle_never_returns_content() {
    let (mut provider, source, exchange, mut hbc) = preview_fixture();
    hbc["parsed"]["helper_report"]["artwork_reads"][0]["source_key_after"]["version"] =
        json!("changed");
    sync_for_profile(&mut hbc, Profile::Preview);
    assert!(matches!(
        session::settle(
            &mut provider,
            source.key(),
            Ok(exchange),
            Ok(hbc),
            false,
            false
        ),
        Err(ProviderError::Changed)
    ));
    assert!(
        !provider.quarantined,
        "A known closed transfer can still have a stale SourceKey"
    );
    let (mut provider, source, exchange, mut hbc) = preview_fixture();
    hbc["parsed"]["helper_report"]["context_before"]["_reader"]["cleanup_known"] = json!(false);
    sync_for_profile(&mut hbc, Profile::Preview);
    assert!(session::settle(
        &mut provider,
        source.key(),
        Ok(exchange),
        Ok(hbc),
        false,
        false
    )
    .is_err());
    assert!(provider.quarantined);
}
#[test]
fn preview_unknown_or_wrong_role_no_start_keeps_durable_pending() {
    for wrong_command in [false, true] {
        let directory = ObservationDirectory::new();
        let (mut provider, source, exchange, _) = preview_fixture();
        let own = guarded(&mut provider, &directory);
        let ticket = begin_pending(&mut provider, &own).unwrap();
        let completion = if wrong_command {
            Ok(json!({"uri":crate::broker_startup_proof::EXEC_URI,
            "command":crate::broker_startup_proof::fixed_command(Profile::Proof),"attempts":0,
            "raw":null,"parsed":null,"error":"No dispatch from different role"}))
        } else {
            Err(ProviderError::Unavailable)
        };
        let result = session::settle(
            &mut provider,
            source.key(),
            if wrong_command {
                Err(ProviderError::Unavailable)
            } else {
                Ok(exchange)
            },
            completion,
            false,
            false,
        );
        assert!(guard_result(&mut provider, ticket, result).is_err());
        assert!(provider.quarantined);
        assert!(provider.guard.as_ref().unwrap().pending().unwrap());
        let (mut restarted, _, _, _) = preview_fixture();
        let fresh = guarded(&mut restarted, &directory);
        assert!(begin_pending(&mut restarted, &fresh).is_err());
    }
}
#[test]
fn diagnostic_runtime_paths_are_fixed_by_role_and_ordinary_workers_have_no_ticket() {
    assert_eq!(
        ProofObservation::runtime_path(Profile::Proof, 123),
        PathBuf::from("/tmp/com.ash.nativehome.proof/shell-123")
    );
    assert_eq!(
        ProofObservation::runtime_path(Profile::Preview, 123),
        PathBuf::from("/tmp/com.ash.nativehome.preview/shell-123")
    );
    let (mut provider, _, _, _) = preview_fixture();
    assert!(proof_ticket(&mut provider).unwrap().is_none());
}
#[test]
fn complete_finished_and_hbc_agree_before_any_encoded_file_is_returned() {
    let (mut provider, source, exchange, hbc) = fixture();
    let result = session::settle(
        &mut provider,
        source.key(),
        Ok(exchange),
        Ok(hbc),
        false,
        false,
    )
    .unwrap();
    assert_eq!(result.digest, plx_base::sha256::sha256(&result.bytes));
    assert!(!provider.quarantined);
}
#[test]
fn real_receiver_withheld_complete_and_missing_finished_never_commit() {
    let (mut provider, source, exchange, hbc) = fixture();
    if let wire::Outcome::Artwork(artwork) = &exchange.outcome {
        let mut receiver =
            wire::Receiver::new(&exchange.request, &exchange.identity, exchange.root).unwrap();
        assert!(!receiver
            .push_packet(&serde_json::to_vec(&artwork.header).unwrap(), exchange.root)
            .unwrap());
        assert!(!receiver
            .push_packet(
                &wire::encode_chunk(1, 0, &artwork.bytes).unwrap(),
                exchange.root
            )
            .unwrap());
        assert!(receiver.finish().is_err());
    } else {
        panic!("fixture");
    }
    assert!(session::settle(
        &mut provider,
        source.key(),
        Err(ProviderError::Invalid),
        Ok(hbc),
        false,
        false
    )
    .is_err());
    assert!(provider.quarantined);
    let (mut provider, source, mut exchange, hbc) = fixture();
    exchange.finished = None;
    exchange.clean_eof = false;
    assert!(session::settle(
        &mut provider,
        source.key(),
        Ok(exchange),
        Ok(hbc),
        false,
        false
    )
    .is_err());
    assert!(provider.quarantined);
}
#[test]
fn full_source_key_drift_is_completed_changed_not_accepted_or_unknown() {
    for field in ["folder", "relative"] {
        let (mut provider, source, exchange, mut hbc) = fixture();
        hbc["parsed"]["helper_report"]["artwork_reads"][0]["source_key_after"][field] =
            if field == "folder" {
                json!("/different")
            } else {
                json!(["other.png"])
            };
        sync(&mut hbc);
        assert!(matches!(
            session::settle(
                &mut provider,
                source.key(),
                Ok(exchange),
                Ok(hbc),
                false,
                false
            ),
            Err(ProviderError::Changed)
        ));
        assert!(!provider.quarantined);
    }
}
#[test]
fn late_cancel_or_expiry_discards_bytes_after_known_finish_and_join() {
    for (cancelled, expired, wanted) in [
        (true, false, ProviderError::Cancelled),
        (false, true, ProviderError::Expired),
    ] {
        let (mut provider, source, exchange, hbc) = fixture();
        assert!(
            matches!(session::settle(&mut provider,source.key(),Ok(exchange),Ok(hbc),cancelled,expired),Err(value) if value==wanted)
        );
        assert!(!provider.quarantined);
    }
}
#[test]
fn typed_refusal_with_fourteen_known_children_is_not_unknown_startup() {
    let (mut provider, source, mut exchange, mut hbc) = fixture();
    let refusal = wire::Refusal::new(
        &exchange.request,
        &exchange.identity,
        wire::RefusalReason::Denied,
    )
    .unwrap();
    let record = &mut hbc["parsed"]["helper_report"]["artwork_reads"][0];
    record["refusal"] = json!(refusal);
    record["completed"] = json!(false);
    record["encoded_data_released"] = json!(false);
    for key in [
        "header",
        "complete",
        "catalogue_after",
        "source_key_after",
        "read_identity",
        "readback_identity",
        "encoded_sha256",
        "readback_sha256",
        "encoded_bytes",
    ] {
        record.as_object_mut().unwrap().remove(key);
    }
    exchange.outcome = wire::Outcome::Refused(refusal);
    sync(&mut hbc);
    assert!(matches!(
        session::settle(
            &mut provider,
            source.key(),
            Ok(exchange),
            Ok(hbc),
            false,
            false
        ),
        Err(ProviderError::Denied)
    ));
    assert!(!provider.quarantined);
}
#[test]
fn completion_join_and_unknown_child_outcomes_quarantine_without_retry() {
    let (mut provider, source, exchange, _) = fixture();
    assert!(session::settle(
        &mut provider,
        source.key(),
        Ok(exchange),
        Err(ProviderError::Unavailable),
        false,
        false
    )
    .is_err());
    assert!(provider.quarantined);
    let (mut provider, source, exchange, mut hbc) = fixture();
    hbc["parsed"]["helper_report"]["artwork_finish_context"]["_reader"]["cleanup_known"] =
        json!(false);
    sync(&mut hbc);
    assert!(session::settle(
        &mut provider,
        source.key(),
        Ok(exchange),
        Ok(hbc),
        false,
        false
    )
    .is_err());
    assert!(provider.quarantined);
    let closed = Arc::new(AtomicBool::new(false));
    let marker = closed.clone();
    let stop = Arc::new(AtomicBool::new(false));
    let worker_stop = stop.clone();
    {
        let handle = std::thread::spawn(move || {
            while !worker_stop.load(Ordering::Acquire) {
                std::thread::yield_now();
            }
            marker.store(true, Ordering::Release);
            Value::Null
        });
        let _guard = session::ExecWorker::new(handle, stop);
    }
    assert!(closed.load(Ordering::Acquire));
    let joined = Arc::new(AtomicBool::new(false));
    let witness = joined.clone();
    let handle = std::thread::spawn(move || {
        witness.store(true, Ordering::Release);
        json!({"completion":"joined"})
    });
    let completion = session::ExecWorker::new(handle, Arc::new(AtomicBool::new(false)))
        .join()
        .unwrap();
    assert_eq!(completion, json!({"completion":"joined"}));
    assert!(joined.load(Ordering::Acquire));
}
#[test]
fn resource_parser_does_not_widen_positive_or_negative_startup_parser() {
    let (_, _, _, hbc) = fixture();
    let raw = hbc["raw"].as_str().unwrap();
    assert!(crate::broker_startup_proof::parse_exec_resource_reply(raw).is_ok());
    assert!(crate::broker_startup_proof::parse_exec_reply(raw).is_err());
}
#[test]
fn constructor_failure_is_a_normal_worker_error_not_initializer_panic() {
    let mut bad = marker();
    bad.nonce = "invalid".into();
    let worker = InstalledProvider::start_worker(bad).unwrap();
    let result = worker.finish();
    assert!(result.joined);
    assert!(!result.panicked);
}

#[test]
fn refused_record_cannot_also_claim_a_completed_transfer() {
    let (mut provider, source, mut exchange, mut hbc) = fixture();
    let refusal = wire::Refusal::new(
        &exchange.request,
        &exchange.identity,
        wire::RefusalReason::Changed,
    )
    .unwrap();
    hbc["parsed"]["helper_report"]["artwork_reads"][0]["refusal"] = json!(refusal);
    hbc["parsed"]["helper_report"]["artwork_reads"][0]["completed"] = json!(false);
    // A valid late refusal may retain Header/released bytes, but never Complete.
    exchange.outcome = wire::Outcome::Refused(refusal);
    sync(&mut hbc);
    assert!(session::settle(
        &mut provider,
        source.key(),
        Ok(exchange),
        Ok(hbc),
        false,
        false
    )
    .is_err());
    assert!(provider.quarantined);
}

struct ObservationDirectory(std::path::PathBuf);
impl ObservationDirectory {
    fn new() -> Self {
        use std::os::unix::fs::DirBuilderExt;
        static NEXT: std::sync::atomic::AtomicUsize = std::sync::atomic::AtomicUsize::new(0);
        let path = std::env::temp_dir().join(format!(
            "luma-provider-observation-{}-{}",
            std::process::id(),
            NEXT.fetch_add(1, Ordering::Relaxed)
        ));
        std::fs::DirBuilder::new()
            .mode(0o700)
            .create(&path)
            .unwrap();
        Self(path)
    }
    fn observer(&self) -> ProofObservation {
        use std::os::unix::fs::MetadataExt;
        let metadata = std::fs::symlink_metadata(&self.0).unwrap();
        ProofObservation::open_exact(self.0.clone(), &self.0, metadata.uid(), metadata.gid())
            .unwrap()
    }
}
impl Drop for ObservationDirectory {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.0);
    }
}
fn guarded(provider: &mut InstalledProvider, directory: &ObservationDirectory) -> OwnIdentity {
    provider.guard = Some(
        crate::artwork_session_guard::Guard::open_for_test(&directory.0, provider.profile).unwrap(),
    );
    OwnIdentity {
        pid: std::process::id(),
        uid: if provider.profile == Profile::Preview {
            5724
        } else {
            6583
        },
        start_ticks: 456,
        ui_sha256: provider.marker.ui_sha256.clone(),
    }
}
#[test]
fn unknown_join_retains_pending_and_a_new_provider_cannot_dispatch_again() {
    let directory = ObservationDirectory::new();
    let (mut provider, source, exchange, _) = fixture();
    let own = guarded(&mut provider, &directory);
    let ticket = begin_pending(&mut provider, &own).unwrap();
    let result = session::settle(
        &mut provider,
        source.key(),
        Ok(exchange),
        Err(ProviderError::Unavailable),
        false,
        false,
    );
    assert!(guard_result(&mut provider, ticket, result).is_err());
    assert!(provider.quarantined);
    drop(provider);
    let (mut restarted, _, _, _) = fixture();
    let mut fresh_own = guarded(&mut restarted, &directory);
    restarted.marker.nonce = "f".repeat(32);
    fresh_own.start_ticks += 1;
    assert!(restarted.guard.as_ref().unwrap().pending().unwrap());
    assert!(begin_pending(&mut restarted, &fresh_own).is_err());
    assert!(restarted.quarantined);
    assert!(restarted.guard.as_ref().unwrap().pending().unwrap());
}
#[test]
fn only_known_closed_or_no_start_lifecycles_release_pending_without_touching_favourites() {
    for no_start in [false, true] {
        let directory = ObservationDirectory::new();
        let favourites = directory.0.join("preferences.json");
        std::fs::write(&favourites, b"unchanged favourites fixture").unwrap();
        let (mut provider, source, exchange, hbc) = fixture();
        let own = guarded(&mut provider, &directory);
        let ticket = begin_pending(&mut provider, &own).unwrap();
        let result = if no_start {
            session::settle(
                &mut provider,
                source.key(),
                Err(ProviderError::Unavailable),
                Ok(json!({"uri":crate::broker_startup_proof::EXEC_URI,
                    "command":crate::broker_startup_proof::COMMAND,"attempts":0,
                    "raw":null,"parsed":null,"error":"Before fixed dispatch"})),
                false,
                false,
            )
        } else {
            session::settle(
                &mut provider,
                source.key(),
                Ok(exchange),
                Ok(hbc),
                false,
                false,
            )
        };
        let result = guard_result(&mut provider, ticket, result);
        assert_eq!(result.is_ok(), !no_start);
        assert!(!provider.quarantined);
        assert!(!provider.guard.as_ref().unwrap().pending().unwrap());
        assert_eq!(
            std::fs::read(&favourites).unwrap(),
            b"unchanged favourites fixture"
        );
        let next = begin_pending(&mut provider, &own).unwrap();
        provider.quarantined = false; // Controlled known no-start in this host fixture.
        resolve_pending(&mut provider, next).unwrap();
    }
}
#[test]
fn pending_storage_failure_discards_even_a_completed_transfer() {
    let directory = ObservationDirectory::new();
    let (mut provider, source, exchange, hbc) = fixture();
    let own = guarded(&mut provider, &directory);
    let ticket = begin_pending(&mut provider, &own).unwrap();
    let completed = session::settle(
        &mut provider,
        source.key(),
        Ok(exchange),
        Ok(hbc),
        false,
        false,
    );
    assert!(completed.is_ok());
    std::fs::write(directory.0.join("artwork-session-pending.json"), b"changed").unwrap();
    assert!(guard_result(&mut provider, ticket, completed).is_err());
    assert!(provider.quarantined);
    assert!(provider.guard.as_ref().unwrap().pending().is_err());
    let (mut restarted, _, _, _) = fixture();
    let own = guarded(&mut restarted, &directory);
    assert!(begin_pending(&mut restarted, &own).is_err());
}
#[test]
fn unmeasured_or_missing_guard_cannot_reach_dispatch() {
    let directory = ObservationDirectory::new();
    let (mut provider, _, _, _) = fixture();
    let own = OwnIdentity {
        pid: std::process::id(),
        uid: if provider.profile == Profile::Preview {
            5724
        } else {
            6583
        },
        start_ticks: 456,
        ui_sha256: provider.marker.ui_sha256.clone(),
    };
    assert!(begin_pending(&mut provider, &own).is_err());
    assert!(provider.quarantined);
    for altered in 0..3 {
        let (mut provider, _, _, _) = fixture();
        let mut own = guarded(&mut provider, &directory);
        match altered {
            0 => own.pid += 1,
            1 => own.ui_sha256 = "0".repeat(64),
            _ => own.start_ticks = 0,
        }
        assert!(begin_pending(&mut provider, &own).is_err());
        assert!(provider.quarantined);
        assert!(!provider.guard.as_ref().unwrap().pending().unwrap());
    }
}
#[test]
fn proof_observation_names_are_fixed_and_bounded() {
    assert!(ProofObservation::filename(0, true).is_err());
    assert!(ProofObservation::filename(17, false).is_err());
    assert_eq!(
        ProofObservation::filename(1, true).unwrap(),
        "installed-artwork-1-connected.json"
    );
    assert_eq!(
        ProofObservation::filename(16, false).unwrap(),
        "installed-artwork-16-result.json"
    );
    let directory = ObservationDirectory::new();
    let observer = directory.observer();
    assert!(observer.write("../arbitrary.json", &json!({})).is_err());
    assert!(observer
        .write("installed-artwork-17-result.json", &json!({}))
        .is_err());
    assert_eq!(std::fs::read_dir(&directory.0).unwrap().count(), 0);
}
#[test]
fn proof_observation_directory_requires_exact_path_owner_mode_and_no_symlink() {
    use std::os::unix::fs::{symlink, MetadataExt, PermissionsExt};
    let directory = ObservationDirectory::new();
    let metadata = std::fs::symlink_metadata(&directory.0).unwrap();
    let open = |path: PathBuf, expected: &std::path::Path, uid: u32| {
        ProofObservation::open_exact(path, expected, uid, metadata.gid())
    };
    assert!(open(
        directory.0.clone(),
        &directory.0.join("different"),
        metadata.uid()
    )
    .is_err());
    assert!(open(
        directory.0.clone(),
        &directory.0,
        metadata.uid().wrapping_add(1)
    )
    .is_err());
    std::fs::set_permissions(&directory.0, std::fs::Permissions::from_mode(0o755)).unwrap();
    assert!(open(directory.0.clone(), &directory.0, metadata.uid()).is_err());
    std::fs::set_permissions(&directory.0, std::fs::Permissions::from_mode(0o700)).unwrap();
    let alias = directory.0.join("alias");
    symlink(&directory.0, &alias).unwrap();
    assert!(open(alias.clone(), &alias, metadata.uid()).is_err());
}
#[test]
fn proof_observation_receipts_are_private_create_new_and_do_not_follow_links() {
    use std::os::unix::fs::{symlink, MetadataExt};
    let directory = ObservationDirectory::new();
    let mut observer = directory.observer();
    let ticket = observer.begin().unwrap();
    let source = source();
    let progress = json!({"own":{"pid":123,"ui_sha256":"b".repeat(64)},"root_peer":{"pid":456,"uid":0,"gid":0}});
    observer.connected(ticket, source.key(), &progress).unwrap();
    let path = directory.0.join("installed-artwork-1-connected.json");
    let first = std::fs::read(&path).unwrap();
    let metadata = std::fs::symlink_metadata(&path).unwrap();
    assert_eq!(metadata.mode() & 0o7777, 0o600);
    assert_eq!(metadata.nlink(), 1);
    let receipt: Value = serde_json::from_slice(&first).unwrap();
    assert_eq!(receipt["source_key"], json!(source.key()));
    assert_eq!(receipt["own"], progress["own"]);
    assert_eq!(receipt["root_peer"], progress["root_peer"]);
    assert!(observer
        .connected(ticket, source.key(), &json!({}))
        .is_err());
    assert_eq!(std::fs::read(&path).unwrap(), first);
    let target = directory.0.join("unchanged");
    std::fs::write(&target, b"preserved").unwrap();
    symlink(&target, directory.0.join("installed-artwork-1-result.json")).unwrap();
    assert!(observer
        .result(ticket, &json!({"outcome":"invalid"}))
        .is_err());
    assert_eq!(std::fs::read(target).unwrap(), b"preserved");
}
#[test]
fn proof_observation_replacement_of_output_directory_is_rejected() {
    use std::os::unix::fs::DirBuilderExt;
    let directory = ObservationDirectory::new();
    let observer = directory.observer();
    let displaced = directory.0.with_extension("displaced");
    std::fs::rename(&directory.0, &displaced).unwrap();
    std::fs::DirBuilder::new()
        .mode(0o700)
        .create(&directory.0)
        .unwrap();
    assert!(observer
        .write("installed-artwork-1-result.json", &json!({}))
        .is_err());
    assert_eq!(std::fs::read_dir(&directory.0).unwrap().count(), 0);
    assert_eq!(std::fs::read_dir(&displaced).unwrap().count(), 0);
    std::fs::remove_dir(displaced).unwrap();
}
#[test]
fn proof_only_first_fetch_holds_and_seventeenth_denial_records_no_root_dispatch() {
    let directory = ObservationDirectory::new();
    let (mut provider, _, _, _) = fixture();
    provider.observation = Some(directory.observer());
    for index in 1..=16 {
        let ticket = proof_ticket(&mut provider).unwrap().unwrap();
        assert_eq!(ticket.index, index);
        assert_eq!(ticket.hold, index == 1);
    }
    assert!(matches!(
        proof_ticket(&mut provider),
        Err(ProviderError::Unavailable)
    ));
    assert!(!provider.quarantined);
    let path = directory.0.join("installed-artwork-cap.json");
    let bytes = std::fs::read(&path).unwrap();
    let receipt: Value = serde_json::from_slice(&bytes).unwrap();
    assert_eq!(receipt["started_fetches"], 16);
    assert_eq!(receipt["cap_reached"], true);
    assert_eq!(receipt["root_dispatch_requested"], false);
    assert_eq!(receipt["unknown_startup_claimed"], false);
    assert!(matches!(
        proof_ticket(&mut provider),
        Err(ProviderError::Unavailable)
    ));
    assert_eq!(std::fs::read(path).unwrap(), bytes);
    assert_eq!(std::fs::read_dir(&directory.0).unwrap().count(), 1);
    let (mut regular, _, _, _) = fixture();
    assert!(proof_ticket(&mut regular).unwrap().is_none());
    assert!(!regular.quarantined);
}
#[test]
fn observation_result_write_failure_quarantines_without_overwriting_original() {
    let directory = ObservationDirectory::new();
    let (mut provider, _, _, _) = fixture();
    provider.observation = Some(directory.observer());
    let ticket = proof_ticket(&mut provider).unwrap().unwrap();
    let first = json!({"schema":1,"outcome":"source_changed","exec_worker_joined":true,"encoded_image_bytes_logged":false});
    write_observed_result(&mut provider, ticket, &first).unwrap();
    assert!(write_observed_result(
        &mut provider,
        ticket,
        &json!({"outcome":"encoded_accepted"})
    )
    .is_err());
    assert!(provider.quarantined);
    let actual: Value = serde_json::from_slice(
        &std::fs::read(directory.0.join("installed-artwork-1-result.json")).unwrap(),
    )
    .unwrap();
    assert_eq!(actual, first);
}
#[test]
fn observation_outcomes_preserve_cancel_expiry_and_known_refusal_classes() {
    for (error, name) in [
        (ProviderError::Unavailable, "unavailable"),
        (ProviderError::Denied, "denied"),
        (ProviderError::Changed, "source_changed"),
        (ProviderError::Invalid, "invalid"),
        (ProviderError::Cancelled, "cancelled"),
        (ProviderError::Expired, "expired"),
    ] {
        assert_eq!(observed_outcome(&Err(error)), name);
    }
    let (mut provider, source, exchange, hbc) = fixture();
    let result = session::settle(
        &mut provider,
        source.key(),
        Ok(exchange),
        Ok(hbc),
        false,
        false,
    );
    assert_eq!(observed_outcome(&result), "encoded_accepted");
}

#[test]
fn framework_runtime_parent_is_sticky_shared_while_observation_leaf_stays_private() {
    use std::os::unix::fs::{MetadataExt, PermissionsExt};
    let directory = ObservationDirectory::new();
    let metadata = std::fs::symlink_metadata(&directory.0).unwrap();
    assert!(!ProofObservation::runtime_parent_valid(
        &metadata,
        metadata.uid(),
        metadata.gid()
    ));
    std::fs::set_permissions(&directory.0, std::fs::Permissions::from_mode(0o1777)).unwrap();
    let shared = std::fs::symlink_metadata(&directory.0).unwrap();
    assert!(ProofObservation::runtime_parent_valid(
        &shared,
        shared.uid(),
        shared.gid()
    ));
    assert!(ProofObservation::open_exact(
        directory.0.clone(),
        &directory.0,
        shared.uid(),
        shared.gid()
    )
    .is_err());
    std::fs::set_permissions(&directory.0, std::fs::Permissions::from_mode(0o777)).unwrap();
    let not_sticky = std::fs::symlink_metadata(&directory.0).unwrap();
    assert!(!ProofObservation::runtime_parent_valid(
        &not_sticky,
        not_sticky.uid(),
        not_sticky.gid()
    ));
}

// Foundation-only trace tests. All packets/identities are controlled host fixtures.
fn trace_for(exchange: &session::Exchange, source: &Source) -> AttemptTrace {
    let mut trace = AttemptTrace::new(Profile::Proof, source.key());
    trace.stage = AttemptStage::ReadArtwork;
    trace.own = Some(exchange.own.clone());
    trace.client = Some(exchange.client);
    trace.root = Some(exchange.root);
    trace.identify_message = Some(exchange.message);
    trace.identify_request = Some(crate::broker_protocol::Request {
        schema: 1,
        op: crate::broker_protocol::Operation::Identify,
        nonce: exchange.identity.nonce.clone(),
    });
    trace.identify_response = Some(exchange.identity.clone());
    trace.identity = Some(exchange.identity.clone());
    trace.identify_bound = true;
    trace.request = Some(exchange.request.clone());
    trace
}
fn fixture_packets(exchange: &session::Exchange) -> Vec<crate::broker_transport::Packet> {
    let wire::Outcome::Artwork(image) = &exchange.outcome else {
        panic!("Synthetic image fixture")
    };
    vec![
        serde_json::to_vec(&image.header).unwrap(),
        wire::encode_chunk(exchange.request.request_id, 0, &image.bytes).unwrap(),
        serde_json::to_vec(&image.complete).unwrap(),
    ]
    .into_iter()
    .map(|bytes| crate::broker_transport::Packet {
        bytes,
        credentials: exchange.root,
    })
    .collect()
}
#[test]
fn no_writer_still_retains_typed_failure_metadata_without_encoded_payload() {
    let (mut provider, source, exchange, _) = fixture();
    let mut observation = Observation {
        writer: None,
        ticket: None,
        trace: trace_for(&exchange, &source),
    };
    observation.connected(&source).unwrap();
    let mut receiver =
        wire::Receiver::new(&exchange.request, &exchange.identity, exchange.root).unwrap();
    let mut packets = fixture_packets(&exchange);
    assert!(!observation
        .trace
        .push_resource(&mut receiver, &packets.remove(0))
        .unwrap());
    observation.trace.fail(
        AttemptFailureKind::ProviderError,
        ProviderError::Unavailable,
    );
    retain_attempt(&mut provider, observation.trace);
    assert!(provider.observation.is_none());
    let retained = provider.last_attempt.as_ref().unwrap();
    assert_eq!(retained.identity.as_ref(), Some(&exchange.identity));
    assert!(retained.header.is_some());
    assert_eq!(retained.resource_packets_validated, 1);
    assert_eq!(retained.failure.unwrap().stage, AttemptStage::ReadArtwork);
    assert!(!retained.clean_eof);
    assert!(!provider.ticket_resolved);
}
#[test]
fn receiver_rejection_never_becomes_validated_header_or_progress() {
    let (_, source, exchange, _) = fixture();
    let mut trace = trace_for(&exchange, &source);
    let mut receiver =
        wire::Receiver::new(&exchange.request, &exchange.identity, exchange.root).unwrap();
    let mut packet = fixture_packets(&exchange).remove(0);
    let mut header: wire::Header = serde_json::from_slice(&packet.bytes).unwrap();
    header.identity.nonce = "f".repeat(32);
    packet.bytes = serde_json::to_vec(&header).unwrap();
    assert_eq!(
        trace.push_resource(&mut receiver, &packet),
        Err(ProviderError::Invalid)
    );
    assert_eq!(trace.resource_packets_received, 1);
    assert_eq!(trace.resource_packets_validated, 0);
    assert!(trace.header.is_none());
    assert!(trace.complete.is_none());
    assert!(trace.last_validated_credentials.is_none());
    assert!(!trace.report().as_object().unwrap().contains_key("header"));
}
#[test]
fn invalid_packet_retains_prior_valid_metadata_and_blocks_later_eof_read() {
    let (_, source, exchange, _) = fixture();
    let mut trace = trace_for(&exchange, &source);
    let mut receiver =
        wire::Receiver::new(&exchange.request, &exchange.identity, exchange.root).unwrap();
    let mut packets = fixture_packets(&exchange);
    trace
        .push_resource(&mut receiver, &packets.remove(0))
        .unwrap();
    packets[0].credentials.pid += 1;
    assert_eq!(
        trace.push_resource(&mut receiver, &packets[0]),
        Err(ProviderError::Invalid)
    );
    let first_failure = trace.failure;
    let called = std::cell::Cell::new(false);
    assert!(receive_attempt_packet(&mut trace, || {
        called.set(true);
        Ok(crate::broker_transport::Incoming::Eof)
    })
    .is_err());
    assert!(
        !called.get(),
        "A terminal failed attempt must not consume another message"
    );
    assert_eq!(trace.failure, first_failure);
    assert!(!trace.observed_unexpected_eof);
    assert_eq!(trace.resource_packets_received, 2);
    assert_eq!(trace.resource_packets_validated, 1);
    assert_eq!(trace.last_validated_credentials, Some(exchange.root));
    assert!(trace.header.is_some());
    assert!(trace.complete.is_none());
}
#[test]
fn typed_unexpected_eof_and_transport_error_still_settle_unknown() {
    for unexpected_eof in [false, true] {
        let (mut provider, source, exchange, hbc) = fixture();
        let mut trace = trace_for(&exchange, &source);
        let error = receive_attempt_packet(&mut trace, || {
            if unexpected_eof {
                Ok(crate::broker_transport::Incoming::Eof)
            } else {
                Err(std::io::ErrorKind::TimedOut.into())
            }
        })
        .unwrap_err();
        assert_eq!(error, ProviderError::Unavailable);
        assert_eq!(trace.observed_unexpected_eof, unexpected_eof);
        retain_attempt(&mut provider, trace);
        let settled = session::settle(
            &mut provider,
            source.key(),
            Err(error),
            Ok(hbc),
            false,
            false,
        );
        assert!(settled.is_err());
        assert!(provider.quarantined);
        assert!(!provider.ticket_resolved);
        assert_eq!(provider.transition_settlement(), SessionSettlement::Unknown);
        assert!(provider.last_attempt.as_ref().unwrap().failure.is_some());
    }
}
fn contains_fixture_payload(value: &Value, bytes: &[u8]) -> bool {
    match value {
        Value::Array(values) => {
            (values.len() == bytes.len()
                && values
                    .iter()
                    .zip(bytes)
                    .all(|(v, b)| v.as_u64() == Some(u64::from(*b))))
                || values.iter().any(|v| contains_fixture_payload(v, bytes))
        }
        Value::Object(values) => values.values().any(|v| contains_fixture_payload(v, bytes)),
        Value::String(value) => value.as_bytes() == bytes,
        _ => false,
    }
}
#[test]
fn complete_metadata_survives_generic_finish_failure_without_invented_eof() {
    let (mut provider, source, exchange, _) = fixture();
    let mut trace = trace_for(&exchange, &source);
    let mut receiver =
        wire::Receiver::new(&exchange.request, &exchange.identity, exchange.root).unwrap();
    for packet in fixture_packets(&exchange) {
        trace.push_resource(&mut receiver, &packet).unwrap();
    }
    assert!(matches!(
        receiver.finish().unwrap(),
        wire::Outcome::Artwork(_)
    ));
    trace.resource_complete_validated = Some(true);
    trace.finish_request = Some(wire::FinishRequest::new(2, &marker().nonce).unwrap());
    trace.stage = AttemptStage::Finish;
    trace.fail(
        AttemptFailureKind::ProviderError,
        ProviderError::Unavailable,
    );
    retain_attempt(&mut provider, trace);
    let report = provider.last_attempt.as_ref().unwrap().report();
    assert!(report["header"].is_object());
    assert!(report["complete"].is_object());
    assert!(report["finish_request"].is_object());
    assert_eq!(report["attempt_failure"]["stage"], "finish");
    assert_eq!(report["observed_unexpected_eof"], false);
    assert_eq!(report["clean_eof"], false);
    let wire::Outcome::Artwork(image) = &exchange.outcome else {
        panic!("Synthetic image fixture")
    };
    assert!(contains_fixture_payload(
        &json!({"accidental_payload":image.bytes}),
        &image.bytes
    ));
    assert!(contains_fixture_payload(
        &json!({"accidental_payload":"bounded encoded fixture bytes"}),
        &image.bytes
    ));
    assert!(!contains_fixture_payload(&report, &image.bytes));
}
#[test]
fn successful_trace_projection_preserves_existing_protocol_metadata() {
    let (_, source, exchange, _) = fixture();
    let mut trace = trace_for(&exchange, &source);
    let mut receiver =
        wire::Receiver::new(&exchange.request, &exchange.identity, exchange.root).unwrap();
    for packet in fixture_packets(&exchange) {
        trace.push_resource(&mut receiver, &packet).unwrap();
    }
    trace.resource_complete_validated = Some(true);
    trace.finish_request = Some(wire::FinishRequest::new(2, &marker().nonce).unwrap());
    trace.own_after = Some(exchange.own.clone());
    trace.finished = exchange.finished.clone();
    trace.clean_eof = true;
    trace.stage = AttemptStage::Closed;
    let report = trace.report();
    for (key, expected) in [
        ("own", json!(exchange.own)),
        ("own_after", json!(exchange.own)),
        (
            "root_peer",
            json!({"pid":exchange.root.pid,"uid":exchange.root.uid,"gid":exchange.root.gid}),
        ),
        ("identity", json!(exchange.identity)),
        ("identify_response", json!(exchange.identity)),
        ("request", json!(exchange.request)),
        ("finished", json!(exchange.finished)),
    ] {
        assert_eq!(report[key], expected, "Preserved protocol field {key}");
    }
    assert_eq!(report["resource_packets_received"], 3);
    assert_eq!(report["resource_packets_validated"], 3);
    assert_eq!(report["header_matches"], true);
    assert_eq!(report["resource_complete_validated"], true);
    assert_eq!(report["clean_eof"], true);
    assert!(report["attempt_failure"].is_null());
    assert_eq!(report["closed_negative_grader_available"], true);
    assert!(report.get("negative_acceptance_enabled").is_none());
}
#[test]
fn pre_identify_eof_and_all_stage_names_supply_no_session_authority() {
    let (_, source, _, _) = fixture();
    let mut trace = AttemptTrace::new(Profile::Proof, source.key());
    for (stage, name) in [
        (AttemptStage::AwaitStartup, "await_startup"),
        (
            AttemptStage::ExecWorkerNotStarted,
            "exec_worker_not_started",
        ),
        (AttemptStage::Connecting, "connecting"),
        (
            AttemptStage::ConnectedBeforeIdentify,
            "connected_before_identify",
        ),
        (AttemptStage::Identify, "identify"),
        (AttemptStage::ReadArtwork, "read_artwork"),
        (AttemptStage::Finish, "finish"),
        (AttemptStage::Closed, "closed"),
    ] {
        trace.stage = stage;
        let report = trace.report();
        assert_eq!(report["stage"], name);
        assert_eq!(report["closed_negative_grader_available"], true);
        assert!(report.get("negative_acceptance_enabled").is_none());
        assert!(report.get("identity").is_none());
        assert_eq!(
            report["clean_eof"], false,
            "A stage label cannot invent closure"
        );
    }
    trace.stage = AttemptStage::Identify;
    assert!(
        receive_attempt_packet(&mut trace, || Ok(crate::broker_transport::Incoming::Eof)).is_err()
    );
    let report = trace.report();
    assert_eq!(report["attempt_failure"]["stage"], "identify");
    assert_eq!(report["observed_unexpected_eof"], true);
    assert_eq!(report["clean_eof"], false);
    assert!(report.get("identity").is_none());
}

#[test]
fn next_fetch_preconditions_clear_prior_trace_without_inventing_attempt_or_settlement() {
    use crate::artwork_worker::Worker;
    use std::sync::mpsc;
    struct Held {
        cancellation: mpsc::Sender<Cancellation>,
        release: mpsc::Receiver<()>,
    }
    impl Provider for Held {
        fn fetch(
            &mut self,
            _: &Source,
            _: Instant,
            cancellation: &Cancellation,
        ) -> Result<EncodedFile, ProviderError> {
            self.cancellation.send(cancellation.clone()).unwrap();
            self.release.recv_timeout(Duration::from_secs(2)).unwrap();
            Err(ProviderError::Unavailable)
        }
    }
    let (send, receive) = mpsc::channel();
    let (release, wait) = mpsc::channel();
    let mut worker = Worker::start(move || Held {
        cancellation: send,
        release: wait,
    })
    .unwrap();
    let id = "synthetic.fixture.installed";
    let raw = json!({"replies":{"apps":{"returnValue":true,"apps":[{"id":id,"title":"Fixture","type":"native","visible":true,"version":"1.0.0","folderPath":format!("/media/developer/apps/usr/palm/applications/{id}"),"icon":"icon.png","largeIcon":"images/large.png"}]},"launch_points":{"returnValue":true,"launchPoints":[{"id":id}]}}});
    let sources = app_artwork::sources_from_raw(&raw, crate::identity::PROOF).unwrap();
    let mut cache =
        crate::artwork_cache::ArtworkCache::new(crate::artwork_cache::Config::default()).unwrap();
    cache.update_catalogue(&sources).unwrap();
    cache.set_interest(&[id.to_owned()], &[]).unwrap();
    let work = cache.next_work(0).unwrap();
    let source = sources[id].preferred().unwrap().clone();
    let future = Instant::now() + Duration::from_secs(2);
    worker.try_submit(work, source.clone(), future).unwrap();
    let cancellation = receive.recv_timeout(Duration::from_secs(2)).unwrap();
    assert!(!cancellation.is_cancelled());
    for (quarantined, deadline, expected) in [
        (true, future, ProviderError::Unavailable),
        (false, Instant::now(), ProviderError::Expired),
    ] {
        let (mut provider, fixture_source, exchange, _) = fixture();
        provider.last_attempt = Some(trace_for(&exchange, &fixture_source));
        provider.ticket_resolved = true; // Previous job's metadata, never current authority.
        provider.quarantined = quarantined;
        assert!(
            matches!(provider.fetch(&source, deadline, &cancellation), Err(error) if error == expected)
        );
        assert!(provider.last_attempt.is_none());
        assert!(!provider.ticket_resolved);
        assert_eq!(provider.quarantined, quarantined);
        assert_eq!(provider.transition_settlement(), SessionSettlement::Unknown);
    }
    assert!(worker.cancel());
    assert!(cancellation.is_cancelled());
    let (mut provider, fixture_source, exchange, _) = fixture();
    provider.last_attempt = Some(trace_for(&exchange, &fixture_source));
    provider.ticket_resolved = true;
    assert!(matches!(
        provider.fetch(&source, future, &cancellation),
        Err(ProviderError::Cancelled)
    ));
    assert!(provider.last_attempt.is_none());
    assert!(!provider.ticket_resolved);
    assert!(!provider.quarantined);
    assert_eq!(provider.transition_settlement(), SessionSettlement::Unknown);
    release.send(()).unwrap();
    let finished = worker.finish();
    assert!(finished.joined && !finished.panicked);
}

// The candidate controls observation lifetime only; it is not a closure grant.
fn read_eof_candidate() -> (InstalledProvider, Source, AttemptTrace) {
    let (provider, source, exchange, _) = fixture();
    let mut trace = trace_for(&exchange, &source);
    let mut receiver =
        wire::Receiver::new(&exchange.request, &exchange.identity, exchange.root).unwrap();
    let packets = fixture_packets(&exchange);
    for packet in packets.iter().take(2) {
        trace.push_resource(&mut receiver, packet).unwrap();
    }
    assert!(
        receive_attempt_packet(&mut trace, || Ok(crate::broker_transport::Incoming::Eof)).is_err()
    );
    trace.discarded_read = Some(receiver.discard_incomplete_after_eof().unwrap());
    trace.final_root_peer = Some(exchange.root);
    trace.own_after = Some(exchange.own);
    (provider, source, trace)
}
#[test]
fn authenticated_body_eof_is_candidate_only_and_projection_contains_no_encoded_body() {
    let (mut provider, source, trace) = read_eof_candidate();
    assert!(trace.bounded_eof_candidate());
    let report = trace.report();
    assert!(report["discarded_read"].is_object());
    assert!(!contains_fixture_payload(
        &report,
        b"bounded encoded fixture bytes"
    ));
    retain_attempt(&mut provider, trace);
    assert!(session::settle(
        &mut provider,
        source.key(),
        Err(ProviderError::Unavailable),
        Err(ProviderError::Unavailable),
        false,
        false
    )
    .is_err());
    assert!(provider.quarantined);
    assert_eq!(provider.transition_settlement(), SessionSettlement::Unknown);
}
#[test]
fn candidate_requires_final_kernel_peer_own_after_and_no_prior_failure() {
    let (_, _, trace) = read_eof_candidate();
    let mut changed = trace.clone();
    changed.final_root_peer.as_mut().unwrap().pid += 1;
    assert!(!changed.bounded_eof_candidate());
    changed = trace.clone();
    changed.own_after.as_mut().unwrap().start_ticks += 1;
    assert!(!changed.bounded_eof_candidate());
    changed = trace.clone();
    changed.discarded_read = None;
    assert!(!changed.bounded_eof_candidate());
    changed = trace.clone();
    changed.failure.as_mut().unwrap().kind = AttemptFailureKind::ReceiveError;
    assert!(!changed.bounded_eof_candidate());
    changed = trace.clone();
    changed.stage = AttemptStage::Identify;
    assert!(!changed.bounded_eof_candidate());
}
#[test]
fn only_bounded_candidate_preserves_existing_observer_until_join() {
    use std::sync::mpsc;
    for candidate in [false, true] {
        let (_, _, mut trace) = read_eof_candidate();
        if !candidate {
            trace.discarded_read = None;
        }
        let stop = Arc::new(AtomicBool::new(false));
        let observed = stop.clone();
        let (release, wait) = mpsc::channel();
        let handle = std::thread::spawn(move || {
            wait.recv().unwrap();
            json!({"stop_before_join":observed.load(Ordering::Acquire),"one_existing_worker":true})
        });
        let worker = session::ExecWorker::new(handle, stop.clone());
        worker.after_exchange_error(&trace);
        release.send(()).unwrap();
        let joined = worker.join().unwrap();
        assert_eq!(joined["stop_before_join"], !candidate);
        assert_eq!(joined["one_existing_worker"], true);
        // Drop after join may set stop, but cannot replace the observed result.
        assert!(stop.load(Ordering::Acquire));
    }
}
#[test]
fn invalid_packet_then_later_eof_cannot_keep_observer_alive() {
    let (_, source, exchange, _) = fixture();
    let mut trace = trace_for(&exchange, &source);
    let mut receiver =
        wire::Receiver::new(&exchange.request, &exchange.identity, exchange.root).unwrap();
    let mut packets = fixture_packets(&exchange);
    trace
        .push_resource(&mut receiver, &packets.remove(0))
        .unwrap();
    packets[0].credentials.uid += 1;
    assert!(trace.push_resource(&mut receiver, &packets[0]).is_err());
    assert!(receive_attempt_packet(&mut trace, || panic!(
        "Failed attempt cannot receive later EOF"
    ))
    .is_err());
    assert!(!trace.bounded_eof_candidate());
    let stop = Arc::new(AtomicBool::new(false));
    let handle = std::thread::spawn(|| json!({"not_a_closure_proof":true}));
    let worker = session::ExecWorker::new(handle, stop.clone());
    worker.after_exchange_error(&trace);
    assert!(stop.load(Ordering::Acquire));
    worker.join().unwrap();
}

fn current_negative_fixture(
    directory: &ObservationDirectory,
    finish: bool,
) -> (
    InstalledProvider,
    Source,
    AttemptTrace,
    Value,
    CurrentAttempt,
) {
    let (mut provider, source, mut trace, hbc, pending) =
        negative::tests::fixture_for_current(finish, Profile::Proof, std::process::id());
    provider.guard = Some(
        crate::artwork_session_guard::Guard::open_for_test(&directory.0, provider.profile).unwrap(),
    );
    let current = CurrentAttempt::begin(&mut provider, trace.own.as_ref().unwrap()).unwrap();
    trace.seal = current.seal().clone();
    assert_eq!(current.identity(), &pending);
    (provider, source, trace, hbc, current)
}
fn proof_for_current(
    provider: &InstalledProvider,
    current: &CurrentAttempt,
    source: &Source,
    trace: &AttemptTrace,
    hbc: &Value,
) -> negative::KnownClosed {
    negative::grade_closed_negative(negative::NegativeEvidence {
        trace,
        joined_hbc: hbc,
        marker: &provider.marker,
        profile: provider.profile,
        source: source.key(),
        current_pending: current.identity(),
    })
    .unwrap()
}
#[test]
fn group16_current_ticket_only_clears_after_opaque_proof_and_preserves_preferences() {
    for finish in [false, true] {
        let directory = ObservationDirectory::new();
        let preferences = directory.0.join("preferences.json");
        std::fs::write(&preferences, b"synthetic untouched favourites").unwrap();
        let before = std::fs::read(&preferences).unwrap();
        let (mut provider, source, trace, hbc, current) =
            current_negative_fixture(&directory, finish);
        retain_attempt(&mut provider, trace);
        assert!(provider.guard.as_ref().unwrap().pending().unwrap());
        assert_eq!(provider.transition_settlement(), SessionSettlement::Unknown);
        let result =
            settle_closed_negative(&mut provider, current, source.key(), Ok(hbc), false, false);
        assert!(matches!(result, Err(ProviderError::Denied)));
        assert!(provider.ticket_resolved);
        assert!(!provider.quarantined);
        assert!(!provider.guard.as_ref().unwrap().pending().unwrap());
        assert_eq!(
            provider.transition_settlement(),
            SessionSettlement::KnownClosed
        );
        assert_eq!(std::fs::read(&preferences).unwrap(), before);
    }
}
#[test]
fn group16_old_trace_and_hbc_cannot_clear_new_ticket_even_with_equal_pending_identity() {
    let directory = ObservationDirectory::new();
    let (mut provider, source, trace, hbc, first) = current_negative_fixture(&directory, false);
    let stale = proof_for_current(&provider, &first, &source, &trace, &hbc);
    let legitimate = proof_for_current(&provider, &first, &source, &trace, &hbc);
    let old_identity = first.identity().clone();
    first.resolve_negative(&mut provider, legitimate).unwrap();
    let next = CurrentAttempt::begin(&mut provider, trace.own.as_ref().unwrap()).unwrap();
    assert_eq!(
        next.identity(),
        &old_identity,
        "Persistent comparison fields can be identical across jobs"
    );
    let marker = directory.0.join("artwork-session-pending.json");
    let before = std::fs::read(&marker).unwrap();
    assert!(matches!(
        next.resolve_negative(&mut provider, stale),
        Err(ProviderError::Denied)
    ));
    assert_eq!(std::fs::read(&marker).unwrap(), before);
    assert!(provider.quarantined);
    assert!(!provider.ticket_resolved);
    assert_eq!(provider.transition_settlement(), SessionSettlement::Unknown);
}
#[test]
fn group16_missing_captured_ticket_cannot_be_replaced_by_pending_comparison_data() {
    let directory = ObservationDirectory::new();
    let (mut provider, source, trace, hbc, mut current) =
        current_negative_fixture(&directory, false);
    let proof = proof_for_current(&provider, &current, &source, &trace, &hbc);
    drop(current.captured.take());
    assert!(current.resolve_negative(&mut provider, proof).is_err());
    assert!(provider.guard.as_ref().unwrap().pending().unwrap());
    assert!(provider.quarantined);
    assert!(!provider.ticket_resolved);
}
#[test]
fn group16_storage_failure_before_or_after_unlink_never_claims_known_settlement() {
    for after_unlink in [false, true] {
        let directory = ObservationDirectory::new();
        let preferences = directory.0.join("preferences.json");
        std::fs::write(&preferences, b"unchanged synthetic favourites").unwrap();
        let (mut provider, source, trace, hbc, current) =
            current_negative_fixture(&directory, false);
        let proof = proof_for_current(&provider, &current, &source, &trace, &hbc);
        let result = current.resolve_with(&mut provider, proof, |ticket| {
            if after_unlink {
                ticket.finish_known()?;
            } else {
                drop(ticket);
            }
            // Models a failing storage operation, including failure after a
            // successful unlink. Marker survival is deliberately not promised.
            Err(std::io::Error::other("Controlled storage failure"))
        });
        assert!(result.is_err());
        assert!(provider.quarantined);
        assert!(!provider.ticket_resolved);
        assert_eq!(provider.transition_settlement(), SessionSettlement::Unknown);
        assert_eq!(
            provider.guard.as_ref().unwrap().pending().unwrap(),
            !after_unlink
        );
        assert_eq!(
            std::fs::read(&preferences).unwrap(),
            b"unchanged synthetic favourites"
        );
    }
}
#[test]
fn group17_cancellation_or_expiry_after_known_closure_releases_no_encoded_file() {
    for (cancelled, expired, expected) in [
        (true, false, ProviderError::Cancelled),
        (false, true, ProviderError::Expired),
    ] {
        let directory = ObservationDirectory::new();
        let (mut provider, source, trace, hbc, current) =
            current_negative_fixture(&directory, true);
        assert!(trace.complete.is_some());
        retain_attempt(&mut provider, trace);
        let result = settle_closed_negative(
            &mut provider,
            current,
            source.key(),
            Ok(hbc),
            cancelled,
            expired,
        );
        assert!(matches!(result,Err(error) if error==expected));
        assert_eq!(
            provider.transition_settlement(),
            SessionSettlement::KnownClosed
        );
        assert!(provider.ticket_resolved);
    }
}
#[test]
fn group17_pending_is_not_cleared_before_existing_worker_join_and_full_grade() {
    use std::sync::mpsc;
    let directory = ObservationDirectory::new();
    let (mut provider, source, trace, hbc, current) = current_negative_fixture(&directory, false);
    let stop = Arc::new(AtomicBool::new(false));
    let worker_stop = stop.clone();
    let joined = Arc::new(AtomicBool::new(false));
    let joined_witness = joined.clone();
    let (release, wait) = mpsc::channel();
    let handle = std::thread::spawn(move || {
        wait.recv_timeout(Duration::from_secs(2)).unwrap();
        assert!(!worker_stop.load(Ordering::Acquire));
        joined_witness.store(true, Ordering::Release);
        hbc
    });
    let worker = session::ExecWorker::new(handle, stop);
    worker.after_exchange_error(&trace);
    retain_attempt(&mut provider, trace);
    assert!(!joined.load(Ordering::Acquire));
    assert!(provider.guard.as_ref().unwrap().pending().unwrap());
    assert_eq!(provider.transition_settlement(), SessionSettlement::Unknown);
    release.send(()).unwrap();
    let completion = worker.join();
    assert!(joined.load(Ordering::Acquire));
    assert!(provider.guard.as_ref().unwrap().pending().unwrap());
    assert!(matches!(
        settle_closed_negative(
            &mut provider,
            current,
            source.key(),
            completion,
            false,
            false
        ),
        Err(ProviderError::Denied)
    ));
    assert_eq!(
        provider.transition_settlement(),
        SessionSettlement::KnownClosed
    );
}
#[test]
fn group17_join_failure_or_invalid_terminal_retains_current_pending_and_quarantine() {
    for panic_worker in [false, true] {
        let directory = ObservationDirectory::new();
        let (mut provider, source, trace, _, current) = current_negative_fixture(&directory, false);
        let worker = session::ExecWorker::new(
            std::thread::spawn(move || {
                if panic_worker {
                    panic!("Controlled observer panic");
                }
                json!({"not_the_required_joined_hbc":true})
            }),
            Arc::new(AtomicBool::new(false)),
        );
        worker.after_exchange_error(&trace);
        retain_attempt(&mut provider, trace);
        let completion = worker.join();
        assert!(settle_closed_negative(
            &mut provider,
            current,
            source.key(),
            completion,
            false,
            false
        )
        .is_err());
        assert!(provider.guard.as_ref().unwrap().pending().unwrap());
        assert!(provider.quarantined);
        assert!(!provider.ticket_resolved);
        assert_eq!(provider.transition_settlement(), SessionSettlement::Unknown);
    }
}

#[test]
fn group17_cancelled_token_does_not_erase_bounded_terminal_eof_authentication_or_join() {
    use std::sync::mpsc;
    struct Held {
        token: mpsc::Sender<Cancellation>,
        release: mpsc::Receiver<()>,
    }
    impl Provider for Held {
        fn fetch(
            &mut self,
            _: &Source,
            _: Instant,
            token: &Cancellation,
        ) -> Result<EncodedFile, ProviderError> {
            self.token.send(token.clone()).unwrap();
            self.release.recv_timeout(Duration::from_secs(2)).unwrap();
            Err(ProviderError::Cancelled)
        }
    }
    let (send, receive) = mpsc::channel();
    let (release, wait) = mpsc::channel();
    let mut held = crate::artwork_worker::Worker::start(move || Held {
        token: send,
        release: wait,
    })
    .unwrap();
    let id = "synthetic.fixture.installed";
    let raw = json!({"replies":{"apps":{"returnValue":true,"apps":[{"id":id,"title":"Fixture","type":"native","visible":true,"version":"1.0.0","folderPath":format!("/media/developer/apps/usr/palm/applications/{id}"),"icon":"icon.png","largeIcon":"images/large.png"}]},"launch_points":{"returnValue":true,"launchPoints":[{"id":id}]}}});
    let sources = app_artwork::sources_from_raw(&raw, crate::identity::PROOF).unwrap();
    let mut cache =
        crate::artwork_cache::ArtworkCache::new(crate::artwork_cache::Config::default()).unwrap();
    cache.update_catalogue(&sources).unwrap();
    cache.set_interest(&[id.to_owned()], &[]).unwrap();
    held.try_submit(
        cache.next_work(0).unwrap(),
        sources[id].preferred().unwrap().clone(),
        Instant::now() + Duration::from_secs(30),
    )
    .unwrap();
    let cancellation = receive.recv_timeout(Duration::from_secs(2)).unwrap();
    assert!(held.cancel());
    assert!(cancellation.is_cancelled());
    let directory = ObservationDirectory::new();
    let (mut provider, source, mut trace, hbc, current) =
        current_negative_fixture(&directory, false);
    trace.final_root_peer = None;
    trace.own_after = None;
    let root = trace.root.unwrap();
    let own = trace.own.clone().unwrap();
    authenticate_eof(
        &mut trace,
        &provider.marker,
        Instant::now() + Duration::from_secs(30),
        || Ok(root),
        || Ok(own),
    )
    .unwrap();
    assert!(trace.bounded_eof_candidate());
    let stop = Arc::new(AtomicBool::new(false));
    let observed = stop.clone();
    let worker = session::ExecWorker::new(
        std::thread::spawn(move || {
            assert!(!observed.load(Ordering::Acquire));
            hbc
        }),
        stop.clone(),
    );
    worker.after_exchange_error(&trace);
    assert!(!stop.load(Ordering::Acquire));
    retain_attempt(&mut provider, trace);
    let joined = worker.join();
    assert!(matches!(
        settle_closed_negative(
            &mut provider,
            current,
            source.key(),
            joined,
            cancellation.is_cancelled(),
            false
        ),
        Err(ProviderError::Cancelled)
    ));
    assert_eq!(
        provider.transition_settlement(),
        SessionSettlement::KnownClosed
    );
    release.send(()).unwrap();
    assert!(held.finish().joined);
}
#[test]
fn terminal_eof_authentication_still_refuses_expired_deadline_before_measurement() {
    let (provider, _, mut trace) = read_eof_candidate();
    trace.final_root_peer = None;
    trace.own_after = None;
    let called = std::cell::Cell::new(false);
    assert!(matches!(
        authenticate_eof(
            &mut trace,
            &provider.marker,
            Instant::now(),
            || {
                called.set(true);
                Err(ProviderError::Unavailable)
            },
            || panic!("Expired auth must not measure own identity")
        ),
        Err(ProviderError::Expired)
    ));
    assert!(!called.get());
    assert!(!trace.bounded_eof_candidate());
}

#[test]
fn terminal_eof_authentication_rejects_changed_measured_peer_or_self_without_candidate() {
    for change in 0..4 {
        let (provider, _, mut trace) = read_eof_candidate();
        trace.final_root_peer = None;
        trace.own_after = None;
        let mut root = trace.root.unwrap();
        let mut own = trace.own.clone().unwrap();
        match change {
            0 => root.pid += 1,
            1 => own.pid += 1,
            2 => own.start_ticks += 1,
            _ => own.ui_sha256 = "f".repeat(64),
        }
        assert!(authenticate_eof(
            &mut trace,
            &provider.marker,
            Instant::now() + Duration::from_secs(30),
            || Ok(root),
            || Ok(own)
        )
        .is_err());
        assert!(trace.final_root_peer.is_none());
        assert!(trace.own_after.is_none());
        assert!(!trace.bounded_eof_candidate());
    }
}

#[test]
fn closed_recipe_caps_one_job_and_keeps_ordinary_connected_hold() {
    let directory = ObservationDirectory::new();
    let mut ordinary = directory.observer();
    assert!(ordinary.begin().unwrap().hold);
    assert!(ordinary.begin().is_ok());
    let mut recipe = directory.observer();
    recipe.closed_case = Some(crate::artwork_closed_negative_proof::Case::ReadAfter);
    assert!(!recipe.begin().unwrap().hold);
    assert!(matches!(recipe.begin(), Err(ObservationError::Limit)));
    assert!(!directory.0.join("installed-artwork-cap.json").exists());
}
#[test]
fn closed_progress_publication_is_fixed_private_and_ordinary_mode_cannot_write_it() {
    use crate::artwork_closed_negative_proof::ProgressFile;
    use std::os::unix::fs::MetadataExt;
    let directory = ObservationDirectory::new();
    let mut observer = directory.observer();
    assert!(observer
        .write(ProgressFile::ReadBodyReady.name(), &json!({"schema":1}))
        .is_err());
    observer.closed_case = Some(crate::artwork_closed_negative_proof::Case::ReadAfter);
    observer
        .write(
            ProgressFile::ReadBodyReady.name(),
            &json!({"schema":1,"body_digest_validated":false,"complete_validated":false}),
        )
        .unwrap();
    let path = directory.0.join(ProgressFile::ReadBodyReady.name());
    let metadata = std::fs::symlink_metadata(&path).unwrap();
    assert_eq!(metadata.mode() & 0o7777, 0o600);
    assert_eq!(metadata.nlink(), 1);
    let before = std::fs::read(&path).unwrap();
    assert!(observer
        .write(ProgressFile::ReadBodyReady.name(), &json!({"schema":2}))
        .is_err());
    assert_eq!(std::fs::read(path).unwrap(), before);
}
#[test]
fn closed_recipe_wrong_target_and_second_fetch_cannot_reach_inner_provider() {
    fn work(id: &str) -> (crate::artwork_cache::Work, Source) {
        let raw = json!({"replies":{"apps":{"returnValue":true,"apps":[{"id":id,"title":"Fixture","type":"native","visible":true,"version":"1.0.0","folderPath":format!("/media/developer/apps/usr/palm/applications/{id}"),"icon":"icon.png"}]},"launch_points":{"returnValue":true,"launchPoints":[{"id":id}]}}});
        let sources = app_artwork::sources_from_raw(&raw, crate::identity::PROOF).unwrap();
        let mut cache =
            crate::artwork_cache::ArtworkCache::new(crate::artwork_cache::Config::default())
                .unwrap();
        cache.update_catalogue(&sources).unwrap();
        (
            cache.next_work(0).unwrap(),
            sources[id].preferred().unwrap().clone(),
        )
    }
    let (tx, rx) = std::sync::mpsc::sync_channel(1);
    let mut worker = crate::artwork_worker::Worker::start(move || ClosedNegativeProvider {
        inner: Init(Err(ProviderError::Unavailable)),
        case: crate::artwork_closed_negative_proof::Case::Finish,
        sent: false,
        observation: tx,
    })
    .unwrap();
    for id in ["synthetic.fixture.wrong", crate::identity::TARGET] {
        let (work, source) = work(id);
        worker
            .try_submit(work, source, Instant::now() + Duration::from_secs(2))
            .unwrap();
        let deadline = Instant::now() + Duration::from_secs(2);
        let done = loop {
            if let Some(done) = worker.poll() {
                break done;
            }
            assert!(Instant::now() < deadline);
            std::thread::yield_now();
        };
        assert_eq!(
            done.status,
            crate::artwork_worker::Status::ProviderFailed(ProviderError::Denied)
        );
        assert_eq!(done.settlement, SessionSettlement::Unknown);
        assert!(rx.try_recv().is_err());
    }
    assert!(worker.finish().joined);
}
#[test]
fn closed_recipe_discards_positive_file_before_actual_worker_decoder() {
    struct DropBeforeDecode(Option<EncodedFile>);
    impl Provider for DropBeforeDecode {
        fn initial_settlement(&self) -> SessionSettlement {
            SessionSettlement::NoExternalSession
        }
        fn transition_settlement(&self) -> SessionSettlement {
            SessionSettlement::KnownClosed
        }
        fn fetch(
            &mut self,
            _: &Source,
            _: Instant,
            _: &Cancellation,
        ) -> Result<EncodedFile, ProviderError> {
            let (result, outcome) = discard_closed_recipe_result(Ok(self.0.take().unwrap()), false);
            assert_eq!(outcome, ClosedNegativeOutcome::PositiveTimingMiss);
            result
        }
    }
    let (_, _, exchange, _) = fixture();
    let wire::Outcome::Artwork(image) = exchange.outcome else {
        panic!("synthetic file")
    };
    let encoded = EncodedFile {
        identity: FileIdentity {
            device: 1,
            inode: 1,
            bytes: image.bytes.len() as u64,
            mode: 0o100644,
            uid: 0,
            gid: 0,
            modified_seconds: 0,
            modified_nanos: 0,
            changed_seconds: 0,
            changed_nanos: 0,
        },
        digest: plx_base::sha256::sha256(&image.bytes),
        bytes: image.bytes,
        png_dimensions: None,
    };
    let source = source();
    let id = source.app_id().to_owned();
    let raw = json!({"replies":{"apps":{"returnValue":true,"apps":[{"id":id,"title":"Fixture","type":"native","visible":true,"version":"1.0.0","folderPath":format!("/media/developer/apps/usr/palm/applications/{id}"),"icon":"icon.png","largeIcon":"images/large.png"}]},"launch_points":{"returnValue":true,"launchPoints":[{"id":id}]}}});
    let sources = app_artwork::sources_from_raw(&raw, crate::identity::PROOF).unwrap();
    let mut cache =
        crate::artwork_cache::ArtworkCache::new(crate::artwork_cache::Config::default()).unwrap();
    cache.update_catalogue(&sources).unwrap();
    let work = cache.next_work(0).unwrap();
    let mut worker =
        crate::artwork_worker::Worker::start(move || DropBeforeDecode(Some(encoded))).unwrap();
    worker
        .try_submit(work, source, Instant::now() + Duration::from_secs(2))
        .unwrap();
    let deadline = Instant::now() + Duration::from_secs(2);
    let completed = loop {
        if let Some(done) = worker.poll() {
            break done;
        }
        assert!(Instant::now() < deadline);
        std::thread::yield_now();
    };
    assert_eq!(
        completed.status,
        crate::artwork_worker::Status::ProviderFailed(ProviderError::Denied)
    );
    assert!(matches!(
        completed.result,
        crate::artwork_cache::FetchResult::Failed
    ));
    assert_eq!(completed.settlement, SessionSettlement::KnownClosed);
    assert!(worker.finish().joined);
}

#[test]
fn closed_read_progress_requires_all_validated_chunks_and_is_not_complete_or_digest_proof() {
    use crate::artwork_closed_negative_proof::Case;
    let (_, source, exchange, _) = fixture();
    let mut trace = trace_for(&exchange, &source);
    let mut receiver =
        wire::Receiver::new(&exchange.request, &exchange.identity, exchange.root).unwrap();
    let packets = fixture_packets(&exchange);
    assert!(!closed_read_body_ready(Some(Case::ReadAfter), &trace));
    trace.push_resource(&mut receiver, &packets[0]).unwrap();
    assert!(!closed_read_body_ready(Some(Case::ReadAfter), &trace));
    trace.push_resource(&mut receiver, &packets[1]).unwrap();
    assert!(closed_read_body_ready(Some(Case::ReadAfter), &trace));
    assert!(!closed_read_body_ready(Some(Case::Finish), &trace));
    assert!(!closed_read_body_ready(None, &trace));
    assert!(trace.complete.is_none());
    assert!(trace.discarded_read.is_none());
    trace.push_resource(&mut receiver, &packets[2]).unwrap();
    assert!(!closed_read_body_ready(Some(Case::ReadAfter), &trace));
}

#[test]
fn closed_recipe_reports_only_its_selected_negative_stage_without_weakening_generic_closure() {
    use crate::artwork_closed_negative_proof::Case;
    for (finish, expected, other) in [
        (false, Case::ReadAfter, Case::Finish),
        (true, Case::Finish, Case::ReadAfter),
    ] {
        let (_, _, trace, _, _) =
            negative::tests::fixture_for_current(finish, Profile::Proof, std::process::id());
        assert!(trace.bounded_eof_candidate());
        assert!(closed_case_stage_matches(expected, &trace));
        assert!(!closed_case_stage_matches(other, &trace));
        let mut missing = trace.clone();
        missing.failure = None;
        assert!(!closed_case_stage_matches(expected, &missing));
        let mut mismatched = trace.clone();
        mismatched.failure.as_mut().unwrap().stage = AttemptStage::Identify;
        assert!(!closed_case_stage_matches(expected, &mismatched));
        let directory = ObservationDirectory::new();
        let (mut provider, source, trace, hbc, current) =
            current_negative_fixture(&directory, finish);
        retain_attempt(&mut provider, trace);
        assert!(matches!(
            settle_closed_negative(&mut provider, current, source.key(), Ok(hbc), false, false),
            Err(ProviderError::Denied)
        ));
        assert_eq!(
            provider.transition_settlement(),
            SessionSettlement::KnownClosed
        );
        assert!(closed_negative_for_case(expected, &provider));
        assert!(!closed_negative_for_case(other, &provider));
        let (_, matching) = discard_closed_recipe_result(
            Err(ProviderError::Denied),
            closed_negative_for_case(expected, &provider),
        );
        let (_, cross_case) = discard_closed_recipe_result(
            Err(ProviderError::Denied),
            closed_negative_for_case(other, &provider),
        );
        assert_eq!(matching, ClosedNegativeOutcome::ClosedNegative);
        assert_eq!(cross_case, ClosedNegativeOutcome::Unknown);
        assert_eq!(
            provider.transition_settlement(),
            SessionSettlement::KnownClosed
        );
    }
}

// Optional ordinary publication component tests. Real current Guard/Ticket
// files; positive exchange/HBC are controlled fixtures, never native proof.
fn cache_expected(
    directory: &ObservationDirectory,
    provider: &InstalledProvider,
) -> crate::cache_producer_profile::ExpectedProducer {
    use std::os::unix::fs::PermissionsExt;
    let path = directory.0.join("artwork-cache-producer.json");
    std::fs::write(
        &path,
        serde_json::to_vec(&json!({"schema":1,"app_id":crate::identity::PREVIEW,
        "producer_kind":"joined-single-positive-v1","source_revision":"a".repeat(40),
        "ui_sha256":provider.marker.ui_sha256,"broker_sha256":provider.marker.broker_sha256}))
        .unwrap(),
    )
    .unwrap();
    std::fs::set_permissions(path, std::fs::Permissions::from_mode(0o644)).unwrap();
    crate::cache_producer_profile::ExpectedProducer::load_for_test(
        &std::fs::File::open(&directory.0).unwrap(),
        &provider.marker.ui_sha256,
    )
    .unwrap()
    .unwrap()
}
fn closed_preview(
    directory: &ObservationDirectory,
) -> (
    InstalledProvider,
    Source,
    Result<EncodedFile, ProviderError>,
) {
    let (mut provider, source, exchange, hbc) = preview_fixture();
    let own = guarded(&mut provider, directory);
    let ticket = begin_pending(&mut provider, &own).unwrap();
    let result = session::settle(
        &mut provider,
        source.key(),
        Ok(exchange),
        Ok(hbc),
        false,
        false,
    );
    let result = guard_result(&mut provider, ticket, result);
    assert!(result.is_ok());
    (provider, source, result)
}
#[test]
fn cache_publication_waits_for_real_current_ticket_resolution_and_borrows_original_bytes() {
    use std::cell::Cell;
    let directory = ObservationDirectory::new();
    let (mut p, source, exchange, hbc) = preview_fixture();
    let expected = cache_expected(&directory, &p);
    let own = guarded(&mut p, &directory);
    let ticket = begin_pending(&mut p, &own).unwrap();
    let result = session::settle(&mut p, source.key(), Ok(exchange), Ok(hbc), false, false);
    assert!(result.is_ok());
    let count = Cell::new(0);
    let deadline = Instant::now() + Duration::from_secs(5);
    assert!(publish_positive_with(
        &p,
        &source,
        &result,
        (deadline, || false),
        &expected,
        |_, _, _| {
            count.set(count.get() + 1);
            Ok(())
        },
        |_| Ok(())
    )
    .is_none());
    assert!(p.guard.as_ref().unwrap().pending().unwrap());
    let result = guard_result(&mut p, ticket, result);
    let file = result.as_ref().unwrap();
    let pointer = file.bytes.as_ptr();
    let bytes = file.bytes.clone();
    let identity = file.identity.clone();
    let content = file.digest;
    let outcome = publish_positive_with(
        &p,
        &source,
        &result,
        (deadline, || false),
        &expected,
        |actual, encoded, producer| {
            assert_eq!(actual.key(), source.key());
            assert_eq!(encoded.bytes.as_ptr(), pointer);
            assert_eq!(encoded.bytes, bytes);
            assert_eq!(encoded.identity, identity);
            assert_eq!(encoded.digest, content);
            assert_eq!(producer, expected.version());
            assert!(!p.guard.as_ref().unwrap().pending().unwrap());
            count.set(count.get() + 1);
            Ok(())
        },
        |record| {
            assert_eq!(record["publication_succeeded"], true);
            assert_eq!(record["encoded_image_bytes_logged"], false);
            Ok(())
        },
    );
    assert_eq!(outcome, Some((true, true)));
    assert_eq!(count.get(), 1);
    assert!(result.is_ok());
    assert_eq!(p.transition_settlement(), SessionSettlement::KnownClosed);
}
#[test]
fn lifecycle_known_without_an_ok_image_never_publishes() {
    for error in [
        ProviderError::Denied,
        ProviderError::Unavailable,
        ProviderError::Cancelled,
        ProviderError::Expired,
        ProviderError::Invalid,
    ] {
        let directory = ObservationDirectory::new();
        let (mut p, source, image_result) = closed_preview(&directory);
        let expected = cache_expected(&directory, &p);
        let result: Result<EncodedFile, ProviderError> = Err(error);
        assert_eq!(p.transition_settlement(), SessionSettlement::KnownClosed);
        assert!(publish_positive_with(
            &p,
            &source,
            &result,
            (Instant::now() + Duration::from_secs(5), || false),
            &expected,
            |_, _, _| panic!("No image must publish"),
            |_| panic!("No publication receipt")
        )
        .is_none());
        assert!(!p.quarantined);
        assert!(p.ticket_resolved);
        // Simulate an older unresolved record appearing after known completion:
        // it blocks publication even when an independently owned Ok buffer exists.
        let own = guarded(&mut p, &directory);
        let retained = begin_pending(&mut p, &own).unwrap();
        drop(retained);
        assert_eq!(p.transition_settlement(), SessionSettlement::Unknown);
        assert!(publish_positive_with(
            &p,
            &source,
            &image_result,
            (Instant::now() + Duration::from_secs(5), || false),
            &expected,
            |_, _, _| panic!("Old resolved image cannot publish against new pending record"),
            |_| panic!("No receipt")
        )
        .is_none());
    }
}
#[test]
fn changed_current_pending_file_prevents_publication_and_preserves_quarantine() {
    let directory = ObservationDirectory::new();
    let (mut p, source, exchange, hbc) = preview_fixture();
    let expected = cache_expected(&directory, &p);
    let own = guarded(&mut p, &directory);
    let ticket = begin_pending(&mut p, &own).unwrap();
    let result = session::settle(&mut p, source.key(), Ok(exchange), Ok(hbc), false, false);
    assert!(result.is_ok());
    std::fs::write(
        directory.0.join("artwork-session-pending.json"),
        b"changed pending evidence",
    )
    .unwrap();
    let result = guard_result(&mut p, ticket, result);
    assert!(result.is_err());
    assert!(p.quarantined);
    assert!(!p.ticket_resolved);
    assert!(publish_positive_with(
        &p,
        &source,
        &result,
        (Instant::now() + Duration::from_secs(5), || false),
        &expected,
        |_, _, _| panic!("No failed-ticket publication"),
        |_| panic!("No receipt")
    )
    .is_none());
    assert!(p.quarantined);
    assert!(!p.ticket_resolved);
}
#[test]
fn ordinary_cache_and_optional_receipt_errors_never_rewrite_image_or_root_state() {
    for (publish_ok, receipt_ok) in [(false, true), (true, false), (false, false)] {
        let directory = ObservationDirectory::new();
        let (p, source, result) = closed_preview(&directory);
        let expected = cache_expected(&directory, &p);
        let file = result.as_ref().unwrap();
        let pointer = file.bytes.as_ptr();
        let count = std::cell::Cell::new(0);
        let got = publish_positive_with(
            &p,
            &source,
            &result,
            (Instant::now() + Duration::from_secs(5), || false),
            &expected,
            |_, _, _| {
                count.set(count.get() + 1);
                if publish_ok {
                    Ok(())
                } else {
                    Err(std::io::Error::other("Controlled ordinary cache error"))
                }
            },
            |receipt| {
                assert_eq!(receipt["publication_succeeded"], publish_ok);
                if receipt_ok {
                    Ok(())
                } else {
                    Err(std::io::Error::other("Controlled optional receipt error"))
                }
            },
        );
        assert_eq!(got, Some((publish_ok, receipt_ok)));
        assert_eq!(count.get(), 1);
        assert_eq!(result.as_ref().unwrap().bytes.as_ptr(), pointer);
        assert!(!p.quarantined);
        assert!(p.ticket_resolved);
        assert_eq!(p.transition_settlement(), SessionSettlement::KnownClosed);
    }
}
#[test]
fn final_cancel_expiry_marker_and_profile_gates_skip_all_ordinary_writes() {
    let directory = ObservationDirectory::new();
    let (mut p, source, result) = closed_preview(&directory);
    let expected = cache_expected(&directory, &p);
    for (cancelled, deadline) in [
        (true, Instant::now() + Duration::from_secs(5)),
        (false, Instant::now()),
    ] {
        assert!(publish_positive_with(
            &p,
            &source,
            &result,
            (deadline, || cancelled),
            &expected,
            |_, _, _| panic!("No interrupted publication"),
            |_| panic!("No receipt")
        )
        .is_none());
    }
    p.marker.broker_sha256 = "d".repeat(64);
    assert!(publish_positive_with(
        &p,
        &source,
        &result,
        (Instant::now() + Duration::from_secs(5), || false),
        &expected,
        |_, _, _| panic!("Mismatched expected helper"),
        |_| panic!("No receipt")
    )
    .is_none());
    p.marker.broker_sha256 = "c".repeat(64);
    p.profile = Profile::Proof;
    assert!(publish_positive_with(
        &p,
        &source,
        &result,
        (Instant::now() + Duration::from_secs(5), || false),
        &expected,
        |_, _, _| panic!("No Proof publisher"),
        |_| panic!("No receipt")
    )
    .is_none());
    assert!(result.is_ok());
    assert!(!p.quarantined);
    assert!(p.ticket_resolved);
}
#[test]
fn existing_provider_fixture_has_no_publisher_and_does_not_even_run_optional_gate() {
    let directory = ObservationDirectory::new();
    let (mut p, source, result) = closed_preview(&directory);
    assert!(p.cache_publisher.is_none());
    p.publish_after_positive(
        &source,
        &result,
        Instant::now() + Duration::from_secs(5),
        || panic!("Default None must bypass all publication work"),
    );
    assert!(p.cache_publisher.is_none());
    assert!(result.is_ok());
    assert_eq!(p.transition_settlement(), SessionSettlement::KnownClosed);
}
#[test]
fn required_diagnostic_result_write_error_remains_nonpublishing() {
    let directory = ObservationDirectory::new();
    let (mut p, source, result) = closed_preview(&directory);
    let expected = cache_expected(&directory, &p);
    p.observation = Some(directory.observer());
    let ticket = proof_ticket(&mut p).unwrap().unwrap();
    write_observed_result(&mut p, ticket, &json!({"outcome":"first fixture"})).unwrap();
    let result =
        write_observed_result(&mut p, ticket, &json!({"outcome":"must not overwrite"})).and(result);
    assert!(result.is_err());
    assert!(p.quarantined);
    assert!(publish_positive_with(
        &p,
        &source,
        &result,
        (Instant::now() + Duration::from_secs(5), || false),
        &expected,
        |_, _, _| panic!("Required observation failed"),
        |_| panic!("No optional receipt")
    )
    .is_none());
    assert!(p.quarantined);
    assert!(p.ticket_resolved);
    assert!(!p.guard.as_ref().unwrap().pending().unwrap());
}
#[test]
fn optional_cache_receipt_uses_only_current_fixed_index_and_cannot_overwrite() {
    let directory = ObservationDirectory::new();
    let (mut p, _, _, _) = preview_fixture();
    p.observation = Some(directory.observer());
    assert!(p
        .observation
        .as_ref()
        .unwrap()
        .cache_result(&json!({"schema":1}))
        .is_err());
    let ticket = proof_ticket(&mut p).unwrap().unwrap();
    assert_eq!(ticket.index, 1);
    let receipt =
        json!({"schema":1,"publication_succeeded":false,"encoded_image_bytes_logged":false});
    p.observation
        .as_ref()
        .unwrap()
        .cache_result(&receipt)
        .unwrap();
    assert!(p
        .observation
        .as_ref()
        .unwrap()
        .cache_result(&json!({"must_not":"replace"}))
        .is_err());
    let bytes = std::fs::read(directory.0.join("installed-artwork-1-cache.json")).unwrap();
    assert_eq!(serde_json::from_slice::<Value>(&bytes).unwrap(), receipt);
    // Required diagnostic writes retain their original allowlist and failure policy.
    assert!(p
        .observation
        .as_ref()
        .unwrap()
        .write("installed-artwork-1-cache.json", &receipt)
        .is_err());
    assert!(!p.quarantined);
    assert!(!p.ticket_resolved);
}

#[test]
fn population_initializer_requires_its_own_fixed_sidecar_load_before_any_fetch() {
    use std::os::unix::fs::PermissionsExt;
    let directory = ObservationDirectory::new();
    let (provider, _, _, _) = preview_fixture();
    let load = || {
        crate::cache_producer_profile::ExpectedProducer::load_for_test(
            &std::fs::File::open(&directory.0).unwrap(),
            &provider.marker.ui_sha256,
        )
    };
    assert!(matches!(
        population_expected(&provider.marker, load),
        Err(ProviderError::Unavailable)
    ));
    let path = directory.0.join("artwork-cache-producer.json");
    std::fs::write(&path, b"{}").unwrap();
    std::fs::set_permissions(&path, std::fs::Permissions::from_mode(0o644)).unwrap();
    assert!(matches!(
        population_expected(&provider.marker, load),
        Err(ProviderError::Denied)
    ));
    let expected = cache_expected(&directory, &provider);
    assert_eq!(
        population_expected(&provider.marker, load)
            .unwrap()
            .version(),
        expected.version()
    );
    let mut other = provider.marker.clone();
    other.broker_sha256 = "d".repeat(64);
    assert!(matches!(
        population_expected(&other, load),
        Err(ProviderError::Denied)
    ));
    assert!(provider.cache_publisher.is_none());
    assert!(!provider.ticket_resolved);
    assert!(!provider.quarantined);
}
#[test]
fn population_factory_host_api_has_no_caller_supplied_expected_producer() {
    #[cfg(not(target_arch = "arm"))]
    {
        let result = InstalledProvider::start_preview_cache_population_worker(
            marker(),
            std::path::PathBuf::from("/nonexistent-no-worker-created"),
        );
        let error = match result {
            Ok(_) => panic!("Host cannot create native population worker"),
            Err(error) => error,
        };
        assert_eq!(error.kind(), std::io::ErrorKind::Unsupported);
    }
}
