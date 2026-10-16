//! Exclusive one-job Proof observation. No Target launch, keyboard or Root policy
//! lives here; actual Target foreground and remote absence remain host evidence.
use crate::{app_artwork::SourceMap, broker_protocol, runtime::Window, shell::Shell};
use serde::{Deserialize, Serialize};
#[cfg(any(target_arch = "arm", test))]
use serde_json::json;
use serde_json::Value;
use std::{error::Error, path::Path};

pub const MARKER_NAME: &str = "artwork-closed-negative-proof.json";
pub const RECIPE_VERSION: &str = "0.53.0";
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum Case {
    ReadAfter,
    Finish,
}
impl<'de> Deserialize<'de> for Case {
    fn deserialize<D: serde::Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        match String::deserialize(deserializer)?.as_str() {
            "read_after" => Ok(Self::ReadAfter),
            "finish" => Ok(Self::Finish),
            _ => Err(serde::de::Error::custom("Only read_after or finish case")),
        }
    }
}
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Marker {
    pub schema: u32,
    pub nonce: String,
    pub ui_sha256: String,
    pub broker_sha256: String,
    pub case: Case,
}
impl Marker {
    pub fn broker(&self) -> broker_protocol::Marker {
        broker_protocol::Marker {
            schema: self.schema,
            nonce: self.nonce.clone(),
            ui_sha256: self.ui_sha256.clone(),
            broker_sha256: self.broker_sha256.clone(),
        }
    }
    fn parse(raw: &[u8]) -> Result<Self, Box<dyn Error>> {
        if raw.is_empty() || raw.len() > 1024 {
            return Err("Closed-negative marker bound".into());
        }
        let marker: Self = serde_json::from_slice(raw)?;
        marker.broker().validate()?;
        Ok(marker)
    }
}
/// Fixed marker only; no recipe path or role comes from its contents.
pub fn read_marker(assets: &Path) -> Result<Option<Marker>, Box<dyn Error>> {
    use std::{
        io::Read,
        os::unix::fs::{MetadataExt, OpenOptionsExt},
    };
    let path = assets.join(MARKER_NAME);
    let named = match std::fs::symlink_metadata(&path) {
        Ok(value) => value,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Ok(None),
        Err(error) => return Err(error.into()),
    };
    let stable = |m: &std::fs::Metadata| {
        (
            m.dev(),
            m.ino(),
            m.mode(),
            m.uid(),
            m.gid(),
            m.nlink(),
            m.len(),
            m.mtime(),
            m.mtime_nsec(),
            m.ctime(),
            m.ctime_nsec(),
        )
    };
    if !named.is_file()
        || named.file_type().is_symlink()
        || named.uid() != 0
        || named.gid() != 0
        || named.mode() & 0o7777 != 0o644
        || named.nlink() != 1
        || named.len() > 1024
    {
        return Err("Closed-negative marker ownership/type/mode".into());
    }
    let mut file = std::fs::OpenOptions::new()
        .read(true)
        .custom_flags(libc::O_NOFOLLOW | libc::O_CLOEXEC)
        .open(&path)?;
    let before = file.metadata()?;
    if stable(&before) != stable(&named) {
        return Err("Closed-negative marker replaced".into());
    }
    let mut raw = Vec::new();
    (&mut file).take(1025).read_to_end(&mut raw)?;
    if stable(&file.metadata()?) != stable(&before)
        || stable(&std::fs::symlink_metadata(path)?) != stable(&before)
        || raw.len() as u64 != before.len()
    {
        return Err("Closed-negative marker changed".into());
    }
    Ok(Some(Marker::parse(&raw)?))
}
#[cfg(any(target_arch = "arm", test))]
#[derive(Clone, Copy, Debug)]
pub(crate) enum ProgressFile {
    PreDispatch,
    ReadBodyReady,
    FinishReady,
    Final,
    ShellReceipt,
}
#[cfg(any(target_arch = "arm", test))]
impl ProgressFile {
    pub(crate) fn name(self) -> &'static str {
        match self {
            Self::PreDispatch => "artwork-closed-negative-pre-dispatch.json",
            Self::ReadBodyReady => "artwork-closed-negative-read-body-ready.json",
            Self::FinishReady => "artwork-closed-negative-finish-ready.json",
            Self::Final => "artwork-closed-negative-proof.json",
            Self::ShellReceipt => "shell-receipt.json",
        }
    }
    pub(crate) fn allows(name: &str) -> bool {
        [
            Self::PreDispatch,
            Self::ReadBodyReady,
            Self::FinishReady,
            Self::Final,
            Self::ShellReceipt,
        ]
        .into_iter()
        .any(|file| file.name() == name)
    }
}
#[cfg(any(target_arch = "arm", test))]
#[derive(Default)]
struct RenderGate {
    stopped: bool,
    departure: bool,
    interrupted: bool,
    #[cfg(target_arch = "arm")]
    draws: u64,
    #[cfg(target_arch = "arm")]
    swaps: u64,
}
#[cfg(any(target_arch = "arm", test))]
impl RenderGate {
    fn observe(&mut self, kind: u32) {
        if matches!(kind, 7 | 8) {
            self.departure = true;
            self.stopped = true;
        } else if matches!(kind, 1..=6 | 10 | 11) {
            self.interrupted = true;
            self.stopped = true;
        }
    }
    #[cfg(target_arch = "arm")]
    fn poll(&mut self) {
        while let Some(event) = crate::runtime::poll() {
            self.observe(event.kind);
        }
    }
    #[cfg(target_arch = "arm")]
    fn frame(
        &mut self,
        shell: &mut Shell,
        window: &Window,
        now: &mut u32,
    ) -> Result<(), Box<dyn Error>> {
        self.poll();
        if self.stopped {
            return Ok(());
        }
        *now = now.wrapping_add(17);
        shell.tick(*now, 1.0 / 60.0);
        self.poll();
        if self.stopped {
            return Ok(());
        }
        shell.draw();
        self.draws += 1;
        self.poll();
        if self.stopped {
            return Ok(());
        }
        window.swap();
        self.swaps += 1;
        Ok(())
    }
}

