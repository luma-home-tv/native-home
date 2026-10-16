//! Pure finite preferred-catalogue enumeration. No I/O, worker, GL or Root grant.
//!
//! A Cursor only selects an enumeration range. Advancing it never acknowledges
//! execution, publication, settlement or prepared coverage. A new Plan with the
//! same metadata cannot establish continuity with an earlier Plan's lifetime.
//! NOW and observed_ms must share Main live_origin.elapsed() wrapping domain;
//! animation ticks or a newly reset runner clock do not satisfy that contract.
use crate::app_artwork::{AppSources, IconKind, Source, SourceKey, SourceMap};
use crate::live::Freshness;
use serde::{Deserialize, Serialize};

pub const MAX_SOURCE_ENTRIES: usize = 512;
pub const MAX_BATCH: usize = 16;
const CURSOR_LIMIT: usize = 2048;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Error {
    UnavailableCatalogue,
    SimulatedCatalogue,
    StaleCatalogue,
    TooManySources,
    IncoherentEntry,
    Serialization,
    InvalidCursor,
    ChangedCatalogue,
    Withdrawn,
    UnplannedSource,
}

/// Borrowed caller observations, not a constructor-time freshness certificate.
/// Every Plan operation checks this stamp against its explicit current NOW.
#[derive(Clone, Copy)]
pub struct Catalogue<'a> {
    sources: Option<&'a SourceMap>,
    observed_ms: Option<u32>,
    simulated: bool,
    catalogue_available: bool,
}
impl<'a> Catalogue<'a> {
    pub fn new(
        sources: Option<&'a SourceMap>,
        observed_ms: Option<u32>,
        simulated: bool,
        catalogue_available: bool,
    ) -> Self {
        Self {
            sources,
            observed_ms,
            simulated,
            catalogue_available,
        }
    }
    fn current(self, now_ms: u32) -> Result<&'a SourceMap, Error> {
        if self.simulated {
            return Err(Error::SimulatedCatalogue);
        }
        if !self.catalogue_available || self.observed_ms.is_none() {
            return Err(Error::UnavailableCatalogue);
        }
        let sources = self.sources.ok_or(Error::UnavailableCatalogue)?;
        if !Freshness::current(self.observed_ms, now_ms) {
            return Err(Error::StaleCatalogue);
        }
        Ok(sources)
    }
}

/// Untrusted enumeration data; neither secret nor nonforgeable execution proof.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Cursor {
    schema: u8,
    keyset_sha256: String,
    next_ordinal: usize,
}
impl Cursor {
    pub fn from_json(raw: &[u8]) -> Result<Self, Error> {
        if raw.is_empty() || raw.len() > CURSOR_LIMIT {
            return Err(Error::InvalidCursor);
        }
        let value: Self = serde_json::from_slice(raw).map_err(|_| Error::InvalidCursor)?;
        if value.schema != 1
            || value.keyset_sha256.len() != 64
            || !value
                .keyset_sha256
                .bytes()
                .all(|b| b.is_ascii_digit() || (b'a'..=b'f').contains(&b))
            || value.next_ordinal > MAX_SOURCE_ENTRIES
        {
            return Err(Error::InvalidCursor);
        }
        Ok(value)
    }
    pub fn next_ordinal(&self) -> usize {
        self.next_ordinal
    }
    pub fn keyset_sha256(&self) -> &str {
        &self.keyset_sha256
    }
}

#[derive(Debug)]
pub struct Plan {
    keyset_sha256: String,
    keys: Vec<SourceKey>,
    source_entries: usize,
    no_preferred: usize,
    metadata_rejections: usize,
    withdrawn: bool,
}
#[derive(Clone, Debug)]
pub struct Batch {
    keyset_sha256: String,
    sources: Vec<Source>,
    start: usize,
    end: usize,
    total: usize,
    source_entries: usize,
    no_preferred: usize,
    metadata_rejections: usize,
    next: Option<Cursor>,
}

