//! Real-file fixtures. No Root/LS2/provider/Ticket or GL calls.
use super::*;
use std::{
    fs,
    os::unix::fs::{symlink, PermissionsExt},
    path::PathBuf,
    sync::atomic::{AtomicU64, Ordering},
};
static SERIAL: AtomicU64 = AtomicU64::new(0);
struct Fixture {
    parent: PathBuf,
    cache: Cache,
}
impl Fixture {
    fn new(config: Config) -> Self {
        let parent = std::env::temp_dir().join(format!(
            "durable-icon-cache-{}-{}",
            std::process::id(),
            SERIAL.fetch_add(1, Ordering::Relaxed)
        ));
        fs::create_dir(&parent).unwrap();
        fs::set_permissions(&parent, fs::Permissions::from_mode(0o700)).unwrap();
        let cache = Cache::open_in(File::open(&parent).unwrap(), PREVIEW_LEAF, config).unwrap();
        Self { parent, cache }
    }
    fn directory(&self) -> PathBuf {
        self.parent.join(PREVIEW_LEAF.to_str().unwrap())
    }
    fn payload(&self, bytes: Vec<u8>) -> EncodedFile {
        let path = self.parent.join("observed-source.png");
        fs::write(&path, &bytes).unwrap();
        let m = fs::metadata(path).unwrap();
        EncodedFile {
            digest: digest(&bytes),
            identity: SourceIdentity {
                device: m.dev(),
                inode: m.ino(),
                bytes: m.len(),
                mode: m.mode(),
                uid: m.uid(),
                gid: m.gid(),
                modified_seconds: m.mtime(),
                modified_nanos: m.mtime_nsec(),
                changed_seconds: m.ctime(),
                changed_nanos: m.ctime_nsec(),
            },
            bytes,
            png_dimensions: Some([1, 1]),
        }
    }
}
impl Drop for Fixture {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.parent);
    }
}
fn source(version: &str, relative: &str, kind: IconKind) -> Source {
    source_in_folder(version, relative, kind, "/usr/palm/applications")
}
fn source_in_folder(version: &str, relative: &str, kind: IconKind, prefix: &str) -> Source {
    let id = "com.example.cache";
    let field = match kind {
        IconKind::Icon => "icon",
        IconKind::LargeIcon => "largeIcon",
    };
    let mut app = serde_json::json!({"id":id,"title":"Cache fixture","type":"native","visible":true,"version":version,"folderPath":format!("{prefix}/{id}")});
    app[field] = relative.into();
    let raw = serde_json::json!({"replies":{"apps":{"returnValue":true,"apps":[app]},"launch_points":{"returnValue":true,"launchPoints":[{"id":id}]}}});
    crate::app_artwork::sources_from_raw(&raw, crate::identity::PREVIEW).unwrap()[id]
        .preferred()
        .unwrap()
        .clone()
}
fn producer() -> ProducerVersion {
    ProducerVersion::new(&"a".repeat(40), [1; 32], [2; 32]).unwrap()
}
fn png() -> Vec<u8> {
    vec![
        137, 80, 78, 71, 13, 10, 26, 10, 0, 0, 0, 13, 73, 72, 68, 82, 0, 0, 0, 2, 0, 0, 0, 1, 8, 6,
        0, 0, 0, 244, 34, 127, 138, 0, 0, 0, 17, 73, 68, 65, 84, 120, 156, 99, 224, 18, 145, 99,
        208, 48, 178, 105, 0, 0, 3, 199, 1, 83, 206, 227, 180, 69, 0, 0, 0, 0, 73, 69, 78, 68, 174,
        66, 96, 130,
    ]
}
fn demand(snapshot: &Snapshot, source: &Source) -> Demand {
    match snapshot.lookup(source, &producer()) {
        Lookup::Ready(x) => *x,
        _ => panic!("Expected unique cache entry"),
    }
}
fn publish(f: &mut Fixture, source: &Source) {
    let payload = f.payload(png());
    f.cache.publish(source, &payload, producer()).unwrap();
}
#[test]
fn actual_png_roundtrip_reopen_preserves_alpha_aspect_and_complete_association() {
    let mut f = Fixture::new(Config::default());
    let src = source("1", "icon.png", IconKind::Icon);
    publish(&mut f, &src);
    let cache = Cache::open_in(
        File::open(&f.parent).unwrap(),
        PREVIEW_LEAF,
        Config::default(),
    )
    .unwrap();
    let snapshot = cache.snapshot().unwrap();
    let cached = snapshot.decode(demand(&snapshot, &src)).unwrap();
    assert_eq!(cached.observed_source_identity().bytes, png().len() as u64);
    let image = cached
        .admit(&snapshot.generation(), &src, &producer())
        .unwrap();
    assert_eq!((image.width, image.height), (2, 1));
    assert_eq!(image.rgba, [10, 20, 30, 0, 40, 50, 60, 128]);
    assert!(image.encoded_identity.bytes > png().len() as u64);
    assert_eq!(cache.stats().unwrap().entries, 1);
}
#[test]
fn source_version_path_kind_or_producer_change_never_borrows_an_id_only_hit() {
    let mut f = Fixture::new(Config::default());
    let src = source("1", "icon.png", IconKind::Icon);
    publish(&mut f, &src);
    let snapshot = f.cache.snapshot().unwrap();
    for changed in [
        source("2", "icon.png", IconKind::Icon),
        source("1", "other.png", IconKind::Icon),
        source("1", "icon.png", IconKind::LargeIcon),
        source_in_folder(
            "1",
            "icon.png",
            IconKind::Icon,
            "/media/developer/apps/usr/palm/applications",
        ),
    ] {
        assert!(matches!(
            snapshot.lookup(&changed, &producer()),
            Lookup::Missing
        ));
    }
    let other = ProducerVersion::new(&"b".repeat(40), [1; 32], [2; 32]).unwrap();
    assert!(matches!(snapshot.lookup(&src, &other), Lookup::Missing));
}
#[test]
fn withdraw_and_identical_re_adoption_reject_old_demands_and_late_pixels() {
    let mut f = Fixture::new(Config::default());
    let src = source("1", "icon.png", IconKind::Icon);
    publish(&mut f, &src);
    let snapshot = f.cache.snapshot().unwrap();
    let old = demand(&snapshot, &src);
    let cached = snapshot.decode(demand(&snapshot, &src)).unwrap();
    let adopted = f.cache.snapshot().unwrap();
    assert_eq!(
        adopted.generation().content_id(),
        snapshot.generation().content_id()
    );
    assert!(adopted.decode(old).is_err());
    assert!(cached
        .admit(&adopted.generation(), &src, &producer())
        .is_none());
    let late = snapshot.decode(demand(&snapshot, &src)).unwrap();
    snapshot.generation().withdraw();
    assert!(matches!(
        snapshot.lookup(&src, &producer()),
        Lookup::Missing
    ));
    assert!(late
        .admit(&snapshot.generation(), &src, &producer())
        .is_none());
}
#[test]
fn same_key_new_content_is_ambiguous_until_explicit_retirement_and_new_adoption() {
    let mut f = Fixture::new(Config::default());
    let src = source("1", "icon.png", IconKind::Icon);
    publish(&mut f, &src);
    let old = f.cache.snapshot().unwrap();
    let old_demand = demand(&old, &src);
    let mut data = png();
    data[44] ^= 1;
    let payload = f.payload(data);
    f.cache.publish(&src, &payload, producer()).unwrap();
    let ambiguous = f.cache.snapshot().unwrap();
    assert!(matches!(
        ambiguous.lookup(&src, &producer()),
        Lookup::Ambiguous
    ));
    old.generation().withdraw();
    ambiguous.generation().withdraw();
    f.cache.retire_after_join(old_demand).unwrap();
    let fresh = f.cache.snapshot().unwrap();
    assert!(old.decode(demand_unchecked(&old)).is_err());
    assert!(matches!(fresh.lookup(&src, &producer()), Lookup::Ready(_)));
}
fn demand_unchecked(snapshot: &Snapshot) -> Demand {
    Demand {
        root: snapshot.root.clone(),
        entry: snapshot.entries[0].clone(),
        generation: snapshot.generation(),
    }
}
#[test]
fn bounded_entry_and_encoded_budgets_refuse_growth_without_overwriting_existing() {
    let length = png().len() as u64;
    let mut f = Fixture::new(Config {
        entries: 1,
        encoded_bytes: length,
    });
    let a = source("1", "a.png", IconKind::Icon);
    publish(&mut f, &a);
    let payload = f.payload(png());
    assert!(f
        .cache
        .publish(&source("1", "b.png", IconKind::Icon), &payload, producer())
        .is_err());
    assert_eq!(
        f.cache.stats().unwrap(),
        Stats {
            entries: 1,
            encoded_bytes: length
        }
    );
    assert!(!f.directory().join(TEMP.to_str().unwrap()).exists());
    let snapshot = f.cache.snapshot().unwrap();
    assert!(snapshot.decode(demand(&snapshot, &a)).is_ok());
}
#[test]
fn symlink_fifo_sparse_oversize_and_named_replacement_never_reach_decoder() {
    for kind in 0..4 {
        let mut f = Fixture::new(Config::default());
        let src = source("1", "icon.png", IconKind::Icon);
        publish(&mut f, &src);
        let snapshot = f.cache.snapshot().unwrap();
        let job = demand(&snapshot, &src);
        let path = f.directory().join(job.entry.name.to_str().unwrap());
        match kind {
            0 => {
                fs::remove_file(&path).unwrap();
                symlink(f.parent.join("observed-source.png"), &path).unwrap();
            }
            1 => {
                fs::remove_file(&path).unwrap();
                let name = CString::new(path.as_os_str().as_encoded_bytes()).unwrap();
                assert_eq!(unsafe { libc::mkfifo(name.as_ptr(), 0o600) }, 0);
            }
            2 => {
                File::options()
                    .write(true)
                    .open(&path)
                    .unwrap()
                    .set_len(MAX_FILE + 1)
                    .unwrap();
            }
            _ => {
                let other = f.directory().join("replacement");
                fs::write(&other, fs::read(&path).unwrap()).unwrap();
                fs::set_permissions(&other, fs::Permissions::from_mode(0o600)).unwrap();
                fs::rename(other, &path).unwrap();
            }
        }
        let mut decoded = false;
        assert!(snapshot
            .decode_using(job, |_| {
                decoded = true;
                None
            })
            .is_err());
        assert!(!decoded);
    }
}
#[test]
fn in_read_same_length_write_growth_and_named_inode_replacement_are_rejected() {
    for kind in 0..3 {
        let mut f = Fixture::new(Config::default());
        let src = source("1", "icon.png", IconKind::Icon);
        publish(&mut f, &src);
        let snapshot = f.cache.snapshot().unwrap();
        let entry = snapshot.entries[0].clone();
        let path = f.directory().join(entry.name.to_str().unwrap());
        let got = read_entry(&snapshot.root, &entry, || match kind {
            0 => {
                let mut bytes = fs::read(&path).unwrap();
                *bytes.last_mut().unwrap() ^= 1;
                fs::write(&path, bytes).unwrap();
                File::options()
                    .write(true)
                    .open(&path)
                    .unwrap()
                    .set_times(std::fs::FileTimes::new().set_modified(
                        std::time::SystemTime::UNIX_EPOCH + std::time::Duration::from_secs(1),
                    ))
                    .unwrap();
            }
            1 => {
                File::options()
                    .write(true)
                    .open(&path)
                    .unwrap()
                    .set_len(entry.identity.bytes + 1)
                    .unwrap();
            }
            _ => {
                let replacement = f.directory().join("replacement");
                fs::write(&replacement, fs::read(&path).unwrap()).unwrap();
                fs::set_permissions(&replacement, fs::Permissions::from_mode(0o600)).unwrap();
                fs::rename(replacement, &path).unwrap();
            }
        });
        assert!(got.is_err());
    }
}
#[test]
fn root_replacement_and_unsafe_existing_permissions_refuse_without_repair() {
    let mut f = Fixture::new(Config::default());
    let src = source("1", "icon.png", IconKind::Icon);
    publish(&mut f, &src);
    let snapshot = f.cache.snapshot().unwrap();
    let renamed = f.parent.join("old-cache");
    fs::rename(f.directory(), &renamed).unwrap();
    fs::create_dir(f.directory()).unwrap();
    fs::set_permissions(f.directory(), fs::Permissions::from_mode(0o700)).unwrap();
    assert!(snapshot.decode(demand(&snapshot, &src)).is_err());
    fs::set_permissions(f.directory(), fs::Permissions::from_mode(0o777)).unwrap();
    assert!(Cache::open_in(
        File::open(&f.parent).unwrap(),
        PREVIEW_LEAF,
        Config::default()
    )
    .is_err());
    assert_eq!(fs::metadata(f.directory()).unwrap().mode() & 0o7777, 0o777);
}
#[test]
fn ordinary_cache_operations_never_touch_adjacent_preferences_or_pending_marker() {
    let mut f = Fixture::new(Config::default());
    let prefs = f.parent.join(".com.ash.nativehome.preview-state");
    fs::create_dir(&prefs).unwrap();
    fs::write(prefs.join("preferences.json"), b"existing favourite bytes").unwrap();
    fs::write(
        prefs.join("artwork-session-pending.json"),
        b"unknown durable Root barrier",
    )
    .unwrap();
    let src = source("1", "icon.png", IconKind::Icon);
    publish(&mut f, &src);
    let snapshot = f.cache.snapshot().unwrap();
    let entry = demand(&snapshot, &src);
    assert!(f.cache.retire_after_join(entry).is_err());
    let entry = demand(&snapshot, &src);
    snapshot.generation().withdraw();
    f.cache.retire_after_join(entry).unwrap();
    assert_eq!(
        fs::read(prefs.join("preferences.json")).unwrap(),
        b"existing favourite bytes"
    );
    assert_eq!(
        fs::read(prefs.join("artwork-session-pending.json")).unwrap(),
        b"unknown durable Root barrier"
    );
}
#[test]
fn invalid_encoded_or_association_data_is_refused_and_digests_are_checked_on_read() {
    let mut f = Fixture::new(Config::default());
    let src = source("1", "icon.png", IconKind::Icon);
    let mut payload = f.payload(png());
    payload.digest = [0; 32];
    assert!(f.cache.publish(&src, &payload, producer()).is_err());
    assert_eq!(f.cache.stats().unwrap().entries, 0);
    let payload = f.payload(png());
    f.cache.publish(&src, &payload, producer()).unwrap();
    let snapshot = f.cache.snapshot().unwrap();
    let entry = &snapshot.entries[0];
    let path = f.directory().join(entry.name.to_str().unwrap());
    let mut bytes = fs::read(&path).unwrap();
    *bytes.last_mut().unwrap() ^= 1;
    fs::write(&path, bytes).unwrap();
    let adopted = f.cache.snapshot().unwrap();
    let mut decoder = false;
    assert!(adopted
        .decode_using(demand(&adopted, &src), |_| {
            decoder = true;
            None
        })
        .is_err());
    assert!(!decoder);
}

