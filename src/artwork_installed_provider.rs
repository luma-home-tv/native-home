//! Source-selected installed-profile provider. One helper per fetch; no decoder or GL.
//! Unknown command/child completion quarantines this provider instance. A new
//! instance is not a recovery procedure for an unresolved helper. Native dispatch
//! also requires the negative-only own-state pending guard across instances.
use crate::{
    app_artwork::{EncodedFile, Source},
    artwork_worker::{Cancellation, Provider, ProviderError, SessionSettlement},
    broker_installation::Profile,
    broker_protocol::Marker,
};
use std::{
    path::PathBuf,
    time::{Duration, Instant},
};

pub struct InstalledProvider {
    marker: Marker,
    profile: Profile,
    quarantined: bool,
    // Set only by successful resolution of this fetch's existing pending ticket.
    ticket_resolved: bool,
    #[cfg(any(target_arch = "arm", test))]
    observation: Option<ProofObservation>,
    #[cfg(any(target_arch = "arm", test))]
    guard: Option<crate::artwork_session_guard::Guard>,
    // In-memory metadata only; never a settlement or stale-latch recovery grant.
    #[cfg(any(target_arch = "arm", test))]
    last_attempt: Option<AttemptTrace>,
}
struct Init(Result<InstalledProvider, ProviderError>);
impl Provider for Init {
    fn initial_settlement(&self) -> SessionSettlement {
        match &self.0 {
            Ok(provider) => provider.initial_settlement(),
            Err(_) => SessionSettlement::Unknown,
        }
    }
    fn transition_settlement(&self) -> SessionSettlement {
        match &self.0 {
            Ok(provider) => provider.transition_settlement(),
            Err(_) => SessionSettlement::Unknown,
        }
    }
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
        Self::start_observation_worker(marker, output, Profile::Proof)
    }
    pub fn start_preview_worker(marker: Marker) -> std::io::Result<crate::artwork_worker::Worker> {
        crate::artwork_worker::Worker::start(move || {
            Init(Self::new_for_profile(marker, Profile::Preview))
        })
    }
    /// Explicit diagnostic path only: ordinary Preview workers have no hold,
    /// result files or dispatch cap. This method does not qualify activation.
    pub fn start_preview_observation_worker(
        marker: Marker,
        output: PathBuf,
    ) -> std::io::Result<crate::artwork_worker::Worker> {
        Self::start_observation_worker(marker, output, Profile::Preview)
    }
    fn start_observation_worker(
        marker: Marker,
        output: PathBuf,
        profile: Profile,
    ) -> std::io::Result<crate::artwork_worker::Worker> {
        #[cfg(target_arch = "arm")]
        {
            crate::artwork_worker::Worker::start(move || {
                Init(
                    Self::new_for_profile(marker, profile).and_then(|mut provider| {
                        provider.observation = Some(
                            ProofObservation::open(output, profile)
                                .map_err(|_| ProviderError::Denied)?,
                        );
                        Ok(provider)
                    }),
                )
            })
        }
        #[cfg(not(target_arch = "arm"))]
        {
            let _ = (marker, output, profile);
            Err(std::io::Error::new(
                std::io::ErrorKind::Unsupported,
                "Installed observation requires its native ARM profile",
            ))
        }
    }
    pub fn new(marker: Marker) -> Result<Self, ProviderError> {
        Self::new_for_profile(marker, Profile::Proof)
    }
    fn new_for_profile(marker: Marker, profile: Profile) -> Result<Self, ProviderError> {
        marker.validate().map_err(|_| ProviderError::Invalid)?;
        #[cfg(target_arch = "arm")]
        {
            let own = crate::broker_probe::own_identity_for_profile(profile)
                .map_err(|_| ProviderError::Denied)?;
            if own.ui_sha256 != marker.ui_sha256 {
                return Err(ProviderError::Denied);
            }
            let guard = crate::artwork_session_guard::Guard::open(profile)
                .map_err(|_| ProviderError::Denied)?;
            if guard.pending().map_err(|_| ProviderError::Denied)? {
                return Err(ProviderError::Unavailable);
            }
            Ok(Self {
                marker,
                profile,
                quarantined: false,
                ticket_resolved: false,
                observation: None,
                guard: Some(guard),
                last_attempt: None,
            })
        }
        #[cfg(not(target_arch = "arm"))]
        {
            let _ = (marker, profile);
            Err(ProviderError::Unavailable)
        }
    }
}
impl Provider for InstalledProvider {
    fn initial_settlement(&self) -> SessionSettlement {
        if self.quarantined {
            return SessionSettlement::Unknown;
        }
        #[cfg(any(target_arch = "arm", test))]
        {
            // No session has started in this new provider, and the existing
            // guard must positively show no older unresolved record.
            if self
                .guard
                .as_ref()
                .is_some_and(|guard| matches!(guard.pending(), Ok(false)))
            {
                return SessionSettlement::NoExternalSession;
            }
        }
        SessionSettlement::Unknown
    }
    fn transition_settlement(&self) -> SessionSettlement {
        if !self.ticket_resolved || self.quarantined {
            return SessionSettlement::Unknown;
        }
        #[cfg(any(target_arch = "arm", test))]
        {
            // Worker-only check after current-ticket resolution; not a UI-thread
            // pending-file probe, nor an API to clear or adopt an older record.
            if self
                .guard
                .as_ref()
                .is_some_and(|guard| matches!(guard.pending(), Ok(false)))
            {
                return SessionSettlement::KnownClosed;
            }
        }
        SessionSettlement::Unknown
    }
    fn fetch(
        &mut self,
        source: &Source,
        deadline: Instant,
        cancellation: &Cancellation,
    ) -> Result<EncodedFile, ProviderError> {
        self.ticket_resolved = false;
        #[cfg(any(target_arch = "arm", test))]
        {
            self.last_attempt = None;
        }
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
            let _ = (&self.marker, self.profile, source, deadline);
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
    fn open(path: PathBuf, profile: Profile) -> std::io::Result<Self> {
        let expected = Self::runtime_path(profile, std::process::id());
        let uid = unsafe { libc::geteuid() };
        let gid = unsafe { libc::getegid() };
        if !crate::broker_probe::ordinary_for_profile(plx_base::paths::app_id(), uid, profile) {
            return Err(std::io::Error::new(
                std::io::ErrorKind::PermissionDenied,
                "Observation requires its exact ordinary installed profile",
            ));
        }
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
    #[cfg(any(target_arch = "arm", test))]
    fn runtime_path(profile: Profile, pid: u32) -> PathBuf {
        PathBuf::from(format!("/tmp/{}/shell-{pid}", profile.app_id()))
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
                provider.profile.app_id(),
                own.uid,
                provider.profile,
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
                profile: provider.profile,
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
    })?;
    provider.ticket_resolved = true;
    Ok(())
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
        profile: Profile,
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
        if !catalogue && !crate::broker_probe::context_valid_for_profile(value, profile) {
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
        profile: Profile,
    ) -> Result<&'a Value, ProviderError> {
        if hbc["uri"] != crate::broker_startup_proof::EXEC_URI
            || hbc["command"] != crate::broker_startup_proof::fixed_command(profile)
            || hbc["attempts"] != 1
            || !hbc["error"].is_null()
            || hbc["raw_truncated"] != false
            || hbc["own_at_call"] != json!(exchange.own)
            || !crate::broker_probe::context_valid_for_profile(&hbc["context"], profile)
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
            || !crate::broker_probe::context_valid_for_profile(&cap["context_before"], profile)
            || !crate::broker_probe::context_valid_for_profile(&cap["context_after"], profile)
            || cap["response"] != crate::artwork_startup_probe::parse_response(cap_raw)
            || cap["response"]["outcome"] != "root_true"
        {
            return Err(ProviderError::Denied);
        }
        let raw = hbc["raw"].as_str().ok_or(ProviderError::Invalid)?;
        if hbc["raw_bytes"].as_u64() != Some(raw.len() as u64) {
            return Err(ProviderError::Invalid);
        }
        let parsed =
            crate::broker_startup_proof::parse_exec_resource_reply_for_profile(raw, profile)
                .map_err(|_| ProviderError::Invalid)?;
        if hbc["parsed"] != parsed || parsed["envelope_success"] != true || parsed["stderr"] != "" {
            return Err(ProviderError::Invalid);
        }
        let helper = &hbc["parsed"]["helper_report"];
        let own = &exchange.own;
        let root = exchange.root;
        crate::broker_probe::bind_response_for_profile(
            marker,
            own,
            own,
            root,
            exchange.message,
            &exchange.identity,
            profile,
        )
        .map_err(|_| ProviderError::Denied)?;
        let root_process = &helper["root_process"];
        if !exchange.clean_eof
            || exchange.finished.is_none()
            || helper["qualified"] != true
            || helper["profile"] != json!(profile)
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
            || root_process["exe_path"] != crate::broker_startup_proof::fixed_command(profile)
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
            || pins["profile"] != json!(profile)
            || pins["rendezvous_nonce"] != marker.nonce
            || pins["ui_sha256"] != marker.ui_sha256
            || pins["broker_sha256"] != marker.broker_sha256
            || installation["profile"] != json!(profile)
            || installation["app_id"] != profile.app_id()
            || installation["root_process"] != *root_process
            || installation["files"]["helper"]["path"]
                != crate::broker_startup_proof::fixed_command(profile)
            || installation["files"]["helper"]["sha256"] != marker.broker_sha256
            || installation["files"]["helper"]["trusted_for_execution"] != true
            || installation["files"]["configuration"]["path"] != profile.config_path()
            || installation["files"]["configuration"]["trusted_for_execution"] != true
            || installation["files"]["ui"]["sha256"] != marker.ui_sha256
            || installation["files"]["ui"]["path"] != profile.ui_path()
            || installation["files"]["ui"]["trusted_for_execution"] != false
        {
            return Err(ProviderError::Invalid);
        }
        let peer = json!({"pid":own.pid,"uid_fields":[own.uid,own.uid,own.uid,own.uid],"start_ticks":own.start_ticks,"exe_path":profile.peer_exe_path(),"root_path":profile.jail_root(),"exe_sha256":own.ui_sha256});
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
        known_reader(&helper["context_before"], false, core, &mut events, profile)?;
        known_reader(&record["context_before"], false, core, &mut events, profile)?;
        known_reader(
            &record["catalogue_before"],
            true,
            core,
            &mut events,
            profile,
        )?;
        if !record["catalogue_after"].is_null() {
            known_reader(&record["catalogue_after"], true, core, &mut events, profile)?;
        }
        known_reader(&record["context_after"], false, core, &mut events, profile)?;
        known_reader(
            &helper["artwork_finish_context"],
            false,
            core,
            &mut events,
            profile,
        )?;
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
                    .validate_for_in_profile(&exchange.request, &exchange.identity, profile)
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
            .validate_for_in_profile(&finish, &exchange.identity, profile)
            .map_err(|_| ProviderError::Invalid)?;
        Ok(record)
    }
    pub(super) fn commit(
        source: &SourceKey,
        exchange: Exchange,
        record: &Value,
        profile: Profile,
    ) -> Result<EncodedFile, ProviderError> {
        match exchange.outcome {
            wire::Outcome::Refused(refusal) => {
                refusal
                    .validate_for_in_profile(&exchange.request, &exchange.identity, profile)
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
                    .validate_for_in_profile(&exchange.request, &exchange.identity, profile)
                    .map_err(|_| ProviderError::Invalid)?;
                artwork
                    .complete
                    .validate_for_in_profile(&artwork.header, profile)
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
            && completion["command"] == crate::broker_startup_proof::fixed_command(provider.profile)
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
        let record = match lifecycle(&completion, &exchange, &provider.marker, provider.profile) {
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
        commit(source, exchange, record, provider.profile)
    }
}

// A trace is metadata about this attempt, never proof of settlement. In
// particular, unexpected EOF remains the same failed operation as before.
#[cfg(any(target_arch = "arm", test))]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum AttemptStage {
    AwaitStartup,
    ExecWorkerNotStarted,
    Connecting,
    ConnectedBeforeIdentify,
    Identify,
    ReadArtwork,
    Finish,
    Closed,
}
#[cfg(any(target_arch = "arm", test))]
impl AttemptStage {
    fn name(self) -> &'static str {
        match self {
            Self::AwaitStartup => "await_startup",
            Self::ExecWorkerNotStarted => "exec_worker_not_started",
            Self::Connecting => "connecting",
            Self::ConnectedBeforeIdentify => "connected_before_identify",
            Self::Identify => "identify",
            Self::ReadArtwork => "read_artwork",
            Self::Finish => "finish",
            Self::Closed => "closed",
        }
    }
}
#[cfg(any(target_arch = "arm", test))]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum AttemptFailureKind {
    ProviderError,
    ReceiveError,
    UnexpectedEof,
}
#[cfg(any(target_arch = "arm", test))]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
struct AttemptFailure {
    stage: AttemptStage,
    kind: AttemptFailureKind,
    provider_error: ProviderError,
}
#[cfg(any(target_arch = "arm", test))]
#[derive(Clone, Debug)]
struct AttemptTrace {
    profile: Profile,
    source_key: crate::app_artwork::SourceKey,
    stage: AttemptStage,
    failure: Option<AttemptFailure>,
    own: Option<crate::broker_probe::OwnIdentity>,
    own_after: Option<crate::broker_probe::OwnIdentity>,
    client: Option<crate::broker_transport::Credentials>,
    root: Option<crate::broker_transport::Credentials>,
    identify_message: Option<crate::broker_transport::Credentials>,
    identify_request: Option<crate::broker_protocol::Request>,
    identify_response: Option<crate::broker_protocol::Response>,
    identity: Option<crate::broker_protocol::Response>,
    identify_bound: bool,
    request: Option<crate::artwork_transfer::Request>,
    finish_request: Option<crate::artwork_transfer::FinishRequest>,
    header: Option<crate::artwork_transfer::Header>,
    complete: Option<crate::artwork_transfer::Complete>,
    refusal: Option<crate::artwork_transfer::Refusal>,
    finished: Option<crate::artwork_transfer::Finished>,
    header_matches: Option<bool>,
    resource_complete_validated: Option<bool>,
    resource_packets_received: u32,
    resource_packets_validated: u32,
    last_received_credentials: Option<crate::broker_transport::Credentials>,
    last_validated_credentials: Option<crate::broker_transport::Credentials>,
    proof_binding_hold_ms: u64,
    clean_eof: bool,
    observed_unexpected_eof: bool,
}
#[cfg(any(target_arch = "arm", test))]
impl AttemptTrace {
    fn new(profile: Profile, source: &crate::app_artwork::SourceKey) -> Self {
        Self {
            profile,
            source_key: source.clone(),
            stage: AttemptStage::AwaitStartup,
            failure: None,
            own: None,
            own_after: None,
            client: None,
            root: None,
            identify_message: None,
            identify_request: None,
            identify_response: None,
            identity: None,
            identify_bound: false,
            request: None,
            finish_request: None,
            header: None,
            complete: None,
            refusal: None,
            finished: None,
            header_matches: None,
            resource_complete_validated: None,
            resource_packets_received: 0,
            resource_packets_validated: 0,
            last_received_credentials: None,
            last_validated_credentials: None,
            proof_binding_hold_ms: 0,
            clean_eof: false,
            observed_unexpected_eof: false,
        }
    }
    fn fail(&mut self, kind: AttemptFailureKind, error: ProviderError) {
        // First failure is terminal metadata. A later EOF cannot erase it.
        if self.failure.is_none() {
            self.observed_unexpected_eof = kind == AttemptFailureKind::UnexpectedEof;
            self.failure = Some(AttemptFailure {
                stage: self.stage,
                kind,
                provider_error: error,
            });
        }
    }
    fn push_resource(
        &mut self,
        receiver: &mut crate::artwork_transfer::Receiver,
        packet: &crate::broker_transport::Packet,
    ) -> Result<bool, ProviderError> {
        if let Some(failure) = self.failure {
            return Err(failure.provider_error);
        }
        self.resource_packets_received += 1;
        self.last_received_credentials = Some(packet.credentials);
        let ended = match receiver.push_packet(&packet.bytes, packet.credentials) {
            Ok(ended) => ended,
            Err(_) => {
                self.fail(AttemptFailureKind::ProviderError, ProviderError::Invalid);
                return Err(ProviderError::Invalid);
            }
        };
        // Parsed metadata is authoritative progress ONLY after Receiver accepted
        // this exact packet and its credentials. No raw image data is retained.
        self.resource_packets_validated += 1;
        self.last_validated_credentials = Some(packet.credentials);
        if let Ok(header) =
            crate::artwork_transfer::parse_header_for_profile(&packet.bytes, self.profile)
        {
            self.header_matches = Some(session::header_matches(&header, &self.source_key));
            self.header = Some(header);
        }
        if let Ok(complete) =
            crate::artwork_transfer::parse_complete_for_profile(&packet.bytes, self.profile)
        {
            self.complete = Some(complete);
        }
        if let Ok(refusal) =
            crate::artwork_transfer::parse_refusal_for_profile(&packet.bytes, self.profile)
        {
            self.refusal = Some(refusal);
        }
        Ok(ended)
    }
    fn report(&self) -> serde_json::Value {
        use serde_json::json;
        let failure = self.failure.map(|f| json!({
            "stage":f.stage.name(),
            "kind":match f.kind { AttemptFailureKind::ProviderError=>"provider_error", AttemptFailureKind::ReceiveError=>"receive_error", AttemptFailureKind::UnexpectedEof=>"unexpected_eof" },
            "provider_error":observed_outcome(&Err(f.provider_error)),
        }));
        let mut value = json!({"stage":self.stage.name(),"clean_eof":self.clean_eof,
            "proof_binding_hold_ms":self.proof_binding_hold_ms,
            "attempt_source_key":self.source_key,"attempt_failure":failure,
            "observed_unexpected_eof":self.observed_unexpected_eof,
            "failure_evidence_retained":true,"negative_acceptance_enabled":false});
        // Preserve successful optional-JSON field names and values. Absence of
        // an unobserved field stays absence, not an invented typed observation.
        macro_rules! optional {
            ($key:literal,$field:expr) => {
                if let Some(field) = &$field {
                    value[$key] = json!(field);
                }
            };
        }
        macro_rules! optional_credentials {
            ($key:literal, $field:expr) => {
                if let Some(field) = &$field {
                    value[$key] = json!({"pid":field.pid,"uid":field.uid,"gid":field.gid});
                }
            };
        }
        optional!("own", self.own);
        optional!("own_after", self.own_after);
        optional_credentials!("client_credentials", self.client);
        optional_credentials!("root_peer", self.root);
        optional_credentials!("root_message_credentials", self.identify_message);
        optional!("identify_request", self.identify_request);
        optional!("identify_response", self.identify_response);
        optional!("identity", self.identity);
        if self.identify_bound {
            value["identify_bound"] = json!(true);
        }
        optional!("request", self.request);
        optional!("finish_request", self.finish_request);
        optional!("header", self.header);
        optional!("complete", self.complete);
        optional!("refusal", self.refusal);
        optional!("finished", self.finished);
        optional!("header_matches", self.header_matches);
        optional!(
            "resource_complete_validated",
            self.resource_complete_validated
        );
        if self.resource_packets_received != 0 {
            value["resource_packets_received"] = json!(self.resource_packets_received);
            if self.resource_packets_validated != 0 {
                value["resource_packets_validated"] = json!(self.resource_packets_validated);
            }
            optional_credentials!(
                "last_received_resource_message_credentials",
                self.last_received_credentials
            );
            optional_credentials!(
                "last_resource_message_credentials",
                self.last_validated_credentials
            );
        }
        value
    }
}
#[cfg(any(target_arch = "arm", test))]
fn receive_attempt_packet(
    trace: &mut AttemptTrace,
    receive: impl FnOnce() -> std::io::Result<crate::broker_transport::Incoming>,
) -> Result<crate::broker_transport::Packet, ProviderError> {
    if let Some(failure) = trace.failure {
        return Err(failure.provider_error);
    }
    match receive() {
        Ok(crate::broker_transport::Incoming::Packet(packet)) => Ok(packet),
        Ok(crate::broker_transport::Incoming::Eof) => {
            trace.fail(
                AttemptFailureKind::UnexpectedEof,
                ProviderError::Unavailable,
            );
            Err(ProviderError::Unavailable)
        }
        Err(_) => {
            trace.fail(AttemptFailureKind::ReceiveError, ProviderError::Unavailable);
            Err(ProviderError::Unavailable)
        }
    }
}
#[cfg(any(target_arch = "arm", test))]
struct Observation<'a> {
    writer: Option<&'a ProofObservation>,
    ticket: Option<ProofTicket>,
    trace: AttemptTrace,
}
#[cfg(any(target_arch = "arm", test))]
impl Observation<'_> {
    fn connected(&self, source: &Source) -> Result<(), ProviderError> {
        if let (Some(writer), Some(ticket)) = (self.writer, self.ticket) {
            writer
                .connected(ticket, source.key(), &self.trace.report())
                .map_err(|_| ProviderError::Invalid)?;
        }
        Ok(())
    }
}
#[cfg(any(target_arch = "arm", test))]
fn retain_attempt(provider: &mut InstalledProvider, trace: AttemptTrace) {
    // Metadata only. No-writer workers retain it in this provider instance;
    // final diagnostic projection occurs only after the existing join/settle.
    provider.last_attempt = Some(trace);
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
        profile: Profile,
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
        let own = probe::own_identity_for_profile(profile).map_err(|_| ProviderError::Denied)?;
        observation.trace.stage = AttemptStage::Connecting;
        observation.trace.own = Some(own.clone());
        observation.trace.client = Some(Credentials {
            pid: std::process::id() as i32,
            uid: unsafe { libc::geteuid() },
            gid: unsafe { libc::getegid() },
        });
        let connection = Connection::connect_until(&marker.nonce, deadline)
            .map_err(|_| ProviderError::Unavailable)?;
        let root = connection
            .peer_credentials()
            .map_err(|_| ProviderError::Denied)?;
        observation.trace.root = Some(root);
        if root.uid != 0 || root.pid <= 1 {
            return Err(ProviderError::Denied);
        }
        if probe::own_identity_for_profile(profile).map_err(|_| ProviderError::Denied)? != own {
            return Err(ProviderError::Denied);
        }
        observation.trace.stage = AttemptStage::ConnectedBeforeIdentify;
        observation.connected(source)?;
        if observation.ticket.is_some_and(|ticket| ticket.hold) {
            let began = Instant::now();
            while began.elapsed() < Duration::from_secs(8) {
                check(deadline, cancellation, stop)?;
                std::thread::sleep(Duration::from_millis(20));
            }
            observation.trace.proof_binding_hold_ms = began.elapsed().as_millis() as u64;
        }
        check(deadline, cancellation, stop)?;
        let request = protocol::Request {
            schema: 1,
            op: protocol::Operation::Identify,
            nonce: marker.nonce.clone(),
        };
        observation.trace.identify_request = Some(request.clone());
        observation.trace.stage = AttemptStage::Identify;
        connection
            .send_until(
                &serde_json::to_vec(&request).map_err(|_| ProviderError::Invalid)?,
                deadline,
            )
            .map_err(|_| ProviderError::Unavailable)?;
        let packet = receive_attempt_packet(&mut observation.trace, || {
            connection.recv_or_eof_until(protocol::RESPONSE_LIMIT, deadline)
        })?;
        observation.trace.identify_message = Some(packet.credentials);
        let identity = protocol::parse_response_for_profile(&packet.bytes, profile)
            .map_err(|_| ProviderError::Invalid)?;
        observation.trace.identify_response = Some(identity.clone());
        probe::bind_response_for_profile(
            marker,
            &own,
            &probe::own_identity_for_profile(profile).map_err(|_| ProviderError::Denied)?,
            root,
            packet.credentials,
            &identity,
            profile,
        )
        .map_err(|_| ProviderError::Denied)?;
        observation.trace.identity = Some(identity.clone());
        observation.trace.identify_bound = true;
        check(deadline, cancellation, stop)?;
        let read = wire::Request::new(1, &marker.nonce, source.app_id(), wire::Variant::Preferred)
            .map_err(|_| ProviderError::Invalid)?;
        observation.trace.request = Some(read.clone());
        observation.trace.stage = AttemptStage::ReadArtwork;
        wire::send_request(&connection, &read, deadline).map_err(|_| ProviderError::Unavailable)?;
        let mut receiver = wire::Receiver::new_for_profile(&read, &identity, root, profile)
            .map_err(|_| ProviderError::Invalid)?;
        loop {
            let packet = receive_attempt_packet(&mut observation.trace, || {
                connection.recv_or_eof_until(wire::MESSAGE_LIMIT, deadline)
            })?;
            if observation.trace.push_resource(&mut receiver, &packet)? {
                break;
            }
        }
        let header_matches = observation.trace.header_matches.unwrap_or(true);
        let outcome = receiver.finish().map_err(|_| ProviderError::Invalid)?;
        observation.trace.resource_complete_validated =
            Some(matches!(&outcome, wire::Outcome::Artwork(_)));
        // Once a bounded transaction ended, Finish is cleanup even if the job
        // was cancelled. Its bytes can still never be returned after cancellation.
        if Instant::now() >= deadline {
            return Err(ProviderError::Expired);
        }
        let finish =
            wire::FinishRequest::new(2, &marker.nonce).map_err(|_| ProviderError::Invalid)?;
        observation.trace.finish_request = Some(finish.clone());
        observation.trace.stage = AttemptStage::Finish;
        wire::send_finish_request(&connection, &finish, deadline)
            .map_err(|_| ProviderError::Unavailable)?;
        let finished = wire::receive_finished_for_profile(
            &connection,
            &finish,
            &identity,
            root,
            deadline,
            profile,
        )
        .map_err(|_| ProviderError::Unavailable)?;
        let own_after =
            probe::own_identity_for_profile(profile).map_err(|_| ProviderError::Denied)?;
        probe::bind_response_for_profile(
            marker,
            &own,
            &own_after,
            root,
            packet.credentials,
            &identity,
            profile,
        )
        .map_err(|_| ProviderError::Denied)?;
        observation.trace.own_after = Some(own_after);
        observation.trace.finished = Some(finished.clone());
        observation.trace.clean_eof = true;
        observation.trace.stage = AttemptStage::Closed;
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
        let profile = provider.profile;
        let ticket = proof_ticket(provider)?;
        let own = probe::own_identity_for_profile(profile).map_err(|_| {
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
                crate::broker_startup_proof::native::invoke_resource_for_profile(
                    &marker,
                    &invoke_stop,
                    tx,
                    deadline,
                    profile,
                )
            }) {
            Ok(handle) => handle,
            Err(_) => {
                let mut trace = AttemptTrace::new(profile, source.key());
                trace.stage = AttemptStage::ExecWorkerNotStarted;
                trace.fail(
                    AttemptFailureKind::ProviderError,
                    ProviderError::Unavailable,
                );
                retain_attempt(provider, trace);
                provider.quarantined = false;
                resolve_pending(provider, pending)?;
                if let Some(ticket) = ticket {
                    let protocol = provider
                        .last_attempt
                        .as_ref()
                        .map(AttemptTrace::report)
                        .unwrap_or_default();
                    write_observed_result(
                        provider,
                        ticket,
                        &serde_json::json!({"schema":1,"index":ticket.index,"source_key":source.key(),"protocol":protocol,"hbc":null,"root_helper":null,"outcome":"unavailable","quarantined":false,"exec_worker_started":false,"exec_worker_joined":false,"root_dispatch_requested":false,"known_no_start":true,"performance_qualified":false,"encoded_image_bytes_logged":false}),
                    )?;
                }
                return Err(ProviderError::Unavailable);
            }
        };
        let worker = session::ExecWorker::new(handle, stop.clone());
        let marker = provider.marker.clone();
        let (result, trace) = {
            let mut observation = Observation {
                writer: provider.observation.as_ref(),
                ticket,
                trace: AttemptTrace::new(profile, source.key()),
            };
            let result = exchange(
                &marker,
                profile,
                source,
                deadline,
                cancellation,
                &stop,
                rx,
                &mut observation,
            );
            if let Err(error) = &result {
                observation
                    .trace
                    .fail(AttemptFailureKind::ProviderError, *error);
            }
            (result, observation.trace)
        };
        retain_attempt(provider, trace);
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
            let protocol = provider
                .last_attempt
                .as_ref()
                .map(AttemptTrace::report)
                .unwrap_or_default();
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

#[cfg(test)]
mod settlement_tests {
    use super::*;
    use crate::artwork_session_guard::{Guard, PendingIdentity};
    use std::os::unix::fs::DirBuilderExt;

    struct Directory(std::path::PathBuf);
    impl Directory {
        fn new() -> Self {
            static NEXT: std::sync::atomic::AtomicUsize = std::sync::atomic::AtomicUsize::new(0);
            let path = std::env::temp_dir().join(format!(
                "luma-settlement-{}-{}",
                std::process::id(),
                NEXT.fetch_add(1, std::sync::atomic::Ordering::Relaxed)
            ));
            std::fs::DirBuilder::new()
                .mode(0o700)
                .create(&path)
                .unwrap();
            Self(path)
        }
    }
    impl Drop for Directory {
        fn drop(&mut self) {
            let _ = std::fs::remove_dir_all(&self.0);
        }
    }
    fn provider(directory: &Directory) -> InstalledProvider {
        InstalledProvider {
            marker: Marker {
                schema: 1,
                nonce: "a".repeat(32),
                ui_sha256: "b".repeat(64),
                broker_sha256: "c".repeat(64),
            },
            profile: Profile::Proof,
            quarantined: false,
            ticket_resolved: false,
            observation: None,
            guard: Some(Guard::open_for_test(&directory.0, Profile::Proof).unwrap()),
            last_attempt: None,
        }
    }
    fn identity(p: &InstalledProvider) -> PendingIdentity {
        PendingIdentity {
            schema: 1,
            profile: p.profile,
            caller_pid: std::process::id(),
            caller_start_ticks: 123,
            ui_sha256: p.marker.ui_sha256.clone(),
            nonce: p.marker.nonce.clone(),
        }
    }
    #[test]
    fn empty_guard_is_not_current_ticket_closure_and_init_error_stays_unknown() {
        let directory = Directory::new();
        let p = provider(&directory);
        assert_eq!(p.transition_settlement(), SessionSettlement::Unknown);
        assert_eq!(
            Init(Err(ProviderError::Unavailable)).transition_settlement(),
            SessionSettlement::Unknown
        );
    }
    #[test]
    fn resolved_current_ticket_allows_failed_image_but_new_pending_or_quarantine_blocks() {
        let directory = Directory::new();
        let mut p = provider(&directory);
        let ticket = p.guard.as_ref().unwrap().begin(identity(&p)).unwrap();
        assert!(guard_result(&mut p, ticket, Err(ProviderError::Invalid)).is_err());
        assert_eq!(p.transition_settlement(), SessionSettlement::KnownClosed);
        let _retained = p.guard.as_ref().unwrap().begin(identity(&p)).unwrap();
        assert_eq!(p.transition_settlement(), SessionSettlement::Unknown);
        p.quarantined = true;
        assert_eq!(p.transition_settlement(), SessionSettlement::Unknown);
    }
    #[test]
    fn replaced_guard_directory_is_unknown_even_after_prior_ticket_resolution() {
        let directory = Directory::new();
        let mut p = provider(&directory);
        let ticket = p.guard.as_ref().unwrap().begin(identity(&p)).unwrap();
        resolve_pending(&mut p, ticket).unwrap();
        std::fs::remove_dir(&directory.0).unwrap();
        std::fs::DirBuilder::new()
            .mode(0o700)
            .create(&directory.0)
            .unwrap();
        assert_eq!(p.transition_settlement(), SessionSettlement::Unknown);
    }
    #[test]
    fn initialization_observes_old_pending_instead_of_declaring_ready() {
        let directory = Directory::new();
        let p = provider(&directory);
        assert_eq!(p.initial_settlement(), SessionSettlement::NoExternalSession);
        let retained = p.guard.as_ref().unwrap().begin(identity(&p)).unwrap();
        drop(retained); // Deliberately retains the unresolved old record.
        let fresh = provider(&directory);
        assert_eq!(fresh.initial_settlement(), SessionSettlement::Unknown);
        assert_eq!(
            Init(Err(ProviderError::Unavailable)).initial_settlement(),
            SessionSettlement::Unknown
        );
        assert!(fresh.guard.as_ref().unwrap().pending().unwrap());
    }
}
