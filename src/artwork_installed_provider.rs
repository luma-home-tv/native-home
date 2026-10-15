//! Proof-only worker provider. One installed helper per fetch; no decoder or GL.
//! Unknown command/child completion quarantines this provider instance. A new
//! instance is not a recovery procedure for an unresolved helper.
use crate::{
    app_artwork::{EncodedFile, Source},
    artwork_worker::{Cancellation, Provider, ProviderError},
    broker_protocol::Marker,
};
use std::time::{Duration, Instant};

pub struct InstalledProvider {
    marker: Marker,
    quarantined: bool,
}
struct Init(Result<InstalledProvider, ProviderError>);
impl Provider for Init {
    fn fetch(
        &mut self,
        source: &Source,
        deadline: Instant,
        cancellation: &Cancellation,
    ) -> Result<EncodedFile, ProviderError> {
        match &mut self.0 {
            Ok(provider) => provider.fetch(source, deadline, cancellation),
            Err(error) => Err(*error),
        }
    }
}
impl InstalledProvider {
    pub fn start_worker(marker: Marker) -> std::io::Result<crate::artwork_worker::Worker> {
        crate::artwork_worker::Worker::start(move || Init(Self::new(marker)))
    }
    pub fn new(marker: Marker) -> Result<Self, ProviderError> {
        marker.validate().map_err(|_| ProviderError::Invalid)?;
        #[cfg(target_arch = "arm")]
        {
            let own = crate::broker_probe::own_identity().map_err(|_| ProviderError::Denied)?;
            if own.ui_sha256 != marker.ui_sha256 {
                return Err(ProviderError::Denied);
            }
            Ok(Self {
                marker,
                quarantined: false,
            })
        }
        #[cfg(not(target_arch = "arm"))]
        {
            let _ = marker;
            Err(ProviderError::Unavailable)
        }
    }
}
impl Provider for InstalledProvider {
    fn fetch(
        &mut self,
        source: &Source,
        deadline: Instant,
        cancellation: &Cancellation,
    ) -> Result<EncodedFile, ProviderError> {
        if self.quarantined {
            return Err(ProviderError::Unavailable);
        }
        if cancellation.is_cancelled() {
            return Err(ProviderError::Cancelled);
        }
        if Instant::now() >= deadline {
            return Err(ProviderError::Expired);
        }
        let deadline = deadline.min(Instant::now() + Duration::from_secs(30));
        #[cfg(target_arch = "arm")]
        {
            native::fetch(self, source, deadline, cancellation)
        }
        #[cfg(not(target_arch = "arm"))]
        {
            let _ = (&self.marker, source, deadline);
            Err(ProviderError::Unavailable)
        }
    }
}

#[cfg(any(target_arch = "arm", test))]
mod session {
    use super::*;
    use crate::{
        app_artwork::{FileIdentity, SourceKey},
        artwork_transfer as wire,
        broker_probe::OwnIdentity,
        broker_transport::Credentials,
    };
    use serde_json::{json, Value};
    use std::{
        sync::{
            atomic::{AtomicBool, Ordering},
            Arc,
        },
        thread::JoinHandle,
    };
    pub(super) struct ExecWorker {
        handle: Option<JoinHandle<serde_json::Value>>,
        cancel: Arc<AtomicBool>,
    }
    impl ExecWorker {
        pub(super) fn new(handle: JoinHandle<serde_json::Value>, cancel: Arc<AtomicBool>) -> Self {
            Self {
                handle: Some(handle),
                cancel,
            }
        }

