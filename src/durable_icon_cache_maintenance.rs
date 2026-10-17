//! Pure supplied-byte inspection, never a filesystem/mutation/Root permit.
//!
//! All accepted producer profiles are independent inputs. Cache declarations
//! become neither Sources nor expected producers. Any uncertainty refuses the
//! whole proposal; no partial deletion list escapes. Actual freshness, namespace
//! mapping, withdrawal, worker join and exclusive writer ownership remain future
//! caller obligations. This module has no I/O or production callsite.
use super::{
    digest, invalid, Config, FileIdentity, ProducerVersion, Record, HEADER_CAP, MAGIC, MAX_FILE,
};
use crate::app_artwork::SourceKey;
use std::{collections::BTreeSet, io};

/// Independently supplied ordinary process identity, not a measured credential.
#[derive(Clone, Copy, Debug)]
pub struct SuppliedOwner {
    pub uid: u32,
    pub gid: u32,
}

/// Complete independently bound profile union. It can never be used as a
/// publisher, ReadExpectation, Root admission or proof that inputs are current.
#[derive(Clone, Debug)]
pub struct RetainedProducers {
    versions: Vec<ProducerVersion>,
    current_sidecar_sha256: [u8; 32],
    selector_sha256: Vec<[u8; 32]>,
}
impl RetainedProducers {
    pub fn from_profiles(
        current_sidecar: &[u8],
        selectors: &[&[u8]],
        required_selector_sha256: &[[u8; 32]],
        independently_bound_consumer_ui: &str,
    ) -> io::Result<Self> {
        if selectors.is_empty()
            || selectors.len() > 8
            || required_selector_sha256.is_empty()
            || required_selector_sha256.len() > 8
        {
            return Err(invalid(
                "Explicit complete bounded selector registry required",
            ));
        }
        let expected = required_selector_sha256
            .iter()
            .copied()
            .collect::<BTreeSet<_>>();
        let actual = selectors
            .iter()
            .map(|raw| digest(raw))
            .collect::<BTreeSet<_>>();
        if expected.len() != required_selector_sha256.len()
            || actual.len() != selectors.len()
            || expected != actual
        {
            return Err(invalid("Selector registry omitted, replaced or duplicated"));
        }
        let current = crate::cache_producer_profile::inspect_for_maintenance(
            current_sidecar,
            independently_bound_consumer_ui,
        )?;
        let mut versions = vec![current];
        for raw in selectors {
            versions.push(crate::cache_consumer_profile::inspect_for_maintenance(
                raw,
                independently_bound_consumer_ui,
            )?);
        }
        versions.sort_by(|a, b| {
            (&a.source_revision, a.ui_sha256, a.broker_sha256).cmp(&(
                &b.source_revision,
                b.ui_sha256,
                b.broker_sha256,
            ))
        });
        versions.dedup();
        Ok(Self {
            versions,
            current_sidecar_sha256: digest(current_sidecar),
            selector_sha256: actual.into_iter().collect(),
        })
    }
    pub fn versions(&self) -> &[ProducerVersion] {
        &self.versions
    }
    pub fn current_sidecar_sha256(&self) -> [u8; 32] {
        self.current_sidecar_sha256
    }
    pub fn selector_sha256(&self) -> &[[u8; 32]] {
        &self.selector_sha256
    }
}

/// Supplied ordinary file bytes and their complete before/after observations.
/// No path or FD is opened by this planner; observations are not provenance.
pub struct SuppliedLeaf<'a> {
    pub name: &'a str,
    pub bytes: &'a [u8],
    pub before: &'a FileIdentity,
    pub after: &'a FileIdentity,
}
pub struct SuppliedInventory<'a> {
    pub root_before: &'a FileIdentity,
    pub root_after: &'a FileIdentity,
    pub names_before: &'a [&'a str],
    pub names_after: &'a [&'a str],
    pub files: &'a [SuppliedLeaf<'a>],
}

/// Ordinary untrusted data only, deliberately not Demand or a Source capability.
#[derive(Clone, Debug)]
pub struct InspectedRecord {
    filename: String,
    file_identity: FileIdentity,
    entry_sha256: [u8; 32],
    producer: ProducerVersion,
    declared_source_key: SourceKey,
    encoded_bytes: u64,
}
impl InspectedRecord {
    pub fn filename(&self) -> &str {
        &self.filename
    }
    pub fn file_identity(&self) -> &FileIdentity {
        &self.file_identity
    }
    pub fn entry_sha256(&self) -> [u8; 32] {
        self.entry_sha256
    }
    pub fn producer(&self) -> &ProducerVersion {
        &self.producer
    }
    pub fn declared_source_key(&self) -> &SourceKey {
        &self.declared_source_key
    }
}

