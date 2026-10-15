//! Isolated fixed-command startup and kernel-bound Identify/Finish observation.
//! Two joined workers keep LS2's command completion wait off the native UI.
use crate::{broker_protocol::Marker, model::Page, runtime::Window, shell::Shell};
use serde_json::{json, Value};
use std::{
    io::Write,
    os::unix::fs::OpenOptionsExt,
    path::Path,
    sync::{
        atomic::{AtomicBool, Ordering},
        Arc,
    },
    time::{Duration, Instant},
};

pub const EXEC_URI: &str = "luna://org.webosbrew.hbchannel.service/exec";
pub const COMMAND: &str = "/var/lib/luma-home-proof/artwork-broker-startup-proof";
#[cfg(any(target_arch = "arm", test))]
const REPLY_LIMIT: usize = 512 * 1024;

pub fn read_marker(path: &Path) -> Result<Marker, String> {
    #[cfg(target_arch = "arm")]
    {
        crate::broker_probe::read_marker_file(path)
    }
    #[cfg(not(target_arch = "arm"))]
    {
        let _ = path;
        Err("Installed broker startup requires native Proof".into())
    }
}

#[cfg(any(target_arch = "arm", test))]
fn base64(bytes: &[u8]) -> String {
    const TABLE: &[u8] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";
    let mut result = String::new();
    for part in bytes.chunks(3) {
        let value = (u32::from(part[0]) << 16)
            | (u32::from(*part.get(1).unwrap_or(&0)) << 8)
            | u32::from(*part.get(2).unwrap_or(&0));
        for shift in [18, 12, 6, 0] {
            result.push(
                if shift == 0 && part.len() < 3 || shift == 6 && part.len() < 2 {
                    '='
                } else {
                    TABLE[((value >> shift) & 63) as usize] as char
                },
            );
        }
    }
    result
}

/// The inspected installed HBC uses String plus base64 representations, not
/// JSON-encoded stdout strings. Refusals preserve output for child reconciliation.
#[cfg(any(target_arch = "arm", test))]
pub(crate) fn parse_exec_reply(raw: &str) -> Result<Value, String> {
    if raw.len() > REPLY_LIMIT {
        return Err("Oversized SDK exec reply".into());
    }
    let value = crate::broker_context::strict_object(raw.as_bytes()).map_err(|e| e.to_string())?;
    let object = value.as_object().ok_or("Expected HBC object")?;
    let success = value["returnValue"]
        .as_bool()
        .ok_or("Literal HBC returnValue required")?;
    let mut expected = vec![
        "returnValue",
        "stdoutString",
        "stdoutBytes",
        "stderrString",
        "stderrBytes",
    ];
    if !success {
        expected.push("errorText");
    }
    if object.len() != expected.len()
        || expected.iter().any(|key| !object.contains_key(*key))
        || (!success && !value["errorText"].is_string())
    {
        return Err("Unrecognized HBC exec envelope".into());
    }
    let stdout = value["stdoutString"]
        .as_str()
        .ok_or("Expected stdoutString")?;
    let stderr = value["stderrString"]
        .as_str()
        .ok_or("Expected stderrString")?;
    if stdout.len() > 192 * 1024
        || stderr.len() > 8192
        || value["stdoutBytes"].as_str() != Some(base64(stdout.as_bytes()).as_str())
        || value["stderrBytes"].as_str() != Some(base64(stderr.as_bytes()).as_str())
        || (success && !stderr.is_empty())
    {
        return Err("HBC output representation or bound mismatch".into());
    }
    let mut rows = Vec::new();
    for line in stdout.lines() {
        if rows.len() >= 32 || line.is_empty() {
            return Err("Invalid helper output framing".into());
        }
        rows.push(
            crate::broker_context::strict_object(line.as_bytes()).map_err(|e| e.to_string())?,
        );
    }
    let report = rows.pop().ok_or("Missing helper final report")?;
    if report["schema"] != 1
        || report["stage"] != "startup_proof_complete"
        || report["profile"] != "proof"
    {
        return Err("Missing fixed helper completion report".into());
    }
    for row in &rows {
        if row.as_object().map(|o| o.len()) != Some(5)
            || row["schema"] != 1
            || row["stage"] != "context_child_started"
            || row["child_pid"].as_u64().is_none_or(|p| p <= 1)
            || row["child_pid"] != row["process_group"]
            || row["program_sha256"].as_str().is_none_or(|s| {
                s.len() != 64
                    || !s
                        .bytes()
                        .all(|c| c.is_ascii_digit() || (b'a'..=b'f').contains(&c))
            })
        {
            return Err("Unrecognized helper child journal".into());
        }
    }
    Ok(
        json!({"envelope_success":success,"error_text":value.get("errorText"),
        "journals":rows,"helper_report":report,"stderr":stderr}),
    )
}

