//! Overtime requests (plan §6.3): the types the screens use, and the pure rules for where
//! an overtime block falls and whether it clashes with the employee's shift.

use super::attendance_calc::{is_work_day, Interval, Shift};
use chrono::{Duration, NaiveDate, NaiveTime};
use serde::{Deserialize, Serialize};
use sqlx::FromRow;
use ts_rs::TS;

/// Longest overtime block one request may cover (plan §6.3).
pub const MAX_MINUTES: i64 = 12 * 60;

#[derive(Debug, Clone, Serialize, FromRow, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct OvertimeRequest {
    #[ts(type = "number")]
    pub id: i64,
    #[ts(type = "number")]
    pub employee_id: i64,
    pub employee_no: String,
    pub employee_name: String,
    pub work_date: String,
    /// Company-local `YYYY-MM-DDTHH:MM:SS`.
    pub start_at: String,
    pub end_at: String,
    #[ts(type = "number")]
    pub minutes: i64,
    pub reason: String,
    #[ts(type = "\"PENDING\" | \"APPROVED\" | \"REJECTED\" | \"CANCELLED\"")]
    pub status: String,
    /// The account that filed it: the employee, or HR on their behalf.
    pub filed_by_username: Option<String>,
    pub decided_by_username: Option<String>,
    /// UTC, `YYYY-MM-DDTHH:MM:SSZ`.
    pub decided_at: Option<String>,
    pub decision_note: Option<String>,
    /// The date is in a posted payroll period, so the request can't change.
    pub locked: bool,
}

/// Filing overtime. Times are `HH:MM` on the office clock; an end time not after the
/// start time is the next day. With no employee number, it's for the signed-in person.
#[derive(Debug, Clone, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct OvertimeInput {
    pub employee_no: Option<String>,
    pub work_date: String,
    pub start_time: String,
    pub end_time: String,
    pub reason: String,
}

/// When an overtime block starts and ends. It starts on `work_date`, except after a shift
/// that crosses midnight: overtime typed as 06:00 after a 22:00–06:00 shift is the next
/// morning.
pub fn place(
    work_date: NaiveDate,
    start: NaiveTime,
    end: NaiveTime,
    shift: Option<&Shift>,
) -> Interval {
    let next_morning = shift.is_some_and(|s| s.crosses_midnight() && start <= s.end);
    let day = if next_morning {
        work_date + Duration::days(1)
    } else {
        work_date
    };
    let start_at = day.and_time(start);
    let mut end_at = day.and_time(end);
    if end_at <= start_at {
        end_at += Duration::days(1);
    }
    Interval::new(start_at, end_at)
}

/// True when the block overlaps a scheduled shift. Overtime must fall outside the
/// schedule (plan §6.3). The shift on `work_date` doesn't count when that day is off
/// (a holiday with no work), and the shifts on the days either side are checked too, since
/// a night shift or a long block can reach into them.
pub fn clashes_with_shift(
    ot: &Interval,
    work_date: NaiveDate,
    shift: &Shift,
    work_days: &str,
    work_date_is_off: bool,
) -> bool {
    [-1, 0, 1].into_iter().any(|offset| {
        let day = work_date + Duration::days(offset);
        let scheduled = is_work_day(work_days, day) && !(offset == 0 && work_date_is_off);
        scheduled && ot.overlaps(&shift.window(day))
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn t(s: &str) -> NaiveTime {
        NaiveTime::parse_from_str(s, "%H:%M").unwrap()
    }
    fn d(s: &str) -> NaiveDate {
        NaiveDate::parse_from_str(s, "%Y-%m-%d").unwrap()
    }
    fn shift(start: &str, end: &str) -> Shift {
        Shift {
            start: t(start),
            end: t(end),
            break_minutes: 60,
            grace_minutes: 0,
        }
    }
    const WEEKDAYS: &str = "MON,TUE,WED,THU,FRI";

    #[test]
    fn a_block_after_a_day_shift_stays_on_the_day() {
        let day = shift("08:00", "17:00");
        let ot = place(d("2026-10-07"), t("17:00"), t("20:00"), Some(&day));
        assert_eq!(ot.start.to_string(), "2026-10-07 17:00:00");
        assert_eq!(ot.minutes(), 180);
        assert!(!clashes_with_shift(
            &ot,
            d("2026-10-07"),
            &day,
            WEEKDAYS,
            false
        ));
    }

    #[test]
    fn an_end_time_past_midnight_is_the_next_day() {
        let ot = place(d("2026-10-07"), t("20:00"), t("02:00"), None);
        assert_eq!(ot.end.to_string(), "2026-10-08 02:00:00");
        assert_eq!(ot.minutes(), 360);
    }

    #[test]
    fn overtime_after_a_night_shift_is_the_next_morning() {
        let night = shift("22:00", "06:00");
        let after = place(d("2026-10-07"), t("06:00"), t("08:00"), Some(&night));
        assert_eq!(after.start.to_string(), "2026-10-08 06:00:00");
        assert!(!clashes_with_shift(
            &after,
            d("2026-10-07"),
            &night,
            WEEKDAYS,
            false
        ));
        let before = place(d("2026-10-07"), t("20:00"), t("22:00"), Some(&night));
        assert_eq!(before.start.to_string(), "2026-10-07 20:00:00");
        assert!(!clashes_with_shift(
            &before,
            d("2026-10-07"),
            &night,
            WEEKDAYS,
            false
        ));
        let inside = place(d("2026-10-07"), t("23:00"), t("01:00"), Some(&night));
        assert!(clashes_with_shift(
            &inside,
            d("2026-10-07"),
            &night,
            WEEKDAYS,
            false
        ));
    }

    #[test]
    fn a_block_inside_the_shift_clashes() {
        let day = shift("08:00", "17:00");
        let ot = place(d("2026-10-07"), t("16:00"), t("19:00"), Some(&day));
        assert!(clashes_with_shift(
            &ot,
            d("2026-10-07"),
            &day,
            WEEKDAYS,
            false
        ));
        // Reaching into the next morning's shift clashes too.
        let long = place(d("2026-10-07"), t("21:00"), t("09:00"), Some(&day));
        assert!(clashes_with_shift(
            &long,
            d("2026-10-07"),
            &day,
            WEEKDAYS,
            false
        ));
    }

    #[test]
    fn rest_days_and_holidays_off_allow_overtime_in_shift_hours() {
        let day = shift("08:00", "17:00");
        // Saturday is a rest day for a Mon–Fri schedule.
        let sat = place(d("2026-10-10"), t("08:00"), t("17:00"), Some(&day));
        assert!(!clashes_with_shift(
            &sat,
            d("2026-10-10"),
            &day,
            WEEKDAYS,
            false
        ));
        // Bonifacio Day, a Monday, is a regular holiday.
        let holiday = place(d("2026-11-30"), t("08:00"), t("17:00"), Some(&day));
        assert!(!clashes_with_shift(
            &holiday,
            d("2026-11-30"),
            &day,
            WEEKDAYS,
            true
        ));
        assert!(clashes_with_shift(
            &holiday,
            d("2026-11-30"),
            &day,
            WEEKDAYS,
            false
        ));
    }
}
