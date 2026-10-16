//! Private closed-negative proof. No I/O, image return, or stale-latch authority.
use super::{session, AttemptFailureKind, AttemptSeal, AttemptStage, AttemptTrace, Profile};
use crate::{
    app_artwork::SourceKey, artwork_session_guard::PendingIdentity, artwork_transfer as wire,
    artwork_worker::ProviderError, broker_protocol::Marker,
};
use serde_json::{json, Value};
use std::{collections::BTreeSet, sync::Arc};

pub(super) struct NegativeEvidence<'a> {
    pub(super) trace: &'a AttemptTrace,
    pub(super) joined_hbc: &'a Value,
    pub(super) marker: &'a Marker,
    pub(super) profile: Profile,
    pub(super) source: &'a SourceKey,
    pub(super) current_pending: &'a PendingIdentity,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Stage {
    ReadAfter,
    Finish,
}
impl Stage {
    fn name(self) -> &'static str {
        match self {
            Self::ReadAfter => "read_after",
            Self::Finish => "finish",
        }
    }
    fn request_id(self) -> u32 {
        match self {
            Self::ReadAfter => 1,
            Self::Finish => 2,
        }
    }
}
/// Deliberately not Clone/Copy/Serialize/Deserialize. Only the owner of the
/// current captured Ticket may consume this proof after checking its Arc seal.
pub(super) struct KnownClosed {
    seal: AttemptSeal,
}
impl KnownClosed {
    pub(super) fn matches_current(&self, current: &AttemptSeal) -> bool {
        Arc::ptr_eq(&self.seal, current)
    }
}
fn require(condition: bool) -> Result<(), ProviderError> {
    if condition {
        Ok(())
    } else {
        Err(ProviderError::Invalid)
    }
}
fn hash(value: &Value) -> bool {
    value.as_str().is_some_and(|s| {
        s.len() == 64
            && s.bytes()
                .all(|b| b.is_ascii_digit() || (b'a'..=b'f').contains(&b))
    })
}
fn file_identity(value: &Value, mode: u64, cap: u64) -> Result<(), ProviderError> {
    require(
        value["device"].as_u64().is_some()
            && value["inode"].as_u64().is_some_and(|n| n > 0)
            && value["mode"].as_u64() == Some(0o100000 | mode)
            && value["uid"].as_u64() == Some(0)
            && value["gid"].as_u64() == Some(0)
            && value["links"].as_u64() == Some(1)
            && value["bytes"].as_u64().is_some_and(|n| n > 0 && n <= cap)
            && value["mtime"].as_i64().is_some()
            && value["ctime"].as_i64().is_some()
            && ["mtime_ns", "ctime_ns"].iter().all(|key| {
                value[*key]
                    .as_i64()
                    .is_some_and(|n| (0..1_000_000_000).contains(&n))
            }),
    )
}
fn anchors(value: &Value, path: &str, authority: bool) -> Result<(), ProviderError> {
    let rows = value.as_array().ok_or(ProviderError::Invalid)?;
    let mut paths = vec!["/".to_owned()];
    let mut current = String::new();
    for part in path.trim_start_matches('/').split('/') {
        require(!part.is_empty() && !matches!(part, "." | ".."))?;
        current.push('/');
        current.push_str(part);
        paths.push(current.clone());
    }
    require(rows.len() == paths.len())?;
    for (row, path) in rows.iter().zip(paths) {
        let id = &row["identity"];
        let mode = id["mode"].as_u64().ok_or(ProviderError::Invalid)?;
        require(
            row["path"] == path
                && id["device"].as_u64().is_some()
                && id["inode"].as_u64().is_some_and(|n| n > 0)
                && id["uid"].as_u64() == Some(0)
                && id["gid"].as_u64() == Some(0)
                && mode & 0o170000 == 0o040000
                && (!authority || mode & 0o022 == 0),
        )?;
    }
    if authority {
        require(
            rows.last()
                .is_some_and(|row| row["identity"]["mode"] == 0o40700),
        )?;
    }
    Ok(())
}
fn installed_metadata(installation: &Value, profile: Profile) -> Result<(), ProviderError> {
    require(
        installation["schema"].as_u64() == Some(1)
            && installation["profile"] == json!(profile)
            && installation["bundle"] == profile.bundle()
            && installation["context_reader_measured_here"] == false
            && installation["normal_runtime_qualified"] == false,
    )?;
    for (name, mode, cap) in [
        ("configuration", 0o600, 4096),
        ("helper", 0o700, 8 * 1024 * 1024),
        ("ui", 0o755, 8 * 1024 * 1024),
    ] {
        let row = &installation["files"][name];
        require(hash(&row["sha256"]))?;
        file_identity(&row["identity"], mode, cap)?;
    }
    anchors(
        &installation["authority_anchors"],
        profile.installation_dir(),
        true,
    )?;
    anchors(&installation["ui_lookup_anchors"], profile.bundle(), false)
}

