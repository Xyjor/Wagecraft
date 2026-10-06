//! Report exports. Each needs `report.export` (Admin and HR), except saving a PDF the
//! UI built, which anyone signed in may do (plan §6.6).

use crate::auth::permissions::Permission;
use crate::domain::employee::EmployeeQuery;
use crate::error::AppError;
use crate::export::{files, pdf};
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

/// Saves a PDF the UI rendered, such as a payslip. Every role has `self.profile`, so this
/// only checks that someone is signed in: Rust can't tell whose payslip the bytes hold,
/// and the real ownership check is in the command that returned the payslip
/// (`payslip_my_get` for staff, `payslip_get` for HR). Returns where it was saved, or
/// nothing if the user cancelled.
#[tauri::command]
pub async fn export_save_pdf(
    app: AppHandle,
    state: State<'_, AppState>,
    file_name: String,
    bytes: Vec<u8>,
) -> Result<Option<String>, AppError> {
    state.require(Permission::SelfProfile).await?;
    let name = pdf::checked(&file_name, &bytes)?;
    let saved = files::save_as(&app, &name, ("PDF file", &["pdf"]), bytes).await?;
    Ok(saved.map(|p| p.display().to_string()))
}
