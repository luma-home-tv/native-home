use super::*;
use std::{cell::Cell, rc::Rc};
struct Fake<'a> {
    values: std::iter::Copied<std::slice::Iter<'a, u64>>,
    reads: Rc<Cell<usize>>,
}
impl Clock for Fake<'_> {
    fn micros(&mut self) -> u64 {
        self.reads.set(self.reads.get() + 1);
        self.values.next().unwrap()
    }
}
fn timed(values: &[u64]) -> (Timed<Fake<'_>>, Rc<Cell<usize>>) {
    let reads = Rc::new(Cell::new(0));
    (
        Timed::new(Fake {
            values: values.iter().copied(),
            reads: reads.clone(),
        }),
        reads,
    )
}
fn complete() -> Times {
    let (mut p, _) = timed(&[0, 1000, 2000, 3000, 4000]);
    for phase in [Phase::Clear, Phase::Chrome, Phase::Content, Phase::Finalize] {
        p.cut(phase);
    }
    p.finish()
}
#[test]
fn exclusive_ordered_cuts_and_residual_bind_one_frame() {
    let (mut p, reads) = timed(&[100, 110, 125, 140, 150]);
    for phase in [Phase::Clear, Phase::Chrome, Phase::Content, Phase::Finalize] {
        p.cut(phase);
    }
    let t = p.finish();
    assert_eq!(t.phases, [10, 15, 15, 10]);
    assert_eq!(t.fault, Fault::None);
    assert_eq!(reads.get(), 5);
    let mut c = Collector::default();
    let ctx = c.context(Page::Home);
    c.record(
        Some(ctx),
        t,
        Duration::from_micros(20000),
        Duration::from_micros(80),
        Duration::from_millis(123),
    );
    let s = c.summary();
    let r = s.top[0].unwrap();
    assert_eq!(r.frame_sequence, 1);
    assert_eq!(r.elapsed_ms, 123);
    assert_eq!(r.unattributed_us, Some(19950));
    assert!(!r.incoherent);
    assert!(s.valid());
}
#[test]
fn missing_duplicate_out_of_order_and_regressed_clocks_are_explicit() {
    let (mut p, _) = timed(&[0, 5]);
    p.cut(Phase::Clear);
    assert_eq!(p.finish().fault, Fault::MissingCut);
    let (mut p, _) = timed(&[0, 5]);
    p.cut(Phase::Clear);
    p.cut(Phase::Clear);
    assert_eq!(p.finish().fault, Fault::UnexpectedCut);
    let (mut p, _) = timed(&[0]);
    p.cut(Phase::Chrome);
    assert_eq!(p.finish().fault, Fault::UnexpectedCut);
    let (mut p, _) = timed(&[10, 5]);
    p.cut(Phase::Clear);
    assert_eq!(p.finish().fault, Fault::ClockRegressed);
}
#[test]
fn top8_is_fixed_order_ties_keep_earliest_and_excluded_counts_are_preserved() {
    let mut c = Collector::default();
    for n in 0..10 {
        let ctx = c.context(Page::Home);
        c.record(
            Some(ctx),
            complete(),
            Duration::from_micros(20000),
            Duration::ZERO,
            Duration::from_millis(n),
        );
    }
    let s = c.summary();
    assert_eq!(s.retained, 8);
    assert_eq!(s.slow_seen, 10);
    assert_eq!(s.slow_not_retained, 2);
    assert_eq!(
        s.top.map(|r| r.unwrap().frame_sequence),
        [1, 2, 3, 4, 5, 6, 7, 8]
    );
    let ctx = c.context(Page::Applications);
    assert!(ctx.page_changed);
    c.record(
        Some(ctx),
        complete(),
        Duration::from_micros(30000),
        Duration::ZERO,
        Duration::from_millis(11),
    );
    let s = c.summary();
    assert_eq!(s.top[0].unwrap().frame_sequence, 11);
    assert_eq!(s.top[7].unwrap().frame_sequence, 7);
    assert_eq!(s.slow_not_retained, 3);
    assert!(s.valid());
}
#[test]
fn incoherent_sums_overflow_and_context_unavailability_are_not_hidden() {
    let mut c = Collector::default();
    let t = Times {
        phases: [u64::MAX, 1, 0, 0],
        fault: Fault::None,
    };
    c.record(
        None,
        t,
        Duration::from_secs(u64::MAX),
        Duration::ZERO,
        Duration::ZERO,
    );
    let s = c.summary();
    let r = s.top[0].unwrap();
    assert!(r.sum_overflow && r.incoherent && s.saturated);
    assert_eq!(r.unattributed_us, None);
    assert_eq!(s.context_unavailable, 1);
    assert!(s.valid());
    let mut c = Collector::default();
    c.record(
        None,
        Times {
            phases: [30000, 0, 0, 0],
            fault: Fault::None,
        },
        Duration::from_micros(20000),
        Duration::ZERO,
        Duration::ZERO,
    );
    let r = c.summary().top[0].unwrap();
    assert!(r.incoherent && !r.sum_overflow);
    assert_eq!(r.unattributed_us, None);
}
#[test]
fn counter_saturation_and_safe_context_have_no_dynamic_identifiers() {
    let mut c = Collector::default();
    assert!(c.context(Page::Home).first_draw);
    c.input(7);
    assert_eq!(c.context(Page::Home).last_key_class, KeyClass::Editing);
    c.summary.frames_seen = u64::MAX;
    c.summary.slow_seen = u64::MAX;
    c.summary.context_unavailable = u64::MAX;
    c.record(
        None,
        complete(),
        Duration::from_micros(20000),
        Duration::ZERO,
        Duration::ZERO,
    );
    let s = c.summary();
    assert!(s.saturated);
    assert_eq!(s.frames_seen, u64::MAX);
    assert!(s.valid());
    let context = Context {
        page: Page::Home,
        first_draw: true,
        page_changed: false,
        last_key_class: KeyClass::None,
    };
    let mut json = serde_json::to_value(context).unwrap();
    json["app_id"] = "private.app".into();
    assert!(serde_json::from_value::<Context>(json).is_err());
    assert_eq!(std::mem::size_of::<Noop>(), 0);
}
