//! Proof-only worker provider. One installed helper per fetch; no decoder or GL.
//! Unknown command/child completion quarantines this provider instance. A new
//! instance is not a recovery procedure for an unresolved helper. Native dispatch
//! also requires the negative-only own-state pending guard across instances.
use crate::{
    app_artwork::{EncodedFile, Source},
    artwork_worker::{Cancellation, Provider, ProviderError},
    broker_protocol::Marker,
};
use std::{
    path::PathBuf,
    time::{Duration, Instant},
};

pub struct InstalledProvider {
    marker: Marker,
    quarantined: bool,
    #[cfg(any(target_arch = "arm", test))]
    observation: Option<ProofObservation>,
    #[cfg(any(target_arch = "arm", test))]
    guard: Option<crate::artwork_session_guard::Guard>,
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
    pub fn start_proof_worker(
        marker: Marker,
        output: PathBuf,
    ) -> std::io::Result<crate::artwork_worker::Worker> {
        #[cfg(target_arch = "arm")]
        {
            crate::artwork_worker::Worker::start(move || {
                Init(Self::new(marker).and_then(|mut provider| {
                    provider.observation =
                        Some(ProofObservation::open(output).map_err(|_| ProviderError::Denied)?);
                    Ok(provider)
                }))
            })
        }
        #[cfg(not(target_arch = "arm"))]
        {
            let _ = (marker, output);
            Err(std::io::Error::new(
                std::io::ErrorKind::Unsupported,
                "Proof observation requires native ARM Proof",
            ))
        }
    }
    pub fn new(marker: Marker) -> Result<Self, ProviderError> {
        marker.validate().map_err(|_| ProviderError::Invalid)?;
        #[cfg(target_arch = "arm")]
        {
            let own = crate::broker_probe::own_identity().map_err(|_| ProviderError::Denied)?;
            if own.ui_sha256 != marker.ui_sha256 {
                return Err(ProviderError::Denied);
            }
            let guard = crate::artwork_session_guard::Guard::open(
                crate::broker_installation::Profile::Proof,
            )
            .map_err(|_| ProviderError::Denied)?;
            if guard.pending().map_err(|_| ProviderError::Denied)? {
                return Err(ProviderError::Unavailable);
            }
            Ok(Self {
                marker,
                quarantined: false,
                observation: None,
                guard: Some(guard),
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
#[derive(Clone, Copy)]
struct ProofTicket {
    index: u8,
    hold: bool,
}
#[cfg(any(target_arch = "arm", test))]
#[derive(Debug)]
enum ObservationError {
    Limit,
    Io,
}
#[cfg(any(target_arch = "arm", test))]
struct ProofObservation {
    path: PathBuf,
    directory: std::fs::File,
    stamp: (u64, u64, u32, u32, u32),
    started: u8,
    cap_written: bool,
}
#[cfg(any(target_arch = "arm", test))]
impl ProofObservation {
    fn stamp(meta: &std::fs::Metadata) -> (u64, u64, u32, u32, u32) {
        use std::os::unix::fs::MetadataExt;
        (meta.dev(), meta.ino(), meta.uid(), meta.gid(), meta.mode())
    }
    #[cfg(target_arch = "arm")]
    fn open(path: PathBuf) -> std::io::Result<Self> {
        let expected = PathBuf::from(format!(
            "/tmp/{}/shell-{}",
            crate::identity::PROOF,
            std::process::id()
        ));
        let uid = unsafe { libc::geteuid() };
        let gid = unsafe { libc::getegid() };
        let parent = std::fs::symlink_metadata(
            expected
                .parent()
                .ok_or_else(|| std::io::Error::other("Missing own Proof parent"))?,
        )?;
        if !Self::runtime_parent_valid(&parent, uid, gid) {
            return Err(std::io::Error::new(
                std::io::ErrorKind::PermissionDenied,
                "Invalid sticky Proof runtime parent",
            ));
        }
        Self::open_exact(path, &expected, uid, gid)
    }
    // The framework runtime root is shared sticky1777; only the held shell-PID leaf is private0700.
    fn runtime_parent_valid(metadata: &std::fs::Metadata, uid: u32, gid: u32) -> bool {
        use std::os::unix::fs::MetadataExt;
        metadata.is_dir()
            && !metadata.file_type().is_symlink()
            && metadata.mode() & 0o7777 == 0o1777
            && ((metadata.uid() == uid && metadata.gid() == gid)
                || (metadata.uid() == 0 && metadata.gid() == 0))
    }
    fn open_exact(
        path: PathBuf,
        expected: &std::path::Path,
        uid: u32,
        gid: u32,
    ) -> std::io::Result<Self> {
        use std::os::unix::fs::{MetadataExt, OpenOptionsExt};
        let denied = || {
            std::io::Error::new(
                std::io::ErrorKind::PermissionDenied,
                "Invalid own Proof observation directory",
            )
        };
        if path.as_os_str() != expected.as_os_str() {
            return Err(denied());
        }
        let named = std::fs::symlink_metadata(&path)?;
        if !named.is_dir()
            || named.file_type().is_symlink()
            || named.uid() != uid
            || named.gid() != gid
            || named.mode() & 0o7777 != 0o700
        {
            return Err(denied());
        }
        let directory = std::fs::OpenOptions::new()
            .read(true)
            .custom_flags(libc::O_DIRECTORY | libc::O_NOFOLLOW | libc::O_CLOEXEC)
            .open(&path)?;
        let stamp = Self::stamp(&directory.metadata()?);
        if stamp != Self::stamp(&named) {
            return Err(denied());
        }
        Ok(Self {
            path,
            directory,
            stamp,
            started: 0,
            cap_written: false,
        })
    }
    fn verify(&self) -> std::io::Result<()> {
        let named = std::fs::symlink_metadata(&self.path)?;
        if named.file_type().is_symlink()
            || Self::stamp(&named) != self.stamp
            || Self::stamp(&self.directory.metadata()?) != self.stamp
        {
            return Err(std::io::Error::new(
                std::io::ErrorKind::PermissionDenied,
                "Proof observation directory changed",
            ));
        }
        Ok(())
    }
    fn filename(index: u8, connected: bool) -> std::io::Result<String> {
        if !(1..=16).contains(&index) {
            return Err(std::io::Error::new(
                std::io::ErrorKind::InvalidInput,
                "Proof observation index exceeds16",
            ));
        }
        Ok(format!(
            "installed-artwork-{index}-{}.json",
            if connected { "connected" } else { "result" }
        ))
    }
    fn write(&self, name: &str, value: &serde_json::Value) -> std::io::Result<()> {
        use std::{
            io::Write,
            os::{
                fd::{AsRawFd, FromRawFd},
                unix::fs::MetadataExt,
            },
        };
        if name != "installed-artwork-cap.json"
            && !(1..=16).any(|index| {
                Self::filename(index, true).is_ok_and(|v| v == name)
                    || Self::filename(index, false).is_ok_and(|v| v == name)
            })
        {
            return Err(std::io::Error::new(
                std::io::ErrorKind::InvalidInput,
                "Only fixed Proof observation filenames",
            ));
        }
        self.verify()?;
        let bytes = serde_json::to_vec(value).map_err(std::io::Error::other)?;
        if bytes.len() > 2 * 1024 * 1024 {
            return Err(std::io::Error::new(
                std::io::ErrorKind::InvalidData,
                "Proof result metadata exceeds bound",
            ));
        }
        let named_path = self.path.join(name);
        let name = std::ffi::CString::new(name).map_err(|_| {
            std::io::Error::new(std::io::ErrorKind::InvalidInput, "Invalid Proof filename")
        })?;
        let fd = unsafe {
            libc::openat(
                self.directory.as_raw_fd(),
                name.as_ptr(),
                libc::O_WRONLY | libc::O_CREAT | libc::O_EXCL | libc::O_NOFOLLOW | libc::O_CLOEXEC,
                0o600,
            )
        };
        if fd < 0 {
            return Err(std::io::Error::last_os_error());
        }
        let mut file = unsafe { std::fs::File::from_raw_fd(fd) };
        let meta = file.metadata()?;
        if !meta.is_file()
            || meta.uid() != self.stamp.2
            || meta.gid() != self.stamp.3
            || meta.mode() & 0o7777 != 0o600
            || meta.nlink() != 1
        {
            return Err(std::io::Error::new(
                std::io::ErrorKind::PermissionDenied,
                "Proof receipt metadata differs",
            ));
        }
        file.write_all(&bytes)?;
        file.sync_all()?;
        let named = std::fs::symlink_metadata(named_path)?;
        let actual = file.metadata()?;
        if named.file_type().is_symlink()
            || Self::stamp(&named) != Self::stamp(&actual)
            || named.nlink() != 1
            || actual.nlink() != 1
            || actual.len() != bytes.len() as u64
        {
            return Err(std::io::Error::new(
                std::io::ErrorKind::PermissionDenied,
                "Proof receipt changed after write",
            ));
        }
        self.verify()
    }
    fn begin(&mut self) -> Result<ProofTicket, ObservationError> {
        if self.started == 16 {
            if !self.cap_written {
                self.write("installed-artwork-cap.json",&serde_json::json!({"schema":1,"maximum_started_fetches":16,"started_fetches":16,"cap_reached":true,"further_dispatch_denied":true,"root_dispatch_requested":false,"unknown_startup_claimed":false})).map_err(|_|ObservationError::Io)?;
                self.cap_written = true;
            }
            return Err(ObservationError::Limit);
        }
        self.verify().map_err(|_| ObservationError::Io)?;
        self.started += 1;
        Ok(ProofTicket {
            index: self.started,
            hold: self.started == 1,
        })
    }
    fn connected(
        &self,
        ticket: ProofTicket,
        source: &crate::app_artwork::SourceKey,
        progress: &serde_json::Value,
    ) -> std::io::Result<()> {
        self.write(&Self::filename(ticket.index,true)?,&serde_json::json!({"schema":1,"index":ticket.index,"source_key":source,"own":progress["own"],"root_peer":progress["root_peer"]}))
    }
    fn result(&self, ticket: ProofTicket, value: &serde_json::Value) -> std::io::Result<()> {
        self.write(&Self::filename(ticket.index, false)?, value)
    }
}

#[cfg(any(target_arch = "arm", test))]
fn proof_ticket(provider: &mut InstalledProvider) -> Result<Option<ProofTicket>, ProviderError> {
    match provider.observation.as_mut() {
        None => Ok(None),
        Some(observer) => match observer.begin() {
            Ok(ticket) => Ok(Some(ticket)),
            Err(ObservationError::Limit) => Err(ProviderError::Unavailable),
            Err(ObservationError::Io) => {
                provider.quarantined = true;
                Err(ProviderError::Invalid)
            }
        },
    }
}
/// Negative-only dispatch arbitration. Own identity is measured by the native
/// caller; the state file supplies no authority to the Root helper.
#[cfg(any(target_arch = "arm", test))]
fn begin_pending(
    provider: &mut InstalledProvider,
    own: &crate::broker_probe::OwnIdentity,
) -> Result<crate::artwork_session_guard::Ticket, ProviderError> {
    let attempt = (|| {
        if own.ui_sha256 != provider.marker.ui_sha256
            || !crate::broker_probe::ordinary_for_profile(
                crate::identity::PROOF,
                own.uid,
                crate::broker_installation::Profile::Proof,
            )
        {
            return Err(ProviderError::Denied);
        }
        provider
            .guard
            .as_ref()
            .ok_or(ProviderError::Denied)?
            .begin(crate::artwork_session_guard::PendingIdentity {
                schema: 1,
                profile: crate::broker_installation::Profile::Proof,
                caller_pid: own.pid,
                caller_start_ticks: own.start_ticks,
                ui_sha256: own.ui_sha256.clone(),
                nonce: provider.marker.nonce.clone(),
            })
            .map_err(|_| ProviderError::Unavailable)
    })();
    if attempt.is_err() {
        provider.quarantined = true;
    }
    attempt
}
/// Only the existing lifecycle grader may clear quarantine. Unknown completion
/// drops the ticket without unlinking, blocking a new provider/process too.
#[cfg(any(target_arch = "arm", test))]
fn resolve_pending(
    provider: &mut InstalledProvider,
    ticket: crate::artwork_session_guard::Ticket,
) -> Result<(), ProviderError> {
    if provider.quarantined {
        return Ok(());
    }
    ticket.finish_known().map_err(|_| {
        provider.quarantined = true;
        ProviderError::Invalid
    })
}
#[cfg(any(target_arch = "arm", test))]
fn guard_result(
    provider: &mut InstalledProvider,
    ticket: crate::artwork_session_guard::Ticket,
    result: Result<EncodedFile, ProviderError>,
) -> Result<EncodedFile, ProviderError> {
    resolve_pending(provider, ticket)?;
    if provider.quarantined && result.is_ok() {
        return Err(ProviderError::Invalid);
    }
    result
}
#[cfg(any(target_arch = "arm", test))]
fn observed_outcome(result: &Result<EncodedFile, ProviderError>) -> &'static str {
    match result {
        Ok(_) => "encoded_accepted",
        Err(error) => match error {
            ProviderError::Unavailable => "unavailable",
            ProviderError::Denied => "denied",
            ProviderError::Changed => "source_changed",
            ProviderError::Invalid => "invalid",
            ProviderError::Cancelled => "cancelled",
            ProviderError::Expired => "expired",
        },
    }
}
#[cfg(any(target_arch = "arm", test))]
fn write_observed_result(
    provider: &mut InstalledProvider,
    ticket: ProofTicket,
    value: &serde_json::Value,
) -> Result<(), ProviderError> {
    if provider
        .observation
        .as_ref()
        .is_none_or(|observer| observer.result(ticket, value).is_err())
    {
        provider.quarantined = true;
        Err(ProviderError::Invalid)
    } else {
        Ok(())
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
    struct Observation<'a> {
        writer: Option<&'a ProofObservation>,
        ticket: Option<ProofTicket>,
        protocol: serde_json::Value,
    }
    impl Observation<'_> {
        fn set(&mut self, key: &str, value: serde_json::Value) {
            if self.writer.is_some() {
                self.protocol[key] = value;
            }
        }
        fn connected(&self, source: &Source) -> Result<(), ProviderError> {
            if let (Some(writer), Some(ticket)) = (self.writer, self.ticket) {
                writer
                    .connected(ticket, source.key(), &self.protocol)
                    .map_err(|_| ProviderError::Invalid)?;
            }
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
        observation: &mut Observation<'_>,
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
        observation.set("stage", serde_json::json!("connecting"));
        observation.set("own", serde_json::json!(own));
        observation.set("client_credentials",serde_json::json!({"pid":std::process::id(),"uid":unsafe{libc::geteuid()},"gid":unsafe{libc::getegid()}}));
        let connection = Connection::connect_until(&marker.nonce, deadline)
            .map_err(|_| ProviderError::Unavailable)?;
        let root = connection
            .peer_credentials()
            .map_err(|_| ProviderError::Denied)?;
        observation.set(
            "root_peer",
            serde_json::json!({"pid":root.pid,"uid":root.uid,"gid":root.gid}),
        );
        if root.uid != 0 || root.pid <= 1 {
            return Err(ProviderError::Denied);
        }
        if probe::own_identity().map_err(|_| ProviderError::Denied)? != own {
            return Err(ProviderError::Denied);
        }
        observation.set("stage", serde_json::json!("connected_before_identify"));
        observation.connected(source)?;
        if observation.ticket.is_some_and(|ticket| ticket.hold) {
            let began = Instant::now();
            while began.elapsed() < Duration::from_secs(8) {
                check(deadline, cancellation, stop)?;
                std::thread::sleep(Duration::from_millis(20));
            }
            observation.set(
                "proof_binding_hold_ms",
                serde_json::json!(began.elapsed().as_millis()),
            );
        }
        check(deadline, cancellation, stop)?;
        let request = protocol::Request {
            schema: 1,
            op: protocol::Operation::Identify,
            nonce: marker.nonce.clone(),
        };
        observation.set("identify_request", serde_json::json!(request));
        observation.set("stage", serde_json::json!("identify"));
        connection
            .send_until(
                &serde_json::to_vec(&request).map_err(|_| ProviderError::Invalid)?,
                deadline,
            )
            .map_err(|_| ProviderError::Unavailable)?;
        let packet = connection
            .recv_until(protocol::RESPONSE_LIMIT, deadline)
            .map_err(|_| ProviderError::Unavailable)?;
        observation.set("root_message_credentials",serde_json::json!({"pid":packet.credentials.pid,"uid":packet.credentials.uid,"gid":packet.credentials.gid}));
        let identity =
            protocol::parse_response(&packet.bytes).map_err(|_| ProviderError::Invalid)?;
        observation.set("identify_response", serde_json::json!(identity));
        probe::bind_response(
            marker,
            &own,
            &probe::own_identity().map_err(|_| ProviderError::Denied)?,
            root,
            packet.credentials,
            &identity,
        )
        .map_err(|_| ProviderError::Denied)?;
        observation.set("identity", serde_json::json!(identity));
        observation.set("identify_bound", serde_json::json!(true));
        check(deadline, cancellation, stop)?;
        let read = wire::Request::new(1, &marker.nonce, source.app_id(), wire::Variant::Preferred)
            .map_err(|_| ProviderError::Invalid)?;
        observation.set("request", serde_json::json!(read));
        observation.set("stage", serde_json::json!("read_artwork"));
        wire::send_request(&connection, &read, deadline).map_err(|_| ProviderError::Unavailable)?;
        let mut receiver =
            wire::Receiver::new(&read, &identity, root).map_err(|_| ProviderError::Invalid)?;
        let mut header_matches = true;
        let mut resource_packets = 0u32;
        loop {
            let packet = connection
                .recv_until(wire::MESSAGE_LIMIT, deadline)
                .map_err(|_| ProviderError::Unavailable)?;
            resource_packets += 1;
            observation.set(
                "resource_packets_received",
                serde_json::json!(resource_packets),
            );
            observation.set("last_resource_message_credentials",serde_json::json!({"pid":packet.credentials.pid,"uid":packet.credentials.uid,"gid":packet.credentials.gid}));
            if let Ok(header) = wire::parse_header(&packet.bytes) {
                header_matches = session::header_matches(&header, source.key());
                observation.set("header", serde_json::json!(header));
                observation.set("header_matches", serde_json::json!(header_matches));
            }
            if let Ok(complete) = wire::parse_complete(&packet.bytes) {
                observation.set("complete", serde_json::json!(complete));
            }
            if let Ok(refusal) = wire::parse_refusal(&packet.bytes) {
                observation.set("refusal", serde_json::json!(refusal));
            }
            let ended = receiver
                .push_packet(&packet.bytes, packet.credentials)
                .map_err(|_| ProviderError::Invalid)?;
            observation.set(
                "resource_packets_validated",
                serde_json::json!(resource_packets),
            );
            if ended {
                break;
            }
        }
        let outcome = receiver.finish().map_err(|_| ProviderError::Invalid)?;
        observation.set(
            "resource_complete_validated",
            serde_json::json!(matches!(&outcome, wire::Outcome::Artwork(_))),
        );
        // Once a bounded transaction ended, Finish is cleanup even if the job
        // was cancelled. Its bytes can still never be returned after cancellation.
        if Instant::now() >= deadline {
            return Err(ProviderError::Expired);
        }
        let finish =
            wire::FinishRequest::new(2, &marker.nonce).map_err(|_| ProviderError::Invalid)?;
        observation.set("finish_request", serde_json::json!(finish));
        observation.set("stage", serde_json::json!("finish"));
        wire::send_finish_request(&connection, &finish, deadline)
            .map_err(|_| ProviderError::Unavailable)?;
        let finished = wire::receive_finished(&connection, &finish, &identity, root, deadline)
            .map_err(|_| ProviderError::Unavailable)?;
        let own_after = probe::own_identity().map_err(|_| ProviderError::Denied)?;
        probe::bind_response(
            marker,
            &own,
            &own_after,
            root,
            packet.credentials,
            &identity,
        )
        .map_err(|_| ProviderError::Denied)?;
        observation.set("own_after", serde_json::json!(own_after));
        observation.set("finished", serde_json::json!(finished));
        observation.set("clean_eof", serde_json::json!(true));
        observation.set("stage", serde_json::json!("closed"));
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
        let ticket = proof_ticket(provider)?;
        let own = probe::own_identity().map_err(|_| {
            provider.quarantined = true;
            ProviderError::Denied
        })?;
        let pending = begin_pending(provider, &own)?;
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
                resolve_pending(provider, pending)?;
                if let Some(ticket) = ticket {
                    write_observed_result(
                        provider,
                        ticket,
                        &serde_json::json!({"schema":1,"index":ticket.index,"source_key":source.key(),"protocol":{"stage":"exec_worker_not_started","clean_eof":false},"hbc":null,"root_helper":null,"outcome":"unavailable","quarantined":false,"exec_worker_started":false,"exec_worker_joined":false,"root_dispatch_requested":false,"known_no_start":true,"performance_qualified":false,"encoded_image_bytes_logged":false}),
                    )?;
                }
                return Err(ProviderError::Unavailable);
            }
        };
        let worker = session::ExecWorker::new(handle, stop.clone());
        let marker = provider.marker.clone();
        let (result, protocol) = {
            let mut observation = Observation {
                writer: provider.observation.as_ref(),
                ticket,
                protocol: serde_json::json!({"stage":"await_startup","clean_eof":false,"proof_binding_hold_ms":0}),
            };
            let result = exchange(
                &marker,
                source,
                deadline,
                cancellation,
                &stop,
                rx,
                &mut observation,
            );
            (result, observation.protocol)
        };
        if result.is_err() {
            stop.store(true, Ordering::Release);
        }
        let completion = worker.join();
        let worker_panicked = completion.is_err();
        let hbc = if ticket.is_some() {
            completion.as_ref().ok().cloned()
        } else {
            None
        };
        let cancelled = cancellation.is_cancelled();
        let expired = Instant::now() >= deadline;
        let result = session::settle(
            provider,
            source.key(),
            result,
            completion,
            cancelled,
            expired,
        );
        let result = guard_result(provider, pending, result);
        if let Some(ticket) = ticket {
            let helper = hbc
                .as_ref()
                .map(|value| value["parsed"]["helper_report"].clone())
                .unwrap_or(serde_json::Value::Null);
            let record = &helper["artwork_reads"][0];
            let value = serde_json::json!({"schema":1,"index":ticket.index,"source_key":source.key(),"protocol":protocol,"hbc":hbc,"root_helper":helper,"source_key_before":record["source_key"],"source_key_after":record["source_key_after"],"outcome":observed_outcome(&result),"encoded_release_eligible":result.is_ok(),"result_requires_successful_observation_write":true,"quarantined":provider.quarantined,"cancelled":cancelled,"expired":expired,"exec_worker_started":true,"exec_worker_joined":true,"exec_worker_panicked":worker_panicked,"proof_first_fetch_hold":ticket.hold,"performance_qualified":false,"encoded_image_bytes_logged":false});
            write_observed_result(provider, ticket, &value)?;
        }
        result
    }
}

#[cfg(test)]
#[path = "artwork_installed_provider_tests.rs"]
mod tests;
