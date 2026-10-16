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
fn source() -> Source {
    let id = "synthetic.fixture.installed";
    let raw = json!({"replies":{"apps":{"returnValue":true,"apps":[{"id":id,"title":"Fixture","type":"native","visible":true,"version":"1.0.0","folderPath":format!("/media/developer/apps/usr/palm/applications/{id}"),"icon":"icon.png","largeIcon":"images/large.png"}]},"launch_points":{"returnValue":true,"launchPoints":[{"id":id}]}}});
    app_artwork::sources_from_raw(&raw, crate::identity::PROOF).unwrap()[id]
        .preferred()
        .unwrap()
        .clone()
}
fn marker() -> Marker {
    Marker {
        schema: 1,
        nonce: "a".repeat(32),
        ui_sha256: "b".repeat(64),
        broker_sha256: "c".repeat(64),
    }
}
fn context() -> Value {
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
fn base64(bytes: &[u8]) -> String {
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
fn reader(catalogue: bool, next: &mut u64) -> Value {
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
fn sync(hbc: &mut Value) {
    sync_for_profile(hbc, Profile::Proof);
}
fn sync_for_profile(hbc: &mut Value, profile: Profile) {
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
fn fixture() -> (InstalledProvider, Source, session::Exchange, Value) {
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
fn preview_fixture() -> (InstalledProvider, Source, session::Exchange, Value) {
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
