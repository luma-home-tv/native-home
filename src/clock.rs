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
    date: Option<Date>,
}
#[derive(Clone, Debug, Serialize)]
struct Date {
    year: i32,
    month: u32,
    day: u32,
}
impl Date {
    fn days(year: i32, month: u32) -> u32 {
        match month {
            4 | 6 | 9 | 11 => 30,
            2 => {
                if year % 4 == 0 && (year % 100 != 0 || year % 400 == 0) {
                    29
                } else {
                    28
                }
            }
            _ => 31,
        }
    }
    fn from_value(value: &Value) -> Option<Self> {
        let year = value["year"]
            .as_i64()
            .filter(|v| (1970..=2100).contains(v))? as i32;
        let month = value["month"].as_u64().filter(|v| (1..=12).contains(v))? as u32;
        let day = value["day"]
            .as_u64()
            .filter(|v| *v >= 1 && *v <= u64::from(Self::days(year, month)))?
            as u32;
        Some(Self { year, month, day })
    }
    fn next(&mut self) {
        self.day += 1;
        if self.day > Self::days(self.year, self.month) {
            self.day = 1;
            self.month += 1;
            if self.month > 12 {
                self.month = 1;
                self.year += 1;
            }
        }
    }
    fn label(&self) -> String {
        let offsets = [0, 3, 2, 5, 0, 3, 5, 1, 4, 6, 2, 4];
        let year = self.year - i32::from(self.month < 3);
        let weekday = (year + year / 4 - year / 100
            + year / 400
            + offsets[self.month as usize - 1]
            + self.day as i32)
            % 7;
        let day = [
            "Sunday",
            "Monday",
            "Tuesday",
            "Wednesday",
            "Thursday",
            "Friday",
            "Saturday",
        ][weekday as usize];
        let month = [
            "January",
            "February",
            "March",
            "April",
            "May",
            "June",
            "July",
            "August",
            "September",
            "October",
            "November",
            "December",
        ][self.month as usize - 1];
        format!("{day}, {} {month}", self.day)
    }
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
        date: Date::from_value(&raw["localtime"]),
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
                let mut date = reading.date.clone();
                if u64::from(reading.second_of_day) + age.as_secs() >= 86400 {
                    if let Some(date) = date.as_mut() {
                        date.next();
                    }
                }
                let source = if reading.simulated {
                    format!("Simulated time • {}", reading.zone)
                } else if let Some(date) = date {
                    format!("{} • {}", date.label(), reading.zone)
                } else {
                    format!("TV time • {}", reading.zone)
                };
                self.view = View {
                    time: format!("{:02}:{:02}", minute / 60, minute % 60),
                    source,
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
    #[test]
    fn date_uses_tv_calendar_and_rolls_over_midnight_without_host_timezone() {
        let mut data = sample();
        data["localtime"]["year"] = 2026.into();
        data["localtime"]["month"] = 10.into();
        data["localtime"]["day"] = 14.into();
        let mut clock = Clock::default();
        clock.accept(parse(&data, Duration::ZERO, false));
        assert_eq!(
            clock.view(Duration::ZERO).source,
            "Wednesday, 14 October • GMT"
        );
        assert_eq!(
            clock.view(Duration::from_secs(2)).source,
            "Thursday, 15 October • GMT"
        );
        data["localtime"]["month"] = 2.into();
        data["localtime"]["day"] = 30.into();
        clock.accept(parse(&data, Duration::ZERO, false));
        assert_eq!(clock.view(Duration::ZERO).source, "TV time • GMT");
    }
}
