//! Bounded encoded artwork packets, separate from the unchanged Identify wire.
//! No paths, commands, decoding, GL, or authority to read a resource lives here.
//! Complete ends one transaction; only Finished plus clean EOF ends the session.
use crate::app_artwork::{EncodedFile, FileIdentity, IconKind, RootClass, SourceKey};
use crate::broker_installation::Profile;
pub use crate::broker_protocol::Response as Identity;
use crate::broker_transport::{Connection, Credentials};
use serde::{Deserialize, Serialize};
use std::{fmt, io, time::Instant};

pub const MESSAGE_LIMIT: usize = 4096;
pub const ENCODED_LIMIT: usize = 1024 * 1024;
pub const CHUNK_BYTES: usize = 4000;
const CHUNK_HEADER: usize = 14;
const MAGIC: &[u8; 4] = b"LHA1";

#[derive(Debug)]
pub enum Error {
    Message,
    Bounds,
    Identity,
    Request,
    Credentials,
    Sequence,
    Digest,
    Incomplete,
    Extra,
    Allocation,
    Transport(io::Error),
}
impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(match self {
            Self::Message => "Invalid strict artwork message",
            Self::Bounds => "Artwork transfer bound exceeded",
            Self::Identity => "Artwork session identity differs",
            Self::Request => "Artwork request binding differs",
            Self::Credentials => "Artwork kernel peer credentials differ",
            Self::Sequence => "Artwork packet sequence differs",
            Self::Digest => "Artwork content or header digest differs",
            Self::Incomplete => "Artwork transfer lacks completion",
            Self::Extra => "Unexpected packet after transaction completion",
            Self::Allocation => "Artwork bounded allocation declined",
            Self::Transport(_) => "Artwork sequenced-packet transport failed",
        })
    }
}
impl std::error::Error for Error {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::Transport(error) => Some(error),
            _ => None,
        }
    }
}
impl From<io::Error> for Error {
    fn from(error: io::Error) -> Self {
        Self::Transport(error)
    }
}