#[derive(Clone, Debug)]
pub struct MaintenancePlan {
    retained: Vec<InspectedRecord>,
    candidates: Vec<InspectedRecord>,
    encoded_bytes: u64,
}
impl MaintenancePlan {
    pub fn retained(&self) -> &[InspectedRecord] {
        &self.retained
    }
    pub fn retirement_candidates(&self) -> &[InspectedRecord] {
        &self.candidates
    }
    pub fn encoded_bytes(&self) -> u64 {
        self.encoded_bytes
    }
    /// A proposal never confers delete/Root/session/Ticket/freshness authority.
    pub const fn mutations_authorized(&self) -> bool {
        false
    }
    pub const fn source_adoption_or_root_authority(&self) -> bool {
        false
    }
    pub const fn input_provenance_or_currentness_proven(&self) -> bool {
        false
    }
}

fn scalar_metadata(meta: &FileIdentity, owner: SuppliedOwner) -> bool {
    meta.uid == owner.uid
        && meta.gid == owner.gid
        && meta.inode > 0
        && (0..1_000_000_000).contains(&meta.modified_nanos)
        && (0..1_000_000_000).contains(&meta.changed_nanos)
}
fn inspect(leaf: &SuppliedLeaf<'_>, owner: SuppliedOwner) -> io::Result<InspectedRecord> {
    if leaf.before != leaf.after
        || !scalar_metadata(leaf.before, owner)
        || leaf.before.mode != 0o100000 | 0o600
        || leaf.before.links != 1
        || leaf.before.bytes != leaf.bytes.len() as u64
        || leaf.bytes.len() <= 12
        || leaf.bytes.len() as u64 > MAX_FILE
        || &leaf.bytes[..8] != MAGIC
    {
        return Err(invalid(
            "Changed, foreign, linked or malformed ordinary file",
        ));
    }
    let size = u32::from_be_bytes(
        leaf.bytes[8..12]
            .try_into()
            .map_err(|_| invalid("Record prefix"))?,
    ) as usize;
    if size == 0 || size > HEADER_CAP || 12 + size >= leaf.bytes.len() {
        return Err(invalid("Bounded complete ordinary record header"));
    }
    // Exact existing serde structs/deny_unknown_fields, validation and filename
    // serializer; no parallel format grammar or cache-derived expected Source.
    let record: Record = serde_json::from_slice(&leaf.bytes[12..12 + size])
        .map_err(|_| invalid("Malformed, duplicate or unknown record metadata"))?;
    record.validate()?;
    let body = &leaf.bytes[12 + size..];
    if body.len() as u64 != u64::from(record.encoded_bytes)
        || digest(body) != record.encoded_sha256
        || record.name()?.as_bytes() != leaf.name.as_bytes()
    {
        return Err(invalid("Ordinary record name/body length/digest differs"));
    }
    Ok(InspectedRecord {
        filename: leaf.name.into(),
        file_identity: leaf.before.clone(),
        entry_sha256: digest(leaf.bytes),
        declared_source_key: record.source.key()?,
        producer: record.producer,
        encoded_bytes: u64::from(record.encoded_bytes),
    })
}

pub fn plan(
    inventory: SuppliedInventory<'_>,
    owner: SuppliedOwner,
    retention: &RetainedProducers,
    config: Config,
) -> io::Result<MaintenancePlan> {
    let config = config.valid()?;
    if owner.uid < 1000
        || inventory.root_before != inventory.root_after
        || !scalar_metadata(inventory.root_before, owner)
        || inventory.root_before.mode != 0o040000 | 0o700
        || inventory.root_before.links < 2
        || inventory.names_before != inventory.names_after
        || inventory.names_before.len() > config.entries
        || inventory.names_before.len() != inventory.files.len()
    {
        return Err(invalid(
            "Complete stable private ordinary inventory required",
        ));
    }
    let names = inventory
        .names_before
        .iter()
        .copied()
        .collect::<BTreeSet<_>>();
    let supplied = inventory
        .files
        .iter()
        .map(|leaf| leaf.name)
        .collect::<BTreeSet<_>>();
    if names.len() != inventory.names_before.len()
        || supplied.len() != inventory.files.len()
        || names != supplied
        || names.contains(".publish-next")
    {
        return Err(invalid(
            "Partial, duplicate or writer-slot inventory; no slot stealing",
        ));
    }
    let mut retained = Vec::new();
    let mut candidates = Vec::new();
    let mut total = 0_u64;
    for leaf in inventory.files {
        let entry = inspect(leaf, owner)?;
        total = total
            .checked_add(entry.encoded_bytes)
            .ok_or_else(|| invalid("Byte overflow"))?;
        if total > config.encoded_bytes {
            return Err(invalid("Whole inventory encoded budget exceeded"));
        }
        if retention.versions.contains(&entry.producer) {
            retained.push(entry);
        } else {
            candidates.push(entry);
        }
    }
    retained.sort_by(|a, b| a.filename.cmp(&b.filename));
    candidates.sort_by(|a, b| a.filename.cmp(&b.filename));
    Ok(MaintenancePlan {
        retained,
        candidates,
        encoded_bytes: total,
    })
}

#[cfg(test)]
#[path = "durable_icon_cache_maintenance_tests.rs"]
mod tests;
