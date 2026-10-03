//! Departments and positions (plan §6.2).

use serde::{Deserialize, Serialize};
use sqlx::FromRow;
use ts_rs::TS;

#[derive(Debug, Clone, Serialize, FromRow, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct Department {
    #[ts(type = "number")]
    pub id: i64,
    pub code: String,
    pub name: String,
    pub description: Option<String>,
    pub is_active: bool,
}

#[derive(Debug, Clone, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct DepartmentInput {
    pub code: String,
    pub name: String,
    pub description: Option<String>,
}

#[derive(Debug, Clone, Serialize, FromRow, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct Position {
    #[ts(type = "number")]
    pub id: i64,
    #[ts(type = "number")]
    pub department_id: i64,
    pub department_name: String,
    pub title: String,
    /// Monthly salary range in centavos. Outside it is a warning, never an error.
    #[ts(type = "number | null")]
    pub min_rate_cents: Option<i64>,
    #[ts(type = "number | null")]
    pub max_rate_cents: Option<i64>,
    pub is_active: bool,
}

#[derive(Debug, Clone, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct PositionInput {
    #[ts(type = "number")]
    pub department_id: i64,
    pub title: String,
    #[ts(type = "number | null")]
    pub min_rate_cents: Option<i64>,
    #[ts(type = "number | null")]
    pub max_rate_cents: Option<i64>,
}

/// Weekday codes as stored in `work_schedules.work_days`, in calendar order.
pub const WEEKDAYS: [&str; 7] = ["MON", "TUE", "WED", "THU", "FRI", "SAT", "SUN"];

/// A work schedule (plan §5.3). Times are company-local `HH:MM`; an end time that is not
/// after the start time means the shift ends the next day (22:00–06:00).
#[derive(Debug, Clone, Serialize, FromRow, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct WorkSchedule {
    #[ts(type = "number")]
    pub id: i64,
    pub name: String,
    pub start_time: String,
    pub end_time: String,
    #[ts(type = "number")]
    pub break_minutes: i64,
    #[ts(type = "number")]
    pub grace_minutes: i64,
    /// Comma-separated weekday codes in calendar order, like `MON,TUE,WED,THU,FRI`.
    pub work_days: String,
    pub is_active: bool,
    /// How many employees (archived ones included) follow this schedule.
    #[ts(type = "number")]
    pub employee_count: i64,
}

#[derive(Debug, Clone, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct WorkScheduleInput {
    pub name: String,
    pub start_time: String,
    pub end_time: String,
    #[ts(type = "number")]
    pub break_minutes: i64,
    #[ts(type = "number")]
    pub grace_minutes: i64,
    pub work_days: String,
}
