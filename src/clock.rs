//! Read-only LG system time, extrapolated briefly by monotonic time, never by host timezone.
use serde::Serialize;
use serde_json::Value;
use std::time::Duration;
const MAX_AGE: Duration = Duration::from_secs(65);

#[derive(Clone, Debug, Serialize)]
pub struct Reading {
    pub observed: Duration,
    pub utc: u64,
    pub second_of_day: u32,
    pub zone: String,
    pub simulated: bool,
}
pub fn parse(raw: &Value, observed: Duration, simulated: bool) -> Result<Reading, String> {
    if raw["returnValue"].as_bool() != Some(true) || raw["timeValid"].as_bool() != Some(true) {
        return Err("TV time unavailable".into());
    }
    let number = |key: &str, max: u64| {
        raw["localtime"][key]
            .as_u64()
            .filter(|v| *v <= max)
            .ok_or_else(|| format!("Invalid clock {key}"))
    };
    let hour = number("hour", 23)?;
    let minute = number("minute", 59)?;
    let second = number("second", 59)?;
    let zone = raw["TZ"]
        .as_str()
        .filter(|z| {
            !z.is_empty()
                && z.len() <= 16
                && z.bytes()
                    .all(|c| c.is_ascii_alphanumeric() || b"+-:".contains(&c))
        })
        .ok_or("Invalid TV timezone label")?;
    Ok(Reading {
        observed,
        utc: raw["utc"].as_u64().ok_or("Missing clock epoch")?,
        second_of_day: (hour * 3600 + minute * 60 + second) as u32,
        zone: zone.into(),
        simulated,
    })
}
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct View {
    pub time: String,
    pub source: String,
    pub current: bool,
}
impl Default for View {
    fn default() -> Self {
        Self {
            time: "--:--".into(),
            source: "TV time unavailable".into(),
            current: false,
        }
    }
}
#[derive(Default)]
pub struct Clock {
    reading: Option<Reading>,
    minute: Option<u32>,
    view: View,
}
impl Clock {
    pub fn accept(&mut self, reading: Result<Reading, String>) {
        self.reading = reading.ok();
        self.minute = None;
    }
    pub fn view(&mut self, now: Duration) -> &View {
        let current = self.reading.as_ref().and_then(|r| {
            now.checked_sub(r.observed)
                .filter(|age| *age <= MAX_AGE)
                .map(|age| (r, age))
        });
        if let Some((reading, age)) = current {
            let minute = ((u64::from(reading.second_of_day) + age.as_secs()) % 86400 / 60) as u32;
            if self.minute != Some(minute) || !self.view.current {
                self.view = View {
                    time: format!("{:02}:{:02}", minute / 60, minute % 60),
                    source: format!(
                        "{} • {}",
                        if reading.simulated {
                            "Simulated time"
                        } else {
                            "TV time"
                        },
                        reading.zone
                    ),
                    current: true,
                };
                self.minute = Some(minute);
            }
        } else if self.view.current {
            self.view = View::default();
            self.minute = None;
        }
        &self.view
    }
}

#[cfg(target_arch = "arm")]
pub struct Worker {
    latest: std::sync::Arc<std::sync::Mutex<Option<Result<Reading, String>>>>,
    stop: std::sync::mpsc::Sender<()>,
    thread: std::thread::JoinHandle<()>,
}
#[cfg(target_arch = "arm")]
impl Worker {
    pub fn native(origin: std::time::Instant) -> std::io::Result<Self> {
        use std::sync::{mpsc, Arc, Mutex};
        let latest = Arc::new(Mutex::new(None));
        let output = latest.clone();
        let (stop, receive) = mpsc::channel();
        let thread = std::thread::Builder::new()
            .name("home-clock-read".into())
            .spawn(move || {
                let mut bus = None;
                loop {
                    if bus.is_none() {
                        bus = plx_platform::webos::ls2::register().ok();
                    }
                    let reading = bus
                        .as_ref()
                        .ok_or_else(|| "Clock registration unavailable".to_owned())
                        .and_then(|bus| {
                            bus.call(
                                "luna://com.webos.service.systemservice/time/getSystemTime",
                                "{}",
                                Duration::from_millis(1500),
                            )
                            .map_err(|_| "Clock service unavailable".to_owned())
                        })
                        .and_then(|raw| {
                            serde_json::from_str::<Value>(&raw)
                                .map_err(|_| "Invalid clock response".to_owned())
                        })
                        .and_then(|raw| parse(&raw, origin.elapsed(), false));
                    *output.lock().unwrap_or_else(|e| e.into_inner()) = Some(reading);
                    match receive.recv_timeout(Duration::from_secs(30)) {
                        Err(mpsc::RecvTimeoutError::Timeout) => {}
                        _ => break,
                    }
                }
            })?;
        Ok(Self {
            latest,
            stop,
            thread,
        })
    }
    pub fn poll(&self) -> Option<Result<Reading, String>> {
        self.latest.lock().unwrap_or_else(|e| e.into_inner()).take()
    }
    pub fn finish(self) {
        let _ = self.stop.send(());
        let _ = self.thread.join();
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    fn sample() -> Value {
        serde_json::json!({"returnValue":true,"timeValid":true,"utc":1792014633_u64,"TZ":"GMT","localtime":{"hour":23,"minute":59,"second":58}})
    }
    #[test]
    fn minute_rollover_uses_tv_local_fields_and_no_host_timezone() {
        let mut clock = Clock::default();
        clock.accept(parse(&sample(), Duration::ZERO, false));
        assert_eq!(clock.view(Duration::ZERO).time, "23:59");
        assert_eq!(clock.view(Duration::from_secs(2)).time, "00:00");
        assert_eq!(clock.view(Duration::from_secs(2)).source, "TV time • GMT");
    }
    #[test]
    fn failed_invalid_future_and_stale_readings_never_show_current_time() {
        let mut clock = Clock::default();
        clock.accept(parse(&sample(), Duration::from_secs(10), false));
        assert!(!clock.view(Duration::ZERO).current);
        assert!(clock.view(Duration::from_secs(75)).current);
        assert!(!clock.view(Duration::from_secs(76)).current);
        clock.accept(parse(&sample(), Duration::ZERO, false));
        assert!(clock.view(Duration::ZERO).current);
        clock.accept(Err("Denied".into()));
        assert_eq!(clock.view(Duration::from_secs(1)).time, "--:--");
        for (key, value) in [("hour", 24), ("minute", 60), ("second", 60)] {
            let mut data = sample();
            data["localtime"][key] = value.into();
            assert!(parse(&data, Duration::ZERO, false).is_err());
        }
        for key in ["returnValue", "timeValid"] {
            let mut data = sample();
            data[key] = false.into();
            assert!(parse(&data, Duration::ZERO, false).is_err());
        }
        let mut data = sample();
        data["TZ"] = "GMT\nBAD".into();
        assert!(parse(&data, Duration::ZERO, false).is_err());
    }
    #[test]
    fn timezone_changes_replace_cached_label_and_simulation_is_explicit() {
        let mut clock = Clock::default();
        clock.accept(parse(&sample(), Duration::ZERO, false));
        assert_eq!(clock.view(Duration::ZERO).source, "TV time • GMT");
        let mut data = sample();
        data["TZ"] = "BST".into();
        clock.accept(parse(&data, Duration::ZERO, true));
        assert_eq!(clock.view(Duration::ZERO).source, "Simulated time • BST");
    }
}