// The shared extractor deliberately preserves legacy positive checks. Negative
// closure additionally requires the complete source Measurement schema.
#[derive(serde::Deserialize)]
#[serde(deny_unknown_fields)]
struct CoreMeasurement {
    sha256: String,
    identity: CoreIdentity,
}
#[derive(serde::Deserialize)]
#[serde(deny_unknown_fields)]
struct CoreIdentity {
    device: u64,
    inode: u64,
    bytes: u64,
    mode: u32,
    uid: u32,
    gid: u32,
    links: u64,
    modified_seconds: i64,
    modified_nanos: i64,
    changed_seconds: i64,
    changed_nanos: i64,
}
fn core_measurement(value: &Value, expected: &str) -> Result<(), ProviderError> {
    let measurement: CoreMeasurement =
        serde_json::from_value(value.clone()).map_err(|_| ProviderError::Invalid)?;
    let identity = measurement.identity;
    // These fields are typed observations; every numeric value they can carry
    // is valid here. Equality across all operations/readers binds their values.
    let _observed = (
        identity.device,
        identity.modified_seconds,
        identity.changed_seconds,
    );
    require(
        measurement.sha256 == expected
            && identity.inode > 0
            && identity.mode & 0o170000 == 0o100000
            && matches!(identity.mode & 0o7777, 0o700 | 0o755 | 0o555)
            && identity.uid == 0
            && identity.gid == 0
            && identity.links == 1
            && (52..=8 * 1024 * 1024).contains(&identity.bytes)
            && (0..1_000_000_000).contains(&identity.modified_nanos)
            && (0..1_000_000_000).contains(&identity.changed_nanos),
    )
}

struct Readers<'a> {
    core: &'a str,
    profile: Profile,
    measurement: Option<Value>,
    events: Vec<Value>,
}
impl<'a> Readers<'a> {
    fn new(core: &'a str, profile: Profile) -> Self {
        Self {
            core,
            profile,
            measurement: None,
            events: Vec::new(),
        }
    }
    fn add(&mut self, value: &Value, kind: session::ReaderKind) -> Result<(), ProviderError> {
        let checked = session::reader_evidence(value, kind, self.core)?;
        core_measurement(&checked.measurement, self.core)?;
        if let Some(expected) = &self.measurement {
            require(expected == &checked.measurement)?;
        } else {
            self.measurement = Some(checked.measurement);
        }
        self.events.extend(checked.events);
        Ok(())
    }
    fn admitted(&mut self, value: &Value) -> Result<(), ProviderError> {
        self.add(value, session::ReaderKind::Context)?;
        require(crate::broker_probe::context_valid_for_profile(
            value,
            self.profile,
        ))
    }
    fn catalogue(&mut self, value: &Value) -> Result<(), ProviderError> {
        self.add(value, session::ReaderKind::Catalogue)
    }
    fn withdrawn(&mut self, value: &Value) -> Result<(), ProviderError> {
        self.add(value, session::ReaderKind::Context)?;
        require(matches!(
            crate::broker_server::installed_context_outcome(value, self.core, self.profile),
            Ok(crate::broker_server::InstalledContextOutcome::Withdrawn)
        ))
    }
}

