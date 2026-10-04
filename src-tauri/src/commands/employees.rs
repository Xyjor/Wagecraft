//! Employee records. Reading needs `employee.read_all`; changes need `employee.write`.
//! Staff see only their own record, through `employee_me`.

use crate::auth::kiosk_lock::KioskLock;
use crate::auth::permissions::Permission;
use crate::domain::compensation::{Compensation, CompensationInput};
use crate::domain::employee::{Employee, EmployeeInput, EmployeePage, EmployeeQuery};
use crate::domain::schedule::{ScheduleAssignment, ScheduleChangeInput};
use crate::error::AppError;
use crate::services::accounts::{self, AccountSummary, NewStaffAccount};
use crate::services::{compensation, employees, kiosk, schedules};
use crate::state::AppState;
use chrono::{Local, Utc};
use tauri::State;

#[tauri::command]
pub async fn employee_list(
    state: State<'_, AppState>,
    query: EmployeeQuery,
) -> Result<EmployeePage, AppError> {
    state.require(Permission::EmployeeReadAll).await?;
    employees::list(&state.db, query).await
}

#[tauri::command]
pub async fn employee_get(state: State<'_, AppState>, id: i64) -> Result<Employee, AppError> {
    state.require(Permission::EmployeeReadAll).await?;
    employees::get(&state.db, id).await
}

/// The signed-in user's own record. "Self" commands find the employee from the signed-in
/// account, never from an id the UI sends (plan §3.3).
#[tauri::command]
pub async fn employee_me(state: State<'_, AppState>) -> Result<Employee, AppError> {
    let session = state.require(Permission::SelfProfile).await?;
    // Read the link fresh: HR may have linked or unlinked this account since sign-in.
    let id = accounts::employee_of(&state.db, session.user_id)
        .await?
        .ok_or(AppError::NotFound("Employee"))?;
    employees::get(&state.db, id).await
}

#[tauri::command]
pub async fn employee_create(
    state: State<'_, AppState>,
    input: EmployeeInput,
) -> Result<Employee, AppError> {
    let session = state.require(Permission::EmployeeWrite).await?;
    // "Today" is the PC's calendar day, which is the company's day (plan §5.1).
    let today = Local::now().date_naive();
    employees::create(&state.db, session.actor(), input, today, Utc::now()).await
}

#[tauri::command]
pub async fn employee_update(
    state: State<'_, AppState>,
    id: i64,
    input: EmployeeInput,
) -> Result<Employee, AppError> {
    let session = state.require(Permission::EmployeeWrite).await?;
    let today = Local::now().date_naive();
    employees::update(&state.db, session.actor(), id, input, today, Utc::now()).await
}

#[tauri::command]
pub async fn employee_archive(
    state: State<'_, AppState>,
    id: i64,
    archived: bool,
) -> Result<(), AppError> {
    let session = state.require(Permission::EmployeeWrite).await?;
    employees::set_archived(&state.db, session.actor(), id, archived, Utc::now()).await
}

#[tauri::command]
pub async fn employee_compensation_history(
    state: State<'_, AppState>,
    id: i64,
) -> Result<Vec<Compensation>, AppError> {
    state.require(Permission::EmployeeReadAll).await?;
    compensation::history(&state.db, id).await
}

#[tauri::command]
pub async fn employee_add_compensation(
    state: State<'_, AppState>,
    id: i64,
    input: CompensationInput,
) -> Result<Compensation, AppError> {
    let session = state.require(Permission::EmployeeWrite).await?;
    compensation::add(&state.db, session.actor(), id, input, Utc::now()).await
}

#[tauri::command]
pub async fn employee_schedule_history(
    state: State<'_, AppState>,
    id: i64,
) -> Result<Vec<ScheduleAssignment>, AppError> {
    state.require(Permission::EmployeeReadAll).await?;
    schedules::history(&state.db, id).await
}

#[tauri::command]
pub async fn employee_change_schedule(
    state: State<'_, AppState>,
    id: i64,
    input: ScheduleChangeInput,
) -> Result<Vec<ScheduleAssignment>, AppError> {
    let session = state.require(Permission::EmployeeWrite).await?;
    schedules::change(&state.db, session.actor(), id, input, Utc::now()).await
}

/// The sign-in account linked to this employee, if any.
#[tauri::command]
pub async fn employee_account(
    state: State<'_, AppState>,
    id: i64,
) -> Result<Option<AccountSummary>, AppError> {
    state.require(Permission::EmployeeReadAll).await?;
    accounts::linked(&state.db, id).await
}

/// Accounts not linked to anyone, for linking an Admin or HR user to their own record.
#[tauri::command]
pub async fn employee_linkable_accounts(
    state: State<'_, AppState>,
) -> Result<Vec<AccountSummary>, AppError> {
    state.require(Permission::EmployeeWrite).await?;
    accounts::linkable(&state.db).await
}

#[tauri::command]
pub async fn employee_create_account(
    state: State<'_, AppState>,
    id: i64,
    input: NewStaffAccount,
) -> Result<AccountSummary, AppError> {
    let session = state.require(Permission::EmployeeWrite).await?;
    accounts::create_staff(&state.db, session.actor(), id, input, Utc::now()).await
}

#[tauri::command]
pub async fn employee_link_user(
    state: State<'_, AppState>,
    id: i64,
    user_id: i64,
) -> Result<AccountSummary, AppError> {
    let session = state.require(Permission::EmployeeWrite).await?;
    accounts::link(&state.db, session.actor(), id, user_id, Utc::now()).await
}

#[tauri::command]
pub async fn employee_unlink_user(state: State<'_, AppState>, id: i64) -> Result<(), AppError> {
    let session = state.require(Permission::EmployeeWrite).await?;
    accounts::unlink(&state.db, session.actor(), id, Utc::now()).await
}

/// Sets or replaces the employee's kiosk PIN, and lifts any kiosk lockout on their number.
#[tauri::command]
pub async fn employee_set_kiosk_pin(
    state: State<'_, AppState>,
    id: i64,
    pin: String,
) -> Result<Employee, AppError> {
    let session = state.require(Permission::EmployeeWrite).await?;
    let employee_no = kiosk::set_pin(&state.db, session.actor(), id, &pin, Utc::now()).await?;
    state.kiosk.clear(&KioskLock::key(&employee_no));
    employees::get(&state.db, id).await
}
