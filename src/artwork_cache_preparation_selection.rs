//! Fixed ordinary diagnostic selection data. No role/path/command or Root grant.
use crate::{artwork_cache_preparation::Cursor, broker_protocol::Marker};
use serde::{
    de::{self, MapAccess, Visitor},
    Deserialize,
};
use std::{
    error::Error,
    fs::{File, Metadata, OpenOptions},
    io::{self, Read},
    os::unix::fs::{MetadataExt, OpenOptionsExt},
    path::Path,
};

pub const NAME: &str = "artwork-cache-preparation.json";
const MAX_BYTES: usize = 4096;
fn invalid(message: &str) -> io::Error {
    io::Error::new(io::ErrorKind::InvalidData, message)
}

// deserialize_any is deliberate: serde's missing-field deserializer treats
// deserialize_option as None. A mandatory non-Option field must reject absence.
enum CursorOrNull {
    Null,
    Cursor(Cursor),
}
impl<'de> Deserialize<'de> for CursorOrNull {
    fn deserialize<D: de::Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        struct Exact;
        impl<'de> Visitor<'de> for Exact {
            type Value = CursorOrNull;
            fn expecting(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                f.write_str("literal null or a strict preparation Cursor")
            }
            fn visit_unit<E: de::Error>(self) -> Result<Self::Value, E> {
                Ok(CursorOrNull::Null)
            }
            fn visit_map<A: MapAccess<'de>>(self, map: A) -> Result<Self::Value, A::Error> {
                // MapAccessDeserializer preserves duplicate fields. Do NOT parse
                // through Value, which would silently overwrite duplicate keys.
                let parsed = Cursor::deserialize(de::value::MapAccessDeserializer::new(map))?;
                let canonical = serde_json::to_vec(&parsed).map_err(de::Error::custom)?;
                let checked = Cursor::from_json(&canonical)
                    .map_err(|e| de::Error::custom(format!("Invalid preparation cursor: {e:?}")))?;
                Ok(CursorOrNull::Cursor(checked))
            }
        }
        deserializer.deserialize_any(Exact)
    }
}
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Record {
    schema: u32,
    ui_sha256: String,
    cursor: CursorOrNull,
}
pub struct Selection {
    cursor: Option<Cursor>,
}
impl Selection {
    /// Caller supplies its already-admitted fixed assets directory and Marker.
    /// This checks only data association; the existing worker checks actual self.
    pub fn read(assets: &Path, admitted_marker: &Marker) -> Result<Self, Box<dyn Error>> {
        read_owned(assets, admitted_marker, (0, 0), || {})
    }
    pub fn cursor(&self) -> Option<&Cursor> {
        self.cursor.as_ref()
    }
    fn parse(raw: &[u8], marker: &Marker) -> Result<Self, Box<dyn Error>> {
        marker.validate()?;
        if raw.is_empty() || raw.len() > MAX_BYTES {
            return Err(invalid("Preparation selection size").into());
        }
        let record: Record = serde_json::from_slice(raw)?;
        if record.schema != 1 || record.ui_sha256 != marker.ui_sha256 {
            return Err(invalid("Preparation selection schema/UI association").into());
        }
        Ok(Self {
            cursor: match record.cursor {
                CursorOrNull::Null => None,
                CursorOrNull::Cursor(cursor) => Some(cursor),
            },
        })
    }
}
#[derive(PartialEq, Eq)]
struct Stamp {
    device: u64,
    inode: u64,
    bytes: u64,
    uid: u32,
    gid: u32,
    mode: u32,
    links: u64,
    modified_seconds: i64,
    modified_nanos: i64,
    changed_seconds: i64,
    changed_nanos: i64,
}
impl Stamp {
    fn of(m: &Metadata) -> Self {
        Self {
            device: m.dev(),
            inode: m.ino(),
            bytes: m.len(),
            uid: m.uid(),
            gid: m.gid(),
            mode: m.mode(),
            links: m.nlink(),
            modified_seconds: m.mtime(),
            modified_nanos: m.mtime_nsec(),
            changed_seconds: m.ctime(),
            changed_nanos: m.ctime_nsec(),
        }
    }
}
/// Owner and after-open callback remain private. Production always supplies
/// Root0:0 and no mutation; controlled host fixtures use their actual own UID/GID.
fn read_owned(
    assets: &Path,
    marker: &Marker,
    owner: (u32, u32),
    after_open: impl FnOnce(),
) -> Result<Selection, Box<dyn Error>> {
    marker.validate()?;
    let path = assets.join(NAME);
    let mut file: File = OpenOptions::new()
        .read(true)
        .custom_flags(libc::O_NOFOLLOW | libc::O_NONBLOCK | libc::O_CLOEXEC)
        .open(&path)?;
    let before = file.metadata()?;
    if !before.is_file()
        || before.len() == 0
        || before.len() > MAX_BYTES as u64
        || (before.uid(), before.gid()) != owner
        || before.nlink() != 1
        || ![0o444, 0o644].contains(&(before.mode() & 0o7777))
    {
        return Err(invalid("Preparation selection file type/owner/mode/length/links").into());
    }
    after_open();
    // Fixed cap+1 stack buffer: growth cannot induce an unbounded allocation.
    let mut raw = [0u8; MAX_BYTES + 1];
    let mut count = 0;
    while count < raw.len() {
        match file.read(&mut raw[count..]) {
            Ok(0) => break,
            Ok(n) => count += n,
            Err(e) if e.kind() == io::ErrorKind::Interrupted => {}
            Err(e) => return Err(e.into()),
        }
    }
    let after = file.metadata()?;
    let named = std::fs::symlink_metadata(&path)?;
    if named.file_type().is_symlink()
        || Stamp::of(&before) != Stamp::of(&after)
        || Stamp::of(&before) != Stamp::of(&named)
        || count as u64 != before.len()
        || count > MAX_BYTES
    {
        return Err(invalid("Preparation selection changed during read").into());
    }
    Selection::parse(&raw[..count], marker)
}
#[cfg(test)]
#[path = "artwork_cache_preparation_selection_tests.rs"]
mod tests;