#[cfg(target_arch = "arm")]
pub fn run(
    shell: &mut Shell,
    window: &Window,
    output: &Path,
    marker: &Marker,
    now: &mut u32,
    mut poll_catalogue: impl FnMut(&mut Shell) -> Option<SourceMap>,
) -> Result<Value, Box<dyn Error>> {
    use crate::{
        artwork_cache::{ArtworkCache, Config, FetchResult},
        artwork_installed_provider::{
            ClosedNegativeOutcome, ClosedNegativeWriter, InstalledProvider,
        },
        artwork_worker::{SessionSettlement, Status},
    };
    use std::time::{Duration, Instant};
    let own = crate::broker_probe::own_identity()?;
    if own.ui_sha256 != marker.ui_sha256 || !shell.live_artwork_receipt()["runtime"].is_null() {
        return Err("Closed-negative recipe requires measured Proof and no Runtime".into());
    }
    let writer = ClosedNegativeWriter::open(output.to_path_buf(), marker.case)?;
    let preferences = shell.preferences().clone();
    let began = Instant::now();
    let deadline = began + Duration::from_secs(60);
    let mut gate = RenderGate::default();
    let mut error = None::<String>;
    let mut worker = None;
    let mut observed = None;
    let mut completed = None;
    let mut submitted = 0u64;
    let mut selected = None;
    let mut requests = 0usize;
    let mut hold_ms = 0u64;
    let mut catalogue_reads = 0u64;
    let mut state_store = None;
    let preparation = (|| -> Result<(), Box<dyn Error>> {
        let state_path = crate::identity::preferences_dir(crate::identity::PROOF)
            .ok_or("Fixed Proof state unavailable")?;
        let store = crate::preferences::Store::open(&state_path)?;
        if store.load()?.is_some() {
            return Err("Closed-negative recipe refuses existing preferences".into());
        }
        state_store = Some(store); // Hold ordinary private directory; never save preferences.
        let mut catalogue = None;
        while began.elapsed() < Duration::from_secs(15) {
            if let Some(mut sources) = poll_catalogue(shell) {
                catalogue_reads += 1;
                sources.retain(|id, _| id == crate::identity::TARGET);
                if sources
                    .get(crate::identity::TARGET)
                    .and_then(|set| set.preferred())
                    .is_some()
                {
                    catalogue = Some(sources);
                    break;
                }
            }
            gate.frame(shell, window, now)?;
            requests += shell.take_requests().len();
            if gate.stopped || requests != 0 || shell.preferences() != &preferences {
                return Err("Departure/input/action before catalogue".into());
            }
            std::thread::sleep(Duration::from_millis(17));
        }
        let sources = catalogue.ok_or("Fixed Target source unavailable")?;
        let source = sources[crate::identity::TARGET]
            .preferred()
            .ok_or("Fixed Target preferred source unavailable")?
            .clone();
        selected = Some(source.key().clone());
        writer.write(ProgressFile::PreDispatch,&json!({"schema":1,"marker":marker,"own":own,"target_id":crate::identity::TARGET,"source_key":source.key(),"hold_target_ms":8000,"root_dispatch_requested":false,"progress_is_not_closure_authority":true}))?;
        let held = Instant::now();
        while held.elapsed() < Duration::from_secs(8) {
            if let Some(sources) = poll_catalogue(shell) {
                catalogue_reads += 1;
                if sources
                    .get(crate::identity::TARGET)
                    .and_then(|set| set.preferred())
                    .map(|source| source.key())
                    != Some(source.key())
                {
                    return Err("Fixed Target source changed before dispatch".into());
                }
            }
            gate.frame(shell, window, now)?;
            requests += shell.take_requests().len();
            if gate.stopped || requests != 0 || shell.preferences() != &preferences {
                return Err("Departure/input/action before dispatch".into());
            }
            std::thread::sleep(Duration::from_millis(17));
        }
        hold_ms = held.elapsed().as_millis() as u64;
        if crate::broker_probe::own_identity()? != own {
            return Err("Proof identity changed before dispatch".into());
        }
        let (started, channel) = InstalledProvider::start_closed_negative_proof_worker(
            marker.broker(),
            marker.case,
            output.to_path_buf(),
        )?;
        worker = Some(started);
        observed = Some(channel);
        loop {
            let initial = worker.as_mut().unwrap().initial_settlement();
            if let Some(initial) = initial {
                if initial != SessionSettlement::NoExternalSession {
                    return Err("Provider initialization unresolved".into());
                }
                break;
            }
            gate.frame(shell, window, now)?;
            if gate.stopped || Instant::now() >= deadline {
                return Err("Recipe stopped before worker readiness".into());
            }
            std::thread::sleep(Duration::from_millis(17));
        }
        let mut cache =
            ArtworkCache::new(Config::default()).map_err(|_| "Recipe cache configuration")?;
        cache
            .update_catalogue(&sources)
            .map_err(|_| "Recipe source admission")?;
        cache
            .set_interest(&[crate::identity::TARGET.to_owned()], &[])
            .map_err(|_| "Recipe target interest")?;
        let work = cache.next_work(0).ok_or("Recipe work unavailable")?;
        if work.source() != source.key() {
            return Err("Recipe Work source changed".into());
        }
        gate.poll();
        if gate.stopped {
            return Err("Recipe departed before sole submission".into());
        }
        worker
            .as_mut()
            .unwrap()
            .try_submit(work, source, Instant::now() + Duration::from_secs(30))
            .map_err(|_| "Recipe submission refused")?;
        submitted = 1;
        Ok(())
    })();
    if let Err(failure) = preparation {
        error = Some(failure.to_string());
    }
    // Every post-submit failure shares the same drain. No cancellation, second
    // job, foreground resumption, cache result admission or guard recovery.
    if let Some(worker) = worker.as_mut() {
        while worker.has_outstanding() {
            gate.poll();
            if let Some(done) = worker.poll() {
                completed = Some(done);
                break;
            }
            if Instant::now() >= deadline && error.is_none() {
                error = Some("Logical drain deadline elapsed; actual join still required".into());
            }
            if error.is_none() {
                if let Err(failure) = gate.frame(shell, window, now) {
                    error = Some(failure.to_string());
                }
            }
            requests += shell.take_requests().len();
            if requests != 0 || shell.preferences() != &preferences {
                error.get_or_insert("Unexpected action/preferences change".into());
                gate.stopped = true;
            }
            std::thread::sleep(Duration::from_millis(17));
        }
    }
    let finish = worker.take().map(|worker| worker.finish());
    let observation = observed.and_then(|channel| channel.try_recv().ok());
    let own_after = crate::broker_probe::own_identity();
    // Include events arriving during join/self measurement. Reports describe
    // this final-poll snapshot, never an atomic future focus guarantee.
    gate.poll();
    requests += shell.take_requests().len();
    let no_image_result = completed
        .as_ref()
        .is_none_or(|done| matches!(&done.result, FetchResult::Failed));
    let decoded_transfers = u64::from(
        completed
            .as_ref()
            .is_some_and(|done| done.status == Status::Decoded),
    );
    let mut qualified = false;
    let mut status = Value::Null;
    let mut settlement = Value::Null;
    if let Some(done) = &completed {
        status = json!(format!("{:?}", done.status));
        settlement = json!(format!("{:?}", done.settlement));
        qualified = done.settlement == SessionSettlement::KnownClosed
            && done.status == Status::ProviderFailed(crate::artwork_worker::ProviderError::Denied)
            && matches!(&done.result, FetchResult::Failed)
            && observation.as_ref().is_some_and(|value| {
                value.case == marker.case
                    && value.source_key == *done.work.source()
                    && value.outcome == ClosedNegativeOutcome::ClosedNegative
                    && !value.encoded_bytes_returned_to_worker
            });
    }
    qualified &= no_image_result
        && decoded_transfers == 0
        && submitted == 1
        && error.is_none()
        && gate.departure
        && !gate.interrupted
        && requests == 0
        && shell.preferences() == &preferences
        && own_after.as_ref().is_ok_and(|value| value == &own)
        && finish
            .as_ref()
            .is_some_and(|value| value.joined && !value.panicked && value.outstanding.is_none());
    let report = json!({"schema":1,"recipe_version":RECIPE_VERSION,"case":marker.case,"marker":marker,"own_before":own,"own_after":own_after.ok(),"target_id":crate::identity::TARGET,"source_key":selected,
        "catalogue_reads":catalogue_reads,"state_store_opened_without_save":state_store.is_some(),"preferences_record_written":false,"pre_dispatch_hold_ms":hold_ms,"submitted":submitted,"completed":u64::from(completed.is_some()),"worker_joined":finish.as_ref().is_some_and(|value|value.joined),"worker_panicked":finish.as_ref().is_some_and(|value|value.panicked),
        "provider_observation":observation,"status":status,"settlement":settlement,"departure_observed":gate.departure,"interrupted":gate.interrupted,"rendering_latched_stopped":gate.stopped,"draws":gate.draws,"swaps":gate.swaps,
        "preferences_unchanged":shell.preferences()==&preferences,"unexpected_request_count":requests,"qualified":qualified,"error":error,
        "observation_end":"final_poll_before_reports","target_scene_qualified_by_native":false,"worker_cancel_requested":false,"retry_requested":false,"encoded_bytes_returned_to_worker":false,"no_image_result":no_image_result,"decoded_transfers":decoded_transfers,"uploaded_transfers":0,"normal_loader_qualified":false,"production_startup_qualified":false,"performance_qualified":false,"hard_total_shutdown_bound_claimed":false});
    writer.write(ProgressFile::Final, &report)?;
    writer.write(ProgressFile::ShellReceipt,&json!({"schema":1,"pid":own.pid,"exit_reason":"artwork_closed_negative_complete","artwork_closed_negative_proof":report,"action_receipts":[],"setting_writes":false,"native_read":null,"live_updates":catalogue_reads,"live_history":(0..catalogue_reads).map(|_|json!({"catalogue":"read","simulated":false})).collect::<Vec<_>>(),"preferences_unchanged":shell.preferences()==&preferences}))?;
    Ok(report)
}
#[cfg(not(target_arch = "arm"))]
pub fn run(
    _shell: &mut Shell,
    _window: &Window,
    _output: &Path,
    _marker: &Marker,
    _now: &mut u32,
    _poll_catalogue: impl FnMut(&mut Shell) -> Option<SourceMap>,
) -> Result<Value, Box<dyn Error>> {
    Err("Closed-negative recipe requires native Proof".into())
}

