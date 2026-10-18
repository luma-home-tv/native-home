//! Strict version2 observer wire format. New diagnostics are converted/encoded
//! only by the existing snapshot worker; no private identifiers or text fields.
use crate::draw_diagnostics::{Page, Summary};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use std::io;
pub const SCHEMA: u8 = 2;
pub const MAX_BYTES: usize = 16384;
fn invalid() -> io::Error {
    io::Error::new(
        io::ErrorKind::InvalidData,
        "Version2 draw snapshot schema/bounds",
    )
}
#[derive(Clone, Copy, Debug, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum DisplayPhase {
    Foreground,
    IdleBlack,
    Hidden,
}
#[derive(Debug, Serialize, Deserialize)]
enum Profile {
    #[serde(rename = "preview-passive-observer-draw-v2")]
    V2,
}
#[derive(Debug, Serialize, Deserialize)]
enum Origin {
    #[serde(rename = "native-cumulative-draw-phases-v2")]
    Native,
}
#[derive(Debug, Serialize, Deserialize)]
enum MetricScope {
    #[serde(
        rename = "App-thread draw submission, swap call and intervals. No GPU completion timestamp, physical remote latency or panel photon measurement."
    )]
    AppThread,
}
#[derive(Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct Histogram {
    samples: u64,
    mean_ms: Option<f64>,
    p50_upper_ms: Option<u64>,
    p95_upper_ms: Option<u64>,
    p99_upper_ms: Option<u64>,
    max_ms: f64,
}
fn quantile_bound(max: f64) -> Option<u64> {
    // Invert the actual max_us as f64 /1000 conversion, including rounded u64
    // maxima, then use the source's floor-bin or capped1001 outlier rule.
    let limit = u64::MAX as u128;
    let convert = |n: u128| n as u64 as f64 / 1000.0;
    let (mut lo, mut hi) = (0, limit + 1);
    while lo < hi {
        let mid = (lo + hi) / 2;
        if convert(mid) < max {
            lo = mid + 1;
        } else {
            hi = mid;
        }
    }
    if lo > limit || convert(lo) != max {
        return None;
    }
    hi = limit + 1;
    while lo < hi {
        let mid = (lo + hi) / 2;
        if convert(mid) <= max {
            lo = mid + 1;
        } else {
            hi = mid;
        }
    }
    let last = (lo - 1) as u64;
    Some(if last / 1000 >= 1001 {
        last.div_ceil(1000)
    } else {
        last / 1000 + 1
    })
}
impl Histogram {
    fn valid(&self) -> bool {
        if !self.max_ms.is_finite() || self.max_ms < 0. {
            return false;
        }
        let qs = [self.p50_upper_ms, self.p95_upper_ms, self.p99_upper_ms];
        if self.samples == 0 {
            return self.mean_ms.is_none() && qs == [None; 3] && self.max_ms == 0.;
        }
        let Some(bound) = quantile_bound(self.max_ms) else {
            return false;
        };
        self.mean_ms
            .is_some_and(|v| v.is_finite() && v >= 0. && v <= self.max_ms)
            && qs.iter().all(|v| v.is_some_and(|v| v >= 1 && v <= bound))
            && qs[0] <= qs[1]
            && qs[1] <= qs[2]
    }
}
#[derive(Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct Metrics {
    frames: u64,
    idle_blank_presentations: u64,
    key_down: [u64; 7],
    draw_submission: Histogram,
    swap_call: Histogram,
    foreground_swap_intervals: Histogram,
    dequeued_input_to_next_swap_submission: Histogram,
    scope: MetricScope,
}
impl Metrics {
    fn valid(&self) -> bool {
        let all = [
            &self.draw_submission,
            &self.swap_call,
            &self.foreground_swap_intervals,
            &self.dequeued_input_to_next_swap_submission,
        ];
        all.iter().all(|h| h.valid())
            && self.draw_submission.samples == self.frames
            && self.swap_call.samples == self.frames
            && self.foreground_swap_intervals.samples <= self.frames.saturating_sub(1)
            && self.dequeued_input_to_next_swap_submission.samples <= self.frames
    }
}
#[derive(Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Snapshot {
    schema: u8,
    profile: Profile,
    elapsed_seconds: f64,
    page: Page,
    phase: DisplayPhase,
    metrics: Metrics,
    draw_diagnostics: Summary,
    source: Origin,
    #[serde(rename = "physical_or_GPU_completion_qualified")]
    physical_completion_qualified: bool,
}
impl Snapshot {
    fn valid(&self) -> bool {
        self.schema == SCHEMA
            && self.elapsed_seconds.is_finite()
            && self.elapsed_seconds >= 0.
            && !self.physical_completion_qualified
            && self.metrics.valid()
            && self.draw_diagnostics.valid()
            && self.metrics.frames == self.draw_diagnostics.frames_seen
    }
    pub fn diagnostics(&self) -> &Summary {
        &self.draw_diagnostics
    }
}
/// Existing monitor Value construction remains outside draw. No context/focus
/// strings or diagnostic Value conversion occur in this caller-side builder.
pub fn base(elapsed: f64, page: Page, hidden: bool, blank: bool, metrics: Value) -> Value {
    let phase = if hidden {
        DisplayPhase::Hidden
    } else if blank {
        DisplayPhase::IdleBlack
    } else {
        DisplayPhase::Foreground
    };
    json!({"schema":SCHEMA,"profile":Profile::V2,"elapsed_seconds":elapsed,"page":page,"phase":phase,"metrics":metrics,"source":Origin::Native,"physical_or_GPU_completion_qualified":false})
}
/// Called on the snapshot worker, using the one existing capacity1 message.
pub(crate) fn encode(mut base: Value, diagnostics: Summary) -> io::Result<Vec<u8>> {
    let fields = base.as_object_mut().ok_or_else(invalid)?;
    if fields.contains_key("draw_diagnostics") {
        return Err(invalid());
    }
    fields.insert(
        "draw_diagnostics".into(),
        serde_json::to_value(diagnostics)?,
    );
    let typed: Snapshot = serde_json::from_value(base).map_err(|_| invalid())?;
    if !typed.valid() {
        return Err(invalid());
    }
    let raw = serde_json::to_vec(&typed)?;
    if raw.len() > MAX_BYTES {
        return Err(invalid());
    }
    Ok(raw)
}
pub fn parse(raw: &[u8]) -> io::Result<Snapshot> {
    if raw.is_empty() || raw.len() > MAX_BYTES {
        return Err(invalid());
    }
    let value: Snapshot = serde_json::from_slice(raw).map_err(|_| invalid())?;
    if !value.valid() {
        return Err(invalid());
    }
    Ok(value)
}
#[cfg(test)]
#[path = "draw_snapshot_tests.rs"]
mod tests;
