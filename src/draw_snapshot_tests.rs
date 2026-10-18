use super::*;
use crate::{
    draw_diagnostics::{Clock, Collector, Page, Phase, Probe, Timed},
    performance::Monitor,
};
use std::time::Duration;
struct Fake(u64);
impl Clock for Fake {
    fn micros(&mut self) -> u64 {
        self.0 += 1;
        self.0
    }
}
fn fixture(count: u64) -> (Value, Summary) {
    let mut c = Collector::default();
    let mut m = Monitor::default();
    for n in 0..count {
        let mut p = Timed::new(Fake(0));
        for phase in [Phase::Clear, Phase::Chrome, Phase::Content, Phase::Finalize] {
            p.cut(phase);
        }
        let ctx = c.context(Page::Home);
        let at = Duration::from_millis(n * 30);
        let draw = Duration::from_micros(20000 + n);
        c.record(Some(ctx), p.finish(), draw, Duration::ZERO, at);
        m.presented(at, draw, Duration::ZERO);
    }
    (base(1.0, Page::Home, false, false, m.report()), c.summary())
}
#[test]
fn typed_complete_envelope_rejects_old_schema_private_fields_and_bad_sums() {
    let (b, d) = fixture(9);
    let raw = encode(b, d).unwrap();
    let parsed = parse(&raw).unwrap();
    assert_eq!(parsed.diagnostics().retained, 8);
    for word in ["focus", "category", "app_id", "query", "title", "texture"] {
        assert!(!String::from_utf8_lossy(&raw).contains(word));
    }
    let v: Value = serde_json::from_slice(&raw).unwrap();
    for (key, value) in [
        ("schema", json!(1)),
        ("focus", json!("household.private")),
        ("physical_or_GPU_completion_qualified", json!(true)),
        ("page", json!("private.app")),
    ] {
        let mut bad = v.clone();
        bad[key] = value;
        assert!(parse(&serde_json::to_vec(&bad).unwrap()).is_err());
    }
    let mut bad = v.clone();
    bad["draw_diagnostics"]["top"][0]["unattributed_us"] = json!(1);
    assert!(parse(&serde_json::to_vec(&bad).unwrap()).is_err());
    let mut bad = v;
    bad["draw_diagnostics"]["top"][0]["context"]["app_id"] = json!("private.app");
    assert!(parse(&serde_json::to_vec(&bad).unwrap()).is_err());
}
#[test]
fn worst_numeric_top8_and_complete_cap_are_bounded() {
    let (mut b, mut d) = fixture(8);
    d.frames_seen = u64::MAX;
    d.slow_seen = u64::MAX;
    d.slow_not_retained = u64::MAX - 8;
    d.context_unavailable = u64::MAX;
    d.saturated = true;
    for (i, row) in d.top.iter_mut().flatten().enumerate() {
        row.frame_sequence = u64::MAX - 8 + i as u64;
        row.elapsed_ms = u64::MAX;
        row.swap_us = u64::MAX;
        row.phases_us = [u64::MAX; 4];
        row.draw_total_us = u64::MAX;
        row.sum_overflow = true;
        row.incoherent = true;
        row.unattributed_us = None;
        row.fault = crate::draw_diagnostics::Fault::ClockRegressed;
        row.context = Some(crate::draw_diagnostics::Context {
            page: Page::Applications,
            first_draw: false,
            page_changed: false,
            last_key_class: crate::draw_diagnostics::KeyClass::Navigation,
        });
    }
    b["metrics"]["frames"] = json!(u64::MAX);
    b["elapsed_seconds"] = json!(f64::MAX);
    b["metrics"]["key_down"] = serde_json::to_value([u64::MAX; 7]).unwrap();
    b["metrics"]["idle_blank_presentations"] = json!(u64::MAX);
    for field in [
        "draw_submission",
        "swap_call",
        "foreground_swap_intervals",
        "dequeued_input_to_next_swap_submission",
    ] {
        b["metrics"][field] = json!({"samples": u64::MAX - 1, "mean_ms": u64::MAX as f64 / 1000.,
            "max_ms": u64::MAX as f64 / 1000., "p50_upper_ms": u64::MAX.div_ceil(1000),
            "p95_upper_ms": u64::MAX.div_ceil(1000), "p99_upper_ms": u64::MAX.div_ceil(1000)});
    }
    for field in ["draw_submission", "swap_call"] {
        b["metrics"][field]["samples"] = json!(u64::MAX);
    }
    let raw = encode(b, d).unwrap();
    assert!(raw.len() < MAX_BYTES);
    assert!(parse(&raw).is_ok());
    let mut oversized = raw.clone();
    oversized.resize(MAX_BYTES + 1, b' ');
    assert!(parse(&oversized).is_err());
    let (mut b, d) = fixture(0);
    b["unsafe_extra"] = json!("x".repeat(MAX_BYTES));
    assert!(encode(b, d).is_err());
    let (mut b, d) = fixture(0);
    b["draw_diagnostics"] = json!({});
    assert!(encode(b, d).is_err());
}
#[test]
fn actual_histogram_rounding_boundaries_and_unavailable_values_are_strict() {
    for us in [
        0,
        999,
        1000,
        1001,
        1000999,
        1001000,
        1001001,
        2000000,
        u64::MAX,
    ] {
        let max = us as f64 / 1000.;
        let q = if us / 1000 >= 1001 {
            us.div_ceil(1000)
        } else {
            us / 1000 + 1
        };
        assert!(quantile_bound(max).unwrap() >= q);
    }
    assert_eq!(quantile_bound(0.0005), None);
    assert_eq!(quantile_bound(0.6), Some(1));
    let (b, d) = fixture(1);
    let raw = encode(b, d).unwrap();
    let mut v: Value = serde_json::from_slice(&raw).unwrap();
    v["metrics"]["draw_submission"]["p99_upper_ms"] = json!(22);
    assert!(parse(&serde_json::to_vec(&v).unwrap()).is_err());
    let (mut b, d) = fixture(0);
    b["metrics"]["swap_call"]["mean_ms"] = json!(0);
    assert!(encode(b, d).is_err());
}

