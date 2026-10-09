//! The activity log (plan §6.8). Admin reads and exports it; nothing edits it.

use crate::auth::permissions::Permission;
use crate::domain::audit::{AuditFilters, AuditPage, AuditQuery};
use crate::error::AppError;
use crate::export::files;
use crate::services::audit;
use crate::state::AppState;
use chrono::{FixedOffset, Local, Utc};
use tauri::{AppHandle, State};

/// The office's clock is the PC's clock (ADR-003), so its UTC offset is the PC's.
fn office_offset() -> FixedOffset {
    *Local::now().offset()
}

#[tauri::command]
pub async fn audit_list(
    state: State<'_, AppState>,
    query: AuditQuery,
) -> Result<AuditPage, AppError> {
    state.require(Permission::AuditRead).await?;
    audit::list(&state.db, &query, office_offset()).await
}

#[tauri::command]
pub async fn audit_filters(state: State<'_, AppState>) -> Result<AuditFilters, AppError> {
    state.require(Permission::AuditRead).await?;
    audit::filters(&state.db).await
}

/// Saves the filtered log as CSV. Returns where it went, or nothing if the user cancelled
/// the Save dialog.
#[tauri::command]
pub async fn audit_export_csv(
    app: AppHandle,
    state: State<'_, AppState>,
    query: AuditQuery,
) -> Result<Option<String>, AppError> {
    let session = state.require(Permission::AuditRead).await?;
    let bytes = audit::export_csv(
        &state.db,
        session.actor(),
        &query,
        Utc::now(),
        office_offset(),
    )
    .await?;
    let name = format!("activity-log-{}.csv", Local::now().format("%Y-%m-%d"));
    let saved = files::save_as(&app, &name, ("CSV file", &["csv"]), bytes).await?;
    Ok(saved.map(|p| p.display().to_string()))
}