#[derive(Serialize)]
struct Descriptor<'a> {
    schema: u8,
    entries: &'a SourceMap,
}
fn valid_id(value: &str) -> bool {
    !value.is_empty()
        && value.len() <= 180
        && !matches!(value, "." | "..")
        && value
            .bytes()
            .all(|b| b.is_ascii_alphanumeric() || matches!(b, b'.' | b'_' | b'-'))
}
fn coherent(id: &str, entry: &AppSources) -> bool {
    if !valid_id(id)
        || id != entry.app_id.as_str()
        || entry.rejections.len() > 2
        || entry
            .version
            .as_ref()
            .is_some_and(|v| v.is_empty() || v.len() > 128 || v.chars().any(char::is_control))
    {
        return false;
    }
    for (kind, slot) in [
        (IconKind::Icon, entry.icon.as_ref()),
        (IconKind::LargeIcon, entry.large_icon.as_ref()),
    ] {
        let rejected = entry.rejections.iter().filter(|r| r.kind == kind).count();
        match slot {
            Some(source) => {
                let key = source.key();
                if key.app_id.as_str() != id
                    || entry.version.as_deref() != Some(key.version.as_str())
                    || key.kind != kind
                    || rejected != 0
                {
                    return false;
                }
            }
            None if rejected != 1 => return false,
            None => {}
        }
    }
    true
}
struct Description {
    digest: String,
    keys: Vec<SourceKey>,
    no_preferred: usize,
    rejections: usize,
}
fn describe(sources: &SourceMap) -> Result<Description, Error> {
    if sources.len() > MAX_SOURCE_ENTRIES {
        return Err(Error::TooManySources);
    }
    let mut keys = Vec::with_capacity(sources.len());
    let mut no_preferred = 0;
    let mut rejections = 0;
    for (id, entry) in sources {
        if !coherent(id, entry) {
            return Err(Error::IncoherentEntry);
        }
        rejections += entry.rejections.len();
        if let Some(source) = entry.preferred() {
            keys.push(source.key().clone());
        } else {
            no_preferred += 1;
        }
    }
    keys.sort();
    // BTreeMap ordering and source Serialize declarations make this schema fixed;
    // all admitted metadata (including no-preferred/rejections) binds the cursor.
    let bytes = serde_json::to_vec(&Descriptor {
        schema: 1,
        entries: sources,
    })
    .map_err(|_| Error::Serialization)?;
    let digest = plx_base::sha256::sha256(&bytes);
    let hex = digest
        .iter()
        .map(|b| format!("{b:02x}"))
        .collect::<String>();
    Ok(Description {
        digest: hex,
        keys,
        no_preferred,
        rejections,
    })
}
impl Plan {
    pub fn from_current(catalogue: Catalogue<'_>, now_ms: u32) -> Result<Self, Error> {
        let sources = catalogue.current(now_ms)?;
        let description = describe(sources)?;
        Ok(Self {
            keyset_sha256: description.digest,
            keys: description.keys,
            source_entries: sources.len(),
            no_preferred: description.no_preferred,
            metadata_rejections: description.rejections,
            withdrawn: false,
        })
    }
    pub fn keyset_sha256(&self) -> &str {
        &self.keyset_sha256
    }
    pub fn eligible_preferred(&self) -> usize {
        self.keys.len()
    }
    pub fn withdrawn(&self) -> bool {
        self.withdrawn
    }
    fn check_current<'a>(
        &mut self,
        catalogue: Catalogue<'a>,
        now_ms: u32,
    ) -> Result<&'a SourceMap, Error> {
        if self.withdrawn {
            return Err(Error::Withdrawn);
        }
        let result = (|| {
            let sources = catalogue.current(now_ms)?;
            let description = describe(sources)?;
            if description.digest != self.keyset_sha256 {
                return Err(Error::ChangedCatalogue);
            }
            Ok(sources)
        })();
        if result.is_err() {
            self.withdrawn = true;
        }
        result
    }
    /// Enumerates at most16 preferred keys. A next cursor is not a completed job.
    pub fn next_batch(
        &mut self,
        catalogue: Catalogue<'_>,
        now_ms: u32,
        cursor: Option<&Cursor>,
    ) -> Result<Batch, Error> {
        let sources = self.check_current(catalogue, now_ms)?;
        let start = match cursor {
            None => 0,
            Some(value) => {
                if value.schema != 1
                    || value.keyset_sha256 != self.keyset_sha256
                    || value.next_ordinal > self.keys.len()
                    || (value.next_ordinal != self.keys.len()
                        && value.next_ordinal % MAX_BATCH != 0)
                {
                    return Err(Error::InvalidCursor);
                }
                value.next_ordinal
            }
        };
        let end = start.saturating_add(MAX_BATCH).min(self.keys.len());
        let mut selected = Vec::with_capacity(end - start);
        for key in &self.keys[start..end] {
            let source = sources
                .get(&key.app_id)
                .and_then(AppSources::preferred)
                .filter(|source| source.key() == key)
                .ok_or(Error::UnplannedSource)?;
            selected.push(source.clone());
        }
        let next = (end < self.keys.len()).then(|| Cursor {
            schema: 1,
            keyset_sha256: self.keyset_sha256.clone(),
            next_ordinal: end,
        });
        Ok(Batch {
            keyset_sha256: self.keyset_sha256.clone(),
            sources: selected,
            start,
            end,
            total: self.keys.len(),
            source_entries: self.source_entries,
            no_preferred: self.no_preferred,
            metadata_rejections: self.metadata_rejections,
            next,
        })
    }
    /// The returned Source is selected from this fresh map, never built from a key.
    pub fn before_dispatch<'a>(
        &mut self,
        catalogue: Catalogue<'a>,
        now_ms: u32,
        key: &SourceKey,
    ) -> Result<&'a Source, Error> {
        let sources = self.check_current(catalogue, now_ms)?;
        if self.keys.binary_search(key).is_err() {
            return Err(Error::UnplannedSource);
        }
        sources
            .get(&key.app_id)
            .and_then(AppSources::preferred)
            .filter(|source| source.key() == key)
            .ok_or(Error::UnplannedSource)
    }
}
impl Batch {
    pub fn sources(&self) -> &[Source] {
        &self.sources
    }
    pub fn next_cursor(&self) -> Option<&Cursor> {
        self.next.as_ref()
    }
    pub fn manifest(&self) -> PlanningManifest {
        let capacity = crate::durable_icon_cache::Config::default();
        PlanningManifest {
            schema: 1,
            keyset_sha256: self.keyset_sha256.clone(),
            source_entries: self.source_entries,
            eligible_preferred: self.total,
            no_preferred: self.no_preferred,
            metadata_rejections: self.metadata_rejections,
            batch_start: self.start,
            batch_end: self.end,
            planned_count: self.sources.len(),
            planned_keys: self.sources.iter().map(|s| s.key().clone()).collect(),
            next_cursor: self.next.clone(),
            execution_verified: false,
            prepared_coverage_verified: false,
            root_authority_granted: false,
            prior_batch_completion_verified: false,
            generation_continuity_verified: false,
            default_cache_capacity: CapacityConstraint {
                entries: capacity.entries,
                encoded_bytes: capacity.encoded_bytes,
                execution_capacity_checked: false,
            },
        }
    }
}
#[derive(Clone, Debug, Serialize)]
pub struct CapacityConstraint {
    entries: usize,
    encoded_bytes: u64,
    execution_capacity_checked: bool,
}
#[derive(Clone, Debug, Serialize)]
pub struct PlanningManifest {
    schema: u8,
    keyset_sha256: String,
    source_entries: usize,
    eligible_preferred: usize,
    no_preferred: usize,
    metadata_rejections: usize,
    batch_start: usize,
    batch_end: usize,
    planned_count: usize,
    planned_keys: Vec<SourceKey>,
    next_cursor: Option<Cursor>,
    execution_verified: bool,
    prepared_coverage_verified: bool,
    #[serde(rename = "Root_authority_granted")]
    root_authority_granted: bool,
    prior_batch_completion_verified: bool,
    generation_continuity_verified: bool,
    default_cache_capacity: CapacityConstraint,
}

#[cfg(test)]
#[path = "artwork_cache_preparation_tests.rs"]
mod tests;
