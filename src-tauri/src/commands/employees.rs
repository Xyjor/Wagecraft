//! Employee records. Reading needs `employee.read_all`; changes need `employee.write`.
//! Staff see only their own record, through `employee_me`.

use crate::auth::permissions::Permission;
use crate::domain::employee::{Employee, EmployeeInput, EmployeePage, EmployeeQuery};
use crate::error::AppError;
use crate::services::employees;
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

/// The signed-in user's own record. Uses the session's employee, never an id from the UI.
#[tauri::command]
pub async fn employee_me(state: State<'_, AppState>) -> Result<Employee, AppError> {
    let session = state.require(Permission::SelfProfile).await?;
    let id = session.employee_id.ok_or(AppError::NotFound("Employee"))?;
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
