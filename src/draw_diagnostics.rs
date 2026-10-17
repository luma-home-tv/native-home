//! Observer-only, fixed-storage draw wall spans. Never GPU/CPU, pixel or action proof.
//! The ordinary renderer uses Noop: no clock, collector, allocation or IO is added.
use serde::{Deserialize, Serialize};
use std::time::{Duration, Instant};

pub const TOP: usize = 8;
pub const SLOW_US: u64 = 16_667;
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Phase {
    Clear,
    Chrome,
    Content,
    Finalize,
}
pub trait Probe {
    fn cut(&mut self, phase: Phase);
}
pub struct Noop;
impl Probe for Noop {
    #[inline(always)]
    fn cut(&mut self, _: Phase) {}
}
pub trait Clock {
    fn micros(&mut self) -> u64;
}
pub struct WallClock(Instant);
impl Default for WallClock {
    fn default() -> Self {
        Self(Instant::now())
    }
}
impl Clock for WallClock {
    fn micros(&mut self) -> u64 {
        self.0.elapsed().as_micros().min(u64::MAX as u128) as u64
    }
}
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Fault {
    #[default]
    None,
    MissingCut,
    UnexpectedCut,
    ClockRegressed,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Times {
    phases: [u64; 4],
    fault: Fault,
}
pub struct Timed<C: Clock> {
    clock: C,
    prior: u64,
    next: usize,
    times: Times,
}
impl<C: Clock> Timed<C> {
    pub fn new(mut clock: C) -> Self {
        let prior = clock.micros();
        Self {
            clock,
            prior,
            next: 0,
            times: Times {
                phases: [0; 4],
                fault: Fault::None,
            },
        }
    }
    pub fn finish(mut self) -> Times {
        if self.next != 4 && self.times.fault == Fault::None {
            self.times.fault = Fault::MissingCut;
        }
        self.times
    }
}
impl<C: Clock> Probe for Timed<C> {
    fn cut(&mut self, phase: Phase) {
        if self.times.fault != Fault::None {
            return;
        }
        if phase as usize != self.next {
            self.times.fault = Fault::UnexpectedCut;
            return;
        }
        let now = self.clock.micros();
        match now.checked_sub(self.prior) {
            Some(us) => self.times.phases[self.next] = us,
            None => self.times.fault = Fault::ClockRegressed,
        }
        self.prior = now;
        self.next += 1;
    }
}
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Page {
    Home,
    Applications,
    Inputs,
    Controls,
}
impl From<crate::model::Page> for Page {
    fn from(page: crate::model::Page) -> Self {
        match page {
            crate::model::Page::Home => Self::Home,
            crate::model::Page::Applications => Self::Applications,
            crate::model::Page::Inputs => Self::Inputs,
            _ => Self::Controls,
        }
    }
}
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum KeyClass {
    #[default]
    None,
    Navigation,
    Activate,
    Back,
    Editing,
    Other,
}
impl KeyClass {
    fn from_key(key: u32) -> Self {
        match key {
            1..=4 => Self::Navigation,
            5 => Self::Activate,
            6 => Self::Back,
            7 | 8 => Self::Editing,
            _ => Self::Other,
        }
    }
}
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Context {
    pub page: Page,
    pub first_draw: bool,
    pub page_changed: bool,
    pub last_key_class: KeyClass,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Record {
    pub frame_sequence: u64,
    pub elapsed_ms: u64,
    pub context: Option<Context>,
    pub phases_us: [u64; 4],
    pub draw_total_us: u64,
    pub swap_us: u64,
    pub unattributed_us: Option<u64>,
    pub fault: Fault,
    pub sum_overflow: bool,
    pub incoherent: bool,
    pub input_and_tick_us: Option<u64>,
    pub thread_cpu_us: Option<u64>,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Summary {
    pub schema: u8,
    pub frames_seen: u64,
    pub slow_seen: u64,
    pub retained: u8,
    pub slow_not_retained: u64,
    pub context_unavailable: u64,
    pub saturated: bool,
    pub top: [Option<Record>; TOP],
}
impl Default for Summary {
    fn default() -> Self {
        Self {
            schema: 1,
            frames_seen: 0,
            slow_seen: 0,
            retained: 0,
            slow_not_retained: 0,
            context_unavailable: 0,
            saturated: false,
            top: [None; TOP],
        }
    }
}
impl Summary {
    pub fn valid(&self) -> bool {
        let n = usize::from(self.retained);
        if self.schema != 1
            || n > TOP
            || self.slow_seen > self.frames_seen
            || u64::from(self.retained) > self.slow_seen
            || self.context_unavailable > self.frames_seen
            || self.slow_not_retained != self.slow_seen.saturating_sub(n as u64)
            || self.top[..n].iter().any(Option::is_none)
            || self.top[n..].iter().any(Option::is_some)
        {
            return false;
        }
        let mut prior = None;
        for row in self.top[..n].iter().flatten() {
            let sum = row
                .phases_us
                .iter()
                .try_fold(0u64, |a, b| a.checked_add(*b));
            let residual = if row.fault == Fault::None {
                sum.and_then(|s| row.draw_total_us.checked_sub(s))
            } else {
                None
            };
            if row.frame_sequence == 0
                || row.frame_sequence > self.frames_seen
                || row.draw_total_us < SLOW_US
                || row.sum_overflow != sum.is_none()
                || row.unattributed_us != residual
                || row.incoherent != (residual.is_none())
                || row.input_and_tick_us.is_some()
                || row.thread_cpu_us.is_some()
            {
                return false;
            }
            if prior.is_some_and(|p: Record| outranks(row, &p)) {
                return false;
            }
            prior = Some(*row);
        }
        true
    }
}
fn outranks(a: &Record, b: &Record) -> bool {
    a.draw_total_us > b.draw_total_us
        || (a.draw_total_us == b.draw_total_us && a.frame_sequence < b.frame_sequence)
}
#[derive(Default)]
pub struct Collector {
    summary: Summary,
    last_page: Option<Page>,
    last_key: KeyClass,
}
impl Collector {
    pub fn input(&mut self, key: u32) {
        self.last_key = KeyClass::from_key(key);
    }
    pub fn context(&self, page: Page) -> Context {
        Context {
            page,
            first_draw: self.summary.frames_seen == 0,
            page_changed: self.last_page.is_some_and(|old| old != page),
            last_key_class: self.last_key,
        }
    }
    fn increment(value: &mut u64, saturated: &mut bool) {
        match value.checked_add(1) {
            Some(next) => *value = next,
            None => *saturated = true,
        }
    }
    pub fn record(
        &mut self,
        context: Option<Context>,
        times: Times,
        draw: Duration,
        swap: Duration,
        elapsed: Duration,
    ) {
        Self::increment(&mut self.summary.frames_seen, &mut self.summary.saturated);
        if let Some(c) = context {
            self.last_page = Some(c.page);
        } else {
            Self::increment(
                &mut self.summary.context_unavailable,
                &mut self.summary.saturated,
            );
        }
        let total = draw.as_micros();
        let swap = swap.as_micros();
        let elapsed = elapsed.as_millis();
        self.summary.saturated |=
            total > u64::MAX as u128 || swap > u64::MAX as u128 || elapsed > u64::MAX as u128;
        let total = total.min(u64::MAX as u128) as u64;
        if total < SLOW_US {
            return;
        }
        Self::increment(&mut self.summary.slow_seen, &mut self.summary.saturated);
        let sum = times.phases.iter().try_fold(0u64, |a, b| a.checked_add(*b));
        let residual = if times.fault == Fault::None {
            sum.and_then(|s| total.checked_sub(s))
        } else {
            None
        };
        let row = Record {
            frame_sequence: self.summary.frames_seen,
            elapsed_ms: elapsed.min(u64::MAX as u128) as u64,
            context,
            phases_us: times.phases,
            draw_total_us: total,
            swap_us: swap.min(u64::MAX as u128) as u64,
            unattributed_us: residual,
            fault: times.fault,
            sum_overflow: sum.is_none(),
            incoherent: residual.is_none(),
            input_and_tick_us: None,
            thread_cpu_us: None,
        };
        if let Some(at) = self
            .summary
            .top
            .iter()
            .position(|prior| prior.is_none_or(|p| outranks(&row, &p)))
        {
            for i in (at + 1..TOP).rev() {
                self.summary.top[i] = self.summary.top[i - 1];
            }
            self.summary.top[at] = Some(row);
        }
        self.summary.retained = self.summary.top.iter().flatten().count() as u8;
        self.summary.slow_not_retained = self
            .summary
            .slow_seen
            .saturating_sub(u64::from(self.summary.retained));
    }
    pub fn summary(&self) -> Summary {
        self.summary
    }
}
#[cfg(test)]
#[path = "draw_diagnostics_tests.rs"]
mod tests;
