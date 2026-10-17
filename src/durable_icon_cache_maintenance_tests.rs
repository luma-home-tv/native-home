use super::super::{hex, ObservedSource, SourceRecord};
use super::*;
use serde_json::json;

const UI: [u8; 32] = [0xbb; 32];
fn current() -> ProducerVersion {
    ProducerVersion::new(&"c".repeat(40), UI, [0xcc; 32]).unwrap()
}
fn accepted() -> ProducerVersion {
    ProducerVersion::new(&"f".repeat(40), [0xaa; 32], [0xdd; 32]).unwrap()
}
fn unsupported() -> ProducerVersion {
    ProducerVersion::new(&"e".repeat(40), [0x11; 32], [0x22; 32]).unwrap()
}
fn sidecar(producer: &ProducerVersion) -> serde_json::Value {
    json!({"schema":1,"app_id":crate::identity::PREVIEW,"producer_kind":"joined-single-positive-v1",
        "source_revision":producer.source_revision,"ui_sha256":hex(&producer.ui_sha256),
        "broker_sha256":hex(&producer.broker_sha256)})
}
fn profiles(versions: &[ProducerVersion]) -> (Vec<u8>, Vec<Vec<u8>>, Vec<[u8; 32]>) {
    let current = serde_json::to_vec(&sidecar(&current())).unwrap();
    let selectors = versions
        .iter()
        .map(|p| {
            serde_json::to_vec(&crate::cache_consumer_profile::record_for_test(
                &hex(&UI),
                sidecar(p),
            ))
        })
        .collect::<Result<Vec<_>, _>>()
        .unwrap();
    let hashes = selectors.iter().map(|raw| digest(raw)).collect();
    (current, selectors, hashes)
}
fn retention() -> RetainedProducers {
    let (current, selectors, hashes) = profiles(&[accepted()]);
    RetainedProducers::from_profiles(
        &current,
        &selectors.iter().map(Vec::as_slice).collect::<Vec<_>>(),
        &hashes,
        &hex(&UI),
    )
    .unwrap()
}
fn metadata(bytes: usize, inode: u64) -> FileIdentity {
    FileIdentity {
        device: 1,
        inode,
        uid: 7001,
        gid: 5000,
        mode: 0o100000 | 0o600,
        links: 1,
        bytes: bytes as u64,
        modified_seconds: 1,
        modified_nanos: 0,
        changed_seconds: 1,
        changed_nanos: 0,
    }
}
struct OwnedLeaf {
    name: String,
    raw: Vec<u8>,
    before: FileIdentity,
    after: FileIdentity,
}
fn leaf(producer: ProducerVersion, version: &str, body: &[u8]) -> OwnedLeaf {
    let record = Record {
        schema: 1,
        producer_kind: "joined-single-positive-v1".into(),
        producer,
        source: SourceRecord {
            app_id: "org.synthetic.app".into(),
            version: version.into(),
            class: "lg_system".into(),
            folder: "/usr/palm/applications/org.synthetic.app".into(),
            relative: vec!["icon.png".into()],
            kind: "icon".into(),
        },
        source_identity: ObservedSource {
            device: 8,
            inode: 9,
            bytes: body.len() as u64,
            mode: 0o100000 | 0o777,
            uid: 0,
            gid: 0,
            modified_seconds: 0,
            modified_nanos: 0,
            changed_seconds: 0,
            changed_nanos: 0,
        },
        encoded_bytes: body.len() as u32,
        encoded_sha256: digest(body),
    };
    let header = record.header().unwrap();
    let mut raw = MAGIC.to_vec();
    raw.extend_from_slice(&(header.len() as u32).to_be_bytes());
    raw.extend_from_slice(&header);
    raw.extend_from_slice(body);
    let meta = metadata(raw.len(), 7);
    OwnedLeaf {
        name: record.name().unwrap().to_str().unwrap().into(),
        raw,
        before: meta.clone(),
        after: meta,
    }
}
fn evaluate(
    leaves: &[OwnedLeaf],
    retention: &RetainedProducers,
    config: Config,
) -> io::Result<MaintenancePlan> {
    let names = leaves.iter().map(|f| f.name.as_str()).collect::<Vec<_>>();
    let files = leaves
        .iter()
        .map(|f| SuppliedLeaf {
            name: &f.name,
            bytes: &f.raw,
            before: &f.before,
            after: &f.after,
        })
        .collect::<Vec<_>>();
    let root = FileIdentity {
        mode: 0o040000 | 0o700,
        links: 2,
        bytes: 128,
        ..metadata(128, 6)
    };
    plan(
        SuppliedInventory {
            root_before: &root,
            root_after: &root,
            names_before: &names,
            names_after: &names,
            files: &files,
        },
        SuppliedOwner {
            uid: 7001,
            gid: 5000,
        },
        retention,
        config,
    )
}
fn defaults(leaves: &[OwnedLeaf]) -> io::Result<MaintenancePlan> {
    evaluate(leaves, &retention(), Config::default())
}