        pub(super) fn join(mut self) -> Result<serde_json::Value, ProviderError> {
            self.handle
                .take()
                .ok_or(ProviderError::Unavailable)?
                .join()
                .map_err(|_| ProviderError::Unavailable)
        }
    }
    impl Drop for ExecWorker {
        fn drop(&mut self) {
            self.cancel.store(true, Ordering::Release);
            if let Some(handle) = self.handle.take() {
                let _ = handle.join();
            }
        }
    }
    pub(super) struct Exchange {
        pub own: OwnIdentity,
        pub root: Credentials,
        pub message: Credentials,
        pub client: Credentials,
        pub identity: wire::Identity,
        pub request: wire::Request,
        pub outcome: wire::Outcome,
        pub finished: Option<wire::Finished>,
        pub clean_eof: bool,
        pub header_matches: bool,
    }
    pub(super) fn header_matches(header: &wire::Header, source: &SourceKey) -> bool {
        header.app_id == source.app_id
            && header.version == source.version
            && header.class == source.class.into()
            && header.kind == source.kind.into()
    }
    fn known_reader(
        value: &Value,
        catalogue: bool,
        expected: &str,
        events: &mut Vec<Value>,
    ) -> Result<(), ProviderError> {
        let reader = &value["_reader"];
        if reader["schema"] != 1
            || reader["kind"] != "fixed_core_luna_send"
            || reader["program"] != "/usr/bin/luna-send"
            || reader["approved"] != true
            || reader["qualified"] != true
            || reader["cleanup_known"] != true
            || !reader["error"].is_null()
            || reader["executable"]["sha256"] != expected
            || reader["executable_after"] != reader["executable"]
        {
            return Err(ProviderError::Invalid);
        }
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
        if !catalogue && !crate::broker_probe::context_valid(value) {
            return Err(ProviderError::Denied);
        }
        if catalogue
            && (value["app_count"].as_u64().is_none()
                || value["launch_point_count"].as_u64().is_none())
        {
            return Err(ProviderError::Invalid);
        }
        let rows = reader["operations"]
            .as_array()
            .ok_or(ProviderError::Invalid)?;
        if rows.len() != wanted.len() {
            return Err(ProviderError::Invalid);
        }
        for (row, (key, uri, payload)) in rows.iter().zip(wanted) {
            let pid = row["child_pid"]
                .as_u64()
                .filter(|v| *v > 1)
                .ok_or(ProviderError::Invalid)?;
            if row["key"] != *key
                || row["uri"] != *uri
                || row["argv"]
                    != json!([
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
                || row["process_group"] != pid
                || row["attempted"] != true
                || row["success"] != true
                || row["exit_code"] != 0
                || !row["signal"].is_null()
                || row["timed_out"] != false
                || !row["error"].is_null()
                || row["child_reaped"] != true
                || row["cleanup_known"] != true
                || row["stdout_eof"] != true
                || row["stderr_eof"] != true
                || row["held_empty_stdin"] != true
                || row["stdin_bytes_sent"] != 0
                || row["group_cleanup_attempted"] != true
                || !row["group_kill_errno"].is_null()
                || row["stderr_bytes"] != 0
                || row["stdout_bytes"]
                    .as_u64()
                    .is_none_or(|v| v == 0 || v > if catalogue { 2 * 1024 * 1024 } else { 65536 })
                || row["executable_before"] != reader["executable"]
                || row["executable_after"] != reader["executable"]
            {
                return Err(ProviderError::Invalid);
            }
            events.push(json!({"schema":1,"stage":if catalogue {"catalogue_child_started"} else {"context_child_started"},"child_pid":pid,"process_group":pid,"program_sha256":expected}));
        }
        Ok(())
    }
    pub(super) fn lifecycle<'a>(
        hbc: &'a Value,
        exchange: &Exchange,
        marker: &Marker,
    ) -> Result<&'a Value, ProviderError> {
        if hbc["uri"] != crate::broker_startup_proof::EXEC_URI
            || hbc["command"] != crate::broker_startup_proof::COMMAND
            || hbc["attempts"] != 1
            || !hbc["error"].is_null()
            || hbc["raw_truncated"] != false
            || hbc["own_at_call"] != json!(exchange.own)
            || !crate::broker_probe::context_valid(&hbc["context"])
        {
            return Err(ProviderError::Invalid);
        }
        let cap = &hbc["capability"];
        let cap_raw = cap["response"]["raw"]
            .as_str()
            .ok_or(ProviderError::Invalid)?;
        if cap["verified"] != true
            || cap["registered"] != true
            || cap["request_attempted"] != true
            || cap["check_root_calls"] != 1
            || cap["uri"] != "luna://org.webosbrew.hbchannel.service/checkRoot"
            || cap["payload"] != json!({})
            || cap["identity_stable"] != true
            || cap["operation_deadline_expired"] != false
            || !cap["error"].is_null()
            || cap["own_before"] != json!(exchange.own)
            || cap["own_at_call"] != json!(exchange.own)
            || cap["own_after"] != json!(exchange.own)
            || !crate::broker_probe::context_valid(&cap["context_before"])
            || !crate::broker_probe::context_valid(&cap["context_after"])
            || cap["response"] != crate::artwork_startup_probe::parse_response(cap_raw)
            || cap["response"]["outcome"] != "root_true"
        {
            return Err(ProviderError::Denied);
        }
        let raw = hbc["raw"].as_str().ok_or(ProviderError::Invalid)?;
        if hbc["raw_bytes"].as_u64() != Some(raw.len() as u64) {
            return Err(ProviderError::Invalid);
        }
        let parsed = crate::broker_startup_proof::parse_exec_resource_reply(raw)
            .map_err(|_| ProviderError::Invalid)?;
        if hbc["parsed"] != parsed || parsed["envelope_success"] != true || parsed["stderr"] != "" {
            return Err(ProviderError::Invalid);
        }
        let helper = &hbc["parsed"]["helper_report"];
        let own = &exchange.own;
        let root = exchange.root;
        crate::broker_probe::bind_response(
            marker,
            own,
            own,
            root,
            exchange.message,
            &exchange.identity,
        )
        .map_err(|_| ProviderError::Denied)?;
        let root_process = &helper["root_process"];
        if !exchange.clean_eof
            || exchange.finished.is_none()
            || helper["qualified"] != true
            || helper["outcome"] != "identified_artwork_finished"
            || helper["resource_mode"] != true
            || helper["artwork_session_completed"] != true
            || helper["connection_closed"] != true
            || helper["response_sent"] != true
            || helper["icon_reads"] != true
            || !helper["error"].is_null()
            || helper["server_pid"] != root.pid
            || root_process["pid"] != root.pid
            || root_process["uid_fields"] != json!([0, 0, 0, 0])
            || root_process["start_ticks"].as_u64().is_none_or(|v| v == 0)
            || root_process["root_path"] != "/"
            || root_process["exe_path"] != crate::broker_startup_proof::COMMAND
            || root_process["exe_sha256"] != marker.broker_sha256
            || helper["broker_sha256"] != marker.broker_sha256
            || helper["nonce"] != marker.nonce
            || helper["response"] != json!(exchange.identity)
            || helper["finished_sent"] != true
            || helper["finished"] != json!(exchange.finished)
            || helper["artwork_finished"] != json!(exchange.finished)
        {
            return Err(ProviderError::Invalid);
        }
        for key in [
            "commands",
            "setting_writes",
            "root_file_writes",
            "daemonized",
            "normal_loader_qualified",
            "production_startup_qualified",
        ] {
            if helper[key] != false {
                return Err(ProviderError::Invalid);
            }
        }
        let client =
            json!({"pid":exchange.client.pid,"uid":exchange.client.uid,"gid":exchange.client.gid});
        if helper["connect_credentials"] != client
            || helper["identify_message_credentials"] != client
            || helper["artwork_finish_connect_credentials"] != client
            || helper["artwork_finish_message_credentials"] != client
        {
            return Err(ProviderError::Denied);
        }
        let installation = &helper["installation"];
        let pins = &installation["pins"];
        let core = pins["context_reader_sha256"]
            .as_str()
            .filter(|v| {
                v.len() == 64
                    && v.bytes()
                        .all(|b| b.is_ascii_digit() || (b'a'..=b'f').contains(&b))
            })
            .ok_or(ProviderError::Invalid)?;
        if pins["schema"] != 1
            || pins["profile"] != "proof"
            || pins["rendezvous_nonce"] != marker.nonce
            || pins["ui_sha256"] != marker.ui_sha256
            || pins["broker_sha256"] != marker.broker_sha256
            || installation["profile"] != "proof"
            || installation["app_id"] != crate::identity::PROOF
            || installation["root_process"] != *root_process
            || installation["files"]["helper"]["path"] != crate::broker_startup_proof::COMMAND
            || installation["files"]["helper"]["sha256"] != marker.broker_sha256
            || installation["files"]["helper"]["trusted_for_execution"] != true
            || installation["files"]["configuration"]["path"]
                != "/var/lib/luma-home-proof/broker-installation.json"
            || installation["files"]["configuration"]["trusted_for_execution"] != true
            || installation["files"]["ui"]["sha256"] != marker.ui_sha256
            || installation["files"]["ui"]["trusted_for_execution"] != false
        {
            return Err(ProviderError::Invalid);
        }
        let peer = json!({"pid":own.pid,"uid_fields":[own.uid,own.uid,own.uid,own.uid],"start_ticks":own.start_ticks,"exe_path":crate::broker_protocol::PROOF_EXE_PATH,"root_path":crate::broker_protocol::PROOF_JAIL_ROOT,"exe_sha256":own.ui_sha256});
        for key in [
            "peer_before",
            "peer_at_identify_send",
            "artwork_finish_peer_before",
            "artwork_finish_peer_after",
        ] {
            if helper[key] != peer {
                return Err(ProviderError::Denied);
            }
        }
        let reads = helper["artwork_reads"]
            .as_array()
            .filter(|v| v.len() == 1)
            .ok_or(ProviderError::Invalid)?;
        let record = &reads[0];
        if record["request"] != json!(exchange.request)
            || record["request_bytes"]
                != json!(serde_json::to_vec(&exchange.request).map_err(|_| ProviderError::Invalid)?)
            || record["connect_credentials"] != client
            || record["message_credentials"] != client
            || record["peer_before"] != peer
            || record["peer_after"] != peer
            || !record["error"].is_null()
        {
            return Err(ProviderError::Invalid);
        }
        let mut events = Vec::new();
        known_reader(&helper["context_before"], false, core, &mut events)?;
        known_reader(&record["context_before"], false, core, &mut events)?;
        known_reader(&record["catalogue_before"], true, core, &mut events)?;
        if !record["catalogue_after"].is_null() {
            known_reader(&record["catalogue_after"], true, core, &mut events)?;
        }
        known_reader(&record["context_after"], false, core, &mut events)?;
        known_reader(&helper["artwork_finish_context"], false, core, &mut events)?;
        let expected_count = if record["catalogue_after"].is_null() {
            14
        } else {
            16
        };
        if matches!(&exchange.outcome, wire::Outcome::Artwork(_)) && expected_count != 16 {
            return Err(ProviderError::Invalid);
        }
        if matches!(&exchange.outcome, wire::Outcome::Refused(_)) {
            if record.get("complete").is_some() || record["completed"] != false {
                return Err(ProviderError::Invalid);
            }
            if expected_count == 14
                && (record.get("header").is_some() || record["encoded_data_released"] != false)
            {
                return Err(ProviderError::Invalid);
            }
            if expected_count == 16 {
                let header: wire::Header = serde_json::from_value(record["header"].clone())
                    .map_err(|_| ProviderError::Invalid)?;
                header
                    .validate_for(&exchange.request, &exchange.identity)
                    .map_err(|_| ProviderError::Invalid)?;
                if record["encoded_data_released"] != true {
                    return Err(ProviderError::Invalid);
                }
            }
        }
        let unique = events
            .iter()
            .filter_map(|event| event["child_pid"].as_u64())
            .collect::<std::collections::BTreeSet<_>>();
        if events.len() != expected_count
            || unique.len() != expected_count
            || parsed["journals"] != json!(events)
        {
            return Err(ProviderError::Invalid);
        }
        let finish =
            wire::FinishRequest::new(2, &marker.nonce).map_err(|_| ProviderError::Invalid)?;
        exchange
            .finished
            .as_ref()
            .ok_or(ProviderError::Invalid)?
            .validate_for(&finish, &exchange.identity)
            .map_err(|_| ProviderError::Invalid)?;
        Ok(record)
    }
    pub(super) fn commit(
        source: &SourceKey,
        exchange: Exchange,
        record: &Value,
    ) -> Result<EncodedFile, ProviderError> {
        match exchange.outcome {
            wire::Outcome::Refused(refusal) => {
                refusal
                    .validate_for(&exchange.request, &exchange.identity)
                    .map_err(|_| ProviderError::Invalid)?;
                if record.get("complete").is_some()
                    || record["refusal"] != json!(refusal)
                    || record["completed"] != false
                {
                    return Err(ProviderError::Invalid);
                }
                Err(match refusal.reason {
                    wire::RefusalReason::Denied => ProviderError::Denied,
                    wire::RefusalReason::Changed => ProviderError::Changed,
                    wire::RefusalReason::Invalid | wire::RefusalReason::TooLarge => {
                        ProviderError::Invalid
                    }
                    _ => ProviderError::Unavailable,
                })
            }
            wire::Outcome::Artwork(artwork) => {
                if !exchange.header_matches
                    || !header_matches(&artwork.header, source)
                    || record["source_key"] != json!(source)
                    || record["source_key_after"] != json!(source)
                {
                    return Err(ProviderError::Changed);
                }
                artwork
                    .header
                    .validate_for(&exchange.request, &exchange.identity)
                    .map_err(|_| ProviderError::Invalid)?;
                artwork
                    .complete
                    .validate_for(&artwork.header)
                    .map_err(|_| ProviderError::Invalid)?;
                let digest = plx_base::sha256::sha256(&artwork.bytes);
                use std::fmt::Write;
                let mut digest_hex = String::with_capacity(64);
                for byte in digest {
                    let _ = write!(&mut digest_hex, "{byte:02x}");
                }
                if artwork.header.encoded_sha256 != digest_hex
                    || artwork.bytes.capacity() > wire::ENCODED_LIMIT
                {
                    return Err(ProviderError::Invalid);
                }
                if record["completed"] != true
                    || record["encoded_data_released"] != true
                    || record["header"] != json!(artwork.header)
                    || record["complete"] != json!(artwork.complete)
                    || record["read_identity"] != json!(artwork.header.file_identity)
                    || record["readback_identity"] != record["read_identity"]
                    || record["encoded_sha256"] != artwork.header.encoded_sha256
                    || record["readback_sha256"] != record["encoded_sha256"]
                    || record["encoded_bytes"] != json!(artwork.bytes.len())
                    || artwork.bytes.len() != artwork.header.encoded_bytes as usize
                {
                    return Err(ProviderError::Invalid);
                }
                let identity = artwork.header.file_identity;
                Ok(EncodedFile {
                    bytes: artwork.bytes,
                    identity: FileIdentity {
                        device: identity.device,
                        inode: identity.inode,
                        bytes: identity.bytes,
                        mode: identity.mode,
                        uid: identity.uid,
                        gid: identity.gid,
                        modified_seconds: identity.modified_seconds,
                        modified_nanos: identity.modified_nanos,
                        changed_seconds: identity.changed_seconds,
                        changed_nanos: identity.changed_nanos,
                    },
                    digest,
                    png_dimensions: None,
                })
            }
        }
    }
    pub(super) fn settle(
        provider: &mut InstalledProvider,
        source: &SourceKey,
        exchange: Result<Exchange, ProviderError>,
        joined: Result<Value, ProviderError>,
        cancelled: bool,
        expired: bool,
    ) -> Result<EncodedFile, ProviderError> {
        let completion = match joined {
            Ok(value) => value,
            Err(error) => {
                provider.quarantined = true;
                return Err(error);
            }
        };
        // The pinned invocation records attempts before requesting execution.
        // Only its joined pre-dispatch failure is a known no-helper outcome.
        if exchange.is_err()
            && completion["uri"] == crate::broker_startup_proof::EXEC_URI
            && completion["command"] == crate::broker_startup_proof::COMMAND
            && completion["attempts"] == 0
            && completion["raw"].is_null()
            && completion["parsed"].is_null()
            && completion["error"].is_string()
        {
            provider.quarantined = false;
            return Err(if cancelled {
                ProviderError::Cancelled
            } else if expired {
                ProviderError::Expired
            } else {
                ProviderError::Unavailable
            });
        }
        let exchange = match exchange {
            Ok(value) => value,
            Err(error) => {
                provider.quarantined = true;
                return Err(if cancelled {
                    ProviderError::Cancelled
                } else if expired {
                    ProviderError::Expired
                } else {
                    error
                });
            }
        };
        let record = match lifecycle(&completion, &exchange, &provider.marker) {
            Ok(value) => value,
            Err(error) => {
                provider.quarantined = true;
                return Err(error);
            }
        };
        provider.quarantined = false;
        if cancelled {
            return Err(ProviderError::Cancelled);
        }
        if expired {
            return Err(ProviderError::Expired);
        }
        commit(source, exchange, record)
    }
}