#[test]
fn malformed_duplicate_unknown_and_oversize_headers_never_become_entries() {
    for kind in 0..5 {
        let mut f = Fixture::new(Config::default());
        let src = source("1", "icon.png", IconKind::Icon);
        publish(&mut f, &src);
        let snapshot = f.cache.snapshot().unwrap();
        let entry = snapshot.entries[0].clone();
        let original = f.directory().join(entry.name.to_str().unwrap());
        fs::remove_file(original).unwrap();
        let mut record = entry.record;
        let mut header = match kind {
            0 => {
                record.source.relative[0] = "..".into();
                serde_json::to_vec(&record).unwrap()
            }
            1 => {
                record.source_identity.modified_nanos = 1_000_000_000;
                serde_json::to_vec(&record).unwrap()
            }
            2 => {
                record.schema = 2;
                serde_json::to_vec(&record).unwrap()
            }
            3 => {
                let text = String::from_utf8(serde_json::to_vec(&record).unwrap()).unwrap();
                format!("{{\"schema\":1,{}", &text[1..]).into_bytes()
            }
            _ => {
                let mut value = serde_json::to_value(&record).unwrap();
                value["unknown"] = true.into();
                serde_json::to_vec(&value).unwrap()
            }
        };
        let leaf = format!("{}.icon", hex(&digest(&header)));
        let mut bytes = MAGIC.to_vec();
        bytes.extend_from_slice(&(header.len() as u32).to_be_bytes());
        bytes.append(&mut header);
        bytes.extend(png());
        let path = f.directory().join(leaf);
        fs::write(&path, bytes).unwrap();
        fs::set_permissions(path, fs::Permissions::from_mode(0o600)).unwrap();
        assert!(f.cache.snapshot().is_err());
    }
    let f = Fixture::new(Config::default());
    let path = f.directory().join(format!("{}.icon", "0".repeat(64)));
    let mut bytes = MAGIC.to_vec();
    bytes.extend_from_slice(&((HEADER_CAP + 1) as u32).to_be_bytes());
    fs::write(&path, bytes).unwrap();
    fs::set_permissions(path, fs::Permissions::from_mode(0o600)).unwrap();
    assert!(f.cache.snapshot().is_err());
}
#[test]
fn actual_codec_rejects_malformed_encoded_data_and_returned_axis_budget() {
    let mut f = Fixture::new(Config::default());
    let src = source("1", "icon.png", IconKind::Icon);
    let payload = f.payload(b"not an image".to_vec());
    f.cache.publish(&src, &payload, producer()).unwrap();
    let snapshot = f.cache.snapshot().unwrap();
    assert!(snapshot.decode(demand(&snapshot, &src)).is_err());
    let mut f = Fixture::new(Config::default());
    publish(&mut f, &src);
    let snapshot = f.cache.snapshot().unwrap();
    assert!(snapshot
        .decode_using(demand(&snapshot, &src), |_| Some((
            MAX_AXIS + 1,
            1,
            vec![0; ((MAX_AXIS + 1) * 4) as usize]
        )))
        .is_err());
}