pub(crate) fn save(path: &Path, value: &Value) -> Result<(), String> {
    let mut file = std::fs::OpenOptions::new()
        .write(true)
        .create_new(true)
        .mode(0o600)
        .custom_flags(libc::O_NOFOLLOW | libc::O_CLOEXEC)
        .open(path)
        .map_err(|e| e.to_string())?;
    serde_json::to_writer_pretty(&mut file, value).map_err(|e| e.to_string())?;
    file.write_all(b"\n")
        .and_then(|_| file.sync_all())
        .map_err(|e| e.to_string())
}

#[cfg(target_arch = "arm")]
pub(crate) mod native {
    use super::*;
    use crate::{
        artwork_transfer as wire, broker_probe as probe, broker_protocol as protocol,
        broker_transport::{Connection, Credentials},
    };
    use std::sync::mpsc::{self, Sender};

    pub(crate) fn invoke(marker: &Marker, cancel: &AtomicBool, started: Sender<()>) -> Value {
        let mut report = json!({"uri":EXEC_URI,"command":COMMAND,"attempts":0,
            "capability":null,"context":null,"own_at_call":null,"raw":null,
            "raw_bytes":0,"raw_truncated":false,"parsed":null,"error":null});
        let result = (|| -> Result<(), String> {
            let capability = crate::artwork_startup_probe::collect();
            report["capability"] = capability.clone();
            if capability["verified"] != true || cancel.load(Ordering::Acquire) {
                return Err("Fresh inert HBC capability gate declined".into());
            }
            let own = probe::own_identity()?;
            if own.ui_sha256 != marker.ui_sha256 {
                return Err("UI differs from installed marker".into());
            }
            let bus = plx_platform::webos::ls2::register().map_err(|e| format!("{e:?}"))?;
            let context = probe::read_context(&bus, Instant::now() + Duration::from_secs(3));
            report["context"] = context.clone();
            let current = probe::own_identity()?;
            report["own_at_call"] = json!(current);
            if !probe::context_valid(&context) || current != own || cancel.load(Ordering::Acquire) {
                return Err("Caller/context changed before fixed startup request".into());
            }
            report["attempts"] = json!(1);
            let _ = started.send(());
            let raw = bus
                .call(
                    EXEC_URI,
                    &json!({"command":COMMAND}).to_string(),
                    Duration::from_secs(30),
                )
                .map_err(|e| match e {
                    plx_platform::webos::ls2::Fail::Timeout => {
                        "SDK exec completion unobserved: reply wait timed out".to_owned()
                    }
                    plx_platform::webos::ls2::Fail::Setup {
                        stage,
                        detail,
                        code,
                    } => format!("SDK exec setup failed at {stage}, code {code:?}: {detail}"),
                })?;
            report["raw_bytes"] = json!(raw.len());
            let mut end = raw.len().min(REPLY_LIMIT);
            while !raw.is_char_boundary(end) {
                end -= 1;
            }
            report["raw"] = json!(&raw[..end]);
            report["raw_truncated"] = json!(raw.len() > REPLY_LIMIT);
            report["parsed"] = parse_exec_reply(&raw)?;
            Ok(())
        })();
        if let Err(error) = result {
            report["error"] = json!(error);
        }
        report
    }

