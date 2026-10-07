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
