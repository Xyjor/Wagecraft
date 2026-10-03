//! Reading attendance records (plan §6.3). HR's grid and corrections come next.

use crate::domain::attendance::{AttendanceRecord, DateRange};
use crate::error::AppError;
use crate::repositories::attendance as repo;
use crate::services::auth::field;
use chrono::NaiveDate;
use sqlx::SqlitePool;

/// Longest range one request may ask for: two months and change.
const MAX_RANGE_DAYS: i64 = 62;

/// One employee's records in the range, newest first.
pub async fn for_employee(
    db: &SqlitePool,
    employee_id: i64,
    range: &DateRange,
) -> Result<Vec<AttendanceRecord>, AppError> {
    let (from, to) = check_range(range)?;
    Ok(repo::for_employee(db, employee_id, &from.to_string(), &to.to_string()).await?)
}

fn check_range(range: &DateRange) -> Result<(NaiveDate, NaiveDate), AppError> {
    let from = NaiveDate::parse_from_str(range.from.trim(), "%Y-%m-%d");
    let to = NaiveDate::parse_from_str(range.to.trim(), "%Y-%m-%d");
    let (Ok(from), Ok(to)) = (from, to) else {
        return Err(AppError::Validation(vec![field(
            "from",
            "Pick a valid date range",
        )]));
    };
    if to < from {
        return Err(AppError::Validation(vec![field(
            "to",
            "The end date is before the start date",
        )]));
    }
    if (to - from).num_days() > MAX_RANGE_DAYS {
        return Err(AppError::Validation(vec![field(
            "to",
            "Pick at most two months at a time",
        )]));
    }
    Ok((from, to))
}
