//! Timestamps are stored as UTC text like `2026-10-05T08:00:00Z`, the same format the
//! migrations' `DEFAULT (strftime(...))` columns use, so they sort and compare as text.
//!
//! Attendance times are the exception: they are company-local wall-clock text like
//! `2026-10-05T08:00:00`, because a shift starts at 08:00 on the office clock (plan §5.1).
//! The company's clock is the PC's clock, since Wagecraft runs on one PC (ADR-003).

use chrono::{DateTime, Local, NaiveDateTime, Timelike, Utc};

const LOCAL_FORMAT: &str = "%Y-%m-%dT%H:%M:%S";

pub fn to_db(at: DateTime<Utc>) -> String {
    at.format("%Y-%m-%dT%H:%M:%SZ").to_string()
}

pub fn from_db(s: &str) -> Option<DateTime<Utc>> {
    DateTime::parse_from_rfc3339(s)
        .ok()
        .map(|d| d.with_timezone(&Utc))
}

/// The PC's wall-clock time, to the whole second.
pub fn local_now() -> NaiveDateTime {
    let now = Local::now().naive_local();
    now.with_nanosecond(0).unwrap_or(now)
}

pub fn to_local_db(at: NaiveDateTime) -> String {
    at.format(LOCAL_FORMAT).to_string()
}

pub fn from_local_db(s: &str) -> Option<NaiveDateTime> {
    NaiveDateTime::parse_from_str(s, LOCAL_FORMAT).ok()
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::TimeZone;

    #[test]
    fn round_trips_to_whole_seconds() {
        let at = Utc.with_ymd_and_hms(2026, 10, 5, 8, 30, 15).unwrap();
        assert_eq!(to_db(at), "2026-10-05T08:30:15Z");
        assert_eq!(from_db("2026-10-05T08:30:15Z"), Some(at));
        assert_eq!(from_db("yesterday"), None);
    }

    #[test]
    fn local_times_round_trip_without_a_zone() {
        let at = chrono::NaiveDate::from_ymd_opt(2026, 10, 5)
            .unwrap()
            .and_hms_opt(22, 5, 0)
            .unwrap();
        assert_eq!(to_local_db(at), "2026-10-05T22:05:00");
        assert_eq!(from_local_db("2026-10-05T22:05:00"), Some(at));
        assert_eq!(from_local_db("2026-10-05T22:05:00Z"), None);
        assert_eq!(local_now().nanosecond(), 0);
    }
}
