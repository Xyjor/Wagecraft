//! Departments and positions. Lists need `employee.read_all`; changes need `org.manage`.

use crate::auth::permissions::Permission;
use crate::domain::org::{Department, DepartmentInput, Position, PositionInput};
use crate::error::AppError;
use crate::services::org;
use crate::state::AppState;
use chrono::Utc;
use tauri::State;

#[tauri::command]
pub async fn department_list(state: State<'_, AppState>) -> Result<Vec<Department>, AppError> {
    state.require(Permission::EmployeeReadAll).await?;
    org::list_departments(&state.db).await
}

#[tauri::command]
pub async fn department_create(
    state: State<'_, AppState>,
    input: DepartmentInput,
) -> Result<Department, AppError> {
    let session = state.require(Permission::OrgManage).await?;
    org::create_department(&state.db, session.actor(), input, Utc::now()).await
}

#[tauri::command]
pub async fn department_update(
    state: State<'_, AppState>,
    id: i64,
    input: DepartmentInput,
) -> Result<Department, AppError> {
    let session = state.require(Permission::OrgManage).await?;
    org::update_department(&state.db, session.actor(), id, input, Utc::now()).await
}

#[tauri::command]
pub async fn department_set_active(
    state: State<'_, AppState>,
    id: i64,
    active: bool,
) -> Result<(), AppError> {
    let session = state.require(Permission::OrgManage).await?;
    org::set_department_active(&state.db, session.actor(), id, active, Utc::now()).await
}

#[tauri::command]
pub async fn position_list(state: State<'_, AppState>) -> Result<Vec<Position>, AppError> {
    state.require(Permission::EmployeeReadAll).await?;
    org::list_positions(&state.db).await
}

#[tauri::command]
pub async fn position_create(
    state: State<'_, AppState>,
    input: PositionInput,
) -> Result<Position, AppError> {
    let session = state.require(Permission::OrgManage).await?;
    org::create_position(&state.db, session.actor(), input, Utc::now()).await
}

#[tauri::command]
pub async fn position_update(
    state: State<'_, AppState>,
    id: i64,
    input: PositionInput,
) -> Result<Position, AppError> {
    let session = state.require(Permission::OrgManage).await?;
    org::update_position(&state.db, session.actor(), id, input, Utc::now()).await
}

#[tauri::command]
pub async fn position_set_active(
    state: State<'_, AppState>,
    id: i64,
    active: bool,
) -> Result<(), AppError> {
    let session = state.require(Permission::OrgManage).await?;
    org::set_position_active(&state.db, session.actor(), id, active, Utc::now()).await
}
