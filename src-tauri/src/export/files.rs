//! Asks where to save a file with the system's Save dialog, then writes it.

use crate::error::AppError;
use std::path::PathBuf;
use tauri::AppHandle;
use tauri_plugin_dialog::DialogExt;

/// Shows the Save dialog with `suggested_name` filled in. Returns where the file went,
/// or `None` if the user cancelled.
pub async fn save_as(
    app: &AppHandle,
    suggested_name: &str,
    filter: (&str, &[&str]),
    bytes: Vec<u8>,
) -> Result<Option<PathBuf>, AppError> {
    let dialog = app
        .dialog()
        .file()
        .set_file_name(suggested_name)
        .add_filter(filter.0, filter.1);
    // The dialog blocks until the user answers, so keep it off the async workers.
    let picked = tauri::async_runtime::spawn_blocking(move || dialog.blocking_save_file())
        .await
        .map_err(|e| AppError::Internal(e.into()))?;
    let Some(path) = picked else {
        return Ok(None);
    };
    let path = path
        .into_path()
        .map_err(|e| AppError::Internal(anyhow::anyhow!("unusable save location: {e}")))?;
    write_file(&path, bytes).await?;
    Ok(Some(path))
}

async fn write_file(path: &std::path::Path, bytes: Vec<u8>) -> Result<(), AppError> {
    let target = path.to_path_buf();
    let written = tauri::async_runtime::spawn_blocking(move || std::fs::write(target, bytes))
        .await
        .map_err(|e| AppError::Internal(e.into()))?;
    written.map_err(|e| {
        // Most often the file is open in Excel, which locks it on Windows.
        log::warn!("couldn't write {}: {e}", path.display());
        AppError::Conflict(
            "Couldn't save the file. If it's open in another program, close it and try again",
        )
    })
}
