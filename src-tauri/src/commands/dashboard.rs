//! The dashboards (plan §6.9). Admin and HR see the team's numbers; everyone with an
//! employee record sees their own.

use crate::auth::permissions::Permission;
use crate::domain::dashboard::{MyDashboard, TeamDashboard};
use crate::error::AppError;
use crate::services::{accounts, dashboard};
use crate::state::AppState;
use crate::time::local_now;
use tauri::State;

#[tauri::command]
pub async fn dashboard_team(state: State<'_, AppState>) -> Result<TeamDashboard, AppError> {
    state.require(Permission::AttendanceReadAll).await?;
    dashboard::team(&state.db, local_now().date()).await
}

/// The signed-in person's own dashboard. The employee comes from the session, never the UI.
#[tauri::command]
pub async fn dashboard_mine(state: State<'_, AppState>) -> Result<MyDashboard, AppError> {
    let session = state.require(Permission::SelfAttendance).await?;
    let id = accounts::employee_of(&state.db, session.user_id)
        .await?
        .ok_or(AppError::NotFound("Employee"))?;
    dashboard::mine(&state.db, id, local_now().date()).await
}
