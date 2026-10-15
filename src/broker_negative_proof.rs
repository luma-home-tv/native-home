//! Two isolated nonce-refusal observations. No production or resource operation.
use crate::{broker_protocol::Marker, model::Page, runtime::Window, shell::Shell};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use std::{
    io::Read,
    os::unix::fs::{MetadataExt, OpenOptionsExt},
    path::Path,
    sync::{
        atomic::{AtomicBool, Ordering},
        Arc,
    },
    time::{Duration, Instant},
};

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum Case {
    WrongIdentifyNonce,
    WrongFinishNonce,
}
impl<'de> Deserialize<'de> for Case {
    fn deserialize<D: serde::Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        let case = String::deserialize(deserializer)?;
        match case.as_str() {
            "wrong_identify_nonce" => Ok(Self::WrongIdentifyNonce),
            "wrong_finish_nonce" => Ok(Self::WrongFinishNonce),
            _ => Err(serde::de::Error::unknown_variant(
                &case,
                &["wrong_identify_nonce", "wrong_finish_nonce"],
            )),
        }
    }
}
impl Case {
    #[cfg(any(target_arch = "arm", test))]
    fn hold_stage(self) -> &'static str {
        match self {
            Self::WrongIdentifyNonce => "before_identify",
            Self::WrongFinishNonce => "before_finish",
        }
    }
    #[cfg(any(target_arch = "arm", test))]
    fn refusal(self) -> &'static str {
        match self {
            Self::WrongIdentifyNonce => "Identify nonce mismatch",
            Self::WrongFinishNonce => "Only first Finish request is admitted after Identify",
        }
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct NegativeMarker {
    pub schema: u32,
    pub scenario: Case,
    pub marker: Marker,
}
impl NegativeMarker {
    fn validate(&self) -> Result<(), String> {
        if self.schema != 1 {
            return Err("Unsupported negative proof schema".into());
        }
        self.marker.validate().map_err(|e| e.to_string())
    }
}

pub fn read_marker(path: &Path) -> Result<NegativeMarker, String> {
    let file = std::fs::OpenOptions::new()
        .read(true)
        .custom_flags(libc::O_NOFOLLOW | libc::O_CLOEXEC | libc::O_NONBLOCK)
        .open(path)
        .map_err(|e| e.to_string())?;
    let before = file.metadata().map_err(|e| e.to_string())?;
    if !before.is_file()
        || before.len() == 0
        || before.len() > 4096
        || before.uid() != 0
        || before.gid() != 0
        || before.nlink() != 1
        || before.mode() & 0o7777 != 0o644
    {
        return Err("Invalid installed negative proof marker".into());
    }
    let mut bytes = Vec::new();
    (&file)
        .take(4097)
        .read_to_end(&mut bytes)
        .map_err(|e| e.to_string())?;
    let after = file.metadata().map_err(|e| e.to_string())?;
    let named = std::fs::symlink_metadata(path).map_err(|e| e.to_string())?;
    let identity = |m: &std::fs::Metadata| {
        (
            m.dev(),
            m.ino(),
            m.len(),
            m.mode(),
            m.uid(),
            m.gid(),
            m.nlink(),
            m.mtime(),
            m.mtime_nsec(),
            m.ctime(),
            m.ctime_nsec(),
        )
    };
    if bytes.len() as u64 != before.len()
        || identity(&before) != identity(&after)
        || named.file_type().is_symlink()
        || identity(&after) != identity(&named)
    {
        return Err("Negative proof marker changed during read".into());
    }
    let value: NegativeMarker = serde_json::from_slice(&bytes).map_err(|e| e.to_string())?;
    value.validate()?;
    Ok(value)
}

#[cfg(any(target_arch = "arm", test))]
fn bad_nonce(real: &str) -> Result<String, String> {
    if real.len() != 32
        || !real
            .bytes()
            .all(|b| b.is_ascii_digit() || (b'a'..=b'f').contains(&b))
    {
        return Err("Invalid installed nonce".into());
    }
    let mut bytes = real.as_bytes().to_vec();
    bytes[0] = if bytes[0] == b'0' { b'1' } else { b'0' };
    String::from_utf8(bytes).map_err(|e| e.to_string())
}
#[cfg(any(target_arch = "arm", test))]
fn packet(marker: &NegativeMarker) -> Result<Vec<u8>, String> {
    marker.validate()?;
    let nonce = bad_nonce(&marker.marker.nonce)?;
    match marker.scenario {
        Case::WrongIdentifyNonce => serde_json::to_vec(&crate::broker_protocol::Request {
            schema: 1,
            op: crate::broker_protocol::Operation::Identify,
            nonce,
        })
        .map_err(|e| e.to_string()),
        Case::WrongFinishNonce => serde_json::to_vec(
            &crate::artwork_transfer::FinishRequest::new(1, &nonce).map_err(|e| e.to_string())?,
        )
        .map_err(|e| e.to_string()),
    }
}
#[cfg(any(target_arch = "arm", test))]
fn sha256(bytes: &[u8]) -> String {
    use std::fmt::Write;
    let mut result = String::with_capacity(64);
    for byte in plx_base::sha256::sha256(bytes) {
        let _ = write!(&mut result, "{byte:02x}");
    }
    result
}

