use super::*;
use serde_json::json;

const SHA: &str = "0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef";
fn card(index: usize) -> Value {
    let id = format!("test.synthetic.{index}");
    json!({"id":id,"source_key":{"app_id":id,"version":"1.0","class":"developer_apps",
        "folder":format!("/media/developer/apps/usr/palm/applications/{id}"),"relative":["icons","tile.png"],"kind":"large_icon"},
        "rect":[12.0,40.0,100.0,100.0],"clip":[0.0,0.0,1920.0,1080.0],
        "focused":false,"texture_handle":index as u64+1,"width":512,"height":512})
}
fn frame(cards: Vec<Value>, total: usize) -> Value {
    json!({"schema":1,"kind":"ordinary-cache-frame-v1","page":"apps",
        "catalogue_keyset_sha256":SHA,"eligible_preferred":total,
        "source_observed_ms":100,"source_sample_now_ms":200,"source_age_ms":100,
        "cached_cards":cards,"cache_readiness":"Ready","cache_readiness_is_index_not_images":true,
        "cache_uploads":512,"packaged_uploads":7,"cache_adoptions":1,"texture_entries":64,
        "Root_authority_qualified":false,"positive_Root_producer_qualified":false,
        "native_foreground_scene_qualified":false,"physical_panel_qualified":false,
        "cleanup_qualified":false,"performance_qualified":false,"continuous_source_file_freshness_qualified":false})
}
fn ledger(total: usize) -> Ledger {
    let mut ledger = Ledger::new();
    ledger.catalogue(Some((SHA, total, 1)));
    ledger
}

#[test]
fn multiple_viewports_store_first_full_key_observations_not_aggregate_uploads() {
    let mut ledger = ledger(24);
    ledger.observe(&frame((0..12).map(card).collect(), 24), 1, 10);
    ledger.observe(&frame((8..20).map(card).collect(), 24), 2, 20);
    ledger.observe(&frame((20..24).map(card).collect(), 24), 3, 30);
    ledger.observe(&frame(vec![card(0)], 24), 4, 40);
    ledger.catalogue(Some((SHA, 24, 1)));
    let report = ledger.finish();
    assert!(report.history_valid && report.observed_count_matches_eligible);
    assert_eq!(report.observed_distinct_sources, 24);
    assert_eq!(
        report
            .observations
            .iter()
            .find(|r| r.source_key.app_id == "test.synthetic.0")
            .unwrap()
            .draw_sequence,
        1
    );
    assert!(
        !report.all_catalogue_qualified
            && !report.pixels_qualified
            && !report.current_texture_residency_qualified
    );
    assert_eq!(report.frames_accepted, 4);
}

#[test]
fn entire_bad_frame_is_transactional_and_late_valid_frames_cannot_heal() {
    let mut ledger = ledger(3);
    ledger.observe(&frame(vec![card(0)], 3), 1, 10);
    let mut bad = card(2);
    bad["width"] = json!(513);
    ledger.observe(&frame(vec![card(1), bad], 3), 2, 20);
    ledger.catalogue(Some((SHA, 3, 1)));
    ledger.observe(&frame(vec![card(1), card(2)], 3), 3, 30);
    let report = ledger.finish();
    assert_eq!(report.observed_distinct_sources, 1);
    assert_eq!(report.frames_accepted, 1);
    assert_eq!(report.incomplete_reason, Some(Incomplete::InvalidFrame));
    assert!(!report.history_valid);
}

#[test]
fn duplicate_key_or_handle_in_one_frame_is_not_partial_success() {
    for cards in [vec![card(0), card(0)], {
        let mut second = card(1);
        second["texture_handle"] = json!(1);
        vec![card(0), second]
    }] {
        let mut ledger = ledger(2);
        ledger.observe(&frame(cards, 2), 1, 10);
        let report = ledger.finish();
        assert_eq!(report.frames_accepted, 0);
        assert!(report.observations.is_empty());
        assert_eq!(report.incomplete_reason, Some(Incomplete::InvalidFrame));
    }
}

#[test]
fn catalogue_loss_changes_and_final_only_adoption_change_preserve_partial_history() {
    for change in [
        None,
        Some((SHA, 3, 1)),
        Some((SHA, 2, 2)),
        Some((
            "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa",
            2,
            1,
        )),
    ] {
        let mut ledger = ledger(2);
        ledger.observe(&frame(vec![card(0)], 2), 1, 10);
        ledger.catalogue(change);
        ledger.catalogue(Some((SHA, 2, 1)));
        ledger.observe(&frame(vec![card(1)], 2), 2, 20);
        let report = ledger.finish();
        assert!(!report.history_valid);
        assert_eq!(report.observed_distinct_sources, 1);
        assert!(!report.observed_count_matches_eligible);
    }
}

#[test]
fn initial_unavailable_or_zero_adoption_waits_but_is_not_empty_success() {
    let mut ledger = Ledger::default();
    ledger.catalogue(None);
    ledger.catalogue(Some((SHA, 2, 0)));
    ledger.catalogue(Some((SHA, 2, 1)));
    ledger.observe(&frame(vec![card(0)], 2), 1, 1);
    assert!(ledger.finish().history_valid);
    let mut empty = Ledger::new();
    empty.catalogue(Some((SHA, 0, 1)));
    let report = empty.finish();
    assert_eq!(report.incomplete_reason, Some(Incomplete::NoObservations));
    assert!(!report.observed_count_matches_eligible);
}

