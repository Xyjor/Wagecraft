//! Sign-in commands. Each one only checks the session and calls the auth service.

use crate::auth::permissions::{Permission, Role};
use crate::auth::session::Session;
use crate::error::AppError;
use crate::services::auth::{self, User};
use crate::state::AppState;
use serde::Serialize;
use std::time::Instant;
use tauri::State;
use ts_rs::TS;

#[derive(Debug, Serialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct SetupStatus {
    pub needs_setup: bool,
}

/// The signed-in user, as the UI sees it.
#[derive(Debug, Serialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct Me {
    #[ts(type = "number")]
    pub user_id: i64,
    pub username: String,
    pub role: Role,
    #[ts(type = "number | null")]
    pub employee_id: Option<i64>,
    pub must_change_password: bool,
    pub theme: String,
}

impl From<User> for Me {
    fn from(u: User) -> Self {
        Me {
            user_id: u.id,
            username: u.username,
            role: u.role,
            employee_id: u.employee_id,
            must_change_password: u.must_change_password,
            theme: u.theme,
        }
    }
}

#[tauri::command]
pub async fn auth_setup_status(state: State<'_, AppState>) -> Result<SetupStatus, AppError> {
    Ok(SetupStatus {
        needs_setup: auth::needs_setup(&state.db).await?,
    })
}

#[tauri::command]
pub async fn auth_setup_create_admin(
    state: State<'_, AppState>,
    company_name: String,
    username: String,
    password: String,
) -> Result<(), AppError> {
    auth::create_admin(&state.db, &company_name, &username, &password).await
}

#[tauri::command]
pub async fn auth_login(
    state: State<'_, AppState>,
    username: String,
    password: String,
) -> Result<Me, AppError> {
    let user = auth::login(&state.db, &username, &password, chrono::Utc::now()).await?;
    state.auth.sign_in(Session {
        user_id: user.id,
        employee_id: user.employee_id,
        role: user.role,
        last_activity: Instant::now(),
    });
    Ok(user.into())
}

#[tauri::command]
pub async fn auth_logout(state: State<'_, AppState>) -> Result<(), AppError> {
    state.auth.sign_out();
    Ok(())
}

#[tauri::command]
pub async fn auth_me(state: State<'_, AppState>) -> Result<Me, AppError> {
    let session = state.auth.require(Permission::SelfProfile)?;
    Ok(auth::find_user(&state.db, session.user_id).await?.into())
}

#[tauri::command]
pub async fn auth_change_password(
    state: State<'_, AppState>,
    current_password: String,
    new_password: String,
) -> Result<(), AppError> {
    let session = state.auth.require(Permission::SelfProfile)?;
    auth::change_password(&state.db, session.user_id, &current_password, &new_password).await
}