#[test]
fn full_envelopes_refuse_missing_retention_and_duplicate_frame_aliases() {
    let (b, d) = fixture(2);
    let raw = encode(b, d).unwrap();
    let genuine: Value = serde_json::from_slice(&raw).unwrap();
    let mut missing = genuine.clone();
    missing["draw_diagnostics"]["retained"] = json!(0);
    missing["draw_diagnostics"]["slow_not_retained"] = json!(2);
    missing["draw_diagnostics"]["top"] = json!([null, null, null, null, null, null, null, null]);
    assert!(parse(&serde_json::to_vec(&missing).unwrap()).is_err());
    let mut partial = genuine.clone();
    partial["draw_diagnostics"]["retained"] = json!(1);
    partial["draw_diagnostics"]["slow_not_retained"] = json!(1);
    partial["draw_diagnostics"]["top"][1] = Value::Null;
    assert!(parse(&serde_json::to_vec(&partial).unwrap()).is_err());
    let mut duplicate = genuine.clone();
    duplicate["draw_diagnostics"]["top"][1]["frame_sequence"] =
        duplicate["draw_diagnostics"]["top"][0]["frame_sequence"].clone();
    assert!(parse(&serde_json::to_vec(&duplicate).unwrap()).is_err());
    duplicate["draw_diagnostics"]["saturated"] = json!(true);
    assert!(parse(&serde_json::to_vec(&duplicate).unwrap()).is_err());
    let mut saturated = genuine;
    saturated["draw_diagnostics"]["frames_seen"] = json!(u64::MAX);
    saturated["metrics"]["frames"] = json!(u64::MAX);
    for field in ["draw_submission", "swap_call"] {
        saturated["metrics"][field]["samples"] = json!(u64::MAX);
    }
    saturated["draw_diagnostics"]["saturated"] = json!(true);
    for i in 0..2 {
        saturated["draw_diagnostics"]["top"][i]["frame_sequence"] = json!(u64::MAX);
    }
    assert!(parse(&serde_json::to_vec(&saturated).unwrap()).is_ok());
    saturated["draw_diagnostics"]["saturated"] = json!(false);
    assert!(parse(&serde_json::to_vec(&saturated).unwrap()).is_err());
}