#[test]
fn source_age_wrap_is_valid_but_stale_and_inconsistent_age_are_not() {
    let mut wrapped = frame(vec![card(0)], 1);
    wrapped["source_observed_ms"] = json!(u32::MAX - 4);
    wrapped["source_sample_now_ms"] = json!(5);
    wrapped["source_age_ms"] = json!(10);
    let mut good = ledger(1);
    good.observe(&wrapped, 1, 1);
    assert!(good.finish().history_valid);
    for (age, expected) in [
        (15001, Incomplete::StaleSource),
        (99, Incomplete::InvalidFrame),
    ] {
        let mut bad = frame(vec![card(0)], 1);
        bad["source_age_ms"] = json!(age);
        if age == 15001 {
            bad["source_sample_now_ms"] = json!(15101);
        }
        let mut ledger = ledger(1);
        ledger.observe(&bad, 1, 1);
        assert_eq!(ledger.finish().incomplete_reason, Some(expected));
    }
}

#[test]
fn malformed_geometry_flags_paths_and_typed_numbers_fail_closed() {
    let cases = [
        ("rect", json!([0, 0, 0, 50])),
        ("clip", json!([200, 200, 50, 50])),
        ("rect", json!([0, 0, 1e300, 50])),
        ("height", json!(true)),
        ("height", json!(0)),
        ("texture_handle", json!(0)),
        ("focused", json!(1)),
    ];
    for (field, value) in cases {
        let mut row = card(0);
        row[field] = value;
        let mut ledger = ledger(1);
        ledger.observe(&frame(vec![row], 1), 1, 1);
        assert_eq!(
            ledger.finish().incomplete_reason,
            Some(Incomplete::InvalidFrame),
            "{field}"
        );
    }
    for field in [
        "Root_authority_qualified",
        "cache_readiness_is_index_not_images",
        "source_age_ms",
    ] {
        let mut bad = frame(vec![card(0)], 1);
        bad[field] = json!(1);
        let mut ledger = ledger(1);
        ledger.observe(&bad, 1, 1);
        assert!(!ledger.finish().history_valid);
    }
    for field in ["folder", "relative"] {
        let mut row = card(0);
        row["source_key"][field] = if field == "folder" {
            json!("/tmp/foreign")
        } else {
            json!(["..", "tile.png"])
        };
        let mut ledger = ledger(1);
        ledger.observe(&frame(vec![row], 1), 1, 1);
        assert!(!ledger.finish().history_valid);
    }
}

#[test]
fn changed_source_under_unchanged_keyset_and_nonmonotonic_frames_refuse() {
    let mut changed = ledger(2);
    changed.observe(&frame(vec![card(0)], 2), 1, 10);
    let mut row = card(0);
    row["source_key"]["version"] = json!("2.0");
    changed.observe(&frame(vec![row], 2), 2, 20);
    assert_eq!(
        changed.finish().incomplete_reason,
        Some(Incomplete::SourceConflict)
    );
    for (draw, elapsed, uploads) in [(1, 20, 512), (2, 9, 512), (2, 20, 511)] {
        let mut ledger = ledger(2);
        ledger.observe(&frame(vec![card(0)], 2), 1, 10);
        let mut second = frame(vec![card(1)], 2);
        second["cache_uploads"] = json!(uploads);
        ledger.observe(&second, draw, elapsed);
        assert_eq!(
            ledger.finish().incomplete_reason,
            Some(Incomplete::NonmonotonicFrame)
        );
    }
}

#[test]
fn maximum_key_set_is_bounded_and_frame_or_report_caps_do_not_clear_history() {
    let mut full = ledger(MAX_KEYS);
    for start in (0..MAX_KEYS).step_by(MAX_FRAME_ROWS) {
        full.observe(
            &frame(
                (start..start + MAX_FRAME_ROWS).map(card).collect(),
                MAX_KEYS,
            ),
            start as u64 + 1,
            start as u64,
        );
    }
    full.catalogue(Some((SHA, MAX_KEYS, 1)));
    let report = full.finish();
    assert_eq!(report.observations.len(), MAX_KEYS);
    assert!(report.history_valid && report.observed_count_matches_eligible);
    assert!(serde_json::to_vec(&report).unwrap().len() <= MAX_REPORT_BYTES);
    let mut too_many = ledger(MAX_KEYS);
    too_many.observe(&frame((0..65).map(card).collect(), MAX_KEYS), 1, 1);
    assert_eq!(
        too_many.finish().incomplete_reason,
        Some(Incomplete::InvalidFrame)
    );
    let mut limited = ledger(2);
    limited.observe(&frame(vec![card(0)], 2), 1, 1);
    // Exercise the explicit serialized-history safety gate without constructing
    // an otherwise impossible unbounded public SourceKey shape.
    limited.record_bytes = RECORD_BUDGET;
    limited.observe(&frame(vec![card(1)], 2), 2, 2);
    let report = limited.finish();
    assert_eq!(report.observations.len(), 1);
    assert_eq!(
        report.incomplete_reason,
        Some(Incomplete::SerializationBound)
    );
}

#[test]
fn overlong_frame_and_too_small_eligible_count_reject_whole_candidate_frame() {
    let mut huge = frame(vec![card(0)], 1);
    huge["untrusted_extra"] = json!("x".repeat(MAX_FRAME_BYTES));
    let mut ledger = ledger(1);
    ledger.observe(&huge, 1, 1);
    assert!(ledger.finish().observations.is_empty());
    let mut ledger = super::Ledger::new();
    ledger.catalogue(Some((SHA, 1, 1)));
    ledger.observe(&frame(vec![card(0), card(1)], 1), 1, 1);
    assert_eq!(
        ledger.finish().incomplete_reason,
        Some(Incomplete::Capacity)
    );
}
