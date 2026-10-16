//! Bounded same-worker observations, not independent Root/process authority.
use crate::{app_artwork::SourceKey, artwork_worker::SessionSettlement};
use serde::Serialize;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum Settlement {
    NoExternalSession,
    KnownClosed,
    Unknown,
}
impl From<SessionSettlement> for Settlement {
    fn from(value: SessionSettlement) -> Self {
        match value {
            SessionSettlement::NoExternalSession => Self::NoExternalSession,
            SessionSettlement::KnownClosed => Self::KnownClosed,
            SessionSettlement::Unknown => Self::Unknown,
        }
    }
}
/// Records the real ordinary publisher callback and optional receipt write.
/// Publishing precedes Worker decode, so this cannot establish usable pixels.
#[derive(Clone, Debug, Serialize)]
pub struct Publication {
    pub encoded_sha256: [u8; 32],
    pub source_identity: crate::app_artwork::FileIdentity,
    pub producer: crate::durable_icon_cache::ProducerVersion,
    pub encoded_bytes: usize,
    pub succeeded: bool,
    pub receipt_written: bool,
}
#[derive(Clone, Debug, Serialize)]
pub struct Attempt {
    pub sequence: u8,
    pub source_key: SourceKey,
    pub observation_index: Option<u8>,
    pub current_ticket_resolved: bool,
    pub settlement: Settlement,
    pub encoded_returned: bool,
    pub provider_error: Option<String>,
    pub publication: Option<Publication>,
}
impl Attempt {
    /// Only local pairing consistency; external child/pending/kernel evidence
    /// remains required. This cannot clear a durable Ticket or Unknown latch.
    pub(crate) fn matches_closed(&self, sequence: usize, key: &SourceKey) -> bool {
        usize::from(self.sequence) == sequence
            && self.source_key == *key
            && self.observation_index == Some(self.sequence)
            && (1..=16).contains(&self.sequence)
            && self.current_ticket_resolved
            && self.settlement == Settlement::KnownClosed
            && self.encoded_returned == self.provider_error.is_none()
            && (self.publication.is_none() || self.encoded_returned)
    }
}
