//! The Backups screen (plan §6.7). Admin only: `backup.manage`.

use crate::auth::permissions::Permission;
use crate::db;
use crate::domain::backup::{BackupEntry, RestorePreview};
use crate::error::AppError;
use crate::export::files;
use crate::services::{backups, restore};
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

/// Step one of a restore: Admin picks a backup file, and Rust checks it and says what it
/// holds. Nothing changes yet. Returns nothing if Admin cancelled the Open dialog.
#[tauri::command]
pub async fn backup_restore_choose(
    app: AppHandle,
    state: State<'_, AppState>,
) -> Result<Option<RestorePreview>, AppError> {
    state.require(Permission::BackupManage).await?;
    let filter = ("Wagecraft backup", &["db"][..]);
    let Some(path) = files::pick_file(&app, "Choose a backup to restore", filter).await? else {
        return Ok(None);
    };
    let preview = restore::inspect(&path, db::newest_migration()).await?;
    *pending(&state) = Some(path);
    Ok(Some(preview))
}

/// Step two, once Admin typed RESTORE: backs up the current data, swaps in the chosen
/// backup and restarts the app. Everyone signs in again afterwards.
#[tauri::command]
pub async fn backup_restore(
    app: AppHandle,
    state: State<'_, AppState>,
    confirm: String,
) -> Result<(), AppError> {
    let session = state.require(Permission::BackupManage).await?;
    if confirm != "RESTORE" {
        return Err(AppError::Conflict("Type RESTORE to confirm"));
    }
    let Some(path) = pending(&state).take() else {
        return Err(AppError::Conflict("Choose a backup to restore first"));
    };
    let done = restore::run(
        &state.db,
        &state.data_dir,
        &state.backup_dir().await?,
        &path,
        db::newest_migration(),
        session.actor(),
        Local::now().naive_local(),
        Utc::now(),
    )
    .await;
    match done {
        Ok(()) => {}
        // Refused before anything changed: the app carries on as it was, and Admin can try
        // the same file again.
        Err(e) if !state.db.is_closed() => {
            *pending(&state) = Some(path);
            return Err(e);
        }
        // The pool is closed. The old database is still in place, so a restart reopens it.
        Err(e) => log::error!("restore failed after closing the database: {e}"),
    }
    app.restart()
}

fn pending(state: &AppState) -> std::sync::MutexGuard<'_, Option<std::path::PathBuf>> {
    // A panic elsewhere can't leave this in a bad state, so a poisoned lock is still usable.
    state
        .pending_restore
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner())
}