    fn credentials(value: Credentials) -> Value {
        json!({"pid":value.pid,"uid":value.uid,"gid":value.gid})
    }
    fn exchange(
        marker: &Marker,
        output: &Path,
        cancel: &AtomicBool,
        started: mpsc::Receiver<()>,
    ) -> Result<Value, String> {
        started
            .recv_timeout(Duration::from_secs(12))
            .map_err(|_| "Fixed startup was not attempted")?;
        let deadline = Instant::now() + Duration::from_secs(24);
        if cancel.load(Ordering::Acquire) {
            return Err("Startup observation cancelled".into());
        }
        let own = probe::own_identity()?;
        if own.ui_sha256 != marker.ui_sha256 {
            return Err("UI ELF differs from marker".into());
        }
        // connect_until retries only the transport's not-yet-listening/backlog
        // conditions. No HBC retry, second connection or protocol retry follows.
        let connection = Connection::connect_until(
            &marker.nonce,
            deadline.min(Instant::now() + Duration::from_secs(8)),
        )
        .map_err(|e| e.to_string())?;
        let peer = connection.peer_credentials().map_err(|e| e.to_string())?;
        if peer.uid != 0 || peer.pid <= 1 {
            return Err("Startup peer is not Root".into());
        }
        save(
            &output.join("broker-startup-connected.json"),
            &json!({"schema":1,
            "own":own,"root_peer":credentials(peer),"root_message_credentials":null,
            "identify_received":false}),
        )?;
        let request = protocol::Request {
            schema: 1,
            op: protocol::Operation::Identify,
            nonce: marker.nonce.clone(),
        };
        connection
            .send_until(
                &serde_json::to_vec(&request).map_err(|e| e.to_string())?,
                deadline,
            )
            .map_err(|e| e.to_string())?;
        let packet = connection
            .recv_until(protocol::RESPONSE_LIMIT, deadline)
            .map_err(|e| e.to_string())?;
        let response = protocol::parse_response(&packet.bytes).map_err(|e| e.to_string())?;
        probe::bind_response(
            marker,
            &own,
            &probe::own_identity()?,
            peer,
            packet.credentials,
            &response,
        )?;
        let mut result = json!({"schema":1,"own":own,"root_peer":credentials(peer),
            "root_message_credentials":credentials(packet.credentials),"response":response,
            "finished":null,"clean_eof":false,"hold_ms":0});
        // This is a live kernel-peer channel. HBC buffers stdout until exit,
        // so its eventual output is deliberately not used for live PID binding.
        save(&output.join("broker-startup-identify.json"), &result)?;
        let hold = Instant::now();
        while hold.elapsed() < Duration::from_secs(8) {
            if cancel.load(Ordering::Acquire) {
                return Err("Startup proof interrupted during Root binding".into());
            }
            std::thread::sleep(Duration::from_millis(20));
        }
        result["hold_ms"] = json!(hold.elapsed().as_millis());
        let finish = wire::FinishRequest::new(1, &marker.nonce).map_err(|e| e.to_string())?;
        connection
            .send_until(
                &serde_json::to_vec(&finish).map_err(|e| e.to_string())?,
                deadline,
            )
            .map_err(|e| e.to_string())?;
        let finished = wire::receive_finished(&connection, &finish, &response, peer, deadline)
            .map_err(|e| e.to_string())?;
        probe::bind_response(marker, &own, &probe::own_identity()?, peer, peer, &response)?;
        result["finished"] = json!(finished);
        result["clean_eof"] = json!(true);
        Ok(result)
    }

