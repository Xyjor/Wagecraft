//! Which work schedule applied on a given day. HR can move someone to another schedule
//! from a date, so a past day must be worked out with the schedule it had then.

use chrono::NaiveDate;

/// The schedules an employee has followed. Each applies from its date until the next one;
/// `None` means no schedule from that date.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ScheduleHistory<T> {
    entries: Vec<(NaiveDate, Option<T>)>,
}

impl<T> ScheduleHistory<T> {
    pub fn new(mut entries: Vec<(NaiveDate, Option<T>)>) -> Self {
        entries.sort_by_key(|(from, _)| *from);
        Self { entries }
    }

    /// The schedule in effect on `date`, if any.
    pub fn on(&self, date: NaiveDate) -> Option<&T> {
        self.entries
            .iter()
            .rev()
            .find(|(from, _)| *from <= date)
            .and_then(|(_, schedule)| schedule.as_ref())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn d(s: &str) -> NaiveDate {
        NaiveDate::parse_from_str(s, "%Y-%m-%d").unwrap()
    }

    #[test]
    fn each_schedule_runs_until_the_next_one_starts() {
        // Given out of order on purpose.
        let h = ScheduleHistory::new(vec![
            (d("2026-10-12"), Some("early")),
            (d("2025-01-06"), Some("office")),
            (d("2026-11-02"), None),
        ]);
        assert_eq!(h.on(d("2025-01-05")), None, "before the first one");
        assert_eq!(h.on(d("2025-01-06")), Some(&"office"));
        assert_eq!(h.on(d("2026-10-11")), Some(&"office"));
        assert_eq!(h.on(d("2026-10-12")), Some(&"early"));
        assert_eq!(h.on(d("2026-11-01")), Some(&"early"));
        assert_eq!(h.on(d("2026-11-02")), None, "taken off any schedule");
    }
}
