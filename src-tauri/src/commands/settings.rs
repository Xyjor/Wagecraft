//! The Settings screen (plan §5.3). Admin only: `settings.manage`.

use crate::auth::permissions::Permission;
use crate::domain::settings::{Settings, SettingsInput};
use crate::error::AppError;
use crate::export::files;
use crate::services::settings;
use crate::state::AppState;
use chrono::Utc;
use std::time::Duration;
use tauri::{AppHandle, State};

#[tauri::command]
pub async fn settings_get(state: State<'_, AppState>) -> Result<Settings, AppError> {
    state.require(Permission::SettingsManage).await?;
    settings::load(&state.db, &state.default_backup_dir).await
}

/// Saves the settings. A new idle timeout applies straight away, to this session too.
#[tauri::command]
pub async fn settings_update(
    state: State<'_, AppState>,
    input: SettingsInput,
) -> Result<Settings, AppError> {
    let session = state.require(Permission::SettingsManage).await?;
    let saved = settings::update(
        &state.db,
        session.actor(),
        input,
        &state.default_backup_dir,
        Utc::now(),
    )
    .await?;
    let minutes = saved.idle_timeout_minutes.unsigned_abs();
    state
        .auth
        .set_idle_timeout(Duration::from_secs(minutes * 60));
    Ok(saved)
}

/// Asks for the daily backups' folder. Returns it, or nothing if Admin cancelled. It is
/// only used once Admin saves the settings.
#[tauri::command]
pub async fn settings_pick_backup_folder(
    app: AppHandle,
    state: State<'_, AppState>,
) -> Result<Option<String>, AppError> {
    state.require(Permission::SettingsManage).await?;
    let picked = files::pick_folder(&app, "Choose where daily backups go").await?;
    Ok(picked.map(|p| p.display().to_string()))
}