#[cfg(test)]
mod tests {
    use super::*;
    fn marker() -> Value {
        json!({"schema":1,"nonce":"a".repeat(32),"ui_sha256":"b".repeat(64),"broker_sha256":"c".repeat(64),"case":"read_after"})
    }
    #[test]
    fn closed_marker_has_only_two_fixed_cases_and_no_selectors() {
        for case in ["read_after", "finish"] {
            let mut value = marker();
            value["case"] = json!(case);
            assert!(Marker::parse(&serde_json::to_vec(&value).unwrap()).is_ok());
        }
        for case in [
            json!("read_before"),
            json!("ReadAfter"),
            json!(true),
            json!({"read_after":true}),
            json!({"read_after":null}),
        ] {
            let mut value = marker();
            value["case"] = case;
            assert!(Marker::parse(&serde_json::to_vec(&value).unwrap()).is_err());
        }
        for key in ["target_id", "path", "uri", "command", "duration", "profile"] {
            let mut value = marker();
            value[key] = json!("selected");
            assert!(Marker::parse(&serde_json::to_vec(&value).unwrap()).is_err());
        }
        let raw = marker().to_string();
        assert!(Marker::parse(
            raw.replacen("\"schema\":1", "\"schema\":1,\"schema\":1", 1)
                .as_bytes()
        )
        .is_err());
    }
    #[test]
    fn departure_and_input_latch_never_resume_on_focus_gain() {
        for kind in [1, 2, 3, 4, 5, 6, 7, 8, 10, 11] {
            let mut gate = RenderGate::default();
            gate.observe(kind);
            assert!(gate.stopped);
            gate.observe(9);
            assert!(gate.stopped);
            assert_eq!(gate.departure, matches!(kind, 7 | 8));
        }
        let mut gate = RenderGate::default();
        gate.observe(9);
        assert!(!gate.stopped);
        assert!(!gate.departure);
    }
    #[test]
    fn only_fixed_progress_leaves_are_admitted() {
        for file in [
            ProgressFile::PreDispatch,
            ProgressFile::ReadBodyReady,
            ProgressFile::FinishReady,
            ProgressFile::Final,
            ProgressFile::ShellReceipt,
        ] {
            assert!(ProgressFile::allows(file.name()));
        }
        for file in [
            "../shell-receipt.json",
            "selected.json",
            "/tmp/selected",
            "artwork-closed-negative-finish-ready.json/extra",
        ] {
            assert!(!ProgressFile::allows(file));
        }
    }
}
