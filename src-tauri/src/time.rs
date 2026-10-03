//! Timestamps are stored as UTC text like `2026-10-05T08:00:00Z`, the same format the
//! migrations' `DEFAULT (strftime(...))` columns use, so they sort and compare as text.

use chrono::{DateTime, Utc};

pub fn to_db(at: DateTime<Utc>) -> String {
    at.format("%Y-%m-%dT%H:%M:%SZ").to_string()
}

pub fn from_db(s: &str) -> Option<DateTime<Utc>> {
    DateTime::parse_from_rfc3339(s)
        .ok()
        .map(|d| d.with_timezone(&Utc))
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
}
