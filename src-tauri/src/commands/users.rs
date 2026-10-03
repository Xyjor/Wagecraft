//! User management for Admins. Each command checks `user.manage`, then calls the service.

use crate::auth::permissions::{Permission, Role};
use crate::error::AppError;
use crate::services::users::{self, NewUser, UserSummary};
use crate::state::AppState;
use chrono::Utc;
use tauri::State;

#[tauri::command]
pub async fn user_list(state: State<'_, AppState>) -> Result<Vec<UserSummary>, AppError> {
    state.auth.require(Permission::UserManage)?;
    users::list(&state.db, Utc::now()).await
}

#[tauri::command]
pub async fn user_create(
    state: State<'_, AppState>,
    input: NewUser,
) -> Result<UserSummary, AppError> {
    let session = state.auth.require(Permission::UserManage)?;
    users::create(&state.db, session.actor(), input, Utc::now()).await
}

#[tauri::command]
pub async fn user_update(state: State<'_, AppState>, id: i64, role: Role) -> Result<(), AppError> {
    let session = state.auth.require(Permission::UserManage)?;
    users::set_role(&state.db, session.actor(), id, role, Utc::now()).await
}

#[tauri::command]
pub async fn user_set_active(
    state: State<'_, AppState>,
    id: i64,
    active: bool,
) -> Result<(), AppError> {
    let session = state.auth.require(Permission::UserManage)?;
    users::set_active(&state.db, session.actor(), id, active, Utc::now()).await
}

#[tauri::command]
pub async fn user_reset_password(
    state: State<'_, AppState>,
    id: i64,
    temporary_password: String,
) -> Result<(), AppError> {
    let session = state.auth.require(Permission::UserManage)?;
    users::reset_password(
        &state.db,
        session.actor(),
        id,
        &temporary_password,
        Utc::now(),
    )
    .await
}