#[test]
fn byte_budget_and_entry_budget_are_enforced_independently() {
    for config in [
        Config {
            entries: 1,
            encoded_bytes: 2 * png().len() as u64,
        },
        Config {
            entries: 2,
            encoded_bytes: png().len() as u64,
        },
    ] {
        let mut f = Fixture::new(config);
        let first = source("1", "first.png", IconKind::Icon);
        publish(&mut f, &first);
        let encoded = f.payload(png());
        assert!(f
            .cache
            .publish(
                &source("1", "second.png", IconKind::Icon),
                &encoded,
                producer()
            )
            .is_err());
        assert_eq!(f.cache.stats().unwrap().entries, 1);
        assert!(!f.directory().join(TEMP.to_str().unwrap()).exists());
    }
}
#[test]
fn unrelated_new_entry_does_not_repoint_old_snapshot_or_poison_held_root() {
    let mut f = Fixture::new(Config::default());
    let first = source("1", "first.png", IconKind::Icon);
    publish(&mut f, &first);
    let old = f.cache.snapshot().unwrap();
    let second = source("1", "second.png", IconKind::Icon);
    publish(&mut f, &second);
    assert!(matches!(old.lookup(&second, &producer()), Lookup::Missing));
    assert!(old.decode(demand(&old, &first)).is_ok());
    let new = f.cache.snapshot().unwrap();
    assert!(matches!(new.lookup(&second, &producer()), Lookup::Ready(_)));
}