macro_rules! closed_enum {
    ($name:ident { $($variant:ident => $wire:literal),+ $(,)? }) => {
        #[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
        pub enum $name { $(#[serde(rename=$wire)] $variant),+ }
        impl<'de> Deserialize<'de> for $name {
            fn deserialize<D: serde::Deserializer<'de>>(d: D) -> Result<Self,D::Error> {
                let value=String::deserialize(d)?;
                match value.as_str() { $($wire => Ok(Self::$variant),)+
                    _ => Err(serde::de::Error::unknown_variant(&value,&[$($wire),+])) }
            }
        }
    };
}
closed_enum!(Variant { Preferred => "preferred", Icon => "icon" });
closed_enum!(Class { LgSystem => "lg_system", EncryptedApps => "encrypted_apps",
    DeveloperApps => "developer_apps", SystemApps => "system_apps", OptionalContent => "optional_content" });
closed_enum!(Kind { Icon => "icon", LargeIcon => "large_icon" });
closed_enum!(ReadOp { ReadArtwork => "read_artwork" });
closed_enum!(HeaderOp { Artwork => "artwork" });
closed_enum!(CompleteOp { ArtworkComplete => "artwork_complete" });
closed_enum!(RefusalOp { ArtworkRefused => "artwork_refused" });
closed_enum!(FinishOp { FinishArtwork => "finish_artwork" });
closed_enum!(FinishedOp { ArtworkFinished => "artwork_finished" });
closed_enum!(RefusalReason { Unavailable => "unavailable", NoSource => "no_source",
    Denied => "denied", Missing => "missing", Invalid => "invalid", TooLarge => "too_large",
    Changed => "changed", Context => "context", Expired => "expired" });

impl From<RootClass> for Class {
    fn from(value: RootClass) -> Self {
        match value {
            RootClass::LgSystem => Self::LgSystem,
            RootClass::EncryptedApps => Self::EncryptedApps,
            RootClass::DeveloperApps => Self::DeveloperApps,
            RootClass::SystemApps => Self::SystemApps,
            RootClass::OptionalContent => Self::OptionalContent,
        }
    }
}
impl From<Class> for RootClass {
    fn from(value: Class) -> Self {
        match value {
            Class::LgSystem => Self::LgSystem,
            Class::EncryptedApps => Self::EncryptedApps,
            Class::DeveloperApps => Self::DeveloperApps,
            Class::SystemApps => Self::SystemApps,
            Class::OptionalContent => Self::OptionalContent,
        }
    }
}
impl From<IconKind> for Kind {
    fn from(value: IconKind) -> Self {
        match value {
            IconKind::Icon => Self::Icon,
            IconKind::LargeIcon => Self::LargeIcon,
        }
    }
}
impl From<Kind> for IconKind {
    fn from(value: Kind) -> Self {
        match value {
            Kind::Icon => Self::Icon,
            Kind::LargeIcon => Self::LargeIcon,
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Request {
    pub schema: u32,
    pub op: ReadOp,
    pub request_id: u32,
    pub nonce: String,
    pub app_id: String,
    pub variant: Variant,
}
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct FinishRequest {
    pub schema: u32,
    pub op: FinishOp,
    pub request_id: u32,
    pub nonce: String,
}
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SourceFileIdentity {
    pub device: u64,
    pub inode: u64,
    pub bytes: u64,
    pub mode: u32,
    pub uid: u32,
    pub gid: u32,
    pub modified_seconds: i64,
    pub modified_nanos: i64,
    pub changed_seconds: i64,
    pub changed_nanos: i64,
}
impl From<&FileIdentity> for SourceFileIdentity {
    fn from(value: &FileIdentity) -> Self {
        Self {
            device: value.device,
            inode: value.inode,
            bytes: value.bytes,
            mode: value.mode,
            uid: value.uid,
            gid: value.gid,
            modified_seconds: value.modified_seconds,
            modified_nanos: value.modified_nanos,
            changed_seconds: value.changed_seconds,
            changed_nanos: value.changed_nanos,
        }
    }
}
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Header {
    pub schema: u32,
    pub op: HeaderOp,
    pub request_id: u32,
    pub identity: Identity,
    pub app_id: String,
    pub version: String,
    pub class: Class,
    pub kind: Kind,
    pub file_identity: SourceFileIdentity,
    pub encoded_bytes: u32,
    pub encoded_sha256: String,
    pub chunk_count: u32,
}
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Complete {
    pub schema: u32,
    pub op: CompleteOp,
    pub request_id: u32,
    pub identity: Identity,
    pub app_id: String,
    pub encoded_bytes: u32,
    pub encoded_sha256: String,
    pub header_sha256: String,
}
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Refusal {
    pub schema: u32,
    pub op: RefusalOp,
    pub request_id: u32,
    pub identity: Identity,
    pub app_id: String,
    pub reason: RefusalReason,
}
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Finished {
    pub schema: u32,
    pub op: FinishedOp,
    pub request_id: u32,
    pub identity: Identity,
}
pub struct EncodedArtwork {
    pub header: Header,
    pub bytes: Vec<u8>,
    pub complete: Complete,
}
pub enum Outcome {
    Artwork(Box<EncodedArtwork>),
    Refused(Refusal),
}
impl fmt::Debug for EncodedArtwork {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("EncodedArtwork")
            .field("header", &self.header)
            .field("bytes", &self.bytes.len())
            .field("complete", &self.complete)
            .finish()
    }
}
impl fmt::Debug for Outcome {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Artwork(value) => value.fmt(f),
            Self::Refused(value) => value.fmt(f),
        }
    }
}

fn check(ok: bool, error: Error) -> Result<(), Error> {
    if ok {
        Ok(())
    } else {
        Err(error)
    }
}
fn hex(value: &str, len: usize) -> bool {
    value.len() == len
        && value
            .bytes()
            .all(|b| b.is_ascii_digit() || (b'a'..=b'f').contains(&b))
}
fn app(value: &str) -> bool {
    crate::model::valid_app_id(value) && value != "." && value != ".."
}
fn text(value: &str, limit: usize) -> bool {
    !value.is_empty() && value.len() <= limit && !value.chars().any(char::is_control)
}
fn identity(value: &Identity, profile: Profile) -> Result<(), Error> {
    value
        .validate_for_profile(profile)
        .map_err(|_| Error::Identity)?;
    check(value.client_start_ticks > 0, Error::Identity)
}
fn hash(bytes: &[u8]) -> String {
    plx_base::sha256::sha256(bytes)
        .iter()
        .map(|b| format!("{b:02x}"))
        .collect()
}
fn json<T: Serialize>(value: &T) -> Result<Vec<u8>, Error> {
    let raw = serde_json::to_vec(value).map_err(|_| Error::Message)?;
    check(!raw.is_empty() && raw.len() <= MESSAGE_LIMIT, Error::Bounds)?;
    Ok(raw)
}
fn parse<T: serde::de::DeserializeOwned>(raw: &[u8]) -> Result<T, Error> {
    check(!raw.is_empty() && raw.len() <= MESSAGE_LIMIT, Error::Bounds)?;
    serde_json::from_slice(raw).map_err(|_| Error::Message)
}
impl Request {
    pub fn new(
        request_id: u32,
        nonce: &str,
        app_id: &str,
        variant: Variant,
    ) -> Result<Self, Error> {
        let value = Self {
            schema: 1,
            op: ReadOp::ReadArtwork,
            request_id,
            nonce: nonce.into(),
            app_id: app_id.into(),
            variant,
        };
        value.validate()?;
        Ok(value)
    }
    pub fn validate(&self) -> Result<(), Error> {
        check(
            self.schema == 1
                && (1..=5).contains(&self.request_id)
                && hex(&self.nonce, 32)
                && app(&self.app_id),
            Error::Request,
        )
    }
}
impl FinishRequest {
    pub fn new(request_id: u32, nonce: &str) -> Result<Self, Error> {
        let value = Self {
            schema: 1,
            op: FinishOp::FinishArtwork,
            request_id,
            nonce: nonce.into(),
        };
        value.validate()?;
        Ok(value)
    }
    pub fn validate(&self) -> Result<(), Error> {
        check(
            self.schema == 1 && (1..=6).contains(&self.request_id) && hex(&self.nonce, 32),
            Error::Request,
        )
    }
}
impl SourceFileIdentity {
    pub fn validate(&self) -> Result<(), Error> {
        check(
            self.inode > 0
                && self.bytes > 0
                && self.bytes <= ENCODED_LIMIT as u64
                && self.mode & 0o170000 == 0o100000
                && (0..1_000_000_000).contains(&self.modified_nanos)
                && (0..1_000_000_000).contains(&self.changed_nanos),
            Error::Bounds,
        )
    }
}
impl Header {
    pub fn from_file(
        request: &Request,
        session: &Identity,
        source: &SourceKey,
        file: &EncodedFile,
    ) -> Result<Self, Error> {
        Self::from_file_for_profile(request, session, source, file, Profile::Proof)
    }
    pub fn from_file_for_profile(
        request: &Request,
        session: &Identity,
        source: &SourceKey,
        file: &EncodedFile,
        profile: Profile,
    ) -> Result<Self, Error> {
        request.validate()?;
        identity(session, profile)?;
        check(file.bytes.len() <= ENCODED_LIMIT, Error::Bounds)?;
        let value = Self {
            schema: 1,
            op: HeaderOp::Artwork,
            request_id: request.request_id,
            identity: session.clone(),
            app_id: source.app_id.clone(),
            version: source.version.clone(),
            class: source.class.into(),
            kind: source.kind.into(),
            file_identity: (&file.identity).into(),
            encoded_bytes: file.bytes.len() as u32,
            encoded_sha256: hash(&file.bytes),
            chunk_count: file.bytes.len().div_ceil(CHUNK_BYTES) as u32,
        };
        value.validate_for_in_profile(request, session, profile)?;
        Ok(value)
    }
    pub fn validate(&self) -> Result<(), Error> {
        self.validate_for_profile(Profile::Proof)
    }
    pub fn validate_for_profile(&self, profile: Profile) -> Result<(), Error> {
        identity(&self.identity, profile)?;
        self.file_identity.validate()?;
        check(
            self.schema == 1
                && (1..=5).contains(&self.request_id)
                && app(&self.app_id)
                && text(&self.version, 128)
                && self.encoded_bytes > 0
                && self.encoded_bytes as usize <= ENCODED_LIMIT
                && self.file_identity.bytes == u64::from(self.encoded_bytes)
                && hex(&self.encoded_sha256, 64)
                && self.chunk_count == (self.encoded_bytes as usize).div_ceil(CHUNK_BYTES) as u32,
            Error::Bounds,
        )
    }
    pub fn validate_for(&self, request: &Request, session: &Identity) -> Result<(), Error> {
        self.validate_for_in_profile(request, session, Profile::Proof)
    }
    pub fn validate_for_in_profile(
        &self,
        request: &Request,
        session: &Identity,
        profile: Profile,
    ) -> Result<(), Error> {
        self.validate_for_profile(profile)?;
        request.validate()?;
        check(
            self.identity == *session && self.identity.nonce == request.nonce,
            Error::Identity,
        )?;
        check(
            self.request_id == request.request_id
                && self.app_id == request.app_id
                && (request.variant != Variant::Icon || self.kind == Kind::Icon),
            Error::Request,
        )
    }
}
impl Complete {
    /// Construct only after the server has repeated its context, peer,
    /// enrollment and selected-source checks. Construction itself does no I/O.
    pub fn from_header(header: &Header) -> Result<Self, Error> {
        Self::from_header_for_profile(header, Profile::Proof)
    }
    pub fn from_header_for_profile(header: &Header, profile: Profile) -> Result<Self, Error> {
        header.validate_for_profile(profile)?;
        Ok(Self {
            schema: 1,
            op: CompleteOp::ArtworkComplete,
            request_id: header.request_id,
            identity: header.identity.clone(),
            app_id: header.app_id.clone(),
            encoded_bytes: header.encoded_bytes,
            encoded_sha256: header.encoded_sha256.clone(),
            header_sha256: hash(&json(header)?),
        })
    }
    pub fn validate(&self) -> Result<(), Error> {
        self.validate_for_profile(Profile::Proof)
    }
    pub fn validate_for_profile(&self, profile: Profile) -> Result<(), Error> {
        identity(&self.identity, profile)?;
        check(
            self.schema == 1
                && (1..=5).contains(&self.request_id)
                && app(&self.app_id)
                && self.encoded_bytes > 0
                && self.encoded_bytes as usize <= ENCODED_LIMIT
                && hex(&self.encoded_sha256, 64)
                && hex(&self.header_sha256, 64),
            Error::Bounds,
        )
    }
    pub fn validate_for(&self, header: &Header) -> Result<(), Error> {
        self.validate_for_in_profile(header, Profile::Proof)
    }
    pub fn validate_for_in_profile(&self, header: &Header, profile: Profile) -> Result<(), Error> {
        self.validate_for_profile(profile)?;
        check(
            *self == Self::from_header_for_profile(header, profile)?,
            Error::Digest,
        )
    }
}
impl Refusal {
    pub fn new(
        request: &Request,
        session: &Identity,
        reason: RefusalReason,
    ) -> Result<Self, Error> {
        Self::new_for_profile(request, session, reason, Profile::Proof)
    }
    pub fn new_for_profile(
        request: &Request,
        session: &Identity,
        reason: RefusalReason,
        profile: Profile,
    ) -> Result<Self, Error> {
        request.validate()?;
        identity(session, profile)?;
        check(session.nonce == request.nonce, Error::Identity)?;
        Ok(Self {
            schema: 1,
            op: RefusalOp::ArtworkRefused,
            request_id: request.request_id,
            identity: session.clone(),
            app_id: request.app_id.clone(),
            reason,
        })
    }
    pub fn validate(&self) -> Result<(), Error> {
        self.validate_for_profile(Profile::Proof)
    }
    pub fn validate_for_profile(&self, profile: Profile) -> Result<(), Error> {
        identity(&self.identity, profile)?;
        check(
            self.schema == 1 && (1..=5).contains(&self.request_id) && app(&self.app_id),
            Error::Request,
        )
    }
    pub fn validate_for(&self, request: &Request, session: &Identity) -> Result<(), Error> {
        self.validate_for_in_profile(request, session, Profile::Proof)
    }
    pub fn validate_for_in_profile(
        &self,
        request: &Request,
        session: &Identity,
        profile: Profile,
    ) -> Result<(), Error> {
        self.validate_for_profile(profile)?;
        check(
            self.identity == *session && self.identity.nonce == request.nonce,
            Error::Identity,
        )?;
        check(
            self.request_id == request.request_id && self.app_id == request.app_id,
            Error::Request,
        )
    }
}
impl Finished {
    pub fn new(request: &FinishRequest, session: &Identity) -> Result<Self, Error> {
        Self::new_for_profile(request, session, Profile::Proof)
    }
    pub fn new_for_profile(
        request: &FinishRequest,
        session: &Identity,
        profile: Profile,
    ) -> Result<Self, Error> {
        request.validate()?;
        identity(session, profile)?;
        check(session.nonce == request.nonce, Error::Identity)?;
        Ok(Self {
            schema: 1,
            op: FinishedOp::ArtworkFinished,
            request_id: request.request_id,
            identity: session.clone(),
        })
    }
    pub fn validate(&self) -> Result<(), Error> {
        self.validate_for_profile(Profile::Proof)
    }
    pub fn validate_for_profile(&self, profile: Profile) -> Result<(), Error> {
        identity(&self.identity, profile)?;
        check(
            self.schema == 1 && (1..=6).contains(&self.request_id),
            Error::Request,
        )
    }
    pub fn validate_for(&self, request: &FinishRequest, session: &Identity) -> Result<(), Error> {
        self.validate_for_in_profile(request, session, Profile::Proof)
    }
    pub fn validate_for_in_profile(
        &self,
        request: &FinishRequest,
        session: &Identity,
        profile: Profile,
    ) -> Result<(), Error> {
        self.validate_for_profile(profile)?;
        check(
            *self == Self::new_for_profile(request, session, profile)?,
            Error::Identity,
        )
    }
}

macro_rules! parser {
    ($function:ident,$type:ty) => {
        pub fn $function(raw: &[u8]) -> Result<$type, Error> {
            let value: $type = parse(raw)?;
            value.validate()?;
            Ok(value)
        }
    };
}
parser!(parse_request, Request);
parser!(parse_finish_request, FinishRequest);
// A role is selected by the fixed installed entry/own application, never by
// packet data. These codecs validate structure; they confer no Root authority.
macro_rules! profile_parser {
    ($function:ident,$profiled:ident,$type:ty) => {
        pub fn $function(raw: &[u8]) -> Result<$type, Error> {
            $profiled(raw, Profile::Proof)
        }
        pub fn $profiled(raw: &[u8], profile: Profile) -> Result<$type, Error> {
            let value: $type = parse(raw)?;
            value.validate_for_profile(profile)?;
            Ok(value)
        }
    };
}
profile_parser!(parse_header, parse_header_for_profile, Header);
profile_parser!(parse_complete, parse_complete_for_profile, Complete);
profile_parser!(parse_refusal, parse_refusal_for_profile, Refusal);
profile_parser!(parse_finished, parse_finished_for_profile, Finished);

pub fn send_request(conn: &Connection, request: &Request, deadline: Instant) -> Result<(), Error> {
    request.validate()?;
    conn.send_until(&json(request)?, deadline)?;
    Ok(())
}
pub fn send_finish_request(
    conn: &Connection,
    request: &FinishRequest,
    deadline: Instant,
) -> Result<(), Error> {
    request.validate()?;
    conn.send_until(&json(request)?, deadline)?;
    Ok(())
}
pub fn send_complete(
    conn: &Connection,
    complete: &Complete,
    deadline: Instant,
) -> Result<(), Error> {
    send_complete_for_profile(conn, complete, deadline, Profile::Proof)
}
pub fn send_complete_for_profile(
    conn: &Connection,
    complete: &Complete,
    deadline: Instant,
    profile: Profile,
) -> Result<(), Error> {
    complete.validate_for_profile(profile)?;
    conn.send_until(&json(complete)?, deadline)?;
    Ok(())
}
pub fn send_refusal(conn: &Connection, refusal: &Refusal, deadline: Instant) -> Result<(), Error> {
    send_refusal_for_profile(conn, refusal, deadline, Profile::Proof)
}
pub fn send_refusal_for_profile(
    conn: &Connection,
    refusal: &Refusal,
    deadline: Instant,
    profile: Profile,
) -> Result<(), Error> {
    refusal.validate_for_profile(profile)?;
    conn.send_until(&json(refusal)?, deadline)?;
    Ok(())
}
pub fn send_finished(
    conn: &Connection,
    finished: &Finished,
    deadline: Instant,
) -> Result<(), Error> {
    send_finished_for_profile(conn, finished, deadline, Profile::Proof)
}
pub fn send_finished_for_profile(
    conn: &Connection,
    finished: &Finished,
    deadline: Instant,
    profile: Profile,
) -> Result<(), Error> {
    finished.validate_for_profile(profile)?;
    conn.send_until(&json(finished)?, deadline)?;
    Ok(())
}

pub struct Chunk<'a> {
    pub request_id: u32,
    pub index: u32,
    pub payload: &'a [u8],
}
pub fn encode_chunk(request_id: u32, index: u32, payload: &[u8]) -> Result<Vec<u8>, Error> {
    check(
        (1..=5).contains(&request_id)
            && !payload.is_empty()
            && payload.len() <= CHUNK_BYTES
            && index < (ENCODED_LIMIT.div_ceil(CHUNK_BYTES) as u32),
        Error::Bounds,
    )?;
    let mut raw = Vec::with_capacity(CHUNK_HEADER + payload.len());
    raw.extend_from_slice(MAGIC);
    raw.extend_from_slice(&request_id.to_le_bytes());
    raw.extend_from_slice(&index.to_le_bytes());
    raw.extend_from_slice(&(payload.len() as u16).to_le_bytes());
    raw.extend_from_slice(payload);
    Ok(raw)
}
pub fn decode_chunk(raw: &[u8]) -> Result<Chunk<'_>, Error> {
    check(
        raw.len() >= CHUNK_HEADER && raw.len() <= MESSAGE_LIMIT && &raw[..4] == MAGIC,
        Error::Message,
    )?;
    let request_id = u32::from_le_bytes(raw[4..8].try_into().map_err(|_| Error::Message)?);
    let index = u32::from_le_bytes(raw[8..12].try_into().map_err(|_| Error::Message)?);
    let length = u16::from_le_bytes(raw[12..14].try_into().map_err(|_| Error::Message)?) as usize;
    check(
        (1..=5).contains(&request_id)
            && length > 0
            && length <= CHUNK_BYTES
            && index < ENCODED_LIMIT.div_ceil(CHUNK_BYTES) as u32
            && raw.len() == CHUNK_HEADER + length,
        Error::Bounds,
    )?;
    Ok(Chunk {
        request_id,
        index,
        payload: &raw[CHUNK_HEADER..],
    })
}

