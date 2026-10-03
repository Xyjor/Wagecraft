//! Salary history (plan §6.2). A change never overwrites: it adds a row and closes the
//! one before it, so an old payroll can always be recomputed with the rate it used.

use chrono::{Datelike, NaiveDate};
use serde::{Deserialize, Serialize};
use sqlx::FromRow;
use ts_rs::TS;

pub const PAY_BASES: [&str; 2] = ["MONTHLY", "DAILY"];

#[derive(Debug, Clone, Serialize, FromRow, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct Compensation {
    #[ts(type = "number")]
    pub id: i64,
    #[ts(type = "number")]
    pub employee_id: i64,
    pub pay_basis: String,
    #[ts(type = "number")]
    pub rate_cents: i64,
    pub effective_from: String,
    /// Empty while this is the rate in effect now.
    pub effective_to: Option<String>,
    pub reason: Option<String>,
    /// Who made the change, or empty if that user is gone.
    pub created_by_name: Option<String>,
    pub created_at: String,
}

#[derive(Debug, Clone, Default, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct CompensationInput {
    pub pay_basis: String,
    #[ts(type = "number")]
    pub rate_cents: i64,
    pub effective_from: String,
    pub reason: Option<String>,
}

/// The first day of the semi-monthly pay period that holds `d`: the 1st or the 16th
/// (plan default: semi-monthly pay).
pub fn pay_period_start(d: NaiveDate) -> NaiveDate {
    let day = if d.day() <= 15 { 1 } else { 16 };
    d.with_day(day)
        .expect("the 1st and 16th exist in every month")
}

#[cfg(test)]
mod tests {
    use super::*;

    fn d(s: &str) -> NaiveDate {
        NaiveDate::parse_from_str(s, "%Y-%m-%d").unwrap()
    }

    #[test]
    fn periods_start_on_the_first_and_sixteenth() {
        assert_eq!(pay_period_start(d("2026-02-01")), d("2026-02-01"));
        assert_eq!(pay_period_start(d("2026-02-15")), d("2026-02-01"));
        assert_eq!(pay_period_start(d("2026-02-16")), d("2026-02-16"));
        assert_eq!(pay_period_start(d("2026-02-28")), d("2026-02-16"));
    }
}