#[test]
fn concurrent_module_publishers_do_not_use_stale_capacity_scans() {
    let f = Fixture::new(Config {
        entries: 1,
        encoded_bytes: 2 * png().len() as u64,
    });
    let barrier = Arc::new(std::sync::Barrier::new(3));
    let mut handles = Vec::new();
    for relative in ["first.png", "second.png"] {
        let mut cache = Cache::open_in(
            File::open(&f.parent).unwrap(),
            PREVIEW_LEAF,
            Config {
                entries: 1,
                encoded_bytes: 2 * png().len() as u64,
            },
        )
        .unwrap();
        let encoded = f.payload(png());
        let src = source("1", relative, IconKind::Icon);
        let go = barrier.clone();
        handles.push(std::thread::spawn(move || {
            go.wait();
            cache.publish(&src, &encoded, producer()).is_ok()
        }));
    }
    barrier.wait();
    let successes = handles
        .into_iter()
        .filter_map(|h| h.join().ok())
        .filter(|ok| *ok)
        .count();
    assert_eq!(successes, 1);
    assert_eq!(f.cache.stats().unwrap().entries, 1);
    assert!(!f.directory().join(TEMP.to_str().unwrap()).exists());
}

#[test]
fn existing_temp_is_not_adopted_or_deleted_by_another_writer() {
    let mut f = Fixture::new(Config::default());
    let path = f.directory().join(TEMP.to_str().unwrap());
    fs::write(&path, b"interrupted ordinary publication").unwrap();
    fs::set_permissions(&path, fs::Permissions::from_mode(0o600)).unwrap();
    assert_eq!(f.cache.snapshot().unwrap().entries.len(), 0);
    let encoded = f.payload(png());
    assert!(f
        .cache
        .publish(
            &source("1", "icon.png", IconKind::Icon),
            &encoded,
            producer()
        )
        .is_err());
    assert_eq!(fs::read(path).unwrap(), b"interrupted ordinary publication");
}

