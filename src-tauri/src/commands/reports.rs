//! Report exports. Each needs `report.export` (Admin and HR).

use crate::auth::permissions::Permission;
use crate::domain::employee::EmployeeQuery;
use crate::error::AppError;
use crate::export::files;
use crate::services::reports;
use crate::state::AppState;
use chrono::{Local, Utc};
use tauri::{AppHandle, State};

/// Saves the employee masterlist for the Employees screen's current filters.
/// Returns where it was saved, or nothing if the user cancelled the Save dialog.
#[tauri::command]
pub async fn report_masterlist_csv(
    app: AppHandle,
    state: State<'_, AppState>,
    query: EmployeeQuery,
) -> Result<Option<String>, AppError> {
    let session = state.require(Permission::ReportExport).await?;
    let bytes = reports::masterlist_csv(&state.db, session.actor(), &query, Utc::now()).await?;
    let name = format!(
        "employee-masterlist-{}.csv",
        Local::now().format("%Y-%m-%d")
    );
    let saved = files::save_as(&app, &name, ("CSV file", &["csv"]), bytes).await?;
    Ok(saved.map(|p| p.display().to_string()))
}
