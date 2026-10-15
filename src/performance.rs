//! Bounded app-local timing diagnostics. Submission timings are not GPU completion or glass latency.
use serde::Serialize;
use std::time::Duration;

#[derive(Clone, Serialize)]
pub struct Histogram {
    count: u64,
    total_us: u64,
    max_us: u64,
    bins: Vec<u64>,
}
impl Default for Histogram {
    fn default() -> Self {
        Self {
            count: 0,
            total_us: 0,
            max_us: 0,
            bins: vec![0; 1002],
        }
    }
}
impl Histogram {
    fn sample(&mut self, elapsed: Duration) {
        let us = elapsed.as_micros().min(u64::MAX as u128) as u64;
        self.count += 1;
        self.total_us = self.total_us.saturating_add(us);
        self.max_us = self.max_us.max(us);
        self.bins[(us / 1000).min(1001) as usize] += 1;
    }
    fn quantile_upper_ms(&self, percent: u64) -> Option<u64> {
        if self.count == 0 {
            return None;
        }
        let rank = (self.count * percent).div_ceil(100);
        let mut seen = 0;
        for (i, count) in self.bins.iter().enumerate() {
            seen += count;
            if seen >= rank {
                return Some(if i == 1001 {
                    self.max_us.div_ceil(1000)
                } else {
                    i as u64 + 1
                });
            }
        }
        None
    }
    fn summary(&self) -> serde_json::Value {
        serde_json::json!({"samples":self.count,"mean_ms":if self.count>0 {Some(self.total_us as f64/self.count as f64/1000.0)}else{None},"p50_upper_ms":self.quantile_upper_ms(50),"p95_upper_ms":self.quantile_upper_ms(95),"p99_upper_ms":self.quantile_upper_ms(99),"max_ms":self.max_us as f64/1000.0})
    }
}
#[derive(Default)]
pub struct Monitor {
    pub frames: u64,
    pub idle_blanks: u64,
    pub key_down: [u64; 7],
    draw: Histogram,
    swap: Histogram,
    interval: Histogram,
    input_to_swap: Histogram,
    last_swap: Option<Duration>,
    last_input: Option<Duration>,
}
impl Monitor {
    pub fn input(&mut self, key: u32, at: Duration) {
        if let Some(count) = self.key_down.get_mut(key as usize) {
            *count += 1;
        }
        self.last_input = Some(at);
    }
    pub fn presented(&mut self, at: Duration, draw: Duration, swap: Duration) {
        if let Some(last) = self.last_swap {
            if let Some(delta) = at.checked_sub(last) {
                self.interval.sample(delta);
            }
        }
        self.last_swap = Some(at);
        self.frames += 1;
        self.draw.sample(draw);
        self.swap.sample(swap);
        if let Some(input) = self.last_input.take() {
            if let Some(delta) = at.checked_sub(input) {
                self.input_to_swap.sample(delta);
            }
        }
    }
    pub fn suspended(&mut self) {
        self.last_swap = None;
        self.last_input = None;
    }
    pub fn report(&self) -> serde_json::Value {
        serde_json::json!({"frames":self.frames,"idle_blank_presentations":self.idle_blanks,"key_down":self.key_down,"draw_submission":self.draw.summary(),"swap_call":self.swap.summary(),"foreground_swap_intervals":self.interval.summary(),"dequeued_input_to_next_swap_submission":self.input_to_swap.summary(),"scope":"App-thread draw submission, swap call and intervals. No GPU completion timestamp, physical remote latency or panel photon measurement."})
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn suspension_does_not_turn_idle_time_into_a_frame_stall_or_input_latency() {
        let mut m = Monitor::default();
        m.presented(Duration::ZERO, Duration::from_millis(2), Duration::ZERO);
        m.input(4, Duration::from_millis(10));
        m.presented(
            Duration::from_millis(17),
            Duration::from_millis(2),
            Duration::from_millis(1),
        );
        m.suspended();
        m.presented(
            Duration::from_secs(120),
            Duration::from_millis(2),
            Duration::ZERO,
        );
        assert_eq!(m.interval.count, 1);
        assert_eq!(m.input_to_swap.count, 1);
        assert_eq!(m.input_to_swap.max_us, 7000);
        assert_eq!(m.key_down[4], 1);
    }
    #[test]
    fn histogram_is_bounded_and_keeps_outlier_magnitude_and_empty_state() {
        let mut h = Histogram::default();
        assert_eq!(h.quantile_upper_ms(95), None);
        for _ in 0..99 {
            h.sample(Duration::from_micros(16500));
        }
        h.sample(Duration::from_secs(3));
        assert_eq!(h.bins.len(), 1002);
        assert_eq!(h.quantile_upper_ms(95), Some(17));
        assert_eq!(h.quantile_upper_ms(100), Some(3000));
        assert_eq!(h.max_us, 3000000);
    }
}
