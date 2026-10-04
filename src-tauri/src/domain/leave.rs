//! Leave types, balances and requests (plan §6.4). Days are counted in half days, so 5
//! days is 10 and a half-day leave stays a whole number.

use super::attendance_calc::is_work_day;
use crate::domain::schedule::ScheduleHistory;
use chrono::{Months, NaiveDate};
use serde::{Deserialize, Serialize};
use sqlx::FromRow;
use std::collections::HashSet;
use ts_rs::TS;

#[derive(Debug, Clone, Serialize, FromRow, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct LeaveType {
    #[ts(type = "number")]
    pub id: i64,
    pub code: String,
    pub name: String,
    pub is_paid: bool,
    #[ts(type = "number")]
    pub default_halfdays_per_year: i64,
    /// Months of service before the yearly grant. SIL needs 12.
    #[ts(type = "number")]
    pub min_service_months: i64,
    pub is_active: bool,
}

#[derive(Debug, Clone, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct LeaveTypeInput {
    pub code: String,
    pub name: String,
    pub is_paid: bool,
    #[ts(type = "number")]
    pub default_halfdays_per_year: i64,
    #[ts(type = "number")]
    pub min_service_months: i64,
}

/// One employee's days of one leave type for one year.
#[derive(Debug, Clone, Serialize, FromRow, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct LeaveBalance {
    #[ts(type = "number")]
    pub id: i64,
    #[ts(type = "number")]
    pub employee_id: i64,
    #[ts(type = "number")]
    pub leave_type_id: i64,
    pub leave_type_code: String,
    pub leave_type_name: String,
    #[ts(type = "number")]
    pub year: i64,
    #[ts(type = "number")]
    pub entitled_halfdays: i64,
    #[ts(type = "number")]
    pub used_halfdays: i64,
}

/// HR changing one balance's yearly days, with the reason audited.
#[derive(Debug, Clone, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct BalanceAdjustment {
    #[ts(type = "number")]
    pub entitled_halfdays: i64,
    pub reason: String,
}

/// True when someone hired on `hire_date` has served `months` by `as_of`.
pub fn has_served(hire_date: NaiveDate, months: i64, as_of: NaiveDate) -> bool {
    let Ok(months) = u32::try_from(months) else {
        return false;
    };
    hire_date
        .checked_add_months(Months::new(months))
        .is_some_and(|d| d <= as_of)
}

/// The day the yearly grant judges service on: today for this year, January 1 for a
/// later year.
pub fn grant_as_of(year: i32, today: NaiveDate) -> NaiveDate {
    NaiveDate::from_ymd_opt(year, 1, 1)
        .map(|jan1| jan1.max(today))
        .unwrap_or(today)
}

/// A leave request with what the screens show about it.
#[derive(Debug, Clone, Serialize, FromRow, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct LeaveRequest {
    #[ts(type = "number")]
    pub id: i64,
    #[ts(type = "number")]
    pub employee_id: i64,
    pub employee_no: String,
    pub employee_name: String,
    #[ts(type = "number")]
    pub leave_type_id: i64,
    pub leave_type_code: String,
    pub leave_type_name: String,
    pub is_paid: bool,
    /// `YYYY-MM-DD`, both days included.
    pub start_date: String,
    pub end_date: String,
    pub half_day: bool,
    /// The working time it takes, in half days. Rest days and holidays off don't count.
    #[ts(type = "number")]
    pub halfdays: i64,
    pub reason: String,
    #[ts(type = "\"PENDING\" | \"APPROVED\" | \"REJECTED\" | \"CANCELLED\"")]
    pub status: String,
    /// The account that filed it: the employee, or HR on their behalf.
    pub filed_by_username: Option<String>,
    pub decided_by_username: Option<String>,
    /// UTC, `YYYY-MM-DDTHH:MM:SSZ`.
    pub decided_at: Option<String>,
    pub decision_note: Option<String>,
    /// A day it covers is in a posted payroll period, so the request can't change.
    pub locked: bool,
}

/// Filing leave. With no employee number, it's for the signed-in person.
#[derive(Debug, Clone, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct LeaveRequestInput {
    pub employee_no: Option<String>,
    #[ts(type = "number")]
    pub leave_type_id: i64,
    pub start_date: String,
    pub end_date: String,
    pub half_day: bool,
    pub reason: String,
}