pub(super) fn grade_closed_negative(
    evidence: NegativeEvidence<'_>,
) -> Result<KnownClosed, ProviderError> {
    let NegativeEvidence {
        trace,
        joined_hbc,
        marker,
        profile,
        source,
        current_pending,
    } = evidence;
    require(
        trace.profile == profile
            && &trace.source_key == source
            && source.app_id != profile.app_id(),
    )?;
    let failure = trace.failure.ok_or(ProviderError::Invalid)?;
    require(
        failure.kind == AttemptFailureKind::UnexpectedEof
            && failure.stage == trace.stage
            && failure.provider_error == ProviderError::Unavailable
            && trace.observed_unexpected_eof
            && !trace.clean_eof
            && trace.finished.is_none()
            && trace.refusal.is_none()
            && trace.identify_bound,
    )?;
    let stage = match trace.stage {
        AttemptStage::ReadArtwork => Stage::ReadAfter,
        AttemptStage::Finish => Stage::Finish,
        _ => return Err(ProviderError::Invalid),
    };
    let own = trace.own.as_ref().ok_or(ProviderError::Denied)?;
    let own_after = trace.own_after.as_ref().ok_or(ProviderError::Denied)?;
    let root = trace.root.ok_or(ProviderError::Denied)?;
    require(trace.final_root_peer == Some(root) && root.pid as u32 != own.pid)?;
    let identity = trace.identity.as_ref().ok_or(ProviderError::Denied)?;
    let identify = trace
        .identify_request
        .as_ref()
        .ok_or(ProviderError::Invalid)?;
    identify.validate().map_err(|_| ProviderError::Invalid)?;
    require(identify.nonce == marker.nonce && trace.identify_response.as_ref() == Some(identity))?;
    let expected_pending = PendingIdentity {
        schema: 1,
        profile,
        caller_pid: own.pid,
        caller_start_ticks: own.start_ticks,
        ui_sha256: marker.ui_sha256.clone(),
        nonce: marker.nonce.clone(),
    };
    require(current_pending == &expected_pending)?;
    let binding = session::IdentityBindings {
        own,
        own_after,
        root,
        identify_message: trace.identify_message.ok_or(ProviderError::Denied)?,
        client: trace.client.ok_or(ProviderError::Denied)?,
        identity,
    };
    let invocation = session::validate_invocation(joined_hbc, own, profile)?;
    let helper = invocation.helper;
    let origin = session::validate_origin(helper, &binding, marker, profile)?;
    let error = format!(
        "Installed artwork policy withdrawn at {} request {}",
        stage.name(),
        stage.request_id()
    );
    require(
        !invocation.envelope_success
            && invocation.error_text.is_string()
            && invocation.stderr == format!("Error: {error:?}\n"),
    )?;
    for key in [
        "qualified",
        "finished_sent",
        "artwork_session_completed",
        "closed_negative_acceptance_enabled",
        "commands",
        "setting_writes",
        "root_file_writes",
        "daemonized",
        "normal_loader_qualified",
        "production_startup_qualified",
    ] {
        require(helper[key] == false)?;
    }
    for key in [
        "listener_bound",
        "listener_closed",
        "response_sent",
        "resource_mode",
        "icon_reads",
        "connection_closed",
        "closed_negative_candidate",
        "artwork_reads_only",
    ] {
        require(helper[key] == true)?;
    }
    require(
        helper["outcome"] == "context_changed_closed"
            && helper["error"] == error
            && helper["failure"]
                == json!({"kind":"context_changed","stage":stage.name(),"request_id":stage.request_id()})
            && helper["finished"].is_null()
            && helper["artwork_finished"].is_null()
            && helper["handshake_only"] == false
            && helper["context_cli_gets_only"] == false
            && helper["peer_after_context_withdrawal"] == origin.peer
            && helper["context_withdrawal_message_credentials"] == origin.client
            && helper["installation_after_context_withdrawal"] == helper["installation"],
    )?;
    installed_metadata(&helper["installation"], profile)?;
    require(helper["identify_request"] == json!(identify))?;
    let enrollment: crate::broker_protocol::Enrollment =
        serde_json::from_value(helper["enrollment"].clone()).map_err(|_| ProviderError::Invalid)?;
    require(
        enrollment.nonce == marker.nonce
            && enrollment.ui_sha256 == marker.ui_sha256
            && enrollment.broker_sha256 == marker.broker_sha256
            && enrollment.context_reader_sha256 == origin.core_sha256
            && enrollment.expires_unix > 0,
    )?;
    let peer = crate::broker_protocol::PeerEvidence {
        pid: own.pid,
        uid_fields: [own.uid; 4],
        start_ticks: own.start_ticks,
        exe_sha256: own.ui_sha256.clone(),
        exe_path: profile.peer_exe_path(),
        root_path: profile.jail_root(),
    };
    crate::broker_protocol::authorize_for_profile(
        &enrollment,
        &binding.client,
        &binding.client,
        &peer,
        profile,
    )
    .map_err(|_| ProviderError::Denied)?;
    let request = trace.request.as_ref().ok_or(ProviderError::Invalid)?;
    request.validate().map_err(|_| ProviderError::Invalid)?;
    require(
        request.request_id == 1
            && request.nonce == marker.nonce
            && request.app_id == source.app_id
            && request.variant == wire::Variant::Preferred,
    )?;
    let header = trace.header.as_ref().ok_or(ProviderError::Invalid)?;
    header
        .validate_for_in_profile(request, identity, profile)
        .map_err(|_| ProviderError::Invalid)?;
    require(
        trace.header_matches == Some(true)
            && session::header_matches(header, source)
            && trace.last_received_credentials == Some(root)
            && trace.last_validated_credentials == Some(root)
            && trace.resource_packets_received == trace.resource_packets_validated,
    )?;
    let reads = helper["artwork_reads"]
        .as_array()
        .filter(|rows| rows.len() == 1)
        .ok_or(ProviderError::Invalid)?;
    let record = &reads[0];
    require(
        record["request"] == json!(request)
            && record["request_bytes"]
                == json!(serde_json::to_vec(request).map_err(|_| ProviderError::Invalid)?)
            && record["connect_credentials"] == origin.client
            && record["message_credentials"] == origin.client
            && record["peer_before"] == origin.peer
            && record["peer_after"] == origin.peer
            && record["source_key"] == json!(source)
            && record["source_key_after"] == json!(source)
            && record["header"] == json!(header)
            && record["encoded_data_released"] == true
            && record["read_identity"] == json!(header.file_identity)
            && record["readback_identity"] == record["read_identity"]
            && record["encoded_sha256"] == header.encoded_sha256
            && record["readback_sha256"] == record["encoded_sha256"]
            && record["encoded_bytes"].as_u64() == Some(u64::from(header.encoded_bytes))
            && record.get("refusal").is_none(),
    )?;
    let mut readers = Readers::new(origin.core_sha256, profile);
    readers.admitted(&helper["context_before"])?;
    readers.admitted(&record["context_before"])?;
    readers.catalogue(&record["catalogue_before"])?;
    readers.catalogue(&record["catalogue_after"])?;
    let count = match stage {
        Stage::ReadAfter => {
            let discarded = trace
                .discarded_read
                .as_ref()
                .ok_or(ProviderError::Invalid)?;
            require(
                discarded.header() == header
                    && discarded.validated_bytes() == header.encoded_bytes
                    && discarded.validated_chunks() == header.chunk_count
                    && discarded.encoded_sha256() == header.encoded_sha256
                    && trace.resource_packets_received == header.chunk_count + 1
                    && trace.complete.is_none()
                    && trace.finish_request.is_none()
                    && trace.resource_complete_validated.is_none()
                    && record.get("complete").is_none()
                    && record["completed"] == false
                    && record["error"] == error,
            )?;
            for key in [
                "artwork_finish_request",
                "artwork_finish_context",
                "artwork_finish_connect_credentials",
                "artwork_finish_message_credentials",
                "artwork_finish_peer_before",
                "artwork_finish_peer_after",
            ] {
                require(helper.get(key).is_none())?;
            }
            readers.withdrawn(&record["context_after"])?;
            13
        }
        Stage::Finish => {
            require(
                trace.discarded_read.is_none()
                    && trace.resource_complete_validated == Some(true)
                    && trace.resource_packets_received == header.chunk_count + 2
                    && record["completed"] == true
                    && record["error"].is_null(),
            )?;
            let complete = trace.complete.as_ref().ok_or(ProviderError::Invalid)?;
            complete
                .validate_for_in_profile(header, profile)
                .map_err(|_| ProviderError::Invalid)?;
            require(record["complete"] == json!(complete))?;
            let finish = trace
                .finish_request
                .as_ref()
                .ok_or(ProviderError::Invalid)?;
            finish.validate().map_err(|_| ProviderError::Invalid)?;
            require(
                finish.request_id == 2
                    && finish.nonce == marker.nonce
                    && helper["artwork_finish_request"] == json!(finish)
                    && helper["artwork_finish_connect_credentials"] == origin.client
                    && helper["artwork_finish_message_credentials"] == origin.client
                    && helper["artwork_finish_peer_before"] == origin.peer
                    && helper["artwork_finish_peer_after"] == origin.peer,
            )?;
            readers.admitted(&record["context_after"])?;
            readers.withdrawn(&helper["artwork_finish_context"])?;
            16
        }
    };
    let pids = readers
        .events
        .iter()
        .filter_map(|row| row["child_pid"].as_u64())
        .collect::<BTreeSet<_>>();
    require(
        readers.events.len() == count
            && pids.len() == count
            && !pids.contains(&u64::from(own.pid))
            && !pids.contains(&(root.pid as u64))
            && invocation.journals == &json!(readers.events),
    )?;
    Ok(KnownClosed {
        seal: trace.seal.clone(),
    })
}

#[cfg(test)]
#[path = "artwork_installed_provider_negative_tests.rs"]
pub(super) mod tests;