#[test]
fn unknown_entries_and_overlimit_inventory_refuse_without_deletion() {
    let f = Fixture::new(Config {
        entries: 1,
        encoded_bytes: 1024,
    });
    for (name, bytes) in [
        ("unknown", b"unknown data".as_slice()),
        ("second", b"second".as_slice()),
        ("third", b"third".as_slice()),
    ] {
        fs::write(f.directory().join(name), bytes).unwrap();
        assert!(f.cache.snapshot().is_err());
        assert_eq!(fs::read(f.directory().join(name)).unwrap(), bytes);
    }
}

#[test]
fn cache_directory_symlink_and_metadata_changes_refuse_without_repair() {
    let f = Fixture::new(Config::default());
    let moved = f.parent.join("old-cache");
    fs::rename(f.directory(), &moved).unwrap();
    symlink(&moved, f.directory()).unwrap();
    assert!(f.cache.snapshot().is_err());
    assert!(Cache::open_in(
        File::open(&f.parent).unwrap(),
        PREVIEW_LEAF,
        Config::default()
    )
    .is_err());
    fs::remove_file(f.directory()).unwrap();
    fs::rename(&moved, f.directory()).unwrap();
    fs::set_permissions(f.directory(), fs::Permissions::from_mode(0o750)).unwrap();
    assert!(f.cache.snapshot().is_err());
    assert_eq!(fs::metadata(f.directory()).unwrap().mode() & 0o7777, 0o750);
}

