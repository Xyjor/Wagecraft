//! Payroll periods (plan §6.5). Creating and deleting periods needs `payroll.compute`.

use crate::auth::permissions::Permission;
use crate::domain::payroll_period::{
    PayrollPeriod, PayrollPeriodInput, PeriodChecks, RulePackSummary,
};
use crate::error::AppError;
use crate::services::payroll;
use crate::state::AppState;
use chrono::Utc;
use tauri::State;

#[tauri::command]
pub async fn payroll_period_list(
    state: State<'_, AppState>,
) -> Result<Vec<PayrollPeriod>, AppError> {
    state.require(Permission::PayrollCompute).await?;
    payroll::list(&state.db).await
}

#[tauri::command]
pub async fn rule_pack_list(state: State<'_, AppState>) -> Result<Vec<RulePackSummary>, AppError> {
    state.require(Permission::PayrollCompute).await?;
    payroll::rule_packs(&state.db).await
}

/// Unfinished leave, overtime and time-outs inside a period, shown before creating it.
#[tauri::command]
pub async fn payroll_period_checks(
    state: State<'_, AppState>,
    period_start: String,
) -> Result<PeriodChecks, AppError> {
    state.require(Permission::PayrollCompute).await?;
    payroll::checks(&state.db, &period_start).await
}

#[tauri::command]
pub async fn payroll_period_create(
    state: State<'_, AppState>,
    input: PayrollPeriodInput,
) -> Result<PayrollPeriod, AppError> {
    let session = state.require(Permission::PayrollCompute).await?;
    payroll::create(&state.db, session.actor(), input, Utc::now()).await
}

#[tauri::command]
pub async fn payroll_period_delete(state: State<'_, AppState>, id: i64) -> Result<(), AppError> {
    let session = state.require(Permission::PayrollCompute).await?;
    payroll::delete(&state.db, session.actor(), id, Utc::now()).await
}
