//! Leave types and balances (plan §6.4). Days are counted in half days, so 5 days is 10
//! and a half-day leave stays a whole number.

use chrono::{Months, NaiveDate};
use serde::{Deserialize, Serialize};
use sqlx::FromRow;
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

#[cfg(test)]
mod tests {
    use super::*;

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
}
