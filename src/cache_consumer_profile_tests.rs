use super::*;
use serde_json::json;
use std::{
    fs,
    os::unix::fs::{symlink, PermissionsExt},
    path::PathBuf,
    sync::atomic::{AtomicU64, Ordering},
};
static NEXT: AtomicU64 = AtomicU64::new(0);
struct Fixture(PathBuf);
impl Fixture {
    fn new() -> Self {
        let p = std::env::temp_dir().join(format!(
            "consumer-selector-{}-{}",
            std::process::id(),
            NEXT.fetch_add(1, Ordering::Relaxed)
        ));
        fs::create_dir(&p).unwrap();
        fs::set_permissions(&p, fs::Permissions::from_mode(0o700)).unwrap();
        Self(p)
    }
    fn path(&self) -> PathBuf {
        self.0.join(NAME)
    }
    fn write(&self, raw: &[u8]) {
        fs::write(self.path(), raw).unwrap();
        fs::set_permissions(self.path(), fs::Permissions::from_mode(0o644)).unwrap();
    }
    fn load(&self) -> io::Result<Option<ReadExpectation>> {
        load_for_test(&File::open(&self.0).unwrap(), &"b".repeat(64))
    }
}
impl Drop for Fixture {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.0);
    }
}
fn producer() -> serde_json::Value {
    json!({"schema":1,"app_id":crate::identity::PREVIEW,"producer_kind":"joined-single-positive-v1","source_revision":"d".repeat(40),"ui_sha256":"a".repeat(64),"broker_sha256":"c".repeat(64)})
}
fn record() -> serde_json::Value {
    record_for_test(&"b".repeat(64), producer())
}

#[test]
fn actual_prior_origin_is_not_relabelled_to_current_consumer_or_publisher() {
    let f = Fixture::new();
    let raw = serde_json::to_vec(&record()).unwrap();
    f.write(&raw);
    let (version, association) = f.load().unwrap().unwrap().into_parts();
    assert_eq!(
        version,
        ProducerVersion::new(&"d".repeat(40), [0xaa; 32], [0xcc; 32]).unwrap()
    );
    let observation = serde_json::to_value(association.unwrap()).unwrap();
    assert_eq!(observation["consumer_ui_sha256"], "b".repeat(64));
    assert_eq!(observation["producer"]["ui_sha256"], json!(vec![0xaa; 32]));
    assert_eq!(observation["selection"], "accepted_producer");
    assert_eq!(observation["association_only"], true);
    assert_eq!(fs::read(f.path()).unwrap(), raw);
    let original = f.0.join("artwork-cache-producer.json");
    let original_raw = serde_json::to_vec(&producer()).unwrap();
    fs::write(&original, &original_raw).unwrap();
    fs::set_permissions(&original, fs::Permissions::from_mode(0o644)).unwrap();
    // The existing actual-self publisher reader remains strict and unchanged.
    assert!(
        crate::cache_producer_profile::ExpectedProducer::load_for_test(
            &File::open(&f.0).unwrap(),
            &"b".repeat(64)
        )
        .is_err()
    );
    assert_eq!(fs::read(original).unwrap(), original_raw);
}
#[test]
fn malformed_present_never_uses_current_profile_fallback_but_absence_can() {
    let f = Fixture::new();
    let absent = f.load();
    let mut calls = 0;
    let current = ProducerVersion::new(&"e".repeat(40), [0xbb; 32], [0xcc; 32]).unwrap();
    let result = choose(absent, || {
        calls += 1;
        Ok(Some(expectation(
            current.clone(),
            &"b".repeat(64),
            Selection::CurrentProducer,
            None,
            ReaderContract::current(),
        )))
    })
    .unwrap()
    .unwrap();
    assert_eq!(calls, 1);
    assert_eq!(result.into_parts().0, current);
    f.write(b"{}");
    assert!(choose(f.load(), || panic!(
        "Malformed selector must disable, never select another origin"
    ))
    .is_err());
    f.write(&serde_json::to_vec(&record()).unwrap());
    assert!(choose(f.load(), || panic!(
        "Explicit accepted origin must not invoke publisher/profile fallback"
    ))
    .unwrap()
    .is_some());
}
#[test]
fn every_consumer_contract_field_and_current_identity_is_exact() {
    let f = Fixture::new();
    let original = record();
    for key in [
        "source_grammar_sha256",
        "app_id_grammar_sha256",
        "cargo_semantics_sha256",
        "native_build_selection_sha256",
        "record_parser_sha256",
        "maintenance_planner_sha256",
        "file_reader_sha256",
        "decoder_policy_sha256",
        "cargo_lock_sha256",
        "dependency_lock_sha256",
        "framework_patch_sha256",
    ] {
        let mut value = original.clone();
        value["reader_contract"][key] = json!("0".repeat(64));
        f.write(&serde_json::to_vec(&value).unwrap());
        assert!(f.load().is_err(), "{key}");
    }
    for (key, value) in [
        ("schema", json!(true)),
        ("app_id", json!(crate::identity::PROOF)),
        ("consumer_ui_sha256", json!("a".repeat(64))),
        ("uri", json!("luna://untrusted")),
    ] {
        let mut v = original.clone();
        v[key] = value;
        f.write(&serde_json::to_vec(&v).unwrap());
        assert!(f.load().is_err(), "{key}");
    }
    let mut v = original.clone();
    v["producer"]["broker_sha256"] = json!("C".repeat(64));
    f.write(&serde_json::to_vec(&v).unwrap());
    assert!(f.load().is_err());
    let mut raw = serde_json::to_string(&original).unwrap();
    raw.insert_str(1, "\"schema\":1,");
    f.write(raw.as_bytes());
    assert!(f.load().is_err());
    let mut v = serde_json::to_string(&original).unwrap();
    let p = v.find("\"producer\":{").unwrap() + "\"producer\":{".len();
    v.insert_str(p, "\"schema\":1,");
    f.write(v.as_bytes());
    assert!(f.load().is_err());
}
#[test]
fn filesystem_immutable_selector_checks_do_not_block_or_accept_substitution() {
    let f = Fixture::new();
    let other = f.0.join("other");
    fs::write(&other, serde_json::to_vec(&record()).unwrap()).unwrap();
    symlink(&other, f.path()).unwrap();
    assert!(f.load().is_err());
    fs::remove_file(f.path()).unwrap();
    fs::hard_link(&other, f.path()).unwrap();
    assert!(f.load().is_err());
    fs::remove_file(f.path()).unwrap();
    f.write(&vec![b' '; 4097]);
    assert!(f.load().is_err());
    f.write(&serde_json::to_vec(&record()).unwrap());
    fs::set_permissions(f.path(), fs::Permissions::from_mode(0o600)).unwrap();
    assert!(f.load().is_err());
    f.write(&serde_json::to_vec(&record()).unwrap());
    let parent = File::open(&f.0).unwrap();
    let outcome = load_at(&parent, unsafe { libc::geteuid() }, &"b".repeat(64), || {
        fs::remove_file(f.path()).unwrap();
        f.write(&serde_json::to_vec(&record()).unwrap());
    });
    assert!(outcome.is_err());
}
