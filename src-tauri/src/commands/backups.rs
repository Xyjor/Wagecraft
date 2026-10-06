//! The Backups screen (plan §6.7). Admin only: `backup.manage`.

use crate::auth::permissions::Permission;
use crate::domain::backup::BackupEntry;
use crate::error::AppError;
use crate::export::files;
use crate::services::backups;
use crate::state::AppState;
use chrono::{Local, Utc};
use tauri::{AppHandle, State};

#[tauri::command]
pub async fn backup_list(state: State<'_, AppState>) -> Result<Vec<BackupEntry>, AppError> {
    state.require(Permission::BackupManage).await?;
    backups::history(&state.db).await
}

/// **Back up now**: asks for a folder, such as a USB drive, and writes a backup there.
/// Returns the backup's path, or nothing if Admin cancelled the folder picker.
#[tauri::command]
pub async fn backup_create(
    app: AppHandle,
    state: State<'_, AppState>,
) -> Result<Option<String>, AppError> {
    let session = state.require(Permission::BackupManage).await?;
    let Some(dir) = files::pick_folder(&app, "Choose where to save the backup").await? else {
        return Ok(None);
    };
    let backup = backups::back_up_now(
        &state.db,
        session.actor(),
        &dir,
        Local::now().naive_local(),
        Utc::now(),
    )
    .await?;
    Ok(Some(backup.path.display().to_string()))
}
