//! "Now", as a setting: the real clock in production, a controllable one in tests.

use std::sync::{Arc, Mutex};

use jiff::civil::Date;
use jiff::tz::TimeZone;
use jiff::{SignedDuration, Timestamp};

pub trait Clock: Send + Sync + 'static {
    fn now(&self) -> Timestamp;
}

pub struct SystemClock;

impl Clock for SystemClock {
    fn now(&self) -> Timestamp {
        Timestamp::now()
    }
}

/// A clock that only moves when told to.
#[derive(Clone)]
pub struct TestClock(Arc<Mutex<Timestamp>>);

impl TestClock {
    pub fn at(now: Timestamp) -> Self {
        TestClock(Arc::new(Mutex::new(now)))
    }

    pub fn set(&self, now: Timestamp) {
        *self.0.lock().unwrap() = now;
    }

    pub fn advance(&self, by: SignedDuration) {
        let mut now = self.0.lock().unwrap();
        *now = now.checked_add(by).expect("test clock overflow");
    }
}

impl Clock for TestClock {
    fn now(&self) -> Timestamp {
        *self.0.lock().unwrap()
    }
}

/// The household's local day at `now`. Daily limits and undo are per local day.
pub fn local_day(now: Timestamp, time_zone: &TimeZone) -> Date {
    now.to_zoned(time_zone.clone()).date()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn local_day_follows_the_household_time_zone() {
        let rome = TimeZone::get("Europe/Rome").unwrap();
        // 23:30 UTC on 14 October is already 15 October in Rome.
        let now: Timestamp = "2026-10-14T23:30:00Z".parse().unwrap();
        assert_eq!(local_day(now, &TimeZone::UTC).to_string(), "2026-10-14");
        assert_eq!(local_day(now, &rome).to_string(), "2026-10-15");
    }

    #[test]
    fn test_clock_moves_only_when_told() {
        let clock = TestClock::at("2026-10-14T08:00:00Z".parse().unwrap());
        assert_eq!(clock.now().to_string(), "2026-10-14T08:00:00Z");
        clock.advance(SignedDuration::from_hours(24));
        assert_eq!(clock.now().to_string(), "2026-10-15T08:00:00Z");
    }
}