    pub(super) fn collect(marker: Marker, output: &Path, cancel: Arc<AtomicBool>) -> Value {
        let mut report = json!({"schema":1,"fixed_command":COMMAND,"exec_uri":EXEC_URI,
            "protocol":null,"hbc":null,"exec_worker_joined":false,"qualified":false,
            "error":null,"resource_reads":false,"normal_loader_qualified":false,
            "production_startup_qualified":false,"unknown_outcome_is_no_start":false});
        let (tx, rx) = mpsc::channel();
        let invoke_marker = marker.clone();
        let invoke_cancel = cancel.clone();
        let worker = match std::thread::Builder::new()
            .name("broker-startup-ls2".into())
            .spawn(move || invoke(&invoke_marker, &invoke_cancel, tx))
        {
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
        // Joining observes command completion independently of the socket. A
        // timeout/malformed completion never means that no Root child started.
        match worker.join() {
            Ok(value) => {
                report["hbc"] = value;
                report["exec_worker_joined"] = json!(true);
            }
            Err(_) => report["error"] = json!("Exec worker panicked; startup outcome unknown"),
        }
        report["qualified"] = json!(qualification(&report, &marker));
        report
    }
}

#[cfg(any(target_arch = "arm", test))]
fn qualification(report: &Value, marker: &Marker) -> bool {
    let rpc = &report["hbc"];
    let observed = &report["protocol"];
    let helper = &rpc["parsed"]["helper_report"];
    let root = &helper["root_process"];
    report["exec_worker_joined"] == true
        && report["error"].is_null()
        && rpc["attempts"] == 1
        && rpc["error"].is_null()
        && rpc["parsed"]["envelope_success"] == true
        && observed["clean_eof"] == true
        && observed["root_peer"]["uid"] == 0
        && observed["root_peer"]["pid"] == observed["response"]["server_pid"]
        && observed["root_peer"] == observed["root_message_credentials"]
        && observed["hold_ms"].as_u64().is_some_and(|v| v >= 8000)
        && helper["qualified"] == true
        && helper["outcome"] == "identified_finished"
        && helper["connection_closed"] == true
        && helper["server_pid"] == observed["response"]["server_pid"]
        && root["pid"] == observed["response"]["server_pid"]
        && root["uid_fields"] == json!([0, 0, 0, 0])
        && root["start_ticks"].as_u64().is_some_and(|v| v > 0)
        && root["root_path"] == "/"
        && root["exe_path"] == COMMAND
        && root["exe_sha256"].as_str() == Some(marker.broker_sha256.as_str())
        && helper["broker_sha256"].as_str() == Some(marker.broker_sha256.as_str())
        && helper["response"] == observed["response"]
        && helper["finished"] == observed["finished"]
        && [
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
        .all(|key| helper[*key] == false)
        && ["context_before", "context_after"].iter().all(|key| {
            let context = &helper[*key];
            let reader = &context["_reader"];
            crate::broker_probe::context_valid(context)
                && reader["approved"] == true
                && reader["qualified"] == true
                && reader["cleanup_known"] == true
                && reader["operations"].as_array().is_some_and(|rows| {
                    rows.len() == 3
                        && rows
                            .iter()
                            .all(|row| row["child_reaped"] == true && row["cleanup_known"] == true)
                })
        })
}

pub fn run(
    shell: &mut Shell,
    window: &Window,
    output: &Path,
    now: &mut u32,
    marker: Marker,
) -> Result<Value, Box<dyn std::error::Error>> {
    let before = shell.preferences().clone();
    shell.open(Page::Home);
    let cancel = Arc::new(AtomicBool::new(false));
    let worker_cancel = cancel.clone();
    let directory = output.to_path_buf();
    let worker = std::thread::Builder::new()
        .name("broker-startup-protocol".into())
        .spawn(move || {
            #[cfg(target_arch = "arm")]
            {
                native::collect(marker, &directory, worker_cancel)
            }
            #[cfg(not(target_arch = "arm"))]
            {
                let _ = (marker, directory, worker_cancel);
                json!({"unsupported_host":true,"qualified":false})
            }
        })?;
    let start = Instant::now();
    let mut interrupted = false;
    while start.elapsed() < Duration::from_secs(8) || !worker.is_finished() {
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
        .map_err(|_| "Broker protocol worker panicked; outcome unknown")?;
    report["protocol_worker_joined"] = json!(true);
    report["interrupted"] = json!(interrupted);
    report["ui_elapsed_ms"] = json!(start.elapsed().as_millis());
    if interrupted {
        report["qualified"] = json!(false);
    }
    save(&output.join("broker-startup-proof.json"), &report)?;
    if interrupted
        || shell.preferences() != &before
        || !shell.take_requests().is_empty()
        || shell.should_exit()
    {
        return Err("Broker startup proof interrupted or changed UI state".into());
    }
    shell.draw();
    if window.graphics_state().error != 0 {
        return Err("Broker startup capture GL error".into());
    }
    window.capture(&output.join("broker-startup-home.ppm"))?;
    window.swap();
    Ok(report)
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn output_bytes_are_checked_as_base64_not_json_string_encoding() {
        for (bytes, expected) in [
            (b"".as_slice(), ""),
            (b"f", "Zg=="),
            (b"fo", "Zm8="),
            (b"foo", "Zm9v"),
        ] {
            assert_eq!(base64(bytes), expected);
        }
        let stdout="{\"schema\":1,\"stage\":\"startup_proof_complete\",\"profile\":\"proof\",\"qualified\":false}\n";
        let value = json!({"returnValue":true,"stdoutString":stdout,"stdoutBytes":base64(stdout.as_bytes()),"stderrString":"","stderrBytes":""});
        assert!(parse_exec_reply(&value.to_string()).unwrap()["envelope_success"] == true);
        let mut bad = value.clone();
        bad["stdoutBytes"] = json!("eA==");
        assert!(parse_exec_reply(&bad.to_string()).is_err());
        let mut bad = value;
        bad["errorText"] = Value::Null;
        assert!(parse_exec_reply(&bad.to_string()).is_err());
    }
    #[test]
    fn negative_exec_keeps_helper_output_without_qualifying_startup() {
        let stdout="{\"schema\":1,\"stage\":\"startup_proof_complete\",\"profile\":\"proof\",\"outcome\":\"refused\"}\n";
        let value = json!({"returnValue":false,"errorText":"child failed","stdoutString":stdout,"stdoutBytes":base64(stdout.as_bytes()),"stderrString":"refused","stderrBytes":base64(b"refused")});
        let parsed = parse_exec_reply(&value.to_string()).unwrap();
        assert_eq!(parsed["envelope_success"], false);
        assert_eq!(parsed["helper_report"]["outcome"], "refused");
    }
    #[test]
    fn duplicate_or_unknown_output_never_accepts_a_success_envelope() {
        assert!(parse_exec_reply("{\"returnValue\":true,\"returnValue\":true}").is_err());
        assert!(parse_exec_reply(&"x".repeat(REPLY_LIMIT + 1)).is_err());
        let marker = Marker {
            schema: 1,
            nonce: "a".repeat(32),
            ui_sha256: "b".repeat(64),
            broker_sha256: "c".repeat(64),
        };
        assert!(!qualification(
            &json!({"exec_worker_joined":true,"hbc":{"attempts":1,"parsed":{"envelope_success":true}}}),
            &marker
        ));
    }
    #[test]
    fn startup_requires_kernel_identity_matching_finish_and_reaped_context_children() {
        let marker = Marker {
            schema: 1,
            nonce: "a".repeat(32),
            ui_sha256: "b".repeat(64),
            broker_sha256: "c".repeat(64),
        };
        let response = crate::broker_protocol::Response {
            schema: 1,
            op: crate::broker_protocol::Operation::Identify,
            nonce: marker.nonce.clone(),
            server_pid: 800,
            server_uid: 0,
            client_pid: 123,
            client_uid: 6583,
            client_start_ticks: 456,
            ui_sha256: marker.ui_sha256.clone(),
            broker_sha256: marker.broker_sha256.clone(),
        };
        let finish = crate::artwork_transfer::FinishRequest::new(1, &marker.nonce).unwrap();
        let finished = crate::artwork_transfer::Finished::new(&finish, &response).unwrap();
        let context = json!({"system_info":{"returnValue":true,"modelName":"OLED55G26LA","firmwareVersion":"23.25.55","sdkVersion":"9.2.2"},
            "foreground":{"returnValue":true,"appId":crate::identity::PROOF},
            "windows":{"returnValue":true,"windows":[{"appId":crate::identity::PROOF,"windowType":"_WEBOS_WINDOW_TYPE_CARD"}]},
            "_reader":{"approved":true,"qualified":true,"cleanup_known":true,
                "operations":[{"child_reaped":true,"cleanup_known":true},{"child_reaped":true,"cleanup_known":true},{"child_reaped":true,"cleanup_known":true}]}});
        let helper = json!({"qualified":true,"outcome":"identified_finished","connection_closed":true,
            "server_pid":800,"broker_sha256":marker.broker_sha256,"response":response,"finished":finished,
            "root_process":{"pid":800,"uid_fields":[0,0,0,0],"start_ticks":999,"exe_path":COMMAND,"exe_sha256":marker.broker_sha256,"root_path":"/"},
            "icon_reads":false,"artwork_reads":false,"commands":false,"setting_writes":false,"root_file_writes":false,
            "daemonized":false,"normal_loader_qualified":false,"production_startup_qualified":false,
            "context_before":context,"context_after":context});
        let good = json!({"exec_worker_joined":true,"error":null,
            "hbc":{"attempts":1,"error":null,"parsed":{"envelope_success":true,"helper_report":helper}},
            "protocol":{"clean_eof":true,"hold_ms":8001,"response":response,"finished":finished,
                "root_peer":{"pid":800,"uid":0,"gid":0},"root_message_credentials":{"pid":800,"uid":0,"gid":0}}});
        assert!(qualification(&good, &marker));
        for pointer in [
            "/hbc/parsed/helper_report/root_process/exe_sha256",
            "/hbc/parsed/helper_report/context_after/_reader/operations/0/child_reaped",
            "/protocol/finished",
            "/protocol/root_message_credentials",
            "/protocol/root_peer/uid",
            "/hbc/parsed/envelope_success",
            "/exec_worker_joined",
        ] {
            let mut changed = good.clone();
            *changed.pointer_mut(pointer).unwrap() = Value::Null;
            assert!(!qualification(&changed, &marker), "{pointer}");
        }
    }
}