/// Sends only the header and encoded data. The caller MUST revalidate its
/// peer/context/enrollment/source before separately sending Complete.
pub fn send_header_chunks(
    conn: &Connection,
    header: &Header,
    bytes: &[u8],
    deadline: Instant,
) -> Result<(), Error> {
    send_header_chunks_for_profile(conn, header, bytes, deadline, Profile::Proof)
}
pub fn send_header_chunks_for_profile(
    conn: &Connection,
    header: &Header,
    bytes: &[u8],
    deadline: Instant,
    profile: Profile,
) -> Result<(), Error> {
    header.validate_for_profile(profile)?;
    check(
        bytes.len() == header.encoded_bytes as usize && hash(bytes) == header.encoded_sha256,
        Error::Digest,
    )?;
    conn.send_until(&json(header)?, deadline)?;
    for (index, payload) in bytes.chunks(CHUNK_BYTES).enumerate() {
        conn.send_until(
            &encode_chunk(header.request_id, index as u32, payload)?,
            deadline,
        )?;
    }
    Ok(())
}

fn expected_peer(session: &Identity, server: Credentials, profile: Profile) -> Result<(), Error> {
    identity(session, profile)?;
    check(
        server.uid == 0
            && server.pid > 1
            && session.server_uid == server.uid
            && session.server_pid == server.pid as u32,
        Error::Credentials,
    )
}
fn connection_peer(
    conn: &Connection,
    session: &Identity,
    server: Credentials,
    profile: Profile,
) -> Result<(), Error> {
    expected_peer(session, server, profile)?;
    check(conn.peer_credentials()? == server, Error::Credentials)
}