#[test]
fn actual_record_serializer_and_retained_union_only_propose_unsupported() {
    let result = defaults(&[
        leaf(current(), "1", b"current"),
        leaf(accepted(), "old", b"accepted"),
        leaf(unsupported(), "1", b"unsupported"),
    ])
    .unwrap();
    assert_eq!(result.retained().len(), 2);
    assert_eq!(result.retirement_candidates().len(), 1);
    assert_eq!(result.retirement_candidates()[0].producer(), &unsupported());
    assert!(!result.mutations_authorized());
    assert!(!result.source_adoption_or_root_authority());
    assert!(!result.input_provenance_or_currentness_proven());
}
#[test]
fn old_source_keys_and_same_key_new_content_of_accepted_producers_are_all_kept() {
    let result = defaults(&[
        leaf(accepted(), "old", b"old"),
        leaf(accepted(), "new", b"new"),
        leaf(current(), "1", b"first"),
        leaf(current(), "1", b"second"),
    ])
    .unwrap();
    assert_eq!(result.retained().len(), 4);
    assert!(result.retirement_candidates().is_empty());
}
#[test]
fn multiple_independently_accepted_producers_are_preserved() {
    let (current, selectors, hashes) = profiles(&[accepted(), unsupported()]);
    let retained = RetainedProducers::from_profiles(
        &current,
        &selectors.iter().map(Vec::as_slice).collect::<Vec<_>>(),
        &hashes,
        &hex(&UI),
    )
    .unwrap();
    assert_eq!(retained.versions().len(), 3);
    assert!(evaluate(
        &[leaf(unsupported(), "1", b"now accepted")],
        &retained,
        Config::default()
    )
    .unwrap()
    .retirement_candidates()
    .is_empty());
}
#[test]
fn missing_empty_replaced_or_duplicate_selector_registry_is_refused() {
    let (current, selectors, hashes) = profiles(&[accepted(), unsupported()]);
    let slices = selectors.iter().map(Vec::as_slice).collect::<Vec<_>>();
    for (raw, required) in [
        (&slices[..0], &hashes[..]),
        (&slices[..1], &hashes[..]),
        (&slices[..], &hashes[..1]),
        (&slices[..], &[][..]),
    ] {
        assert!(RetainedProducers::from_profiles(&current, raw, required, &hex(&UI)).is_err());
    }
    assert!(RetainedProducers::from_profiles(
        &current,
        &[slices[0], slices[0]],
        &[hashes[0], hashes[0]],
        &hex(&UI)
    )
    .is_err());
}
#[test]
fn full_producer_identity_not_revision_alone_controls_retention() {
    let mut other = accepted();
    other.ui_sha256 = [0x55; 32];
    assert_eq!(
        defaults(&[leaf(other, "1", b"different UI")])
            .unwrap()
            .retirement_candidates()
            .len(),
        1
    );
}
#[test]
fn current_sidecar_or_selector_for_another_ui_refuses_without_fallback() {
    let (current, selectors, hashes) = profiles(&[accepted()]);
    assert!(
        RetainedProducers::from_profiles(&current, &[&selectors[0]], &hashes, &"0".repeat(64))
            .is_err()
    );
    let old = serde_json::to_vec(&sidecar(&accepted())).unwrap();
    assert!(RetainedProducers::from_profiles(&old, &[&selectors[0]], &hashes, &hex(&UI)).is_err());
}
#[test]
fn child_source_bytes_are_mandatory_in_actual_reader_contract() {
    let value = crate::cache_consumer_profile::record_for_test(&hex(&UI), sidecar(&accepted()));
    assert_eq!(
        value["reader_contract"]["maintenance_planner_sha256"],
        hex(&digest(include_bytes!("durable_icon_cache_maintenance.rs")))
    );
    let current = serde_json::to_vec(&sidecar(&current())).unwrap();
    for change in [false, true] {
        let mut changed = value.clone();
        if change {
            changed["reader_contract"]["maintenance_planner_sha256"] = json!("0".repeat(64));
        } else {
            changed["reader_contract"]
                .as_object_mut()
                .unwrap()
                .remove("maintenance_planner_sha256");
        }
        let raw = serde_json::to_vec(&changed).unwrap();
        assert!(
            RetainedProducers::from_profiles(&current, &[&raw], &[digest(&raw)], &hex(&UI))
                .is_err()
        );
    }
}
#[test]
fn any_current_other_contract_field_change_is_refused() {
    let mut value = crate::cache_consumer_profile::record_for_test(&hex(&UI), sidecar(&accepted()));
    value["reader_contract"]["cargo_lock_sha256"] = json!("0".repeat(64));
    let raw = serde_json::to_vec(&value).unwrap();
    let current = serde_json::to_vec(&sidecar(&current())).unwrap();
    assert!(
        RetainedProducers::from_profiles(&current, &[&raw], &[digest(&raw)], &hex(&UI)).is_err()
    );
}
#[test]
fn actual_schema_duplicate_or_unknown_fields_refuse_all_records() {
    for inserted in ["\"schema\":1,", "\"unknown\":true,"] {
        let mut f = leaf(unsupported(), "1", b"body");
        let size = u32::from_be_bytes(f.raw[8..12].try_into().unwrap()) as usize;
        let mut header = String::from_utf8(f.raw[12..12 + size].to_vec()).unwrap();
        header.insert_str(1, inserted);
        let body = f.raw[12 + size..].to_vec();
        f.raw = MAGIC.to_vec();
        f.raw
            .extend_from_slice(&(header.len() as u32).to_be_bytes());
        f.raw.extend_from_slice(header.as_bytes());
        f.raw.extend_from_slice(&body);
        f.before.bytes = f.raw.len() as u64;
        f.after = f.before.clone();
        assert!(defaults(&[f]).is_err());
    }
}
#[test]
fn unknown_leaf_or_root_pending_name_blocks_whole_proposal() {
    for name in [
        "foreign",
        "artwork-session-pending.json",
        "../outside",
        "000.icon",
    ] {
        let good = leaf(unsupported(), "1", b"good");
        let mut bad = leaf(accepted(), "1", b"bad");
        bad.name = name.into();
        assert!(defaults(&[good, bad]).is_err());
    }
}
#[test]
fn any_empty_ancient_or_complete_writer_slot_is_never_stolen() {
    for data in [b"".as_slice(), b"ancient partial", b"LUMAIC01"] {
        let mut f = leaf(unsupported(), "1", b"body");
        f.name = ".publish-next".into();
        f.raw = data.to_vec();
        f.before.bytes = data.len() as u64;
        f.before.modified_seconds = 0;
        f.after = f.before.clone();
        assert!(defaults(&[f]).is_err());
    }
}
#[test]
fn changed_name_list_or_missing_leaf_cannot_produce_partial_candidates() {
    let root = FileIdentity {
        mode: 0o040000 | 0o700,
        links: 2,
        ..metadata(128, 6)
    };
    let f = leaf(unsupported(), "1", b"body");
    let names = [f.name.as_str()];
    let files = [SuppliedLeaf {
        name: &f.name,
        bytes: &f.raw,
        before: &f.before,
        after: &f.after,
    }];
    let changed = ["other"];
    for (after, rows) in [(&changed[..], &files[..]), (&names[..], &[][..])] {
        assert!(plan(
            SuppliedInventory {
                root_before: &root,
                root_after: &root,
                names_before: &names,
                names_after: after,
                files: rows
            },
            SuppliedOwner {
                uid: 7001,
                gid: 5000
            },
            &retention(),
            Config::default()
        )
        .is_err());
    }
}
#[test]
fn duplicate_names_are_not_double_counted_or_silently_deduplicated() {
    let a = leaf(current(), "1", b"same");
    let b = leaf(current(), "1", b"same");
    assert!(defaults(&[a, b]).is_err());
}
#[test]
fn root_identity_owner_mode_time_or_link_changes_refuse() {
    let root = FileIdentity {
        mode: 0o040000 | 0o700,
        links: 2,
        ..metadata(128, 6)
    };
    for which in 0..5 {
        let mut changed = root.clone();
        match which {
            0 => changed.inode += 1,
            1 => changed.uid = 0,
            2 => changed.mode = 0o040000 | 0o777,
            3 => changed.links += 1,
            _ => changed.changed_nanos += 1,
        }
        assert!(plan(
            SuppliedInventory {
                root_before: &root,
                root_after: &changed,
                names_before: &[],
                names_after: &[],
                files: &[]
            },
            SuppliedOwner {
                uid: 7001,
                gid: 5000
            },
            &retention(),
            Config::default()
        )
        .is_err());
    }
}
#[test]
fn changed_file_metadata_or_special_type_links_and_owner_refuse() {
    for which in 0..6 {
        let mut f = leaf(unsupported(), "1", b"body");
        match which {
            0 => f.after.inode += 1,
            1 => f.after.uid = 0,
            2 => f.after.mode = 0o100000 | 0o777,
            3 => f.after.links = 2,
            4 => f.after.mode = 0o120000 | 0o600,
            _ => f.after.changed_nanos += 1,
        }
        assert!(defaults(&[f]).is_err());
    }
}
#[test]
fn encoded_digest_header_length_and_canonical_filename_must_match() {
    for which in 0..4 {
        let mut f = leaf(unsupported(), "1", b"body");
        match which {
            0 => *f.raw.last_mut().unwrap() ^= 1,
            1 => f.raw[0] ^= 1,
            2 => f.raw[8..12].copy_from_slice(&u32::MAX.to_be_bytes()),
            _ => f.name = format!("{}.icon", "0".repeat(64)),
        }
        assert!(defaults(&[f]).is_err());
    }
}
#[test]
fn entry_and_encoded_capacity_refuse_complete_plan() {
    let files = [
        leaf(current(), "1", b"one"),
        leaf(unsupported(), "1", b"two"),
    ];
    assert!(evaluate(
        &files,
        &retention(),
        Config {
            entries: 1,
            encoded_bytes: 10
        }
    )
    .is_err());
    assert!(evaluate(
        &files,
        &retention(),
        Config {
            entries: 2,
            encoded_bytes: 5
        }
    )
    .is_err());
}
#[test]
fn ordinary_source_identity_0777_is_observed_not_a_root_grant() {
    let result = defaults(&[leaf(unsupported(), "1", b"body")]).unwrap();
    assert_eq!(
        result.retirement_candidates()[0]
            .declared_source_key()
            .version,
        "1"
    );
    assert!(!result.source_adoption_or_root_authority());
}
#[test]
fn actual_profile_parser_bounds_and_duplicate_schema_are_preserved() {
    let (current, selectors, hashes) = profiles(&[accepted()]);
    let mut invalid = String::from_utf8(selectors[0].clone()).unwrap();
    invalid.insert_str(1, "\"schema\":1,");
    let invalid = invalid.into_bytes();
    assert!(RetainedProducers::from_profiles(
        &current,
        &[&invalid],
        &[digest(&invalid)],
        &hex(&UI)
    )
    .is_err());
    let oversized = vec![b' '; 4097];
    assert!(
        RetainedProducers::from_profiles(&oversized, &[&selectors[0]], &hashes, &hex(&UI)).is_err()
    );
    assert!(RetainedProducers::from_profiles(
        &current,
        &[&oversized],
        &[digest(&oversized)],
        &hex(&UI)
    )
    .is_err());
}
