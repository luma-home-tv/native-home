//! Bounded session-only observations supplied after draw. No I/O, codec, worker,
//! GL, cache refresh, source authority or lifecycle authority. Counts are sets
//! of historical observations, never proof that all textures remain resident.
use crate::app_artwork::{IconKind, RootClass, SourceKey};
use serde::Serialize;
use serde_json::Value;
use std::collections::{BTreeMap, BTreeSet};
use std::io::{self, Write};

pub const MAX_KEYS: usize = 512;
pub const MAX_FRAME_ROWS: usize = 64;
pub const MAX_FRAME_BYTES: usize = 128 * 1024;
/// Compact JSON envelope bound; parent pretty-printing adds whitespace.
pub const MAX_REPORT_BYTES: usize = 1024 * 1024;
const RECORD_BUDGET: usize = MAX_REPORT_BYTES - 16 * 1024;

#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct Epoch {
    pub catalogue_keyset_sha256: String,
    pub eligible_preferred: usize,
    pub cache_adoptions: u64,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum Incomplete {
    CatalogueUnavailable,
    InvalidCatalogue,
    ChangedEpoch,
    InvalidFrame,
    StaleSource,
    NonmonotonicFrame,
    SourceConflict,
    Capacity,
    SerializationBound,
    NoObservations,
}
#[derive(Clone, Debug, Serialize)]
pub struct Observation {
    pub source_key: SourceKey,
    pub page: String,
    pub draw_sequence: u64,
    pub elapsed_ms: u64,
    pub source_age_ms: u32,
    pub texture_handle: u32,
    pub width: u32,
    pub height: u32,
    pub rect: [f64; 4],
    pub clip: [f64; 4],
    pub focused: bool,
}
#[derive(Debug, Serialize)]
pub struct Report {
    pub schema: u8,
    pub kind: &'static str,
    pub epoch: Option<Epoch>,
    pub observations: Vec<Observation>,
    pub observed_distinct_sources: usize,
    /// A numerical comparison only; independent current-key membership and
    /// complete cache/source/producer readback remain the saved auditor's job.
    pub observed_count_matches_eligible: bool,
    pub history_valid: bool,
    pub incomplete_reason: Option<Incomplete>,
    pub frames_attempted: u64,
    pub frames_accepted: u64,
    pub observations_are_historical: bool,
    pub all_catalogue_qualified: bool,
    pub current_texture_residency_qualified: bool,
    pub independent_source_membership_qualified: bool,
    #[serde(rename = "Root_authority_qualified")]
    pub root_authority_qualified: bool,
    #[serde(rename = "positive_Root_producer_qualified")]
    pub positive_root_producer_qualified: bool,
    pub native_foreground_scene_qualified: bool,
    pub physical_panel_qualified: bool,
    pub pixels_qualified: bool,
    pub cleanup_qualified: bool,
    pub performance_qualified: bool,
    pub continuous_source_file_freshness_qualified: bool,
}
struct Prepared {
    epoch: Epoch,
    rows: Vec<Observation>,
    counters: (u64, u64),
    bytes: usize,
}
#[derive(Default)]
pub struct Ledger {
    current: Option<Epoch>,
    epoch: Option<Epoch>,
    records: BTreeMap<SourceKey, Observation>,
    record_bytes: usize,
    incomplete: Option<Incomplete>,
    last: Option<(u64, u64, u64, u64)>,
    attempted: u64,
    accepted: u64,
}
impl Ledger {
    pub fn new() -> Self {
        Self::default()
    }
    fn fail(&mut self, reason: Incomplete) {
        self.incomplete.get_or_insert(reason);
    }
    /// Called on every loop and again after image-worker join, including
    /// loops without an eligible frame. Before the first observation, missing
    /// catalogue or adoption zero simply awaits initial readiness. Afterwards
    /// any loss/change is terminal for this ledger; it cannot reset or heal.
    pub fn catalogue(&mut self, current: Option<(&str, usize, u64)>) {
        if self.incomplete.is_some() {
            return;
        }
        let parsed = match current {
            None => None,
            Some((sha, count, adoptions)) => {
                if !valid_sha(sha) || count > MAX_KEYS {
                    self.fail(Incomplete::InvalidCatalogue);
                    return;
                }
                Some(Epoch {
                    catalogue_keyset_sha256: sha.into(),
                    eligible_preferred: count,
                    cache_adoptions: adoptions,
                })
            }
        };
        if let Some(epoch) = &self.epoch {
            if parsed.as_ref() != Some(epoch) {
                self.fail(if parsed.is_none() {
                    Incomplete::CatalogueUnavailable
                } else {
                    Incomplete::ChangedEpoch
                });
                return;
            }
        }
        self.current = parsed;
    }
    /// Frame comes from Shell's existing current owned-texture + actual drawn
    /// clipping checks. This validates bounded shape, not caller authenticity.
    /// Validate the WHOLE frame and prospective records before committing any.
    pub fn observe(&mut self, frame: &Value, draw_sequence: u64, elapsed_ms: u64) {
        self.attempted = self.attempted.saturating_add(1);
        if self.incomplete.is_some() {
            return;
        }
        let result = self.prepare(frame, draw_sequence, elapsed_ms);
        match result {
            Err(reason) => self.fail(reason),
            Ok(Prepared {
                epoch,
                rows,
                counters,
                bytes,
            }) => {
                self.epoch.get_or_insert(epoch);
                for row in rows {
                    self.records.entry(row.source_key.clone()).or_insert(row);
                }
                self.record_bytes += bytes;
                self.last = Some((draw_sequence, elapsed_ms, counters.0, counters.1));
                self.accepted = self.accepted.saturating_add(1);
            }
        }
    }
    fn prepare(
        &self,
        frame: &Value,
        draw_sequence: u64,
        elapsed_ms: u64,
    ) -> Result<Prepared, Incomplete> {
        let invalid = Incomplete::InvalidFrame;
        if bounded_len(frame, MAX_FRAME_BYTES).is_none()
            || frame["schema"].as_u64() != Some(1)
            || frame["kind"].as_str() != Some("ordinary-cache-frame-v1")
            || frame["cache_readiness"].as_str() != Some("Ready")
            || frame["cache_readiness_is_index_not_images"].as_bool() != Some(true)
            || draw_sequence == 0
        {
            return Err(invalid);
        }
        for flag in [
            "Root_authority_qualified",
            "positive_Root_producer_qualified",
            "native_foreground_scene_qualified",
            "physical_panel_qualified",
            "cleanup_qualified",
            "performance_qualified",
            "continuous_source_file_freshness_qualified",
        ] {
            if frame[flag].as_bool() != Some(false) {
                return Err(invalid);
            }
        }
        let epoch = self
            .current
            .as_ref()
            .ok_or(Incomplete::CatalogueUnavailable)?;
        if epoch.cache_adoptions == 0 || epoch.eligible_preferred == 0 {
            return Err(invalid);
        }
        if frame["catalogue_keyset_sha256"].as_str() != Some(epoch.catalogue_keyset_sha256.as_str())
            || frame["eligible_preferred"].as_u64() != Some(epoch.eligible_preferred as u64)
            || frame["cache_adoptions"].as_u64() != Some(epoch.cache_adoptions)
            || self.epoch.as_ref().is_some_and(|bound| bound != epoch)
        {
            return Err(Incomplete::ChangedEpoch);
        }
        let observed = integer32(&frame["source_observed_ms"]).ok_or(invalid)?;
        let now = integer32(&frame["source_sample_now_ms"]).ok_or(invalid)?;
        let age = integer32(&frame["source_age_ms"]).ok_or(invalid)?;
        if now.wrapping_sub(observed) != age {
            return Err(invalid);
        }
        if !crate::live::Freshness::current(Some(observed), now) {
            return Err(Incomplete::StaleSource);
        }
        let page = frame["page"]
            .as_str()
            .filter(|p| matches!(*p, "home" | "apps"))
            .ok_or(invalid)?;
        let uploads = frame["cache_uploads"].as_u64().ok_or(invalid)?;
        let packaged = frame["packaged_uploads"].as_u64().ok_or(invalid)?;
        if self.last.is_some_and(|(draw, elapsed, cache, pkg)| {
            draw_sequence <= draw || elapsed_ms < elapsed || uploads < cache || packaged < pkg
        }) {
            return Err(Incomplete::NonmonotonicFrame);
        }
        let cards = frame["cached_cards"].as_array().ok_or(invalid)?;
        let entries = frame["texture_entries"].as_u64().ok_or(invalid)?;
        if cards.is_empty()
            || cards.len() > MAX_FRAME_ROWS
            || entries > 64
            || entries < cards.len() as u64
            || uploads < cards.len() as u64
        {
            return Err(invalid);
        }
        let mut rows = Vec::new();
        let mut ids = BTreeSet::new();
        let mut handles = BTreeSet::new();
        let mut added = 0;
        let mut bytes = 0;
        for card in cards {
            let key = parse_key(&card["source_key"]).ok_or(invalid)?;
            if card["id"].as_str() != Some(key.app_id.as_str()) || !ids.insert(key.app_id.clone()) {
                return Err(invalid);
            }
            if self
                .records
                .keys()
                .any(|old| old.app_id == key.app_id && old != &key)
            {
                return Err(Incomplete::SourceConflict);
            }
            let width = integer32(&card["width"]).ok_or(invalid)?;
            let height = integer32(&card["height"]).ok_or(invalid)?;
            let handle = integer32(&card["texture_handle"]).ok_or(invalid)?;
            let rect = rectangle(&card["rect"]).ok_or(invalid)?;
            let clip = rectangle(&card["clip"]).ok_or(invalid)?;
            if width == 0
                || height == 0
                || width > 512
                || height > 512
                || u64::from(width) * u64::from(height) * 4 > 1024 * 1024
                || handle == 0
                || !handles.insert(handle)
                || !intersects(rect, clip)
            {
                return Err(invalid);
            }
            let row = Observation {
                source_key: key,
                page: page.into(),
                draw_sequence,
                elapsed_ms,
                source_age_ms: age,
                texture_handle: handle,
                width,
                height,
                rect,
                clip,
                focused: card["focused"].as_bool().ok_or(invalid)?,
            };
            if !self.records.contains_key(&row.source_key) {
                added += 1;
                bytes +=
                    bounded_len(&row, RECORD_BUDGET).ok_or(Incomplete::SerializationBound)? + 1;
                rows.push(row);
            }
        }
        if self.records.len() + added > MAX_KEYS
            || self.records.len() + added > epoch.eligible_preferred
        {
            return Err(Incomplete::Capacity);
        }
        if self.record_bytes + bytes > RECORD_BUDGET {
            return Err(Incomplete::SerializationBound);
        }
        Ok(Prepared {
            epoch: epoch.clone(),
            rows,
            counters: (uploads, packaged),
            bytes,
        })
    }
    pub fn finish(self) -> Report {
        let reason = self
            .incomplete
            .or_else(|| self.epoch.is_none().then_some(Incomplete::NoObservations));
        let valid = reason.is_none();
        let count = self.records.len();
        Report {
            schema: 1,
            kind: "ordinary-cache-observation-ledger-v1",
            observed_count_matches_eligible: valid
                && self
                    .epoch
                    .as_ref()
                    .is_some_and(|e| e.eligible_preferred == count),
            epoch: self.epoch,
            observations: self.records.into_values().collect(),
            observed_distinct_sources: count,
            history_valid: valid,
            incomplete_reason: reason,
            frames_attempted: self.attempted,
            frames_accepted: self.accepted,
            observations_are_historical: true,
            all_catalogue_qualified: false,
            current_texture_residency_qualified: false,
            independent_source_membership_qualified: false,
            root_authority_qualified: false,
            positive_root_producer_qualified: false,
            native_foreground_scene_qualified: false,
            physical_panel_qualified: false,
            pixels_qualified: false,
            cleanup_qualified: false,
            performance_qualified: false,
            continuous_source_file_freshness_qualified: false,
        }
    }
}
fn valid_sha(value: &str) -> bool {
    value.len() == 64
        && value
            .bytes()
            .all(|b| b.is_ascii_digit() || (b'a'..=b'f').contains(&b))
}
fn integer32(value: &Value) -> Option<u32> {
    u32::try_from(value.as_u64()?).ok()
}
fn rectangle(value: &Value) -> Option<[f64; 4]> {
    let values = value.as_array()?;
    if values.len() != 4 {
        return None;
    }
    let out = [
        values[0].as_f64()?,
        values[1].as_f64()?,
        values[2].as_f64()?,
        values[3].as_f64()?,
    ];
    // Actual canvas coordinates, not an unbounded fabricated geometry domain.
    out.iter()
        .all(|v| v.is_finite() && v.abs() <= 65536.0)
        .then_some(out)
        .filter(|r| r[2] > 0.0 && r[3] > 0.0)
}
fn intersects(a: [f64; 4], b: [f64; 4]) -> bool {
    a[0] < b[0] + b[2] && a[0] + a[2] > b[0] && a[1] < b[1] + b[3] && a[1] + a[3] > b[1]
}
fn parse_key(value: &Value) -> Option<SourceKey> {
    let object = value.as_object()?;
    if object.len() != 6
        || !["app_id", "version", "class", "folder", "relative", "kind"]
            .iter()
            .all(|k| object.contains_key(*k))
    {
        return None;
    }
    let app_id = value["app_id"].as_str()?;
    if app_id.is_empty()
        || app_id.len() > 180
        || matches!(app_id, "." | "..")
        || !app_id
            .bytes()
            .all(|c| c.is_ascii_alphanumeric() || matches!(c, b'.' | b'_' | b'-'))
    {
        return None;
    }
    let version = value["version"].as_str()?;
    if version.is_empty() || version.len() > 128 || version.chars().any(char::is_control) {
        return None;
    }
    let class = match value["class"].as_str()? {
        "lg_system" => RootClass::LgSystem,
        "encrypted_apps" => RootClass::EncryptedApps,
        "developer_apps" => RootClass::DeveloperApps,
        "system_apps" => RootClass::SystemApps,
        "optional_content" => RootClass::OptionalContent,
        _ => return None,
    };
    let folder = value["folder"].as_str()?;
    if folder != format!("{}/{app_id}", class.prefix()) {
        return None;
    }
    let values = value["relative"].as_array()?;
    if values.is_empty() || values.len() > 8 {
        return None;
    }
    let mut relative = Vec::new();
    for v in values {
        let component = v.as_str()?;
        if component.is_empty()
            || component.len() > 128
            || matches!(component, "." | "..")
            || component.contains(['/', '\\', ':'])
            || component.chars().any(char::is_control)
        {
            return None;
        }
        relative.push(component.to_owned());
    }
    if relative.iter().map(String::len).sum::<usize>() + relative.len() - 1 > 512 {
        return None;
    }
    let kind = match value["kind"].as_str()? {
        "icon" => IconKind::Icon,
        "large_icon" => IconKind::LargeIcon,
        _ => return None,
    };
    Some(SourceKey {
        app_id: app_id.into(),
        version: version.into(),
        class,
        folder: folder.into(),
        relative,
        kind,
    })
}
struct Counter {
    len: usize,
    cap: usize,
}
impl Write for Counter {
    fn write(&mut self, bytes: &[u8]) -> io::Result<usize> {
        let len = self
            .len
            .checked_add(bytes.len())
            .filter(|n| *n <= self.cap)
            .ok_or_else(|| io::Error::other("Observation serialization bound"))?;
        self.len = len;
        Ok(bytes.len())
    }
    fn flush(&mut self) -> io::Result<()> {
        Ok(())
    }
}
fn bounded_len(value: &impl Serialize, cap: usize) -> Option<usize> {
    let mut counter = Counter { len: 0, cap };
    serde_json::to_writer(&mut counter, value).ok()?;
    Some(counter.len)
}

#[cfg(test)]
#[path = "session_cache_observation_tests.rs"]
mod tests;
