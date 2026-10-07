//! The dashboard (plan §6.9). Admin and HR see the team's numbers.

use crate::auth::permissions::Permission;
use crate::domain::dashboard::TeamDashboard;
use crate::error::AppError;
use crate::services::dashboard;
use crate::state::AppState;
use crate::time::local_now;
use tauri::State;

#[tauri::command]
pub async fn dashboard_team(state: State<'_, AppState>) -> Result<TeamDashboard, AppError> {
    state.require(Permission::AttendanceReadAll).await?;
    dashboard::team(&state.db, local_now().date()).await
}
