//! Controlled host receipts/packets only. No HBC, filesystem reader or device.
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
    hbc["parsed"] = crate::broker_startup_proof::parse_exec_resource_reply(&raw).unwrap();
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
    let installation = json!({"profile":"proof","app_id":crate::identity::PROOF,"root_process":root_process,"pins":{"schema":1,"profile":"proof","rendezvous_nonce":marker.nonce,"ui_sha256":marker.ui_sha256,"broker_sha256":marker.broker_sha256,"context_reader_sha256":"d".repeat(64)},"files":{"helper":{"path":crate::broker_startup_proof::COMMAND,"sha256":marker.broker_sha256,"trusted_for_execution":true},"configuration":{"path":"/var/lib/luma-home-proof/broker-installation.json","trusted_for_execution":true},"ui":{"sha256":marker.ui_sha256,"trusted_for_execution":false}}});
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
            quarantined: false,
        },
        source,
        exchange,
        hbc,
    )
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