/// Metadata from a consumed body, without image bytes or closure authority.
#[cfg(any(target_arch = "arm", test))]
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub(crate) struct DiscardedRead {
    header: Header,
    validated_bytes: u32,
    validated_chunks: u32,
    encoded_sha256: String,
}
#[cfg(any(target_arch = "arm", test))]
impl DiscardedRead {
    pub(crate) fn header(&self) -> &Header {
        &self.header
    }
    pub(crate) fn validated_bytes(&self) -> u32 {
        self.validated_bytes
    }
    pub(crate) fn validated_chunks(&self) -> u32 {
        self.validated_chunks
    }
    pub(crate) fn encoded_sha256(&self) -> &str {
        &self.encoded_sha256
    }
}

/// Pure transaction assembler. Any error poisons later byte retrieval; only a
/// validated Complete exports encoded bytes.
pub struct Receiver {
    profile: Profile,
    request: Request,
    identity: Identity,
    server: Credentials,
    header: Option<Header>,
    bytes: Vec<u8>,
    digest: plx_base::sha256::Sha256,
    next_chunk: u32,
    terminal: Option<Outcome>,
    failed: bool,
}
impl Receiver {
    pub fn new(request: &Request, session: &Identity, server: Credentials) -> Result<Self, Error> {
        Self::new_for_profile(request, session, server, Profile::Proof)
    }
    pub fn new_for_profile(
        request: &Request,
        session: &Identity,
        server: Credentials,
        profile: Profile,
    ) -> Result<Self, Error> {
        request.validate()?;
        expected_peer(session, server, profile)?;
        check(request.nonce == session.nonce, Error::Identity)?;
        Ok(Self {
            profile,
            request: request.clone(),
            identity: session.clone(),
            server,
            header: None,
            bytes: Vec::new(),
            digest: plx_base::sha256::Sha256::new(),
            next_chunk: 0,
            terminal: None,
            failed: false,
        })
    }
    pub fn push_packet(&mut self, raw: &[u8], credentials: Credentials) -> Result<bool, Error> {
        if self.failed {
            return Err(Error::Incomplete);
        }
        let result = self.accept(raw, credentials);
        if result.is_err() {
            self.failed = true;
            self.bytes.clear();
            self.header = None;
            self.terminal = None;
        }
        result
    }
    fn accept(&mut self, raw: &[u8], credentials: Credentials) -> Result<bool, Error> {
        check(credentials == self.server, Error::Credentials)?;
        check(!raw.is_empty() && raw.len() <= MESSAGE_LIMIT, Error::Bounds)?;
        if self.terminal.is_some() {
            return Err(Error::Extra);
        }
        let Some(header) = self.header.as_ref() else {
            if let Ok(header) = parse_header_for_profile(raw, self.profile) {
                header.validate_for_in_profile(&self.request, &self.identity, self.profile)?;
                self.bytes
                    .try_reserve_exact(header.encoded_bytes as usize)
                    .map_err(|_| Error::Allocation)?;
                self.header = Some(header);
                return Ok(false);
            }
            let refusal = parse_refusal_for_profile(raw, self.profile)?;
            refusal.validate_for_in_profile(&self.request, &self.identity, self.profile)?;
            self.terminal = Some(Outcome::Refused(refusal));
            return Ok(true);
        };
        if self.next_chunk < header.chunk_count {
            let chunk = decode_chunk(raw)?;
            check(
                chunk.request_id == self.request.request_id && chunk.index == self.next_chunk,
                Error::Sequence,
            )?;
            let left = header.encoded_bytes as usize - self.bytes.len();
            check(
                chunk.payload.len() == left.min(CHUNK_BYTES),
                Error::Sequence,
            )?;
            self.digest.update(chunk.payload);
            self.bytes.extend_from_slice(chunk.payload);
            self.next_chunk += 1;
            return Ok(false);
        }
        // A late source/context refusal is a terminal that discards all bytes.
        if let Ok(refusal) = parse_refusal_for_profile(raw, self.profile) {
            refusal.validate_for_in_profile(&self.request, &self.identity, self.profile)?;
            self.bytes.clear();
            self.terminal = Some(Outcome::Refused(refusal));
            return Ok(true);
        }
        let complete = parse_complete_for_profile(raw, self.profile)?;
        complete.validate_for_in_profile(header, self.profile)?;
        check(
            self.bytes.len() == header.encoded_bytes as usize,
            Error::Incomplete,
        )?;
        let digest = self
            .digest
            .clone()
            .finish()
            .iter()
            .map(|b| format!("{b:02x}"))
            .collect::<String>();
        check(digest == header.encoded_sha256, Error::Digest)?;
        let header = self.header.take().ok_or(Error::Incomplete)?;
        self.terminal = Some(Outcome::Artwork(Box::new(EncodedArtwork {
            header,
            bytes: std::mem::take(&mut self.bytes),
            complete,
        })));
        Ok(true)
    }
    /// Consumes and frees a complete body that has no terminal packet. The caller
    /// must establish actual same-connection EOF separately. This metadata is
    /// neither Complete nor session-closure/settlement authority.
    #[cfg(any(target_arch = "arm", test))]
    pub(crate) fn discard_incomplete_after_eof(mut self) -> Result<DiscardedRead, Error> {
        check(!self.failed && self.terminal.is_none(), Error::Incomplete)?;
        let header = self.header.take().ok_or(Error::Incomplete)?;
        check(
            self.next_chunk == header.chunk_count
                && self.bytes.len() == header.encoded_bytes as usize,
            Error::Incomplete,
        )?;
        let digest = self
            .digest
            .clone()
            .finish()
            .iter()
            .map(|b| format!("{b:02x}"))
            .collect::<String>();
        check(digest == header.encoded_sha256, Error::Digest)?;
        let validated_bytes = u32::try_from(self.bytes.len()).map_err(|_| Error::Bounds)?;
        drop(std::mem::take(&mut self.bytes));
        Ok(DiscardedRead {
            header,
            validated_bytes,
            validated_chunks: self.next_chunk,
            encoded_sha256: digest,
        })
    }
    pub fn finish(self) -> Result<Outcome, Error> {
        if self.failed {
            return Err(Error::Incomplete);
        }
        self.terminal.ok_or(Error::Incomplete)
    }
}

/// Consumes one transaction delimiter, not EOF. A request ID prevents stale
/// packets being accepted as a subsequent transaction on the same connection.
/// The caller still performs ordinary-context/source checks before using data.
pub fn receive(
    conn: &Connection,
    request: &Request,
    session: &Identity,
    server: Credentials,
    deadline: Instant,
) -> Result<Outcome, Error> {
    receive_for_profile(conn, request, session, server, deadline, Profile::Proof)
}
pub fn receive_for_profile(
    conn: &Connection,
    request: &Request,
    session: &Identity,
    server: Credentials,
    deadline: Instant,
    profile: Profile,
) -> Result<Outcome, Error> {
    connection_peer(conn, session, server, profile)?;
    let mut receiver = Receiver::new_for_profile(request, session, server, profile)?;
    loop {
        let packet = conn.recv_until(MESSAGE_LIMIT, deadline)?;
        if receiver.push_packet(&packet.bytes, packet.credentials)? {
            connection_peer(conn, session, server, profile)?;
            return receiver.finish();
        }
    }
}

