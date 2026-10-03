//! Overtime requests. Anyone with an employee record files and cancels their own; HR files
//! for others and decides (plan §3.2, §6.3).

use crate::auth::permissions::Permission;
use crate::domain::attendance::DateRange;
use crate::domain::overtime::{OvertimeInput, OvertimeRequest};
use crate::error::AppError;
use crate::services::{accounts, overtime};
use crate::state::AppState;
use chrono::{Local, Utc};
use tauri::State;

/// Files overtime. With an employee number, it's HR filing for someone, which needs
/// `overtime.decide`; without one, it's for the signed-in person.
#[tauri::command]
pub async fn overtime_request_create(
    state: State<'_, AppState>,
    input: OvertimeInput,
) -> Result<OvertimeRequest, AppError> {
    let session = if input.employee_no.is_some() {
        state.require(Permission::OvertimeDecide).await?
    } else {
        state.require(Permission::SelfAttendance).await?
    };
    let own = accounts::employee_of(&state.db, session.user_id).await?;
    let today = Local::now().date_naive();
    overtime::file(&state.db, session.actor(), own, input, today, Utc::now()).await
}

#[tauri::command]
pub async fn overtime_request_cancel(
    state: State<'_, AppState>,
    id: i64,
) -> Result<OvertimeRequest, AppError> {
    let session = state.require(Permission::SelfAttendance).await?;
    let own = accounts::employee_of(&state.db, session.user_id).await?;
    let can_decide = session.role.allows(Permission::OvertimeDecide);
    overtime::cancel(&state.db, session.actor(), own, can_decide, id, Utc::now()).await
}

/// The signed-in person's own requests. The employee comes from the session, never the UI.
#[tauri::command]
pub async fn overtime_mine(
    state: State<'_, AppState>,
    range: DateRange,
) -> Result<Vec<OvertimeRequest>, AppError> {
    let session = state.require(Permission::SelfAttendance).await?;
    let Some(id) = accounts::employee_of(&state.db, session.user_id).await? else {
        return Ok(Vec::new());
    };
    overtime::mine(&state.db, id, &range).await
}

#[tauri::command]
pub async fn overtime_list(
    state: State<'_, AppState>,
    status: Option<String>,
    range: DateRange,
) -> Result<Vec<OvertimeRequest>, AppError> {
    state.require(Permission::OvertimeDecide).await?;
    overtime::list(&state.db, status.as_deref(), &range).await
}

#[tauri::command]
pub async fn overtime_pending(
    state: State<'_, AppState>,
) -> Result<Vec<OvertimeRequest>, AppError> {
    state.require(Permission::OvertimeDecide).await?;
    overtime::pending(&state.db).await
}

#[tauri::command]
pub async fn overtime_decide(
    state: State<'_, AppState>,
    id: i64,
    approve: bool,
    note: Option<String>,
) -> Result<OvertimeRequest, AppError> {
    let session = state.require(Permission::OvertimeDecide).await?;
    let own = accounts::employee_of(&state.db, session.user_id).await?;
    overtime::decide(
        &state.db,
        session.actor(),
        own,
        id,
        approve,
        note,
        Utc::now(),
    )
    .await
}
