//! Controlled metadata-only cases. Sources come only from the real parser.
use super::*;
use crate::app_artwork::{sources_from_raw, MetadataError, Rejection};
use serde_json::{json, Value};
use std::collections::BTreeSet;

fn rows(count: usize) -> Vec<Value> {
    (0..count).map(|i| {
        let id = format!("fixture.{i:03}");
        json!({"id":id,"title":format!("Fixture {i}"),"type":"native","visible":true,
            "version":"1","folderPath":format!("/usr/palm/applications/{id}"),"icon":"icons/icon.png"})
    }).collect()
}
fn parsed(apps: Vec<Value>) -> SourceMap {
    let points = apps
        .iter()
        .map(|v| json!({"id":v["id"]}))
        .collect::<Vec<_>>();
    sources_from_raw(
        &json!({"replies":{"apps":{"returnValue":true,"apps":apps},
        "launch_points":{"returnValue":true,"launchPoints":points}}}),
        crate::identity::PREVIEW,
    )
    .unwrap()
}
fn catalogue(count: usize) -> SourceMap {
    parsed(rows(count))
}
fn view(map: &SourceMap, stamp: u32) -> Catalogue<'_> {
    Catalogue::new(Some(map), Some(stamp), false, true)
}
fn plan(map: &SourceMap) -> Plan {
    Plan::from_current(view(map, 0), 0).unwrap()
}
fn cursor_value(cursor: &Cursor) -> Value {
    serde_json::to_value(cursor).unwrap()
}
fn supplied_cursor(value: Value) -> Result<Cursor, Error> {
    Cursor::from_json(&serde_json::to_vec(&value).unwrap())
}

