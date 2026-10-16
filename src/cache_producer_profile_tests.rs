use super::*;
use serde_json::json;
use std::{
    fs,
    os::unix::fs::{symlink, PermissionsExt},
    path::PathBuf,
    sync::atomic::{AtomicU64, Ordering},
};
static SERIAL: AtomicU64 = AtomicU64::new(0);
struct Fixture(PathBuf);
impl Fixture {
    fn new() -> Self {
        let path = std::env::temp_dir().join(format!(
            "cache-producer-profile-{}-{}",
            std::process::id(),
            SERIAL.fetch_add(1, Ordering::Relaxed)
        ));
        fs::create_dir(&path).unwrap();
        fs::set_permissions(&path, fs::Permissions::from_mode(0o700)).unwrap();
        Self(path)
    }
    fn path(&self) -> PathBuf {
        self.0.join(NAME.to_str().unwrap())
    }
    fn directory(&self) -> File {
        File::open(&self.0).unwrap()
    }
    fn write(&self, raw: &[u8]) {
        fs::write(self.path(), raw).unwrap();
        fs::set_permissions(self.path(), fs::Permissions::from_mode(0o644)).unwrap();
    }
    fn load(&self) -> io::Result<Option<ExpectedProducer>> {
        ExpectedProducer::load_for_test(&self.directory(), &"b".repeat(64))
    }
}
impl Drop for Fixture {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.0);
    }
}
fn record() -> serde_json::Value {
    json!({"schema":1,"app_id":crate::identity::PREVIEW,"producer_kind":"joined-single-positive-v1","source_revision":"a".repeat(40),"ui_sha256":"b".repeat(64),"broker_sha256":"c".repeat(64)})
}
#[test]
fn fixed_sidecar_loads_independent_association_and_does_not_admit_nonce() {
    let f = Fixture::new();
    f.write(&serde_json::to_vec(&record()).unwrap());
    let expected = f.load().unwrap().unwrap();
    assert_eq!(
        expected.version(),
        ProducerVersion::new(&"a".repeat(40), [0xbb; 32], [0xcc; 32]).unwrap()
    );
    let mut marker = Marker {
        schema: 1,
        nonce: "not-an-admission-nonce".into(),
        ui_sha256: "b".repeat(64),
        broker_sha256: "c".repeat(64),
    };
    assert!(expected.matches_marker(&marker)); // Association only: Marker::validate still owns nonce/policy.
    marker.broker_sha256 = "d".repeat(64);
    assert!(!expected.matches_marker(&marker));
    assert!(ExpectedProducer::load_for_test(&f.directory(), &"d".repeat(64)).is_err());
}
#[test]
fn missing_is_optional_but_present_malformed_never_falls_back_to_cache_metadata() {
    let f = Fixture::new();
    assert!(f.load().unwrap().is_none());
    for raw in [
        b"{}".as_slice(),
        b"null",
        b"[]",
        b"{\"schema\":1,\"schema\":1}",
        b"",
        b"not JSON",
    ] {
        f.write(raw);
        assert!(f.load().is_err());
    }
    let mut raw = serde_json::to_string(&record()).unwrap();
    raw.insert_str(1, "\"schema\":1,");
    f.write(raw.as_bytes());
    assert!(f.load().is_err());
}
#[test]
fn every_closed_field_and_hash_is_independently_checked() {
    let f = Fixture::new();
    for (key, value) in [
        ("schema", json!(true)),
        ("schema", json!(2)),
        ("app_id", json!(crate::identity::PROOF)),
        ("producer_kind", json!("generic-positive")),
        ("source_revision", json!("A".repeat(40))),
        ("ui_sha256", json!("b".repeat(63))),
        ("broker_sha256", json!("C".repeat(64))),
        ("uri", json!("luna://untrusted")),
    ] {
        let mut v = record();
        v[key] = value;
        f.write(&serde_json::to_vec(&v).unwrap());
        assert!(f.load().is_err(), "{key}");
    }
    for key in [
        "schema",
        "app_id",
        "producer_kind",
        "source_revision",
        "ui_sha256",
        "broker_sha256",
    ] {
        let mut v = record();
        v.as_object_mut().unwrap().remove(key);
        f.write(&serde_json::to_vec(&v).unwrap());
        assert!(f.load().is_err());
    }
}
#[test]
fn symlink_fifo_directory_hardlink_and_bounds_are_refused_without_blocking() {
    let f = Fixture::new();
    let other = f.0.join("other");
    fs::write(&other, serde_json::to_vec(&record()).unwrap()).unwrap();
    symlink(&other, f.path()).unwrap();
    assert!(f.load().is_err());
    fs::remove_file(f.path()).unwrap();
    let name = std::ffi::CString::new(f.path().to_str().unwrap()).unwrap();
    assert_eq!(unsafe { libc::mkfifo(name.as_ptr(), 0o644) }, 0);
    assert!(f.load().is_err());
    fs::remove_file(f.path()).unwrap();
    fs::create_dir(f.path()).unwrap();
    assert!(f.load().is_err());
    fs::remove_dir(f.path()).unwrap();
    fs::hard_link(&other, f.path()).unwrap();
    assert!(f.load().is_err());
    fs::remove_file(f.path()).unwrap();
    f.write(&vec![b' '; 4097]);
    assert!(f.load().is_err());
    f.write(b"");
    assert!(f.load().is_err());
}
#[test]
fn actual_owner_and_packaged_modes_are_observations_required_without_repair() {
    let f = Fixture::new();
    f.write(&serde_json::to_vec(&record()).unwrap());
    assert!(load_at(
        &f.directory(),
        unsafe { libc::geteuid() }.wrapping_add(1),
        &"b".repeat(64),
        || {}
    )
    .is_err());
    for mode in [0o600, 0o666, 0o755] {
        fs::set_permissions(f.path(), fs::Permissions::from_mode(mode)).unwrap();
        assert!(f.load().is_err());
        assert_eq!(fs::metadata(f.path()).unwrap().mode() & 0o7777, mode);
    }
    fs::set_permissions(f.path(), fs::Permissions::from_mode(0o444)).unwrap();
    assert!(f.load().unwrap().is_some());
}
#[test]
fn held_named_replacement_in_place_and_mode_changes_reject_even_equal_length() {
    for mutation in 0..3 {
        let f = Fixture::new();
        let raw = serde_json::to_vec(&record()).unwrap();
        f.write(&raw);
        let dir = f.directory();
        let got = load_at(
            &dir,
            unsafe { libc::geteuid() },
            &"b".repeat(64),
            || match mutation {
                0 => {
                    let replacement = f.0.join("replacement");
                    fs::write(&replacement, &raw).unwrap();
                    fs::set_permissions(&replacement, fs::Permissions::from_mode(0o644)).unwrap();
                    fs::rename(replacement, f.path()).unwrap();
                }
                1 => {
                    let mut v = record();
                    v["broker_sha256"] = json!("d".repeat(64));
                    fs::write(f.path(), serde_json::to_vec(&v).unwrap()).unwrap();
                }
                _ => fs::set_permissions(f.path(), fs::Permissions::from_mode(0o444)).unwrap(),
            },
        );
        assert!(got.is_err());
    }
}
#[test]
fn unrelated_parent_entry_churn_and_access_time_are_not_false_replacement() {
    let f = Fixture::new();
    f.write(&serde_json::to_vec(&record()).unwrap());
    let got = load_at(
        &f.directory(),
        unsafe { libc::geteuid() },
        &"b".repeat(64),
        || {
            fs::write(f.0.join("unrelated"), b"ordinary sibling").unwrap();
        },
    );
    assert!(got.unwrap().is_some());
    for _ in 0..3 {
        assert!(f.load().unwrap().is_some());
    }
}
#[test]
fn native_entry_is_explicitly_unsupported_on_host() {
    #[cfg(not(target_arch = "arm"))]
    assert_eq!(
        load_current().unwrap_err().kind(),
        io::ErrorKind::Unsupported
    );
}