#[test]
fn stable_hardlinked_regular_cache_data_is_readable_but_later_link_and_mode_changes_are_not() {
    let mut f = Fixture::new(Config::default());
    let src = source("1", "icon.png", IconKind::Icon);
    publish(&mut f, &src);
    let original = f.cache.snapshot().unwrap();
    let path = f
        .directory()
        .join(original.entries[0].name.to_str().unwrap());
    let outside_link = f.parent.join("ordinary-extra-link");
    fs::hard_link(&path, &outside_link).unwrap();
    assert!(original.decode(demand(&original, &src)).is_err());
    let linked = f.cache.snapshot().unwrap();
    assert_eq!(linked.entries[0].identity.links, 2);
    assert!(linked.decode(demand(&linked, &src)).is_ok());
    fs::set_permissions(&path, fs::Permissions::from_mode(0o644)).unwrap();
    assert!(linked.decode(demand(&linked, &src)).is_err());
    assert!(f.cache.snapshot().is_err());
    assert_eq!(fs::metadata(path).unwrap().mode() & 0o7777, 0o644);
}

#[test]
fn truncated_body_and_wrong_held_owner_stamp_do_not_reach_decoder() {
    let mut f = Fixture::new(Config::default());
    let src = source("1", "icon.png", IconKind::Icon);
    publish(&mut f, &src);
    let snapshot = f.cache.snapshot().unwrap();
    let path = f
        .directory()
        .join(snapshot.entries[0].name.to_str().unwrap());
    File::options()
        .write(true)
        .open(path)
        .unwrap()
        .set_len(snapshot.entries[0].identity.bytes - 1)
        .unwrap();
    let mut decoded = false;
    assert!(snapshot
        .decode_using(demand(&snapshot, &src), |_| {
            decoded = true;
            None
        })
        .is_err());
    assert!(!decoded);
    let mut held = Cache::open_in(
        File::open(&f.parent).unwrap(),
        PREVIEW_LEAF,
        Config::default(),
    )
    .unwrap();
    Arc::get_mut(&mut held.root).unwrap().anchor.uid = unsafe { libc::geteuid() }.wrapping_add(1);
    assert!(held.snapshot().is_err());
}