#[cfg(target_arch = "arm")]
mod native {
    use super::*;
    use crate::{
        artwork_transfer as wire, broker_probe as probe, broker_protocol as protocol,
        broker_transport::{Connection, Credentials},
    };
    use std::sync::{
        atomic::{AtomicBool, Ordering},
        mpsc, Arc,
    };
    fn check(
        deadline: Instant,
        cancellation: &Cancellation,
        stop: &AtomicBool,
    ) -> Result<(), ProviderError> {
        if cancellation.is_cancelled() {
            stop.store(true, Ordering::Release);
            Err(ProviderError::Cancelled)
        } else if Instant::now() >= deadline {
            stop.store(true, Ordering::Release);
            Err(ProviderError::Expired)
        } else {
            Ok(())
        }
    }
    fn exchange(
        marker: &Marker,
        source: &Source,
        deadline: Instant,
        cancellation: &Cancellation,
        stop: &AtomicBool,
        started: mpsc::Receiver<()>,
    ) -> Result<session::Exchange, ProviderError> {
        loop {
            check(deadline, cancellation, stop)?;
            match started.recv_timeout(Duration::from_millis(25)) {
                Ok(()) => break,
                Err(mpsc::RecvTimeoutError::Timeout) => continue,
                Err(_) => return Err(ProviderError::Unavailable),
            }
        }
        let own = probe::own_identity().map_err(|_| ProviderError::Denied)?;
        let connection = Connection::connect_until(&marker.nonce, deadline)
            .map_err(|_| ProviderError::Unavailable)?;
        let root = connection
            .peer_credentials()
            .map_err(|_| ProviderError::Denied)?;
        if root.uid != 0 || root.pid <= 1 {
            return Err(ProviderError::Denied);
        }
        check(deadline, cancellation, stop)?;
        let request = protocol::Request {
            schema: 1,
            op: protocol::Operation::Identify,
            nonce: marker.nonce.clone(),
        };
        connection
            .send_until(
                &serde_json::to_vec(&request).map_err(|_| ProviderError::Invalid)?,
                deadline,
            )
            .map_err(|_| ProviderError::Unavailable)?;
        let packet = connection
            .recv_until(protocol::RESPONSE_LIMIT, deadline)
            .map_err(|_| ProviderError::Unavailable)?;
        let identity =
            protocol::parse_response(&packet.bytes).map_err(|_| ProviderError::Invalid)?;
        probe::bind_response(
            marker,
            &own,
            &probe::own_identity().map_err(|_| ProviderError::Denied)?,
            root,
            packet.credentials,
            &identity,
        )
        .map_err(|_| ProviderError::Denied)?;
        check(deadline, cancellation, stop)?;
        let read = wire::Request::new(1, &marker.nonce, source.app_id(), wire::Variant::Preferred)
            .map_err(|_| ProviderError::Invalid)?;
        wire::send_request(&connection, &read, deadline).map_err(|_| ProviderError::Unavailable)?;
        let mut receiver =
            wire::Receiver::new(&read, &identity, root).map_err(|_| ProviderError::Invalid)?;
        let mut header_matches = true;
        loop {
            let packet = connection
                .recv_until(wire::MESSAGE_LIMIT, deadline)
                .map_err(|_| ProviderError::Unavailable)?;
            if let Ok(header) = wire::parse_header(&packet.bytes) {
                header_matches = session::header_matches(&header, source.key());
            }
            if receiver
                .push_packet(&packet.bytes, packet.credentials)
                .map_err(|_| ProviderError::Invalid)?
            {
                break;
            }
        }
        let outcome = receiver.finish().map_err(|_| ProviderError::Invalid)?;
        // Once a bounded transaction ended, Finish is cleanup even if the job
        // was cancelled. Its bytes can still never be returned after cancellation.
        if Instant::now() >= deadline {
            return Err(ProviderError::Expired);
        }
        let finish =
            wire::FinishRequest::new(2, &marker.nonce).map_err(|_| ProviderError::Invalid)?;
        wire::send_finish_request(&connection, &finish, deadline)
            .map_err(|_| ProviderError::Unavailable)?;
        let finished = wire::receive_finished(&connection, &finish, &identity, root, deadline)
            .map_err(|_| ProviderError::Unavailable)?;
        probe::bind_response(
            marker,
            &own,
            &probe::own_identity().map_err(|_| ProviderError::Denied)?,
            root,
            packet.credentials,
            &identity,
        )
        .map_err(|_| ProviderError::Denied)?;
        Ok(session::Exchange {
            own,
            root,
            message: packet.credentials,
            client: Credentials {
                pid: std::process::id() as i32,
                uid: unsafe { libc::geteuid() },
                gid: unsafe { libc::getegid() },
            },
            identity,
            request: read,
            outcome,
            finished: Some(finished),
            clean_eof: true,
            header_matches,
        })
    }
    pub(super) fn fetch(
        provider: &mut InstalledProvider,
        source: &Source,
        deadline: Instant,
        cancellation: &Cancellation,
    ) -> Result<EncodedFile, ProviderError> {
        let stop = Arc::new(AtomicBool::new(false));
        let invoke_stop = stop.clone();
        let marker = provider.marker.clone();
        let (tx, rx) = mpsc::channel();
        provider.quarantined = true; // Cleared only after known no-start or closed/reconciled session.
        let handle = match std::thread::Builder::new()
            .name("artwork-installed-ls2".into())
            .spawn(move || {
                crate::broker_startup_proof::native::invoke_resource(
                    &marker,
                    &invoke_stop,
                    tx,
                    deadline,
                )
            }) {
            Ok(handle) => handle,
            Err(_) => {
                provider.quarantined = false;
                return Err(ProviderError::Unavailable);
            }
        };
        let worker = session::ExecWorker::new(handle, stop.clone());
        let result = exchange(&provider.marker, source, deadline, cancellation, &stop, rx);
        if result.is_err() {
            stop.store(true, Ordering::Release);
        }
        let completion = worker.join();
        session::settle(
            provider,
            source.key(),
            result,
            completion,
            cancellation.is_cancelled(),
            Instant::now() >= deadline,
        )
    }
}

#[cfg(test)]
#[path = "artwork_installed_provider_tests.rs"]
mod tests;
