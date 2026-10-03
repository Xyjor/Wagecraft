//! Employee records (plan §5.3, §6.2). Dates cross IPC as `YYYY-MM-DD` strings.

use serde::{Deserialize, Serialize};
use sqlx::FromRow;
use ts_rs::TS;

/// What HR types on the employee form. Blank optional fields arrive as `null` or "".
#[derive(Debug, Clone, Default, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct EmployeeInput {
    pub employee_no: String,
    pub first_name: String,
    pub middle_name: Option<String>,
    pub last_name: String,
    pub suffix: Option<String>,
    pub birth_date: Option<String>,
    pub sex: Option<String>,
    pub civil_status: Option<String>,
    pub email: Option<String>,
    pub mobile: Option<String>,
    pub address: Option<String>,
    pub hire_date: String,
    pub regularization_date: Option<String>,
    pub separation_date: Option<String>,
    pub employment_status: String,
    #[ts(type = "number | null")]
    pub department_id: Option<i64>,
    #[ts(type = "number | null")]
    pub position_id: Option<i64>,
    #[ts(type = "number | null")]
    pub schedule_id: Option<i64>,
    pub tin: Option<String>,
    pub sss_no: Option<String>,
    pub philhealth_no: Option<String>,
    pub pagibig_no: Option<String>,
    pub bank_name: Option<String>,
    pub bank_account_no: Option<String>,
}

/// The full profile. Never carries the kiosk PIN hash.
#[derive(Debug, Clone, Serialize, FromRow, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct Employee {
    #[ts(type = "number")]
    pub id: i64,
    pub employee_no: String,
    pub first_name: String,
    pub middle_name: Option<String>,
    pub last_name: String,
    pub suffix: Option<String>,
    pub birth_date: Option<String>,
    pub sex: Option<String>,
    pub civil_status: Option<String>,
    pub email: Option<String>,
    pub mobile: Option<String>,
    pub address: Option<String>,
    pub hire_date: String,
    pub regularization_date: Option<String>,
    pub separation_date: Option<String>,
    pub employment_status: String,
    #[ts(type = "number | null")]
    pub department_id: Option<i64>,
    pub department_name: Option<String>,
    #[ts(type = "number | null")]
    pub position_id: Option<i64>,
    pub position_title: Option<String>,
    #[ts(type = "number | null")]
    pub schedule_id: Option<i64>,
    pub tin: Option<String>,
    pub sss_no: Option<String>,
    pub philhealth_no: Option<String>,
    pub pagibig_no: Option<String>,
    pub bank_name: Option<String>,
    pub bank_account_no: Option<String>,
    pub has_kiosk_pin: bool,
    pub archived_at: Option<String>,
}

/// One row of the employee list. Government IDs stay on the profile.
#[derive(Debug, Clone, Serialize, FromRow, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct EmployeeListItem {
    #[ts(type = "number")]
    pub id: i64,
    pub employee_no: String,
    pub last_name: String,
    pub first_name: String,
    pub middle_name: Option<String>,
    pub suffix: Option<String>,
    pub department_name: Option<String>,
    pub position_title: Option<String>,
    pub employment_status: String,
    pub hire_date: String,
    pub archived: bool,
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Deserialize, Serialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub enum EmployeeSort {
    #[default]
    Name,
    EmployeeNo,
    Department,
    HireDate,
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Deserialize, Serialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub enum Archived {
    /// Only current employees (the default list).
    #[default]
    Exclude,
    Only,
    Include,
}

/// Search, filters, sort and page for `employee_list` (plan §4.6).
#[derive(Debug, Clone, Default, Deserialize, Serialize, TS)]
#[serde(rename_all = "camelCase", default)]
#[ts(export)]
pub struct EmployeeQuery {
    /// Matches employee number, first or last name, full name, or email.
    pub search: Option<String>,
    #[ts(type = "number | null")]
    pub department_id: Option<i64>,
    #[ts(type = "number | null")]
    pub position_id: Option<i64>,
    pub employment_status: Option<String>,
    pub archived: Archived,
    pub sort: EmployeeSort,
    pub descending: bool,
    /// 1-based.
    #[ts(type = "number")]
    pub page: i64,
    #[ts(type = "number")]
    pub page_size: i64,
}

#[derive(Debug, Clone, Serialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct EmployeePage {
    pub items: Vec<EmployeeListItem>,
    #[ts(type = "number")]
    pub total: i64,
    #[ts(type = "number")]
    pub page: i64,
    #[ts(type = "number")]
    pub page_size: i64,
}

pub const EMPLOYMENT_STATUSES: [&str; 5] = [
    "PROBATIONARY",
    "REGULAR",
    "CONTRACTUAL",
    "RESIGNED",
    "TERMINATED",
];
pub const SEXES: [&str; 2] = ["MALE", "FEMALE"];
pub const CIVIL_STATUSES: [&str; 4] = ["SINGLE", "MARRIED", "WIDOWED", "SEPARATED"];