#[test]
fn live_writer_slot_serializes_mutations_and_is_never_age_stolen() {
    let mut f = Fixture::new(Config::default());
    let src = source("1", "icon.png", IconKind::Icon);
    publish(&mut f, &src);
    let snapshot = f.cache.snapshot().unwrap();
    snapshot.generation().withdraw();
    let mut another = Cache::open_in(
        File::open(&f.parent).unwrap(),
        PREVIEW_LEAF,
        Config::default(),
    )
    .unwrap();
    // Share the same held root as the originating Cache, exercising the slot
    // rather than the independent-root retirement rejection.
    another.root = f.cache.root.clone();
    let mut slot = f.cache.writer_slot().unwrap();
    slot.file.write_all(b"held writer").unwrap();
    slot.file
        .set_times(std::fs::FileTimes::new().set_modified(std::time::SystemTime::UNIX_EPOCH))
        .unwrap();
    let bytes = f.payload(png());
    assert!(another
        .publish(&source("2", "icon.png", IconKind::Icon), &bytes, producer())
        .is_err());
    assert!(another
        .retire_after_join(demand_unchecked(&snapshot))
        .is_err());
    assert_eq!(
        fs::read(f.directory().join(TEMP.to_str().unwrap())).unwrap(),
        b"held writer"
    );
    drop(slot);
    assert!(!f.directory().join(TEMP.to_str().unwrap()).exists());
    another
        .publish(&source("2", "icon.png", IconKind::Icon), &bytes, producer())
        .unwrap();
    let adopted = another.snapshot().unwrap();
    assert!(matches!(
        adopted.lookup(&src, &producer()),
        Lookup::Ready(_)
    ));
    assert!(matches!(
        adopted.lookup(&source("2", "icon.png", IconKind::Icon), &producer()),
        Lookup::Ready(_)
    ));
}

#[test]
fn changed_writer_temp_is_retained_instead_of_unlinked_by_drop() {
    let f = Fixture::new(Config::default());
    let mut slot = f.cache.writer_slot().unwrap();
    slot.file.write_all(b"uncertain metadata").unwrap();
    let path = f.directory().join(TEMP.to_str().unwrap());
    fs::set_permissions(&path, fs::Permissions::from_mode(0o640)).unwrap();
    drop(slot);
    assert_eq!(fs::read(&path).unwrap(), b"uncertain metadata");
    assert_eq!(fs::metadata(&path).unwrap().mode() & 0o7777, 0o640);
    assert!(f.cache.snapshot().is_err());
}

#[test]
fn actual_cached_decode_scales_patterns_preserves_source_association_and_withdrawal() {
    use crate::icon_decode_test_support::{assert_scaled_pattern, patterned_png};
    for (width, height) in [(640, 160), (160, 640)] {
        let mut f = Fixture::new(Config::default());
        let src = source("1", "pattern.png", IconKind::Icon);
        let bytes = patterned_png(width, height);
        let original_digest = digest(&bytes);
        let mut payload = f.payload(bytes.clone());
        payload.png_dimensions = Some([width, height]);
        let identity = payload.identity.clone();
        f.cache.publish(&src, &payload, producer()).unwrap();
        let snapshot = f.cache.snapshot().unwrap();
        let generation = snapshot.generation();
        let cached = snapshot.decode(demand(&snapshot, &src)).unwrap();
        assert_eq!(cached.observed_source_identity(), identity);
        assert_eq!(cached.record.encoded_sha256, original_digest);
        let selected = snapshot.lookup(&src, &producer());
        assert!(matches!(selected, Lookup::Ready(_)));
        let decoded = cached.admit(&generation, &src, &producer()).unwrap();
        assert_scaled_pattern(decoded.width, decoded.height, &decoded.rgba);
        assert!(decoded.rgba.capacity() <= MAX_RGBA_BYTES);
        // The persisted record still describes original encoded bytes, not the
        // smaller RGBA result; independently checked read_entry rehashes body.
        let cached_again = snapshot.decode(demand(&snapshot, &src)).unwrap();
        assert_eq!(
            cached_again.observed_source_identity().bytes,
            bytes.len() as u64
        );
        assert_eq!(cached_again.record.encoded_sha256, original_digest);
        assert_eq!(f.cache.stats().unwrap().encoded_bytes, bytes.len() as u64);
        generation.withdraw();
        assert!(cached_again.admit(&generation, &src, &producer()).is_none());
    }
}
#[test]
fn actual_cached_decode_refuses_valid_complete_source_over_input_axis() {
    let mut f = Fixture::new(Config::default());
    let src = source("1", "large.png", IconKind::Icon);
    let bytes = crate::icon_decode_test_support::patterned_png(2049, 1);
    assert!(plx_gfx::img::img_decode_owned_bounded(&bytes, 2049, 1024 * 1024).is_some());
    let mut payload = f.payload(bytes);
    payload.png_dimensions = Some([2049, 1]);
    f.cache.publish(&src, &payload, producer()).unwrap();
    let snapshot = f.cache.snapshot().unwrap();
    assert!(matches!(
        snapshot.lookup(&src, &producer()),
        Lookup::Ready(_)
    ));
    assert!(snapshot.decode(demand(&snapshot, &src)).is_err());
}