#[cfg(any(target_arch = "arm", test))]
fn before_context(context: &Value, journals: &[Value], root: &Value) -> bool {
    let reader = &context["_reader"];
    let expected_sha = root["installation"]["pins"]["context_reader_sha256"]
        .as_str()
        .unwrap_or("");
    let Some(operations) = reader["operations"].as_array() else {
        return false;
    };
    if !crate::broker_probe::context_valid(context)
        || expected_sha.len() != 64
        || reader["schema"] != 1
        || !reader["error"].is_null()
        || reader["executable_after"] != reader["executable"]
        || reader["kind"] != "fixed_core_luna_send"
        || reader["program"] != "/usr/bin/luna-send"
        || reader["approved"] != true
        || reader["qualified"] != true
        || reader["cleanup_known"] != true
        || reader["executable"]["sha256"] != expected_sha
        || reader["executable_after"]["sha256"] != expected_sha
        || operations.len() != 3
        || journals.len() != 3
    {
        return false;
    }
    let wanted = [
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
    operations
        .iter()
        .zip(journals)
        .zip(wanted)
        .all(|((op, event), (key, uri, payload))| {
            op["key"] == key
                && op["uri"] == uri
                && op["argv"]
                    == json!([
                        "/usr/bin/luna-send",
                        "-i",
                        "-n",
                        "1",
                        "-w",
                        "5000",
                        "-f",
                        uri,
                        payload
                    ])
                && op["attempted"] == true
                && op["success"] == true
                && op["child_reaped"] == true
                && op["cleanup_known"] == true
                && op["held_empty_stdin"] == true
                && op["stdin_bytes_sent"] == 0
                && op["exit_code"] == 0
                && op["signal"].is_null()
                && op["timed_out"] == false
                && op["stdout_eof"] == true
                && op["stderr_eof"] == true
                && op["stderr_bytes"] == 0
                && op["stdout_bytes"]
                    .as_u64()
                    .is_some_and(|v| v > 0 && v <= 65536)
                && op["stderr_sha256"] == sha256(&[])
                && op["error"].is_null()
                && op["executable_before"] == reader["executable"]
                && op["executable_after"] == reader["executable"]
                && op["group_cleanup_attempted"] == true
                && op["group_kill_errno"].is_null()
                && event["schema"] == 1
                && event["stage"] == "context_child_started"
                && event["child_pid"].as_u64().is_some_and(|pid| pid > 1)
                && event["process_group"] == event["child_pid"]
                && event["program_sha256"] == expected_sha
                && op["child_pid"] == event["child_pid"]
                && op["process_group"] == event["process_group"]
        })
}

#[cfg(any(target_arch = "arm", test))]
fn expected_refusal(report: &Value, marker: &NegativeMarker) -> bool {
    let observed = &report["protocol"];
    let rpc = &report["hbc"];
    let helper = &rpc["parsed"]["helper_report"];
    let root = &helper["root_process"];
    let own = &observed["own"];
    let peer = &observed["root_peer"];
    let Some(pid) = own["pid"].as_u64().filter(|v| *v > 1) else {
        return false;
    };
    let Some(uid) = own["uid"].as_u64().filter(|v| *v >= 1000 && *v != 5724) else {
        return false;
    };
    let Some(root_pid) = peer["pid"].as_u64().filter(|v| *v > 1) else {
        return false;
    };
    let Some(journals) = rpc["parsed"]["journals"].as_array() else {
        return false;
    };
    let Ok(wrong) = packet(marker) else {
        return false;
    };
    let Some(raw) = rpc["raw"].as_str() else {
        return false;
    };
    let Ok(reparsed) = crate::broker_startup_proof::parse_exec_reply(raw) else {
        return false;
    };
    let installation = &helper["installation"];
    let pins = &installation["pins"];
    let context_sha = pins["context_reader_sha256"].as_str().unwrap_or("");
    let expected_peer = json!({"pid":pid,"uid_fields":[uid,uid,uid,uid],"start_ticks":own["start_ticks"],"exe_sha256":marker.marker.ui_sha256,"exe_path":crate::broker_protocol::PROOF_EXE_PATH,"root_path":crate::broker_protocol::PROOF_JAIL_ROOT});
    let cap = &rpc["capability"];
    let Some(cap_raw) = cap["response"]["raw"].as_str() else {
        return false;
    };
    let cap_response = crate::artwork_startup_probe::parse_response(cap_raw);
    let context_before_absent = helper.get("context_before").is_none();
    if report["schema"] != 1
        || report["scenario"] != json!(marker.scenario)
        || report["marker"] != json!(marker.marker)
        || report["fixed_command"] != crate::broker_startup_proof::COMMAND
        || report["exec_uri"] != crate::broker_startup_proof::EXEC_URI
        || !report["error"].is_null()
        || report["exec_worker_joined"] != true
        || report["protocol_worker_joined"] != true
        || report["interrupted"] != false
        || report["positive_startup_qualified"] != false
        || report["resource_reads"] != false
        || report["normal_loader_qualified"] != false
        || report["production_startup_qualified"] != false
        || report["unknown_outcome_is_no_start"] != false
        || observed["negative_clean_eof"] != true
        || observed["finished_received"] != false
        || observed["pre_send_hold_stage"] != marker.scenario.hold_stage()
        || observed["pre_send_hold_ms"]
            .as_u64()
            .is_none_or(|v| v < 8000)
        || own["start_ticks"].as_u64().is_none_or(|v| v == 0)
        || own["ui_sha256"] != marker.marker.ui_sha256
        || observed["own_after"] != *own
        || observed["own_kernel_credentials"]["pid"] != pid
        || observed["own_kernel_credentials"]["uid"] != uid
        || peer["uid"] != 0
        || peer["gid"].as_u64().is_none()
        || rpc["uri"] != crate::broker_startup_proof::EXEC_URI
        || rpc["command"] != crate::broker_startup_proof::COMMAND
        || rpc["attempts"] != 1
        || rpc["raw_bytes"].as_u64() != Some(raw.len() as u64)
        || rpc["raw_truncated"] != false
        || rpc["parsed"] != reparsed
        || !rpc["error"].is_null()
        || rpc["parsed"]["envelope_success"] != false
        || !rpc["parsed"]["error_text"].is_string()
        || cap["verified"] != true
        || cap["schema"] != 1
        || cap["app_id"] != crate::identity::PROOF
        || cap["uid"] != uid
        || cap["registered"] != true
        || cap["request_attempted"] != true
        || cap["check_root_calls"] != 1
        || cap["uri"] != "luna://org.webosbrew.hbchannel.service/checkRoot"
        || cap["payload"] != json!({})
        || cap["identity_stable"] != true
        || cap["operation_deadline_expired"] != false
        || !cap["error"].is_null()
        || cap["own_before"] != *own
        || cap["own_at_call"] != *own
        || cap["own_after"] != *own
        || !crate::broker_probe::context_valid(&cap["context_before"])
        || !crate::broker_probe::context_valid(&cap["context_after"])
        || cap["response"] != cap_response
        || cap_response["outcome"] != "root_true"
        || rpc["own_at_call"] != *own
        || !crate::broker_probe::context_valid(&rpc["context"])
        || helper["schema"] != 1
        || helper["stage"] != "startup_proof_complete"
        || helper["profile"] != "proof"
        || helper["listener_bound"] != true
        || helper["operation_budget_ms"] != 20000
        || helper["own_alarm"]["requested_seconds"] != 30
        || helper["own_alarm"]["sigalrm_unblocked_readback"] != true
        || installation["schema"] != 1
        || installation["profile"] != "proof"
        || installation["app_id"] != crate::identity::PROOF
        || installation["root_process"] != *root
        || pins["schema"] != 1
        || pins["profile"] != "proof"
        || pins["rendezvous_nonce"] != marker.marker.nonce
        || pins["ui_sha256"] != marker.marker.ui_sha256
        || pins["broker_sha256"] != marker.marker.broker_sha256
        || context_sha.len() != 64
        || !context_sha
            .bytes()
            .all(|b| b.is_ascii_digit() || (b'a'..=b'f').contains(&b))
        || installation["files"]["helper"]["path"] != crate::broker_startup_proof::COMMAND
        || installation["files"]["helper"]["sha256"] != marker.marker.broker_sha256
        || installation["files"]["helper"]["trusted_for_execution"] != true
        || installation["files"]["configuration"]["path"]
            != "/var/lib/luma-home-proof/broker-installation.json"
        || installation["files"]["configuration"]["trusted_for_execution"] != true
        || installation["files"]["ui"]["path"]
            != "/media/developer/apps/usr/palm/applications/com.ash.nativehome.proof/home-preview"
        || installation["files"]["ui"]["sha256"] != marker.marker.ui_sha256
        || installation["files"]["ui"]["trusted_for_execution"] != false
        || helper["qualified"] != false
        || helper["outcome"] != "refused"
        || helper["error"] != marker.scenario.refusal()
        || helper["server_pid"] != root_pid
        || root["pid"] != root_pid
        || root["uid_fields"] != json!([0, 0, 0, 0])
        || root["start_ticks"].as_u64().is_none_or(|v| v == 0)
        || root["root_path"] != "/"
        || root["exe_path"] != crate::broker_startup_proof::COMMAND
        || root["exe_sha256"] != marker.marker.broker_sha256
        || helper["broker_sha256"] != marker.marker.broker_sha256
        || helper["nonce"] != marker.marker.nonce
        || helper["connect_credentials"] != observed["own_kernel_credentials"]
        || helper["finished_sent"] != false
        || helper["connection_closed"] != false
        || [
            "icon_reads",
            "artwork_reads",
            "commands",
            "setting_writes",
            "root_file_writes",
            "daemonized",
            "normal_loader_qualified",
            "production_startup_qualified",
        ]
        .iter()
        .any(|k| helper[*k] != false)
        || helper.get("context_after").is_some()
        || helper.get("finish_request").is_some()
        || helper.get("finish_message_credentials").is_some()
        || helper.get("peer_at_finish").is_some()
        || helper.get("peer_after").is_some()
        || helper.get("finished").is_some()
        || observed["wrong_packet"]["raw_utf8"] != String::from_utf8_lossy(&wrong).as_ref()
        || observed["wrong_packet"]["bytes"] != json!(wrong.len())
        || observed["wrong_packet"]["sha256"] != sha256(&wrong)
        || observed["wrong_packet"]["send_completed"] != true
    {
        return false;
    }
    match marker.scenario {
        Case::WrongIdentifyNonce => {
            journals.is_empty()
                && context_before_absent
                && helper["response_sent"] == false
                && helper.get("response").is_none()
                && helper.get("identify_request").is_none()
                && helper.get("identify_message_credentials").is_none()
                && helper.get("peer_before").is_none()
                && helper.get("peer_at_identify_send").is_none()
                && helper.get("enrollment").is_none()
                && observed["identify_response"].is_null()
                && observed["root_message_credentials"].is_null()
        }
        Case::WrongFinishNonce => {
            let expected = json!({"schema":1,"op":"identify","nonce":marker.marker.nonce,"server_pid":root_pid,"server_uid":0,"client_pid":pid,"client_uid":uid,"client_start_ticks":own["start_ticks"],"ui_sha256":marker.marker.ui_sha256,"broker_sha256":marker.marker.broker_sha256});
            helper["response_sent"] == true
                && helper["response"] == expected
                && observed["identify_response"] == expected
                && observed["root_message_credentials"] == *peer
                && helper["identify_message_credentials"] == observed["own_kernel_credentials"]
                && helper["identify_request"]
                    == json!({"schema":1,"op":"identify","nonce":marker.marker.nonce})
                && helper["peer_before"] == expected_peer
                && helper["peer_at_identify_send"] == expected_peer
                && before_context(&helper["context_before"], journals, helper)
                && helper["enrollment"]["client_pid"] == pid
                && helper["enrollment"]["client_uid"] == uid
                && helper["enrollment"]["client_start_ticks"] == own["start_ticks"]
                && helper["enrollment"]["nonce"] == marker.marker.nonce
                && helper["enrollment"]["ui_sha256"] == marker.marker.ui_sha256
                && helper["enrollment"]["broker_sha256"] == marker.marker.broker_sha256
                && helper["enrollment"]["schema"] == 1
                && helper["enrollment"]["context_reader_sha256"] == context_sha
        }
    }
}

#[cfg(target_arch = "arm")]
mod native {
    use super::*;
    use crate::{broker_probe as probe, broker_protocol as protocol, broker_transport::Connection};
    use std::sync::mpsc;
    fn credentials(c: crate::broker_transport::Credentials) -> Value {
        json!({"pid":c.pid,"uid":c.uid,"gid":c.gid})
    }
    fn hold(cancel: &AtomicBool) -> Result<u128, String> {
        let began = Instant::now();
        while began.elapsed() < Duration::from_secs(8) {
            if cancel.load(Ordering::Acquire) {
                return Err("Negative proof cancelled during binding hold".into());
            }
            std::thread::sleep(Duration::from_millis(20));
        }
        Ok(began.elapsed().as_millis())
    }
    fn exchange(
        marker: &NegativeMarker,
        output: &Path,
        cancel: &AtomicBool,
        started: mpsc::Receiver<()>,
    ) -> Result<Value, String> {
        started
            .recv_timeout(Duration::from_secs(12))
            .map_err(|_| "Fixed startup was not attempted")?;
        let deadline = Instant::now() + Duration::from_secs(24);
        let own = probe::own_identity()?;
        if own.ui_sha256 != marker.marker.ui_sha256 || cancel.load(Ordering::Acquire) {
            return Err("Negative caller identity/cancellation gate failed".into());
        }
        let connection = Connection::connect_until(
            &marker.marker.nonce,
            deadline.min(Instant::now() + Duration::from_secs(8)),
        )
        .map_err(|e| e.to_string())?;
        let peer = connection.peer_credentials().map_err(|e| e.to_string())?;
        if peer.uid != 0 || peer.pid <= 1 {
            return Err("Negative proof peer is not Root".into());
        }
        let mut result = json!({"schema":1,"own":own,"own_after":null,"own_kernel_credentials":{"pid":std::process::id(),"uid":unsafe{libc::getuid()},"gid":unsafe{libc::getegid()}},"root_peer":credentials(peer),"root_message_credentials":null,"identify_response":null,"wrong_packet":null,"pre_send_hold_stage":marker.scenario.hold_stage(),"pre_send_hold_ms":0,"negative_clean_eof":false,"finished_received":false});
        crate::broker_startup_proof::save(&output.join("broker-negative-connected.json"), &result)?;
        if marker.scenario == Case::WrongFinishNonce {
            let request = protocol::Request {
                schema: 1,
                op: protocol::Operation::Identify,
                nonce: marker.marker.nonce.clone(),
            };
            connection
                .send_until(
                    &serde_json::to_vec(&request).map_err(|e| e.to_string())?,
                    deadline,
                )
                .map_err(|e| e.to_string())?;
            let received = connection
                .recv_until(protocol::RESPONSE_LIMIT, deadline)
                .map_err(|e| e.to_string())?;
            let response = protocol::parse_response(&received.bytes).map_err(|e| e.to_string())?;
            probe::bind_response(
                &marker.marker,
                &own,
                &probe::own_identity()?,
                peer,
                received.credentials,
                &response,
            )?;
            result["identify_response"] = json!(response);
            result["root_message_credentials"] = credentials(received.credentials);
            crate::broker_startup_proof::save(
                &output.join("broker-negative-identify.json"),
                &result,
            )?;
        }
        result["pre_send_hold_ms"] = json!(hold(cancel)?);
        if probe::own_identity()? != own {
            return Err("Negative caller changed before packet".into());
        }
        let wrong = packet(marker)?;
        result["wrong_packet"] = json!({"raw_utf8":std::str::from_utf8(&wrong).map_err(|e|e.to_string())?,"bytes":wrong.len(),"sha256":sha256(&wrong),"send_completed":false});
        connection
            .send_until(&wrong, deadline)
            .map_err(|e| e.to_string())?;
        result["wrong_packet"]["send_completed"] = json!(true);
        // No application reply or Finished is accepted as a refusal. This
        // primitive also rejects empty credentialled packets and truncation.
        connection
            .expect_eof_until(deadline)
            .map_err(|e| e.to_string())?;
        let after = probe::own_identity()?;
        if after != own {
            return Err("Negative caller changed after EOF".into());
        }
        result["own_after"] = json!(after);
        result["negative_clean_eof"] = json!(true);
        Ok(result)
    }
    pub(super) fn collect(marker: NegativeMarker, output: &Path, cancel: Arc<AtomicBool>) -> Value {
        let mut report = json!({"schema":1,"scenario":marker.scenario,"marker":marker.marker,"fixed_command":crate::broker_startup_proof::COMMAND,"exec_uri":crate::broker_startup_proof::EXEC_URI,"protocol":null,"hbc":null,"exec_worker_joined":false,"error":null,"expected_refusal_verified":false,"positive_startup_qualified":false,"resource_reads":false,"normal_loader_qualified":false,"production_startup_qualified":false,"unknown_outcome_is_no_start":false,"rejected_packet_scm_independently_saved":false});
        let (tx, rx) = mpsc::channel();
        let invoke_marker = marker.marker.clone();
        let invoke_cancel = cancel.clone();
        let worker = match std::thread::Builder::new()
            .name("broker-negative-ls2".into())
            .spawn(move || {
                crate::broker_startup_proof::native::invoke(&invoke_marker, &invoke_cancel, tx)
            }) {
            Ok(worker) => worker,
            Err(error) => {
                report["error"] = json!(error.to_string());
                return report;
            }
        };
        match exchange(&marker, output, &cancel, rx) {
            Ok(value) => report["protocol"] = value,
            Err(error) => report["error"] = json!(error),
        }
        match worker.join() {
            Ok(value) => {
                report["hbc"] = value;
                report["exec_worker_joined"] = json!(true);
            }
            Err(_) => {
                report["error"] = json!("Negative exec worker panicked; startup outcome unknown")
            }
        }
        report
    }
}

pub fn run(
    shell: &mut Shell,
    window: &Window,
    output: &Path,
    now: &mut u32,
    marker: NegativeMarker,
) -> Result<Value, Box<dyn std::error::Error>> {
    marker.validate()?;
    #[cfg(target_arch = "arm")]
    let expected_marker = marker.clone();
    let preferences = shell.preferences().clone();
    shell.open(Page::Home);
    let cancel = Arc::new(AtomicBool::new(false));
    let worker_cancel = cancel.clone();
    let directory = output.to_path_buf();
    let worker = std::thread::Builder::new()
        .name("broker-negative-protocol".into())
        .spawn(move || {
            #[cfg(target_arch = "arm")]
            {
                native::collect(marker, &directory, worker_cancel)
            }
            #[cfg(not(target_arch = "arm"))]
            {
                let _ = (marker, directory, worker_cancel);
                json!({"unsupported_host":true,"expected_refusal_verified":false})
            }
        })?;
    let began = Instant::now();
    let mut interrupted = false;
    while began.elapsed() < Duration::from_secs(8) || !worker.is_finished() {
        while let Some(event) = crate::runtime::poll() {
            if matches!(event.kind, 1..=8 | 10 | 11) {
                interrupted = true;
                cancel.store(true, Ordering::Release);
            }
        }
        if !interrupted {
            *now = now.wrapping_add(17);
            shell.tick(*now, 1.0 / 60.0);
            shell.draw();
            window.swap();
        }
        std::thread::sleep(Duration::from_millis(17));
    }
    let mut report = worker
        .join()
        .map_err(|_| "Negative protocol worker panicked; startup outcome unknown")?;
    report["protocol_worker_joined"] = json!(true);
    report["interrupted"] = json!(interrupted);
    report["ui_elapsed_ms"] = json!(began.elapsed().as_millis());
    #[cfg(target_arch = "arm")]
    {
        report["expected_refusal_verified"] = json!(expected_refusal(&report, &expected_marker));
    }
    if interrupted {
        report["expected_refusal_verified"] = json!(false);
    }
    crate::broker_startup_proof::save(&output.join("broker-negative-proof.json"), &report)?;
    if interrupted
        || shell.preferences() != &preferences
        || !shell.take_requests().is_empty()
        || shell.should_exit()
    {
        return Err("Negative proof interrupted or changed UI state".into());
    }
    shell.draw();
    if window.graphics_state().error != 0 {
        return Err("Negative proof capture GL error".into());
    }
    window.capture(&output.join("broker-negative-home.ppm"))?;
    window.swap();
    Ok(report)
}

#[cfg(test)]
mod tests {
    use super::*;
    fn marker(scenario: Case) -> NegativeMarker {
        NegativeMarker {
            schema: 1,
            scenario,
            marker: Marker {
                schema: 1,
                nonce: "a".repeat(32),
                ui_sha256: "b".repeat(64),
                broker_sha256: "c".repeat(64),
            },
        }
    }
    fn context() -> Value {
        json!({"system_info":{"returnValue":true,"modelName":"OLED55G26LA","firmwareVersion":"23.25.55","sdkVersion":"9.2.2"},"foreground":{"returnValue":true,"appId":crate::identity::PROOF},"windows":{"returnValue":true,"windows":[{"appId":crate::identity::PROOF,"windowType":"_WEBOS_WINDOW_TYPE_CARD","extra":"retained"}]}})
    }
    fn base64(bytes: &[u8]) -> String {
        let alphabet = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";
        let mut result = String::new();
        for part in bytes.chunks(3) {
            result.push(alphabet[(part[0] >> 2) as usize] as char);
            result.push(
                alphabet[(((part[0] & 3) << 4) | (part.get(1).copied().unwrap_or(0) >> 4)) as usize]
                    as char,
            );
            result.push(if part.len() > 1 {
                alphabet
                    [(((part[1] & 15) << 2) | (part.get(2).copied().unwrap_or(0) >> 6)) as usize]
                    as char
            } else {
                '='
            });
            result.push(if part.len() > 2 {
                alphabet[(part[2] & 63) as usize] as char
            } else {
                '='
            });
        }
        result
    }
    fn sync_raw(report: &mut Value) {
        let parsed = &report["hbc"]["parsed"];
        let mut rows = parsed["journals"].as_array().unwrap().clone();
        rows.push(parsed["helper_report"].clone());
        let stdout = rows
            .iter()
            .map(|row| format!("{row}\n"))
            .collect::<String>();
        let stderr = "Error: fixture refusal\n";
        let raw=json!({"returnValue":false,"errorText":"Command failed fixture","stdoutString":stdout,"stdoutBytes":base64(stdout.as_bytes()),"stderrString":stderr,"stderrBytes":base64(stderr.as_bytes())}).to_string();
        report["hbc"]["raw_bytes"] = json!(raw.len());
        report["hbc"]["raw_truncated"] = json!(false);
        report["hbc"]["raw"] = json!(raw.clone());
        report["hbc"]["parsed"] = crate::broker_startup_proof::parse_exec_reply(&raw).unwrap();
    }
    fn fixture(scenario: Case) -> (NegativeMarker, Value) {
        let marker = marker(scenario);
        let own =
            json!({"pid":123,"uid":6583,"start_ticks":456,"ui_sha256":marker.marker.ui_sha256});
        let own_credentials = json!({"pid":123,"uid":6583,"gid":6583});
        let root_peer = json!({"pid":789,"uid":0,"gid":0});
        let root_process = json!({"pid":789,"uid_fields":[0,0,0,0],"start_ticks":321,"exe_path":crate::broker_startup_proof::COMMAND,"exe_sha256":marker.marker.broker_sha256,"root_path":"/"});
        let pins = json!({"schema":1,"profile":"proof","rendezvous_nonce":marker.marker.nonce,"ui_sha256":marker.marker.ui_sha256,"broker_sha256":marker.marker.broker_sha256,"context_reader_sha256":"d".repeat(64)});
        let installation = json!({"schema":1,"profile":"proof","app_id":crate::identity::PROOF,"pins":pins,"root_process":root_process,"files":{"helper":{"path":crate::broker_startup_proof::COMMAND,"sha256":marker.marker.broker_sha256,"trusted_for_execution":true},"configuration":{"path":"/var/lib/luma-home-proof/broker-installation.json","trusted_for_execution":true},"ui":{"path":"/media/developer/apps/usr/palm/applications/com.ash.nativehome.proof/home-preview","sha256":marker.marker.ui_sha256,"trusted_for_execution":false}}});
        let mut helper = json!({"schema":1,"stage":"startup_proof_complete","profile":"proof","server_pid":789,"root_process":root_process,"broker_sha256":marker.marker.broker_sha256,"nonce":marker.marker.nonce,"installation":installation,"listener_bound":true,"operation_budget_ms":20000,"own_alarm":{"requested_seconds":30,"sigalrm_unblocked_readback":true},"qualified":false,"outcome":"refused","error":scenario.refusal(),"response_sent":false,"finished_sent":false,"connection_closed":false,"connect_credentials":own_credentials});
        for key in [
            "icon_reads",
            "artwork_reads",
            "commands",
            "setting_writes",
            "root_file_writes",
            "daemonized",
            "normal_loader_qualified",
            "production_startup_qualified",
        ] {
            helper[key] = json!(false);
        }
        let wrong = packet(&marker).unwrap();
        let mut observed = json!({"schema":1,"own":own,"own_after":own,"own_kernel_credentials":own_credentials,"root_peer":root_peer,"root_message_credentials":null,"identify_response":null,"pre_send_hold_stage":scenario.hold_stage(),"pre_send_hold_ms":8000,"negative_clean_eof":true,"finished_received":false,"wrong_packet":{"raw_utf8":std::str::from_utf8(&wrong).unwrap(),"bytes":wrong.len(),"sha256":sha256(&wrong),"send_completed":true}});
        let mut journals = Vec::new();
        if scenario == Case::WrongFinishNonce {
            let response = json!({"schema":1,"op":"identify","nonce":marker.marker.nonce,"server_pid":789,"server_uid":0,"client_pid":123,"client_uid":6583,"client_start_ticks":456,"ui_sha256":marker.marker.ui_sha256,"broker_sha256":marker.marker.broker_sha256});
            helper["response_sent"] = json!(true);
            helper["response"] = response.clone();
            observed["identify_response"] = response;
            observed["root_message_credentials"] = root_peer;
            helper["identify_request"] =
                json!({"schema":1,"op":"identify","nonce":marker.marker.nonce});
            helper["identify_message_credentials"] = own_credentials;
            let peer = json!({"pid":123,"uid_fields":[6583,6583,6583,6583],"start_ticks":456,"exe_sha256":marker.marker.ui_sha256,"exe_path":crate::broker_protocol::PROOF_EXE_PATH,"root_path":crate::broker_protocol::PROOF_JAIL_ROOT});
            helper["peer_before"] = peer.clone();
            helper["peer_at_identify_send"] = peer;
            helper["enrollment"] = json!({"schema":1,"nonce":marker.marker.nonce,"client_pid":123,"client_uid":6583,"client_start_ticks":456,"ui_sha256":marker.marker.ui_sha256,"broker_sha256":marker.marker.broker_sha256,"context_reader_sha256":"d".repeat(64),"expires_unix":1000});
            let measurement = json!({"sha256":"d".repeat(64),"identity":{"fixture":true}});
            let mut before = context();
            let mut operations = Vec::new();
            let wanted = [
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
            for (index, (key, uri, payload)) in wanted.into_iter().enumerate() {
                let pid = 1001 + index;
                journals.push(json!({"schema":1,"stage":"context_child_started","child_pid":pid,"process_group":pid,"program_sha256":"d".repeat(64)}));
                operations.push(json!({"key":key,"uri":uri,"argv":["/usr/bin/luna-send","-i","-n","1","-w","5000","-f",uri,payload],"attempted":true,"success":true,"child_pid":pid,"process_group":pid,"exit_code":0,"signal":null,"timed_out":false,"stdout_bytes":100,"stderr_bytes":0,"stderr_sha256":sha256(&[]),"stdout_eof":true,"stderr_eof":true,"child_reaped":true,"cleanup_known":true,"held_empty_stdin":true,"stdin_bytes_sent":0,"group_cleanup_attempted":true,"group_kill_errno":null,"error":null,"executable_before":measurement,"executable_after":measurement}));
            }
            before["_reader"] = json!({"schema":1,"kind":"fixed_core_luna_send","program":"/usr/bin/luna-send","approved":true,"qualified":true,"cleanup_known":true,"error":null,"executable":measurement,"executable_after":measurement,"operations":operations});
            helper["context_before"] = before;
        }
        let cap = json!({"schema":1,"app_id":crate::identity::PROOF,"uid":6583,"verified":true,"registered":true,"request_attempted":true,"check_root_calls":1,"uri":"luna://org.webosbrew.hbchannel.service/checkRoot","payload":{},"identity_stable":true,"operation_deadline_expired":false,"error":null,"own_before":own,"own_at_call":own,"own_after":own,"context_before":context(),"context_after":context(),"response":crate::artwork_startup_probe::parse_response(r#"{"returnValue":true}"#)});
        let mut report = json!({"schema":1,"scenario":scenario,"marker":marker.marker,"fixed_command":crate::broker_startup_proof::COMMAND,"exec_uri":crate::broker_startup_proof::EXEC_URI,"exec_worker_joined":true,"protocol_worker_joined":true,"interrupted":false,"error":null,"positive_startup_qualified":false,"resource_reads":false,"normal_loader_qualified":false,"production_startup_qualified":false,"unknown_outcome_is_no_start":false,"protocol":observed,"hbc":{"uri":crate::broker_startup_proof::EXEC_URI,"command":crate::broker_startup_proof::COMMAND,"attempts":1,"error":null,"capability":cap,"own_at_call":own,"context":context(),"parsed":{"journals":journals,"helper_report":helper}}});
        sync_raw(&mut report);
        (marker, report)
    }
    #[test]
    fn strict_cases_and_nonce_mutation_do_not_select_endpoint_or_operation() {
        for case in [Case::WrongIdentifyNonce, Case::WrongFinishNonce] {
            let marker = marker(case);
            let raw = serde_json::to_vec(&marker).unwrap();
            assert!(serde_json::from_slice::<NegativeMarker>(&raw)
                .unwrap()
                .validate()
                .is_ok());
            let different = bad_nonce(&marker.marker.nonce).unwrap();
            assert_ne!(different, marker.marker.nonce);
            assert_eq!(different.len(), 32);
            let bytes = packet(&marker).unwrap();
            let value: Value = serde_json::from_slice(&bytes).unwrap();
            assert_eq!(value["nonce"], different);
            assert_eq!(
                value["op"],
                if case == Case::WrongIdentifyNonce {
                    "identify"
                } else {
                    "finish_artwork"
                }
            );
        }
        for raw in [
            r#"{"schema":1,"scenario":"other","marker":{}}"#,
            r#"{"schema":true,"scenario":"wrong_identify_nonce","marker":{}}"#,
            r#"{"schema":1,"schema":1,"scenario":"wrong_identify_nonce","marker":{}}"#,
        ] {
            assert!(serde_json::from_str::<NegativeMarker>(raw).is_err());
        }
        let mut extra = serde_json::to_value(marker(Case::WrongIdentifyNonce)).unwrap();
        extra["command"] = json!("other");
        assert!(serde_json::from_value::<NegativeMarker>(extra).is_err());
        let original = serde_json::to_string(&marker(Case::WrongIdentifyNonce)).unwrap();
        for replacement in [
            r#""marker":{"schema":1,"schema":1"#,
            r#""marker":{"extra":0,"schema":1"#,
        ] {
            let malformed = original.replace(r#""marker":{"schema":1"#, replacement);
            assert_ne!(malformed, original);
            assert!(serde_json::from_str::<NegativeMarker>(&malformed).is_err());
        }
        let mut mapped = serde_json::to_value(marker(Case::WrongIdentifyNonce)).unwrap();
        mapped["scenario"] = json!({"wrong_identify_nonce":null});
        assert!(serde_json::from_value::<NegativeMarker>(mapped).is_err());
    }
    #[test]
    fn both_synthetic_shapes_grade_expected_refusal_without_positive_startup() {
        for case in [Case::WrongIdentifyNonce, Case::WrongFinishNonce] {
            let (marker, report) = fixture(case);
            assert!(expected_refusal(&report, &marker));
            assert_eq!(report["positive_startup_qualified"], false);
            assert!(!report["hbc"]["parsed"]["stderr"]
                .as_str()
                .unwrap()
                .is_empty());
        }
    }
    #[test]
    fn false_hbc_alone_eof_or_missing_worker_is_never_expected_refusal() {
        let (marker, good) = fixture(Case::WrongIdentifyNonce);
        for pointer in [
            "/exec_worker_joined",
            "/protocol_worker_joined",
            "/protocol/negative_clean_eof",
            "/protocol/wrong_packet/send_completed",
        ] {
            let mut bad = good.clone();
            *bad.pointer_mut(pointer).unwrap() = json!(false);
            assert!(!expected_refusal(&bad, &marker));
        }
        let mut bad = good;
        bad["hbc"]["parsed"]["helper_report"] = Value::Null;
        assert!(!expected_refusal(&bad, &marker));
    }
    #[test]
    fn authority_identity_and_exact_refusal_stage_drift_fail() {
        for case in [Case::WrongIdentifyNonce, Case::WrongFinishNonce] {
            let (marker, good) = fixture(case);
            for (pointer, value) in [
                ("/error", json!("wrong error")),
                ("/qualified", json!(true)),
                ("/connection_closed", json!(true)),
                ("/root_process/start_ticks", json!(0)),
                ("/installation/pins/profile", json!("preview")),
                ("/installation/pins/ui_sha256", json!("e".repeat(64))),
                (
                    "/installation/files/helper/trusted_for_execution",
                    json!(false),
                ),
                ("/installation/files/ui/trusted_for_execution", json!(true)),
                ("/connect_credentials/gid", json!(1)),
                ("/listener_bound", json!(false)),
            ] {
                let mut bad = good.clone();
                *bad["hbc"]["parsed"]["helper_report"]
                    .pointer_mut(pointer)
                    .unwrap() = value;
                sync_raw(&mut bad);
                assert!(!expected_refusal(&bad, &marker), "{pointer}");
            }
            let mut bad = good;
            bad["protocol"]["root_peer"]["pid"] = json!(790);
            assert!(!expected_refusal(&bad, &marker));
        }
    }
    #[test]
    fn rejected_fields_are_not_invented_and_child_count_is_exact() {
        let (marker, good) = fixture(Case::WrongIdentifyNonce);
        for field in [
            "context_before",
            "context_after",
            "enrollment",
            "identify_message_credentials",
            "finish_message_credentials",
            "finished",
        ] {
            let mut bad = good.clone();
            bad["hbc"]["parsed"]["helper_report"][field] = Value::Null;
            sync_raw(&mut bad);
            assert!(!expected_refusal(&bad, &marker), "{field}");
        }
        let (marker, good) = fixture(Case::WrongFinishNonce);
        for pointer in [
            "/peer_before/uid_fields/0",
            "/peer_at_identify_send/start_ticks",
            "/identify_request/nonce",
            "/enrollment/context_reader_sha256",
        ] {
            let mut bad = good.clone();
            *bad["hbc"]["parsed"]["helper_report"]
                .pointer_mut(pointer)
                .unwrap() = json!("bad");
            sync_raw(&mut bad);
            assert!(!expected_refusal(&bad, &marker), "{pointer}");
        }
        let mut bad = good.clone();
        bad["hbc"]["parsed"]["journals"]
            .as_array_mut()
            .unwrap()
            .pop();
        sync_raw(&mut bad);
        assert!(!expected_refusal(&bad, &marker));
        for pointer in [
            "/context_before/_reader/operations/0/child_reaped",
            "/context_before/_reader/cleanup_known",
        ] {
            let mut bad = good.clone();
            *bad["hbc"]["parsed"]["helper_report"]
                .pointer_mut(pointer)
                .unwrap() = json!(false);
            sync_raw(&mut bad);
            assert!(!expected_refusal(&bad, &marker));
        }
    }
}
