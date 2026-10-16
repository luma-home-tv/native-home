//! Synthetic strict parser and actual ordinary-file fixtures, UNRUN by author.
use super::*;
use serde_json::{json, Value};
use std::{
    fs,
    os::unix::fs::{symlink, PermissionsExt},
    path::PathBuf,
    sync::atomic::{AtomicU64, Ordering},
};
static SERIAL: AtomicU64 = AtomicU64::new(0);
fn marker() -> Marker {
    Marker {
        schema: 1,
        nonce: "a".repeat(32),
        ui_sha256: "b".repeat(64),
        broker_sha256: "c".repeat(64),
    }
}
fn cursor() -> Value {
    json!({"schema":1,"keyset_sha256":"d".repeat(64),"next_ordinal":16})
}
fn record(cursor: Value) -> Value {
    json!({"schema":1,"ui_sha256":"b".repeat(64),"cursor":cursor})
}
fn parse(value: Value) -> Result<Selection, Box<dyn Error>> {
    Selection::parse(&serde_json::to_vec(&value)?, &marker())
}
struct Fixture(PathBuf);
impl Fixture {
    fn new() -> Self {
        let path = std::env::temp_dir().join(format!(
            "preparation-selection-{}-{}",
            std::process::id(),
            SERIAL.fetch_add(1, Ordering::Relaxed)
        ));
        fs::DirBuilder::new().create(&path).unwrap();
        fs::set_permissions(&path, fs::Permissions::from_mode(0o700)).unwrap();
        Self(path)
    }
    fn path(&self) -> PathBuf {
        self.0.join(NAME)
    }
    fn write(&self, raw: &[u8]) {
        fs::write(self.path(), raw).unwrap();
        fs::set_permissions(self.path(), fs::Permissions::from_mode(0o644)).unwrap();
    }
    fn owner(&self) -> (u32, u32) {
        (unsafe { libc::geteuid() }, unsafe { libc::getegid() })
    }
    fn read(&self) -> Result<Selection, Box<dyn Error>> {
        read_owned(&self.0, &marker(), self.owner(), || {})
    }
}
impl Drop for Fixture {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.0);
    }
}
#[test]
fn exact_null_and_valid_boundary_cursor_preserve_only_enumeration() {
    assert!(parse(record(Value::Null)).unwrap().cursor().is_none());
    for ordinal in [0, 16, 512] {
        let mut v = cursor();
        v["next_ordinal"] = json!(ordinal);
        let s = parse(record(v)).unwrap();
        assert_eq!(s.cursor().unwrap().next_ordinal(), ordinal);
    }
}
#[test]
fn cursor_is_mandatory_even_though_literal_null_is_valid() {
    let mut v = record(Value::Null);
    v.as_object_mut().unwrap().remove("cursor");
    assert!(parse(v).is_err());
    for bad in [json!(false), json!(0), json!("null"), json!([]), json!({})] {
        assert!(parse(record(bad)).is_err());
    }
}
#[test]
fn duplicate_outer_and_inner_fields_never_pass_through_a_value_projection() {
    let good = serde_json::to_string(&record(cursor())).unwrap();
    for repeated in [
        "\"schema\":1,",
        "\"ui_sha256\":\"different\",",
        "\"cursor\":null,",
    ] {
        let mut raw = good.clone();
        raw.insert_str(1, repeated);
        assert!(Selection::parse(raw.as_bytes(), &marker()).is_err());
    }
    for field in [
        "\"schema\":1,",
        "\"keyset_sha256\":\"bad\",",
        "\"next_ordinal\":16,",
    ] {
        let mut inner = serde_json::to_string(&cursor()).unwrap();
        inner.insert_str(1, field);
        let raw = format!(
            "{{\"schema\":1,\"ui_sha256\":\"{}\",\"cursor\":{inner}}}",
            "b".repeat(64)
        );
        assert!(Selection::parse(raw.as_bytes(), &marker()).is_err());
    }
}
#[test]
fn every_cursor_validation_runs_beyond_derived_field_types() {
    for (key, bad) in [
        ("schema", json!(2)),
        ("schema", json!(true)),
        ("keyset_sha256", json!("D".repeat(64))),
        ("keyset_sha256", json!("d".repeat(63))),
        ("next_ordinal", json!(513)),
        ("next_ordinal", json!(-1)),
        ("next_ordinal", json!(true)),
        ("next_ordinal", json!(1.5)),
        ("unexpected", json!(0)),
    ] {
        let mut c = cursor();
        c[key] = bad;
        assert!(parse(record(c)).is_err(), "{key}");
    }
    for field in ["schema", "keyset_sha256", "next_ordinal"] {
        let mut c = cursor();
        c.as_object_mut().unwrap().remove(field);
        assert!(parse(record(c)).is_err());
    }
}
#[test]
fn wrong_ui_schema_extra_authority_fields_trailing_json_and_bounds_refuse() {
    for (key, bad) in [
        ("schema", json!(2)),
        ("schema", json!(true)),
        ("ui_sha256", json!("c".repeat(64))),
        ("path", json!("/tmp/other")),
        ("duration_ms", json!(5000)),
        ("Root_authority_granted", json!(true)),
    ] {
        let mut v = record(Value::Null);
        v[key] = bad;
        assert!(parse(v).is_err());
    }
    let good = serde_json::to_vec(&record(Value::Null)).unwrap();
    let mut two = good.clone();
    two.extend_from_slice(b" null");
    assert!(Selection::parse(&two, &marker()).is_err());
    assert!(Selection::parse(&[], &marker()).is_err());
    let mut oversized = good;
    oversized.resize(MAX_BYTES + 1, b' ');
    assert!(Selection::parse(&oversized, &marker()).is_err());
    let mut invalid_marker = marker();
    invalid_marker.nonce.clear();
    assert!(Selection::parse(b"{}", &invalid_marker).is_err());
}
#[test]
fn fixed_file_allows_only_canonical_modes_and_does_not_claim_fixture_root_ownership() {
    let f = Fixture::new();
    f.write(&serde_json::to_vec(&record(cursor())).unwrap());
    for mode in [0o444, 0o644] {
        fs::set_permissions(f.path(), fs::Permissions::from_mode(mode)).unwrap();
        assert!(f.read().is_ok());
    }
    for mode in [0o600, 0o666, 0o755, 0o1644] {
        fs::set_permissions(f.path(), fs::Permissions::from_mode(mode)).unwrap();
        assert!(f.read().is_err());
    }
    fs::set_permissions(f.path(), fs::Permissions::from_mode(0o644)).unwrap();
    assert!(read_owned(
        &f.0,
        &marker(),
        (f.owner().0.wrapping_add(1), f.owner().1),
        || {}
    )
    .is_err());
}
#[test]
fn symlink_hardlink_directory_fifo_empty_and_sparse_oversize_refuse_without_waiting() {
    let raw = serde_json::to_vec(&record(Value::Null)).unwrap();
    let f = Fixture::new();
    let outside = f.0.join("other");
    fs::write(&outside, &raw).unwrap();
    symlink(&outside, f.path()).unwrap();
    assert!(f.read().is_err());
    fs::remove_file(f.path()).unwrap();
    f.write(&raw);
    fs::hard_link(f.path(), f.0.join("hard")).unwrap();
    assert!(f.read().is_err());
    fs::remove_file(f.0.join("hard")).unwrap();
    fs::remove_file(f.path()).unwrap();
    fs::create_dir(f.path()).unwrap();
    assert!(f.read().is_err());
    fs::remove_dir(f.path()).unwrap();
    use std::os::unix::ffi::OsStrExt;
    let c = std::ffi::CString::new(f.path().as_os_str().as_bytes()).unwrap();
    assert_eq!(unsafe { libc::mkfifo(c.as_ptr(), 0o644) }, 0);
    assert!(f.read().is_err());
    fs::remove_file(f.path()).unwrap();
    f.write(b"");
    assert!(f.read().is_err());
    let file = OpenOptions::new().write(true).open(f.path()).unwrap();
    file.set_len((MAX_BYTES + 1) as u64).unwrap();
    assert!(f.read().is_err());
}
#[test]
fn held_fd_named_replacement_growth_and_symlink_retarget_are_detected_before_parse() {
    let raw = serde_json::to_vec(&record(cursor())).unwrap();
    for change in [0, 1, 2] {
        let f = Fixture::new();
        f.write(&raw);
        let got = read_owned(&f.0, &marker(), f.owner(), || {
            if change == 0 {
                let mut more = raw.clone();
                more.push(b' ');
                fs::write(f.path(), more).unwrap();
            } else {
                fs::rename(f.path(), f.0.join("old")).unwrap();
                if change == 1 {
                    f.write(&raw);
                } else {
                    symlink(f.0.join("old"), f.path()).unwrap();
                }
            }
        });
        assert!(got.is_err());
    }
}
