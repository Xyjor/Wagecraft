//! The Admin and HR dashboard (plan §6.9, F-14): one summary, so the page makes one call.

use serde::Serialize;
use ts_rs::TS;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct TeamDashboard {
    /// Employees hired by today and not separated before it.
    #[ts(type = "number")]
    pub headcount: i64,
    /// Hired from the 1st of this month up to today.
    #[ts(type = "number")]
    pub new_hires_this_month: i64,
    pub today: TodayCounts,
    #[ts(type = "number")]
    pub pending_leave: i64,
    #[ts(type = "number")]
    pub pending_overtime: i64,
    /// Most people first; employees without a department come last.
    pub by_department: Vec<DepartmentCount>,
    /// The last six posted periods, oldest first.
    pub payroll_cost: Vec<PeriodCost>,
}

/// Today's attendance so far. Someone who is due today but hasn't clocked in yet counts
/// as not in, since the day isn't over.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct TodayCounts {
    #[ts(type = "number")]
    pub present: i64,
    #[ts(type = "number")]
    pub late: i64,
    #[ts(type = "number")]
    pub not_in: i64,
    #[ts(type = "number")]
    pub on_leave: i64,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct DepartmentCount {
    /// `None` for employees without a department.
    pub department: Option<String>,
    #[ts(type = "number")]
    pub count: i64,
}

/// What a posted period cost the company: gross pay plus the employer's contributions.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct PeriodCost {
    pub period_start: String,
    pub period_end: String,
    #[ts(type = "number")]
    pub gross_cents: i64,
    #[ts(type = "number")]
    pub employer_cents: i64,
    #[ts(type = "number")]
    pub cost_cents: i64,
}

/// A staff member's own dashboard (plan §6.9, staff column): today, this cutoff so far,
/// and what they're still waiting on.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct MyDashboard {
    pub today: MyDay,
    pub cutoff: CutoffSummary,
    /// Their own leave requests still waiting for HR.
    #[ts(type = "number")]
    pub pending_leave: i64,
    /// Their own overtime requests still waiting for HR.
    #[ts(type = "number")]
    pub pending_overtime: i64,
}

/// How today stands for them, the same way HR's attendance grid shows it.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct MyDay {
    pub date: String,
    /// `PRESENT`, `ABSENT`, `ON_LEAVE`, `REST_DAY` or `HOLIDAY`. `ABSENT` with no time in
    /// means not clocked in yet. `None` when they have no schedule.
    pub status: Option<String>,
    pub time_in: Option<String>,
    pub time_out: Option<String>,
    #[ts(type = "number")]
    pub late_minutes: i64,
    pub holiday: Option<String>,
    pub leave: Option<String>,
}

/// The semi-monthly cutoff that holds today, counted up to today.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct CutoffSummary {
    pub period_start: String,
    pub period_end: String,
    #[ts(type = "number")]
    pub days_present: i64,
    #[ts(type = "number")]
    pub late_minutes: i64,
    /// Approved overtime with a work date in the cutoff.
    #[ts(type = "number")]
    pub overtime_minutes: i64,
}