#[test]
fn twenty_four_is_exactly_sixteen_then_eight_without_execution_claim() {
    let map = catalogue(24);
    let mut p = plan(&map);
    let first = p.next_batch(view(&map, 0), 0, None).unwrap();
    assert_eq!(first.sources().len(), 16);
    let cursor = first.next_cursor().unwrap();
    assert_eq!(cursor.next_ordinal(), 16);
    let second = p.next_batch(view(&map, 1), 1, Some(cursor)).unwrap();
    assert_eq!(second.sources().len(), 8);
    assert!(second.next_cursor().is_none());
    let all = first
        .sources()
        .iter()
        .chain(second.sources())
        .map(|s| s.key().clone())
        .collect::<Vec<_>>();
    assert_eq!(all.len(), all.iter().collect::<BTreeSet<_>>().len());
    assert_eq!(
        all,
        map.values()
            .filter_map(AppSources::preferred)
            .map(|s| s.key().clone())
            .collect::<Vec<_>>()
    );
    let value = serde_json::to_value(second.manifest()).unwrap();
    assert_eq!(value["eligible_preferred"], 24);
    assert_eq!(value["batch_start"], 16);
    for key in [
        "execution_verified",
        "prepared_coverage_verified",
        "Root_authority_granted",
        "prior_batch_completion_verified",
        "generation_continuity_verified",
    ] {
        assert_eq!(value[key], false);
    }
}
#[test]
fn five_hundred_twelve_is_bounded_planning_not_a_cache_capacity_promise() {
    let map = catalogue(512);
    let mut p = plan(&map);
    let mut cursor = None;
    let mut ids = BTreeSet::new();
    let mut batches = 0;
    loop {
        let batch = p.next_batch(view(&map, 0), 0, cursor.as_ref()).unwrap();
        assert!(batch.sources().len() <= MAX_BATCH);
        for source in batch.sources() {
            assert!(ids.insert(source.app_id().to_owned()));
        }
        let m = serde_json::to_value(batch.manifest()).unwrap();
        assert_eq!(m["default_cache_capacity"]["entries"], 256);
        assert_eq!(
            m["default_cache_capacity"]["encoded_bytes"],
            16 * 1024 * 1024
        );
        assert_eq!(
            m["default_cache_capacity"]["execution_capacity_checked"],
            false
        );
        batches += 1;
        cursor = batch.next_cursor().cloned();
        if cursor.is_none() {
            break;
        }
    }
    assert_eq!(ids.len(), 512);
    assert_eq!(batches, 32);
    let mut too_many = map.clone();
    let extra = parsed(vec![
        json!({"id":"fixture.extra","title":"Extra","type":"native","visible":true,"version":"1","folderPath":"/usr/palm/applications/fixture.extra","icon":"icon.png"}),
    ]);
    too_many.extend(extra);
    assert!(matches!(
        Plan::from_current(view(&too_many, 0), 0),
        Err(Error::TooManySources)
    ));
}
#[test]
fn reused_catalogue_view_expires_on_batch_and_dispatch_at_new_now() {
    let map = catalogue(1);
    let borrowed = view(&map, 0);
    let key = map
        .values()
        .next()
        .unwrap()
        .preferred()
        .unwrap()
        .key()
        .clone();
    let mut p = Plan::from_current(borrowed, 0).unwrap();
    assert!(p.before_dispatch(borrowed, 15_000, &key).is_ok());
    assert!(matches!(
        p.before_dispatch(borrowed, 15_001, &key),
        Err(Error::StaleCatalogue)
    ));
    assert!(p.withdrawn());
    let mut batch_plan = Plan::from_current(borrowed, 0).unwrap();
    assert!(matches!(
        batch_plan.next_batch(borrowed, 15_001, None),
        Err(Error::StaleCatalogue)
    ));
    assert!(matches!(
        batch_plan.next_batch(view(&map, 15_001), 15_001, None),
        Err(Error::Withdrawn)
    ));
}
#[test]
fn wrapping_clock_matches_existing_inclusive_fifteen_second_policy() {
    let map = catalogue(1);
    let stamp = u32::MAX - 9;
    let mut p = Plan::from_current(view(&map, stamp), 5).unwrap();
    assert!(p
        .next_batch(view(&map, stamp), stamp.wrapping_add(15_000), None)
        .is_ok());
    assert!(matches!(
        p.next_batch(view(&map, stamp), stamp.wrapping_add(15_001), None),
        Err(Error::StaleCatalogue)
    ));
    assert!(matches!(
        Plan::from_current(view(&map, 5), 4),
        Err(Error::StaleCatalogue)
    ));
}
#[test]
fn every_full_source_or_preferred_choice_change_withdraws_old_plan() {
    let original = catalogue(2);
    let old_key = original["fixture.000"].preferred().unwrap().key().clone();
    for mutation in 0..6 {
        let mut apps = rows(2);
        match mutation {
            0 => apps[0]["version"] = json!("2"),
            1 => apps[0]["icon"] = json!("icons/replaced.png"),
            2 => apps[0]["largeIcon"] = json!("large.png"),
            3 => {
                apps[0]["folderPath"] =
                    json!("/media/developer/apps/usr/palm/applications/fixture.000")
            }
            4 => {
                apps.remove(0);
            }
            _ => apps[0]["icon"] = Value::Null,
        }
        let changed = parsed(apps);
        let mut p = plan(&original);
        assert_ne!(p.keyset_sha256(), plan(&changed).keyset_sha256());
        assert!(matches!(
            p.before_dispatch(view(&changed, 1), 1, &old_key),
            Err(Error::ChangedCatalogue)
        ));
        assert!(p.withdrawn());
        assert!(matches!(
            p.next_batch(view(&original, 2), 2, None),
            Err(Error::Withdrawn)
        ));
    }
    let mut apps = rows(2);
    apps[0]["largeIcon"] = json!("large.png");
    let large = parsed(apps);
    let mut p = plan(&large);
    assert!(matches!(
        p.next_batch(view(&original, 0), 0, None),
        Err(Error::ChangedCatalogue)
    ));
}
#[test]
fn failed_unavailable_or_simulated_observation_latches_withdrawal_even_after_a_returns() {
    let map = catalogue(1);
    let bad = [
        Catalogue::new(None, Some(0), false, true),
        Catalogue::new(Some(&map), None, false, true),
        Catalogue::new(Some(&map), Some(0), false, false),
        Catalogue::new(Some(&map), Some(0), true, true),
    ];
    for current in bad {
        let mut p = plan(&map);
        assert!(p.next_batch(current, 0, None).is_err());
        assert!(p.withdrawn());
        assert!(matches!(
            p.next_batch(view(&map, 1), 1, None),
            Err(Error::Withdrawn)
        ));
    }
}
#[test]
fn new_equal_metadata_plan_does_not_prove_old_epoch_or_previous_batch_execution() {
    let a = catalogue(24);
    let mut first = plan(&a);
    let cursor = first
        .next_batch(view(&a, 0), 0, None)
        .unwrap()
        .next_cursor()
        .unwrap()
        .clone();
    let b = catalogue(23);
    assert!(first.next_batch(view(&b, 1), 1, Some(&cursor)).is_err());
    let mut fresh = Plan::from_current(view(&a, 2), 2).unwrap();
    // Cursor is ordinary enumeration data; same A may enumerate again, but no
    // completion/generation witness survives or is manufactured by a new Plan.
    let batch = fresh.next_batch(view(&a, 2), 2, Some(&cursor)).unwrap();
    let m = serde_json::to_value(batch.manifest()).unwrap();
    assert_eq!(m["prior_batch_completion_verified"], false);
    assert_eq!(m["generation_continuity_verified"], false);
    assert_eq!(m["execution_verified"], false);
}
#[test]
fn no_icon_and_empty_catalogues_are_enumerable_not_ready_and_bind_identity() {
    let mut apps = rows(3);
    for app in &mut apps {
        app["icon"] = Value::Null;
    }
    let map = parsed(apps);
    let mut p = plan(&map);
    let batch = p.next_batch(view(&map, 0), 0, None).unwrap();
    assert!(batch.sources().is_empty());
    assert!(batch.next_cursor().is_none());
    let m = serde_json::to_value(batch.manifest()).unwrap();
    assert_eq!(m["source_entries"], 3);
    assert_eq!(m["eligible_preferred"], 0);
    assert_eq!(m["no_preferred"], 3);
    assert_eq!(m["metadata_rejections"], 6);
    assert_eq!(m["prepared_coverage_verified"], false);
    assert_ne!(p.keyset_sha256(), plan(&SourceMap::new()).keyset_sha256());
    let mut changed = map.clone();
    changed.get_mut("fixture.000").unwrap().version = Some("2".into());
    assert!(matches!(
        p.next_batch(view(&changed, 1), 1, None),
        Err(Error::ChangedCatalogue)
    ));
}
#[test]
fn map_key_version_variant_and_rejection_incoherence_are_refused_before_hashing() {
    let original = catalogue(1);
    for mutation in 0..5 {
        let mut map = original.clone();
        match mutation {
            0 => map.get_mut("fixture.000").unwrap().app_id = "different".into(),
            1 => map.get_mut("fixture.000").unwrap().version = Some("wrong".into()),
            2 => {
                let e = map.get_mut("fixture.000").unwrap();
                e.large_icon = e.icon.clone();
                e.rejections.retain(|r| r.kind != IconKind::LargeIcon);
            }
            3 => map
                .get_mut("fixture.000")
                .unwrap()
                .rejections
                .push(Rejection {
                    kind: IconKind::Icon,
                    reason: MetadataError::MissingIcon,
                }),
            _ => map.get_mut("fixture.000").unwrap().rejections.extend(vec![
                Rejection {
                    kind: IconKind::LargeIcon,
                    reason: MetadataError::MissingIcon
                };
                3
            ]),
        }
        assert!(matches!(
            Plan::from_current(view(&map, 0), 0),
            Err(Error::IncoherentEntry)
        ));
    }
}
#[test]
fn cursor_shape_keyset_and_ordinal_are_checked_without_granting_execution() {
    let map = catalogue(24);
    let mut p = plan(&map);
    let first = p.next_batch(view(&map, 0), 0, None).unwrap();
    let valid = first.next_cursor().unwrap();
    for value in [
        json!({"schema":2,"keyset_sha256":p.keyset_sha256(),"next_ordinal":16}),
        json!({"schema":1,"keyset_sha256":"A".repeat(64),"next_ordinal":16}),
        json!({"schema":1,"keyset_sha256":p.keyset_sha256(),"next_ordinal":-1}),
        json!({"schema":1,"keyset_sha256":p.keyset_sha256(),"next_ordinal":true}),
        json!({"schema":1,"keyset_sha256":p.keyset_sha256(),"next_ordinal":513}),
        json!({"schema":1,"keyset_sha256":p.keyset_sha256(),"next_ordinal":16,"finished":true}),
    ] {
        assert!(supplied_cursor(value).is_err());
    }
    for (field, value) in [
        ("keyset_sha256", json!("f".repeat(64))),
        ("next_ordinal", json!(32)),
        ("next_ordinal", json!(1)),
    ] {
        let mut data = cursor_value(valid);
        data[field] = value;
        let cursor = supplied_cursor(data).unwrap();
        assert!(matches!(
            p.next_batch(view(&map, 0), 0, Some(&cursor)),
            Err(Error::InvalidCursor)
        ));
    }
    assert!(!p.withdrawn()); // A bad enumeration cursor is not a catalogue event.
    assert_eq!(
        p.next_batch(view(&map, 0), 0, Some(valid))
            .unwrap()
            .sources()
            .len(),
        8
    );
    assert!(Cursor::from_json(&vec![b' '; CURSOR_LIMIT + 1]).is_err());
    let duplicate = format!(
        "{{\"schema\":1,\"schema\":1,\"keyset_sha256\":\"{}\",\"next_ordinal\":16}}",
        p.keyset_sha256()
    );
    assert!(Cursor::from_json(duplicate.as_bytes()).is_err());
}
#[test]
fn dispatch_returns_only_the_exact_current_sealed_preferred_source() {
    let map = catalogue(2);
    let mut p = plan(&map);
    let key = map["fixture.000"].preferred().unwrap().key().clone();
    assert_eq!(
        p.before_dispatch(view(&map, 1), 1, &key).unwrap(),
        map["fixture.000"].preferred().unwrap()
    );
    let other = parsed(vec![
        json!({"id":"fixture.other","title":"Other","type":"native","visible":true,"version":"1","folderPath":"/usr/palm/applications/fixture.other","icon":"other.png"}),
    ]);
    assert!(matches!(
        p.before_dispatch(
            view(&map, 1),
            1,
            other["fixture.other"].preferred().unwrap().key()
        ),
        Err(Error::UnplannedSource)
    ));
    assert!(!p.withdrawn());
}
#[test]
fn unchanged_poll_renews_stamp_without_resetting_batch_or_digest() {
    let map = catalogue(24);
    let mut p = plan(&map);
    let digest = p.keyset_sha256().to_owned();
    let first = p.next_batch(view(&map, 0), 0, None).unwrap();
    let cursor = first.next_cursor().unwrap();
    let second = p
        .next_batch(view(&map, 14_000), 20_000, Some(cursor))
        .unwrap();
    assert_eq!(second.sources().len(), 8);
    assert_eq!(p.keyset_sha256(), digest);
    assert!(!p.withdrawn());
}