/// The working time a leave from `start` to `end` takes, in half days: 2 for each
/// scheduled work day that isn't a holiday off, where each day counts by the schedule it
/// had. A half-day leave is one day and counts 1. Zero means the range has no working
/// day in it; `None` means a day in it has no schedule.
pub fn count_halfdays_by_history(
    start: NaiveDate,
    end: NaiveDate,
    half_day: bool,
    history: &ScheduleHistory<String>,
    days_off: &HashSet<NaiveDate>,
) -> Option<i64> {
    let mut working = 0;
    for day in start.iter_days().take_while(|d| *d <= end) {
        let work_days = history.on(day)?;
        if is_work_day(work_days, day) && !days_off.contains(&day) {
            working += 1;
        }
    }
    Some(if half_day {
        working.min(1)
    } else {
        working * 2
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    /// One schedule the whole time.
    fn count_halfdays(
        start: NaiveDate,
        end: NaiveDate,
        half_day: bool,
        work_days: &str,
        days_off: &HashSet<NaiveDate>,
    ) -> i64 {
        let history = ScheduleHistory::new(vec![(NaiveDate::MIN, Some(work_days.to_string()))]);
        count_halfdays_by_history(start, end, half_day, &history, days_off).expect("scheduled")
    }

    fn d(s: &str) -> NaiveDate {
        NaiveDate::parse_from_str(s, "%Y-%m-%d").unwrap()
    }

    #[test]
    fn service_counts_whole_months_from_the_hire_date() {
        assert!(has_served(d("2025-10-07"), 12, d("2026-10-07")));
        assert!(!has_served(d("2025-10-08"), 12, d("2026-10-07")));
        assert!(has_served(d("2026-10-07"), 0, d("2026-10-07")));
        // Hired on Jan 31: a month of service ends on the last day of February.
        assert!(has_served(d("2026-01-31"), 1, d("2026-02-28")));
        assert!(!has_served(d("2026-01-01"), -1, d("2026-02-01")));
    }

    #[test]
    fn the_grant_judges_a_later_year_from_january_first() {
        assert_eq!(grant_as_of(2026, d("2026-10-07")), d("2026-10-07"));
        assert_eq!(grant_as_of(2027, d("2026-12-15")), d("2027-01-01"));
    }

    const WEEKDAYS: &str = "MON,TUE,WED,THU,FRI";

    #[test]
    fn leave_counts_only_scheduled_days_that_are_not_holidays_off() {
        let none = HashSet::new();
        // Mon Oct 5 to Sun Oct 11, 2026: five work days.
        assert_eq!(
            count_halfdays(d("2026-10-05"), d("2026-10-11"), false, WEEKDAYS, &none),
            10
        );
        let off = HashSet::from([d("2026-10-07")]);
        assert_eq!(
            count_halfdays(d("2026-10-05"), d("2026-10-09"), false, WEEKDAYS, &off),
            8
        );
        // A weekend alone takes nothing.
        assert_eq!(
            count_halfdays(d("2026-10-10"), d("2026-10-11"), false, WEEKDAYS, &none),
            0
        );
        // A six-day schedule counts Saturday.
        assert_eq!(
            count_halfdays(
                d("2026-10-10"),
                d("2026-10-10"),
                false,
                "MON,TUE,WED,THU,FRI,SAT",
                &none
            ),
            2
        );
    }

    #[test]
    fn a_half_day_is_one_on_a_work_day_and_nothing_on_a_day_off() {
        let none = HashSet::new();
        assert_eq!(
            count_halfdays(d("2026-10-05"), d("2026-10-05"), true, WEEKDAYS, &none),
            1
        );
        assert_eq!(
            count_halfdays(d("2026-10-10"), d("2026-10-10"), true, WEEKDAYS, &none),
            0
        );
    }

    #[test]
    fn leave_across_a_schedule_change_counts_each_day_by_its_own_schedule() {
        // Mon–Sat from Mon Oct 12; Mon–Fri before. Fri Oct 16 is a holiday off.
        let history = ScheduleHistory::new(vec![
            (d("2020-03-02"), Some("MON,TUE,WED,THU,FRI".to_string())),
            (d("2026-10-12"), Some("MON,TUE,WED,THU,FRI,SAT".to_string())),
        ]);
        let off = HashSet::from([d("2026-10-16")]);
        // Sat 10 (old: rest) + Mon–Thu 12–15 + Sat 17 (new: work) = 5 days.
        assert_eq!(
            count_halfdays_by_history(d("2026-10-10"), d("2026-10-17"), false, &history, &off),
            Some(10)
        );
        assert_eq!(
            count_halfdays_by_history(d("2026-10-17"), d("2026-10-17"), true, &history, &off),
            Some(1)
        );
        assert_eq!(
            count_halfdays_by_history(d("2020-02-28"), d("2020-03-03"), false, &history, &off),
            None,
            "days before any schedule can't be counted"
        );
    }
}
