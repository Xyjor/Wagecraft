//! Attendance records and kiosk punches as the UI sees them (plan §5.3, §6.3).

use serde::{Deserialize, Serialize};
use sqlx::FromRow;
use ts_rs::TS;

/// One day of attendance for one employee. Times are company-local `YYYY-MM-DDTHH:MM:SS`.
#[derive(Debug, Clone, Serialize, FromRow, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct AttendanceRecord {
    #[ts(type = "number")]
    pub id: i64,
    #[ts(type = "number")]
    pub employee_id: i64,
    pub work_date: String,
    pub time_in: Option<String>,
    pub time_out: Option<String>,
    /// PRESENT, ABSENT, ON_LEAVE, REST_DAY or HOLIDAY.
    pub status: String,
    #[ts(type = "number")]
    pub late_minutes: i64,
    #[ts(type = "number")]
    pub undertime_minutes: i64,
    #[ts(type = "number")]
    pub worked_minutes: i64,
    #[ts(type = "number")]
    pub night_minutes: i64,
    /// CLOCK (kiosk) or MANUAL (entered by HR).
    pub source: String,
    pub needs_review: bool,
    pub review_note: Option<String>,
    /// Inside a posted payroll period, so it can no longer change.
    pub locked: bool,
}

/// Clock in or clock out.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "UPPERCASE")]
#[ts(export)]
pub enum PunchKind {
    In,
    Out,
}

/// What the kiosk shows after a punch. Only the first name, since anyone nearby can see it.
#[derive(Debug, Clone, Serialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct KioskPunch {
    pub first_name: String,
    pub kind: PunchKind,
    /// The local time recorded, `YYYY-MM-DDTHH:MM:SS`.
    pub at: String,
    pub work_date: String,
    #[ts(type = "number")]
    pub late_minutes: i64,
}

/// A date range for listing attendance, both ends included (`YYYY-MM-DD`).
#[derive(Debug, Clone, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct DateRange {
    pub from: String,
    pub to: String,
}

/// One employee's line on HR's day grid.
#[derive(Debug, Clone, Serialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct DayRow {
    #[ts(type = "number")]
    pub employee_id: i64,
    pub employee_no: String,
    /// "Dela Cruz, Juan".
    pub employee_name: String,
    pub department_name: Option<String>,
    /// The record's status, or what the day works out to without one (ABSENT, REST_DAY,
    /// HOLIDAY). `None` when the employee has no work schedule to tell.
    pub status: Option<String>,
    /// The holiday on this date, if any.
    pub holiday: Option<String>,
    pub record: Option<AttendanceRecord>,
}

/// A record flagged for HR, with whose it is.
#[derive(Debug, Clone, Serialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct ReviewItem {
    pub employee_no: String,
    pub employee_name: String,
    pub record: AttendanceRecord,
}

/// HR adding or correcting one day. Times are `HH:MM` on the office clock; a time out
/// earlier than the time in is the next day. The reason is required and audited.
#[derive(Debug, Clone, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct AttendanceInput {
    #[ts(type = "number")]
    pub employee_id: i64,
    pub work_date: String,
    pub time_in: String,
    pub time_out: Option<String>,
    pub reason: String,
}
