//! Leave types and balances. Everyone sees the types and their own balances; Admin
//! manages types; HR grants and adjusts balances (plan §3.2, §6.4).

use crate::auth::permissions::Permission;
use crate::domain::leave::{BalanceAdjustment, LeaveBalance, LeaveType, LeaveTypeInput};
use crate::error::AppError;
use crate::services::{accounts, leave};
use crate::state::AppState;
use chrono::{Local, Utc};
use tauri::State;

#[tauri::command]
pub async fn leave_types_list(state: State<'_, AppState>) -> Result<Vec<LeaveType>, AppError> {
    state.require(Permission::SelfLeave).await?;
    leave::list_types(&state.db).await
}

#[tauri::command]
pub async fn leave_type_create(
    state: State<'_, AppState>,
    input: LeaveTypeInput,
) -> Result<LeaveType, AppError> {
    let session = state.require(Permission::SettingsManage).await?;
    leave::create_type(&state.db, session.actor(), input, Utc::now()).await
}

#[tauri::command]
pub async fn leave_type_update(
    state: State<'_, AppState>,
    id: i64,
    input: LeaveTypeInput,
) -> Result<LeaveType, AppError> {
    let session = state.require(Permission::SettingsManage).await?;
    leave::update_type(&state.db, session.actor(), id, input, Utc::now()).await
}

#[tauri::command]
pub async fn leave_type_set_active(
    state: State<'_, AppState>,
    id: i64,
    active: bool,
) -> Result<(), AppError> {
    let session = state.require(Permission::SettingsManage).await?;
    leave::set_type_active(&state.db, session.actor(), id, active, Utc::now()).await
}

/// The signed-in person's balances. The employee comes from the session, never the UI.
#[tauri::command]
pub async fn leave_my_balances(
    state: State<'_, AppState>,
    year: i32,
) -> Result<Vec<LeaveBalance>, AppError> {
    let session = state.require(Permission::SelfLeave).await?;
    let Some(id) = accounts::employee_of(&state.db, session.user_id).await? else {
        return Ok(Vec::new());
    };
    let today = Local::now().date_naive();
    leave::balances(&state.db, session.actor(), id, year, today, Utc::now()).await
}

#[tauri::command]
pub async fn leave_balances_for(
    state: State<'_, AppState>,
    employee_id: i64,
    year: i32,
) -> Result<Vec<LeaveBalance>, AppError> {
    let session = state.require(Permission::LeaveDecide).await?;
    let today = Local::now().date_naive();
    leave::balances(
        &state.db,
        session.actor(),
        employee_id,
        year,
        today,
        Utc::now(),
    )
    .await
}

/// Adds the year's missing balances for every current employee. Returns how many.
#[tauri::command]
pub async fn leave_balance_grant(state: State<'_, AppState>, year: i32) -> Result<usize, AppError> {
    let session = state.require(Permission::LeaveDecide).await?;
    let today = Local::now().date_naive();
    leave::grant(&state.db, session.actor(), year, today, Utc::now()).await
}

#[tauri::command]
pub async fn leave_balance_adjust(
    state: State<'_, AppState>,
    id: i64,
    input: BalanceAdjustment,
) -> Result<LeaveBalance, AppError> {
    let session = state.require(Permission::LeaveDecide).await?;
    leave::adjust(&state.db, session.actor(), id, input, Utc::now()).await
}
