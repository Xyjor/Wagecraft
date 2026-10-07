//! Asks where to save a file with the system's Save dialog, then writes it. Also asks for
//! a folder or a file, for backups.

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

/// Shows the system's folder picker. Returns the folder, or `None` if the user cancelled.
pub async fn pick_folder(app: &AppHandle, title: &str) -> Result<Option<PathBuf>, AppError> {
    let dialog = app.dialog().file().set_title(title);
    let picked = tauri::async_runtime::spawn_blocking(move || dialog.blocking_pick_folder())
        .await
        .map_err(|e| AppError::Internal(e.into()))?;
    picked
        .map(|p| {
            p.into_path()
                .map_err(|e| AppError::Internal(anyhow::anyhow!("unusable folder: {e}")))
        })
        .transpose()
}

/// Shows the system's Open dialog. Returns the file, or `None` if the user cancelled.
pub async fn pick_file(
    app: &AppHandle,
    title: &str,
    filter: (&str, &[&str]),
) -> Result<Option<PathBuf>, AppError> {
    let dialog = app
        .dialog()
        .file()
        .set_title(title)
        .add_filter(filter.0, filter.1);
    let picked = tauri::async_runtime::spawn_blocking(move || dialog.blocking_pick_file())
        .await
        .map_err(|e| AppError::Internal(e.into()))?;
    picked
        .map(|p| {
            p.into_path()
                .map_err(|e| AppError::Internal(anyhow::anyhow!("unusable file: {e}")))
        })
        .transpose()
}

/// Reads at most `limit` bytes and one more, so a check on the size can't be fooled and a
/// huge file is never loaded whole.
pub async fn read_at_most(path: PathBuf, limit: usize) -> Result<Vec<u8>, AppError> {
    let read = tauri::async_runtime::spawn_blocking(move || {
        use std::io::Read;
        let mut bytes = Vec::new();
        std::fs::File::open(&path)?
            .take(limit as u64 + 1)
            .read_to_end(&mut bytes)?;
        Ok::<_, std::io::Error>(bytes)
    })
    .await
    .map_err(|e| AppError::Internal(e.into()))?;
    read.map_err(|e| {
        log::warn!("couldn't read the chosen file: {e}");
        AppError::Conflict("Couldn't open that file. Check it still exists and try again")
    })
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

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn reads_a_small_file_whole_and_only_the_start_of_a_big_one() {
        let dir = tempfile::tempdir().unwrap();
        let small = dir.path().join("small.png");
        let big = dir.path().join("big.png");
        std::fs::write(&small, b"12345").unwrap();
        std::fs::write(&big, vec![7u8; 1000]).unwrap();

        assert_eq!(read_at_most(small, 10).await.unwrap(), b"12345");
        assert_eq!(read_at_most(big, 10).await.unwrap(), vec![7u8; 11]);
    }

    #[tokio::test]
    async fn a_missing_file_is_a_plain_error() {
        let dir = tempfile::tempdir().unwrap();
        let r = read_at_most(dir.path().join("gone.png"), 10).await;
        assert!(matches!(r, Err(AppError::Conflict(_))), "{r:?}");
    }
}