/// A session is accepted as closed only after the matching Finished packet and
/// clean EOF. The server must close after sending Finished; no stream fallback.
pub fn receive_finished(
    conn: &Connection,
    request: &FinishRequest,
    session: &Identity,
    server: Credentials,
    deadline: Instant,
) -> Result<Finished, Error> {
    receive_finished_for_profile(conn, request, session, server, deadline, Profile::Proof)
}
pub fn receive_finished_for_profile(
    conn: &Connection,
    request: &FinishRequest,
    session: &Identity,
    server: Credentials,
    deadline: Instant,
    profile: Profile,
) -> Result<Finished, Error> {
    request.validate()?;
    connection_peer(conn, session, server, profile)?;
    let packet = conn.recv_until(MESSAGE_LIMIT, deadline)?;
    let finished = checked_finished_packet(&packet, request, session, server, profile)?;
    conn.expect_eof_until(deadline)?;
    connection_peer(conn, session, server, profile)?;
    Ok(finished)
}

fn checked_finished_packet(
    packet: &crate::broker_transport::Packet,
    request: &FinishRequest,
    session: &Identity,
    server: Credentials,
    profile: Profile,
) -> Result<Finished, Error> {
    check(packet.credentials == server, Error::Credentials)?;
    let finished = parse_finished_for_profile(&packet.bytes, profile)?;
    finished.validate_for_in_profile(request, session, profile)?;
    Ok(finished)
}

#[cfg(any(target_arch = "arm", test))]
#[derive(Debug, PartialEq, Eq)]
pub(crate) enum FinishedOrEof {
    Finished(Finished),
    Eof,
}

#[cfg(any(target_arch = "arm", test))]
fn classify_finished_incoming(
    incoming: crate::broker_transport::Incoming,
    request: &FinishRequest,
    session: &Identity,
    server: Credentials,
    profile: Profile,
) -> Result<FinishedOrEof, Error> {
    request.validate()?;
    expected_peer(session, server, profile)?;
    check(request.nonce == session.nonce, Error::Identity)?;
    match incoming {
        crate::broker_transport::Incoming::Eof => Ok(FinishedOrEof::Eof),
        crate::broker_transport::Incoming::Packet(packet) => {
            checked_finished_packet(&packet, request, session, server, profile)
                .map(FinishedOrEof::Finished)
        }
    }
}

/// Distinguishes an actual same-receive EOF from a strict Finished exchange.
/// EOF has no process, child, HBC or pending-record authority.
#[cfg(target_arch = "arm")]
pub(crate) fn receive_finished_or_eof_for_profile(
    conn: &Connection,
    request: &FinishRequest,
    session: &Identity,
    server: Credentials,
    deadline: Instant,
    profile: Profile,
) -> Result<FinishedOrEof, Error> {
    request.validate()?;
    check(request.nonce == session.nonce, Error::Identity)?;
    connection_peer(conn, session, server, profile)?;
    let incoming = conn.recv_or_eof_until(MESSAGE_LIMIT, deadline)?;
    let outcome = classify_finished_incoming(incoming, request, session, server, profile)?;
    if matches!(&outcome, FinishedOrEof::Finished(_)) {
        conn.expect_eof_until(deadline)?;
    }
    connection_peer(conn, session, server, profile)?;
    if Instant::now() >= deadline {
        return Err(io::Error::from(io::ErrorKind::TimedOut).into());
    }
    Ok(outcome)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::broker_protocol::Operation;
    use serde_json::{json as value, Value};
    fn fixture(
        length: usize,
        id: u32,
    ) -> (Request, Identity, Credentials, Header, Vec<u8>, Complete) {
        let request =
            Request::new(id, &"a".repeat(32), "com.synthetic.app", Variant::Icon).unwrap();
        let session = Identity {
            schema: 1,
            op: Operation::Identify,
            nonce: request.nonce.clone(),
            server_pid: 1234,
            server_uid: 0,
            client_pid: 2345,
            client_uid: 6583,
            client_start_ticks: 12345,
            ui_sha256: "b".repeat(64),
            broker_sha256: "c".repeat(64),
        };
        let server = Credentials {
            pid: 1234,
            uid: 0,
            gid: 0,
        };
        let bytes = (0..length).map(|i| (i % 251) as u8).collect::<Vec<_>>();
        let file = EncodedFile {
            identity: FileIdentity {
                device: 1,
                inode: 2,
                bytes: length as u64,
                mode: 0o100644,
                uid: 0,
                gid: 0,
                modified_seconds: 1,
                modified_nanos: 0,
                changed_seconds: 1,
                changed_nanos: 0,
            },
            digest: plx_base::sha256::sha256(&bytes),
            bytes: bytes.clone(),
            png_dimensions: None,
        };
        let source = SourceKey {
            app_id: request.app_id.clone(),
            version: "1.0".into(),
            class: RootClass::DeveloperApps,
            folder: "/synthetic/not-read".into(),
            relative: vec!["icon.png".into()],
            kind: IconKind::Icon,
        };
        let header = Header::from_file(&request, &session, &source, &file).unwrap();
        let complete = Complete::from_header(&header).unwrap();
        (request, session, server, header, bytes, complete)
    }
    fn receiver_with_body(length: usize) -> (Receiver, Header, Vec<u8>, Complete, Credentials) {
        let (request, session, server, header, bytes, complete) = fixture(length, 1);
        let mut receiver = Receiver::new(&request, &session, server).unwrap();
        assert!(!receiver
            .push_packet(&json(&header).unwrap(), server)
            .unwrap());
        for (index, payload) in bytes.chunks(CHUNK_BYTES).enumerate() {
            assert!(!receiver
                .push_packet(&encode_chunk(1, index as u32, payload).unwrap(), server)
                .unwrap());
        }
        (receiver, header, bytes, complete, server)
    }
    fn preview_fixture(
        length: usize,
    ) -> (Request, Identity, Credentials, Header, Vec<u8>, Complete) {
        let (request, mut session, server, mut header, bytes, _) = fixture(length, 1);
        session.client_uid = 5724;
        header.identity = session.clone();
        let complete = Complete::from_header_for_profile(&header, Profile::Preview).unwrap();
        (request, session, server, header, bytes, complete)
    }
    #[test]
    fn explicit_preview_profile_transfers_actual_chunks_and_preserves_proof_rejection() {
        let (request, session, server, header, bytes, complete) = preview_fixture(CHUNK_BYTES + 7);
        assert!(Receiver::new(&request, &session, server).is_err());
        assert!(parse_header(&json(&header).unwrap()).is_err());
        assert!(parse_complete(&json(&complete).unwrap()).is_err());
        let mut receiver =
            Receiver::new_for_profile(&request, &session, server, Profile::Preview).unwrap();
        assert!(!receiver
            .push_packet(&json(&header).unwrap(), server)
            .unwrap());
        for (index, payload) in bytes.chunks(CHUNK_BYTES).enumerate() {
            assert!(!receiver
                .push_packet(&encode_chunk(1, index as u32, payload).unwrap(), server)
                .unwrap());
        }
        assert!(receiver
            .push_packet(&json(&complete).unwrap(), server)
            .unwrap());
        let Outcome::Artwork(artwork) = receiver.finish().unwrap() else {
            panic!("Expected completed Preview transfer");
        };
        assert_eq!(artwork.bytes, bytes);
        assert_eq!(artwork.header.identity, session);
        assert_eq!(artwork.complete, complete);
        assert_eq!(
            parse_header_for_profile(&json(&header).unwrap(), Profile::Preview).unwrap(),
            header
        );
    }
    #[test]
    fn preview_header_constructor_is_profile_bound_without_wire_role_authority() {
        let (request, session, _, header, bytes, _) = preview_fixture(7);
        let m = &header.file_identity;
        let file = EncodedFile {
            identity: FileIdentity {
                device: m.device,
                inode: m.inode,
                bytes: m.bytes,
                mode: m.mode,
                uid: m.uid,
                gid: m.gid,
                modified_seconds: m.modified_seconds,
                modified_nanos: m.modified_nanos,
                changed_seconds: m.changed_seconds,
                changed_nanos: m.changed_nanos,
            },
            digest: plx_base::sha256::sha256(&bytes),
            bytes,
            png_dimensions: None,
        };
        let source = SourceKey {
            app_id: request.app_id.clone(),
            version: header.version.clone(),
            class: RootClass::DeveloperApps,
            folder: "/synthetic/not-read".into(),
            relative: vec!["icon.png".into()],
            kind: IconKind::Icon,
        };
        assert!(Header::from_file(&request, &session, &source, &file).is_err());
        assert_eq!(
            Header::from_file_for_profile(&request, &session, &source, &file, Profile::Preview)
                .unwrap(),
            header
        );
        assert!(Complete::from_header(&header).is_err());
        let mut raw = serde_json::to_value(&header).unwrap();
        assert!(raw.get("profile").is_none());
        raw["profile"] = value!("preview");
        assert!(
            parse_header_for_profile(&serde_json::to_vec(&raw).unwrap(), Profile::Preview).is_err()
        );
        assert!(parse_header(&serde_json::to_vec(&raw).unwrap()).is_err());
    }
    #[test]
    fn preview_refusal_discards_body_and_finished_keeps_strict_identity() {
        let (request, session, server, header, bytes, _) = preview_fixture(7);
        assert!(Refusal::new(&request, &session, RefusalReason::Changed).is_err());
        let refusal =
            Refusal::new_for_profile(&request, &session, RefusalReason::Changed, Profile::Preview)
                .unwrap();
        assert!(parse_refusal(&json(&refusal).unwrap()).is_err());
        let mut receiver =
            Receiver::new_for_profile(&request, &session, server, Profile::Preview).unwrap();
        receiver
            .push_packet(&json(&header).unwrap(), server)
            .unwrap();
        receiver
            .push_packet(&encode_chunk(1, 0, &bytes).unwrap(), server)
            .unwrap();
        assert!(receiver
            .push_packet(&json(&refusal).unwrap(), server)
            .unwrap());
        assert!(matches!(receiver.finish().unwrap(), Outcome::Refused(_)));
        let finish = FinishRequest::new(2, &session.nonce).unwrap();
        assert!(Finished::new(&finish, &session).is_err());
        let finished = Finished::new_for_profile(&finish, &session, Profile::Preview).unwrap();
        assert!(parse_finished(&json(&finished).unwrap()).is_err());
        assert_eq!(
            parse_finished_for_profile(&json(&finished).unwrap(), Profile::Preview).unwrap(),
            finished
        );
        let mut wrong = session.clone();
        wrong.client_pid += 1;
        assert!(finished
            .validate_for_in_profile(&finish, &wrong, Profile::Preview)
            .is_err());
        let wrong_finish = FinishRequest::new(1, &session.nonce).unwrap();
        assert!(finished
            .validate_for_in_profile(&wrong_finish, &session, Profile::Preview)
            .is_err());
    }
    #[test]
    fn preview_profile_does_not_weaken_credentials_completion_or_poisoning() {
        let (request, session, server, header, bytes, complete) = preview_fixture(7);
        for change in 0..3 {
            let mut receiver =
                Receiver::new_for_profile(&request, &session, server, Profile::Preview).unwrap();
            receiver
                .push_packet(&json(&header).unwrap(), server)
                .unwrap();
            receiver
                .push_packet(&encode_chunk(1, 0, &bytes).unwrap(), server)
                .unwrap();
            let mut bad = complete.clone();
            match change {
                0 => bad.identity.client_uid = 6583,
                1 => bad.header_sha256 = "0".repeat(64),
                _ => bad.identity.nonce = "d".repeat(32),
            }
            assert!(receiver.push_packet(&json(&bad).unwrap(), server).is_err());
            assert!(receiver
                .push_packet(&json(&complete).unwrap(), server)
                .is_err());
            assert!(receiver.finish().is_err());
        }
        for wrong in [
            Credentials {
                pid: server.pid + 1,
                ..server
            },
            Credentials { uid: 1, ..server },
            Credentials { gid: 1, ..server },
        ] {
            let mut receiver =
                Receiver::new_for_profile(&request, &session, server, Profile::Preview).unwrap();
            assert!(receiver
                .push_packet(&json(&header).unwrap(), wrong)
                .is_err());
            assert!(receiver.finish().is_err());
        }
        let mut incomplete =
            Receiver::new_for_profile(&request, &session, server, Profile::Preview).unwrap();
        incomplete
            .push_packet(&json(&header).unwrap(), server)
            .unwrap();
        incomplete
            .push_packet(&encode_chunk(1, 0, &bytes).unwrap(), server)
            .unwrap();
        assert!(incomplete.finish().is_err());
    }
    #[test]
    fn strict_request_rejects_paths_unknown_duplicates_maps_and_coercion() {
        let (request, _, _, _, _, _) = fixture(1, 1);
        assert_eq!(parse_request(&json(&request).unwrap()).unwrap(), request);
        for field in ["path", "uri", "command", "version", "uid"] {
            let mut raw = serde_json::to_value(&request).unwrap();
            raw[field] = value!("ignored");
            assert!(parse_request(&serde_json::to_vec(&raw).unwrap()).is_err());
        }
        for (field, bad) in [
            ("schema", value!(true)),
            ("request_id", value!(0)),
            ("request_id", value!(6)),
            ("op", value!({"read_artwork":null})),
            ("variant", value!({"icon":null})),
            ("variant", value!("large_icon")),
            ("app_id", value!("../icon.png")),
            ("app_id", value!("https://example.invalid/icon")),
            ("nonce", value!("A".repeat(32))),
        ] {
            let mut raw = serde_json::to_value(&request).unwrap();
            raw[field] = bad;
            assert!(parse_request(&serde_json::to_vec(&raw).unwrap()).is_err());
        }
        let raw = json(&request).unwrap();
        let duplicate = [b"{\"schema\":1,".as_slice(), &raw[1..]].concat();
        assert!(parse_request(&duplicate).is_err());
        assert!(Request::new(1, &"a".repeat(32), &"x".repeat(181), Variant::Icon).is_err());
    }
    #[test]
    fn header_has_no_paths_and_requires_exact_typed_file_and_byte_bounds() {
        let (request, session, _, header, _, _) = fixture(4001, 1);
        assert_eq!(parse_header(&json(&header).unwrap()).unwrap(), header);
        assert_eq!(header.chunk_count, 2);
        for (field, bad) in [
            ("class", value!("unknown")),
            ("kind", value!({"icon":null})),
            ("version", value!("x".repeat(129))),
            ("version", value!("bad\nversion")),
            ("encoded_bytes", value!(true)),
            ("encoded_bytes", value!(0)),
            ("encoded_bytes", value!(ENCODED_LIMIT + 1)),
            ("chunk_count", value!(1)),
            ("path", value!("/private")),
        ] {
            let mut raw = serde_json::to_value(&header).unwrap();
            raw[field] = bad;
            assert!(parse_header(&serde_json::to_vec(&raw).unwrap()).is_err());
        }
        for (field, bad) in [
            ("mode", value!(0o120644)),
            ("bytes", value!(5)),
            ("inode", value!(0)),
            ("modified_nanos", value!(1_000_000_000)),
            ("uid", value!(-1)),
        ] {
            let mut raw = serde_json::to_value(&header).unwrap();
            raw["file_identity"][field] = bad;
            assert!(parse_header(&serde_json::to_vec(&raw).unwrap()).is_err());
        }
        let mut large = header.clone();
        large.kind = Kind::LargeIcon;
        assert!(large.validate_for(&request, &session).is_err());
        let mut preferred = request;
        preferred.variant = Variant::Preferred;
        assert!(large.validate_for(&preferred, &session).is_ok());
    }
    #[test]
    fn real_encoded_bytes_are_required_and_exact_at_all_chunk_edges() {
        for size in [1, CHUNK_BYTES, CHUNK_BYTES + 1, ENCODED_LIMIT] {
            let (mut receiver, header, bytes, complete, server) = receiver_with_body(size);
            assert!(receiver
                .push_packet(&json(&complete).unwrap(), server)
                .unwrap());
            match receiver.finish().unwrap() {
                Outcome::Artwork(value) => {
                    assert_eq!(value.bytes, bytes);
                    assert_eq!(value.header, header);
                    assert_eq!(value.complete, complete);
                }
                Outcome::Refused(_) => panic!("unexpected refusal"),
            }
        }
    }
    #[test]
    fn chunk_framing_is_little_endian_bounded_and_exact() {
        let encoded = encode_chunk(2, 7, &[1, 2, 3]).unwrap();
        let chunk = decode_chunk(&encoded).unwrap();
        assert_eq!(
            (chunk.request_id, chunk.index, chunk.payload),
            (2, 7, &[1, 2, 3][..])
        );
        assert_eq!(&encoded[..14], b"LHA1\x02\0\0\0\x07\0\0\0\x03\0");
        for raw in [
            vec![],
            encoded[..13].to_vec(),
            [encoded.as_slice(), &[4]].concat(),
            {
                let mut b = encoded.clone();
                b[0] = 0;
                b
            },
            {
                let mut b = encoded.clone();
                b[12] = 4;
                b
            },
        ] {
            assert!(decode_chunk(&raw).is_err());
        }
        assert!(encode_chunk(0, 0, &[1]).is_err());
        assert!(encode_chunk(1, 0, &[]).is_err());
        assert!(encode_chunk(1, 0, &vec![1; CHUNK_BYTES + 1]).is_err());
        assert!(decode_chunk(&vec![1; MESSAGE_LIMIT + 1]).is_err());
    }
    #[test]
    fn no_header_only_or_incomplete_data_can_be_accepted() {
        let (request, session, server, header, bytes, _) = fixture(4001, 1);
        assert!(Receiver::new(&request, &session, server)
            .unwrap()
            .finish()
            .is_err());
        let mut receiver = Receiver::new(&request, &session, server).unwrap();
        receiver
            .push_packet(&json(&header).unwrap(), server)
            .unwrap();
        receiver
            .push_packet(&encode_chunk(1, 0, &bytes[..4000]).unwrap(), server)
            .unwrap();
        assert!(receiver.finish().is_err());
        let (receiver, _, _, _, _) = receiver_with_body(4001);
        assert!(receiver.finish().is_err());
    }
    #[test]
    fn chunks_reject_wrong_request_reorder_repeat_short_middle_and_early_final() {
        let (request, session, server, header, bytes, complete) = fixture(4001, 1);
        for packet in [
            encode_chunk(2, 0, &bytes[..4000]).unwrap(),
            encode_chunk(1, 1, &bytes[..4000]).unwrap(),
            encode_chunk(1, 0, &bytes[..3999]).unwrap(),
            json(&complete).unwrap(),
        ] {
            let mut receiver = Receiver::new(&request, &session, server).unwrap();
            receiver
                .push_packet(&json(&header).unwrap(), server)
                .unwrap();
            assert!(receiver.push_packet(&packet, server).is_err());
            assert!(receiver.finish().is_err());
        }
        let mut receiver = Receiver::new(&request, &session, server).unwrap();
        receiver
            .push_packet(&json(&header).unwrap(), server)
            .unwrap();
        let first = encode_chunk(1, 0, &bytes[..4000]).unwrap();
        receiver.push_packet(&first, server).unwrap();
        assert!(receiver.push_packet(&first, server).is_err());
        assert!(receiver.finish().is_err());
    }
    #[test]
    fn corrupted_payload_cannot_pass_a_matching_final_metadata_message() {
        let (request, session, server, header, mut bytes, complete) = fixture(3, 1);
        bytes[0] ^= 1;
        let mut receiver = Receiver::new(&request, &session, server).unwrap();
        receiver
            .push_packet(&json(&header).unwrap(), server)
            .unwrap();
        receiver
            .push_packet(&encode_chunk(1, 0, &bytes).unwrap(), server)
            .unwrap();
        assert!(matches!(
            receiver.push_packet(&json(&complete).unwrap(), server),
            Err(Error::Digest)
        ));
        assert!(receiver.finish().is_err());
    }
    #[test]
    fn completion_binds_version_kind_fingerprint_and_full_identity() {
        let (_, _, _, header, _, complete) = fixture(1, 1);
        for change in 0..4 {
            let mut changed = header.clone();
            match change {
                0 => changed.version = "2.0".into(),
                1 => changed.kind = Kind::LargeIcon,
                2 => changed.file_identity.changed_seconds += 1,
                _ => changed.identity.client_start_ticks += 1,
            }
            assert!(complete.validate_for(&changed).is_err());
        }
        for field in [
            "identity",
            "request_id",
            "app_id",
            "encoded_bytes",
            "encoded_sha256",
            "header_sha256",
        ] {
            let mut raw = serde_json::to_value(&complete).unwrap();
            raw[field] = match field {
                "identity" => Value::Null,
                "request_id" => value!(2),
                "encoded_bytes" => value!(2),
                _ => value!("d".repeat(64)),
            };
            if let Ok(value) = parse_complete(&serde_json::to_vec(&raw).unwrap()) {
                assert!(value.validate_for(&header).is_err());
            }
        }
    }
    #[test]
    fn every_packet_credentials_are_pinned_and_an_error_poisons_the_result() {
        for wrong in [
            Credentials {
                pid: 1235,
                uid: 0,
                gid: 0,
            },
            Credentials {
                pid: 1234,
                uid: 1,
                gid: 0,
            },
            Credentials {
                pid: 1234,
                uid: 0,
                gid: 1,
            },
        ] {
            let (mut receiver, _, _, complete, _) = receiver_with_body(3);
            assert!(receiver
                .push_packet(&json(&complete).unwrap(), wrong)
                .is_err());
            assert!(receiver.finish().is_err());
        }
        let (request, session, _, header, _, _) = fixture(1, 1);
        assert!(Receiver::new(
            &request,
            &session,
            Credentials {
                pid: 1234,
                uid: 1000,
                gid: 0
            }
        )
        .is_err());
        let mut receiver = Receiver::new(
            &request,
            &session,
            Credentials {
                pid: 1234,
                uid: 0,
                gid: 0,
            },
        )
        .unwrap();
        assert!(receiver
            .push_packet(
                &json(&header).unwrap(),
                Credentials {
                    pid: 1235,
                    uid: 0,
                    gid: 0
                }
            )
            .is_err());
        assert!(receiver.finish().is_err());
    }
    #[test]
    fn extras_before_or_after_complete_are_never_accepted_by_assembler() {
        let (mut receiver, _, _, _, server) = receiver_with_body(3);
        assert!(receiver
            .push_packet(&encode_chunk(1, 1, &[4]).unwrap(), server)
            .is_err());
        assert!(receiver.finish().is_err());
        let (mut receiver, _, _, complete, server) = receiver_with_body(3);
        let final_packet = json(&complete).unwrap();
        assert!(receiver.push_packet(&final_packet, server).unwrap());
        assert!(matches!(
            receiver.push_packet(&final_packet, server),
            Err(Error::Extra)
        ));
        assert!(receiver.finish().is_err());
    }
    #[test]
    fn typed_initial_and_late_refusals_discard_encoded_data() {
        let (request, session, server, _, _, _) = fixture(3, 1);
        let refusal = Refusal::new(&request, &session, RefusalReason::Context).unwrap();
        for mut receiver in [
            Receiver::new(&request, &session, server).unwrap(),
            receiver_with_body(3).0,
        ] {
            assert!(receiver
                .push_packet(&json(&refusal).unwrap(), server)
                .unwrap());
            match receiver.finish().unwrap() {
                Outcome::Refused(value) => assert_eq!(value, refusal),
                _ => panic!("bytes escaped refusal"),
            }
        }
        let mut raw = serde_json::to_value(&refusal).unwrap();
        raw["reason"] = value!("/private/path");
        assert!(parse_refusal(&serde_json::to_vec(&raw).unwrap()).is_err());
        raw = serde_json::to_value(&refusal).unwrap();
        raw["error"] = value!("arbitrary stderr");
        assert!(parse_refusal(&serde_json::to_vec(&raw).unwrap()).is_err());
    }
    #[test]
    fn request_ids_prevent_a_stale_reply_from_becoming_the_next_transaction() {
        let (request, session, server, header, _, _) = fixture(3, 1);
        let next = Request {
            request_id: 2,
            ..request
        };
        let mut receiver = Receiver::new(&next, &session, server).unwrap();
        assert!(receiver
            .push_packet(&json(&header).unwrap(), server)
            .is_err());
        assert!(receiver.finish().is_err());
    }
    #[test]
    fn finish_has_a_separate_strict_next_id_and_identity_contract() {
        let (_, session, _, _, _, _) = fixture(1, 1);
        for id in [1, 6] {
            let request = FinishRequest::new(id, &session.nonce).unwrap();
            let finished = Finished::new(&request, &session).unwrap();
            assert_eq!(
                parse_finish_request(&json(&request).unwrap()).unwrap(),
                request
            );
            assert_eq!(parse_finished(&json(&finished).unwrap()).unwrap(), finished);
            let wrong = FinishRequest {
                request_id: if id == 1 { 2 } else { 5 },
                ..request
            };
            assert!(finished.validate_for(&wrong, &session).is_err());
        }
        assert!(FinishRequest::new(0, &session.nonce).is_err());
        assert!(FinishRequest::new(7, &session.nonce).is_err());
    }
    #[cfg(not(target_os = "linux"))]
    #[test]
    fn unsupported_platform_never_uses_stream_or_filesystem_fallback() {
        let error = Connection::connect_until(&"a".repeat(32), Instant::now()).unwrap_err();
        assert_eq!(error.kind(), io::ErrorKind::Unsupported);
    }

    #[test]
    fn discarded_read_consumes_valid_full_body_without_fabricating_complete() {
        for length in [1, CHUNK_BYTES, CHUNK_BYTES + 7] {
            let (receiver, header, bytes, _, _) = receiver_with_body(length);
            let witness = receiver.discard_incomplete_after_eof().unwrap();
            assert_eq!(witness.header(), &header);
            assert_eq!(witness.validated_bytes(), bytes.len() as u32);
            assert_eq!(witness.validated_chunks(), header.chunk_count);
            assert_eq!(witness.encoded_sha256(), header.encoded_sha256);
            let projected = serde_json::to_value(&witness).unwrap();
            let keys = projected
                .as_object()
                .unwrap()
                .keys()
                .map(String::as_str)
                .collect::<std::collections::BTreeSet<_>>();
            assert_eq!(
                keys,
                [
                    "header",
                    "validated_bytes",
                    "validated_chunks",
                    "encoded_sha256"
                ]
                .into_iter()
                .collect()
            );
            assert!(projected.get("complete").is_none());
            assert!(projected.get("bytes").is_none());
        }
    }
    #[test]
    fn discarded_read_requires_all_chunks_and_verified_digest() {
        let (request, session, server, header, mut bytes, _) = fixture(CHUNK_BYTES + 7, 1);
        let empty = Receiver::new(&request, &session, server).unwrap();
        assert!(matches!(
            empty.discard_incomplete_after_eof(),
            Err(Error::Incomplete)
        ));
        let mut partial = Receiver::new(&request, &session, server).unwrap();
        partial
            .push_packet(&json(&header).unwrap(), server)
            .unwrap();
        partial
            .push_packet(&encode_chunk(1, 0, &bytes[..CHUNK_BYTES]).unwrap(), server)
            .unwrap();
        assert!(matches!(
            partial.discard_incomplete_after_eof(),
            Err(Error::Incomplete)
        ));
        let mut corrupt = Receiver::new(&request, &session, server).unwrap();
        corrupt
            .push_packet(&json(&header).unwrap(), server)
            .unwrap();
        bytes[0] ^= 1;
        for (index, payload) in bytes.chunks(CHUNK_BYTES).enumerate() {
            corrupt
                .push_packet(&encode_chunk(1, index as u32, payload).unwrap(), server)
                .unwrap();
        }
        assert!(matches!(
            corrupt.discard_incomplete_after_eof(),
            Err(Error::Digest)
        ));
    }
    #[test]
    fn discarded_read_rejects_poisoned_complete_and_refusal_receivers() {
        let (request, session, server, _, _, _) = fixture(1, 1);
        let mut poisoned = Receiver::new(&request, &session, server).unwrap();
        assert!(poisoned.push_packet(b"broken", server).is_err());
        assert!(matches!(
            poisoned.discard_incomplete_after_eof(),
            Err(Error::Incomplete)
        ));
        let (mut complete, _, _, delimiter, _) = receiver_with_body(1);
        assert!(complete
            .push_packet(&json(&delimiter).unwrap(), server)
            .unwrap());
        assert!(matches!(
            complete.discard_incomplete_after_eof(),
            Err(Error::Incomplete)
        ));
        let mut refused = Receiver::new(&request, &session, server).unwrap();
        let refusal = Refusal::new(&request, &session, RefusalReason::Denied).unwrap();
        assert!(refused
            .push_packet(&json(&refusal).unwrap(), server)
            .unwrap());
        assert!(matches!(
            refused.discard_incomplete_after_eof(),
            Err(Error::Incomplete)
        ));
    }
    #[test]
    fn typed_finished_classifier_binds_profile_request_peer_and_actual_incoming_kind() {
        for profile in [Profile::Proof, Profile::Preview] {
            let (_, session, server, _, _, _) = if profile == Profile::Proof {
                fixture(1, 1)
            } else {
                preview_fixture(1)
            };
            let request = FinishRequest::new(2, &session.nonce).unwrap();
            let finished = Finished::new_for_profile(&request, &session, profile).unwrap();
            let packet = crate::broker_transport::Packet {
                bytes: json(&finished).unwrap(),
                credentials: server,
            };
            assert_eq!(
                classify_finished_incoming(
                    crate::broker_transport::Incoming::Packet(packet),
                    &request,
                    &session,
                    server,
                    profile
                )
                .unwrap(),
                FinishedOrEof::Finished(finished)
            );
            assert_eq!(
                classify_finished_incoming(
                    crate::broker_transport::Incoming::Eof,
                    &request,
                    &session,
                    server,
                    profile
                )
                .unwrap(),
                FinishedOrEof::Eof
            );
            let mut wrong = request.clone();
            wrong.nonce = "f".repeat(32);
            assert!(matches!(
                classify_finished_incoming(
                    crate::broker_transport::Incoming::Eof,
                    &wrong,
                    &session,
                    server,
                    profile
                ),
                Err(Error::Identity)
            ));
            let mut wrong_peer = server;
            wrong_peer.pid += 1;
            assert!(matches!(
                classify_finished_incoming(
                    crate::broker_transport::Incoming::Eof,
                    &request,
                    &session,
                    wrong_peer,
                    profile
                ),
                Err(Error::Credentials)
            ));
            let other = if profile == Profile::Proof {
                Profile::Preview
            } else {
                Profile::Proof
            };
            let other_profile_eof = classify_finished_incoming(
                crate::broker_transport::Incoming::Eof,
                &request,
                &session,
                server,
                other,
            );
            if profile == Profile::Preview {
                assert!(other_profile_eof.is_err());
            } else {
                // Existing wire validation is structural: a Proof fixture's
                // UID is also structurally valid for Preview. Protected role
                // admission remains the caller's separate required check.
                assert_eq!(other_profile_eof.unwrap(), FinishedOrEof::Eof);
            }
        }
    }
    #[test]
    fn typed_finished_classifier_never_turns_malformed_or_wrong_credentials_into_eof() {
        let (_, session, server, _, _, _) = fixture(1, 1);
        let request = FinishRequest::new(2, &session.nonce).unwrap();
        let finished = Finished::new(&request, &session).unwrap();
        for bytes in [Vec::new(), b"broken".to_vec(), b"{}".to_vec()] {
            let incoming =
                crate::broker_transport::Incoming::Packet(crate::broker_transport::Packet {
                    bytes,
                    credentials: server,
                });
            assert!(classify_finished_incoming(
                incoming,
                &request,
                &session,
                server,
                Profile::Proof
            )
            .is_err());
        }
        let mut wrong = server;
        wrong.uid = 6583;
        let incoming = crate::broker_transport::Incoming::Packet(crate::broker_transport::Packet {
            bytes: json(&finished).unwrap(),
            credentials: wrong,
        });
        assert!(matches!(
            classify_finished_incoming(incoming, &request, &session, server, Profile::Proof),
            Err(Error::Credentials)
        ));
        let wrong_request = FinishRequest::new(3, &session.nonce).unwrap();
        let incoming = crate::broker_transport::Incoming::Packet(crate::broker_transport::Packet {
            bytes: json(&finished).unwrap(),
            credentials: server,
        });
        assert!(matches!(
            classify_finished_incoming(incoming, &wrong_request, &session, server, Profile::Proof),
            Err(Error::Identity)
        ));
    }
}
